use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::error::{MemoryError, MemoryResult};
use crate::types::{MemoryEntry, MemoryId, MemoryTier, MemoryStatus};

/// Snapshot of long-term memory state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    /// Unique identifier.
    pub id: uuid::Uuid,
    /// When the snapshot was taken.
    pub timestamp: DateTime<Utc>,
    /// Number of entries in each tier.
    pub tier_counts: HashMap<String, u64>,
    /// Total entries.
    pub total_entries: u64,
    /// Human-readable description.
    pub description: Option<String>,
    /// Size of the snapshot in bytes (approximate).
    pub size_bytes: u64,
}

/// Compression record for tracking compression operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionRecord {
    /// When compression was performed.
    pub timestamp: DateTime<Utc>,
    /// Number of entries compressed.
    pub entries_compressed: u64,
    /// Original size in bytes.
    pub original_bytes: u64,
    /// Compressed size in bytes.
    pub compressed_bytes: u64,
}

impl CompressionRecord {
    /// Compression ratio (compressed / original).
    #[must_use]
    pub fn ratio(&self) -> f64 {
        if self.original_bytes == 0 {
            1.0
        } else {
            self.compressed_bytes as f64 / self.original_bytes as f64
        }
    }
}

/// Configuration for long-term memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongTermMemoryConfig {
    /// Path for sled persistence.
    pub sled_path: Option<String>,
    /// Whether to enable automatic indexing.
    pub auto_index: bool,
    /// Automatic compression threshold (entries above this get compressed).
    pub auto_compression_threshold: u64,
    /// Maximum snapshots to retain.
    pub max_snapshots: usize,
    /// Snapshot interval in seconds.
    pub snapshot_interval_secs: u64,
    /// Maximum total entries before migration is triggered.
    pub max_entries: u64,
}

impl Default for LongTermMemoryConfig {
    fn default() -> Self {
        Self {
            sled_path: None,
            auto_index: true,
            auto_compression_threshold: 10_000,
            max_snapshots: 10,
            snapshot_interval_secs: 3600,
            max_entries: 1_000_000,
        }
    }
}

/// Long-term persistent memory store with indexing, compression, snapshots, and recovery.
#[derive(Debug)]
pub struct LongTermMemory {
    /// Main storage.
    store: DashMap<MemoryId, MemoryEntry>,
    /// Index by tier.
    tier_index: DashMap<MemoryTier, Vec<MemoryId>>,
    /// Index by namespace.
    namespace_index: DashMap<String, Vec<MemoryId>>,
    /// Snapshots.
    snapshots: RwLock<Vec<MemorySnapshot>>,
    /// Compression records.
    compression_records: RwLock<Vec<CompressionRecord>>,
    /// Sled DB for persistence.
    db: Option<sled::Db>,
    /// Configuration.
    config: LongTermMemoryConfig,
    /// Total entries counter.
    total_entries: AtomicU64,
    /// Total bytes stored.
    total_bytes: AtomicU64,
}

impl LongTermMemory {
    /// Create a new long-term memory store.
    pub fn new(config: LongTermMemoryConfig) -> MemoryResult<Self> {
        let db = if let Some(ref path) = config.sled_path {
            Some(
                sled::open(path)
                    .map_err(|e| MemoryError::PersistenceError(e.to_string()))?,
            )
        } else {
            None
        };

        Ok(Self {
            store: DashMap::new(),
            tier_index: DashMap::new(),
            namespace_index: DashMap::new(),
            snapshots: RwLock::new(Vec::new()),
            compression_records: RwLock::new(Vec::new()),
            db,
            config,
            total_entries: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
        })
    }

    /// Store a memory entry in long-term storage.
    pub fn store(&self, entry: MemoryEntry) -> MemoryResult<MemoryId> {
        if self.total_entries.load(Ordering::SeqCst) >= self.config.max_entries {
            return Err(MemoryError::CapacityExceeded(
                "Long-term memory capacity reached".to_string(),
            ));
        }

        let id = entry.id;
        let tier = entry.tier;
        let ns = entry.namespace.0.clone();

        // Persist to sled DB.
        if let Some(ref db) = self.db {
            let key = id.0.as_bytes().to_vec();
            let value = serde_json::to_vec(&entry)
                .map_err(|e| MemoryError::SerializationError(e.to_string()))?;
            let est_size = value.len() as u64;
            db.insert(key, value)
                .map_err(|e| MemoryError::PersistenceError(e.to_string()))?;
            self.total_bytes.fetch_add(est_size, Ordering::SeqCst);
        }

        // Update indexes.
        self.tier_index
            .entry(tier)
            .or_default()
            .push(id);
        self.namespace_index
            .entry(ns)
            .or_default()
            .push(id);

        self.store.insert(id, entry);
        self.total_entries.fetch_add(1, Ordering::SeqCst);

        Ok(id)
    }

    /// Retrieve a memory entry by id.
    pub fn get(&self, id: MemoryId) -> Option<MemoryEntry> {
        self.store.get(&id).map(|e| {
            let mut entry = e.value().clone();
            entry.access();
            Some(entry)
        })?
    }

    /// Update a memory entry.
    pub fn update(&self, id: MemoryId, updater: impl FnOnce(&mut MemoryEntry)) -> MemoryResult<()> {
        let mut entry = self
            .store
            .get_mut(&id)
            .ok_or_else(|| MemoryError::NotFound(format!("Memory {id} not found")))?;

        updater(&mut entry);
        entry.touch_modified();

        // Persist update.
        if let Some(ref db) = self.db {
            let key = id.0.as_bytes().to_vec();
            let value = serde_json::to_vec(entry.value())
                .map_err(|e| MemoryError::SerializationError(e.to_string()))?;
            db.insert(key, value)
                .map_err(|e| MemoryError::PersistenceError(e.to_string()))?;
        }

        Ok(())
    }

    /// Delete a memory entry.
    pub fn delete(&self, id: MemoryId) -> MemoryResult<bool> {
        if let Some((_, mut entry)) = self.store.remove(&id) {
            entry.mark_deleted();

            // Remove from indexes.
            if let Some(mut ids) = self.tier_index.get_mut(&entry.tier) {
                ids.retain(|&x| x != id);
            }
            if let Some(mut ids) = self.namespace_index.get_mut(&entry.namespace.0) {
                ids.retain(|&x| x != id);
            }

            // Remove from sled DB.
            if let Some(ref db) = self.db {
                let key = id.0.as_bytes().to_vec();
                let _ = db.remove(key);
            }

            self.total_entries.fetch_sub(1, Ordering::SeqCst);
            return Ok(true);
        }
        Ok(false)
    }

    /// Get all entries in a specific tier.
    #[must_use]
    pub fn entries_by_tier(&self, tier: MemoryTier) -> Vec<MemoryEntry> {
        self.tier_index
            .get(&tier)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.store.get(id).map(|e| e.value().clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all entries in a specific namespace.
    #[must_use]
    pub fn entries_by_namespace(&self, namespace: &str) -> Vec<MemoryEntry> {
        self.namespace_index
            .get(namespace)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.store.get(id).map(|e| e.value().clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get active entries (not deleted or expired).
    #[must_use]
    pub fn active_entries(&self) -> Vec<MemoryEntry> {
        self.store
            .iter()
            .filter(|e| e.value().is_active())
            .map(|e| e.value().clone())
            .collect()
    }

    /// Compress all entries older than the threshold age.
    pub fn compress_old_entries(&self, older_than_secs: u64) -> MemoryResult<CompressionRecord> {
        let cutoff = Utc::now() - chrono::Duration::seconds(older_than_secs as i64);
        let mut entries_compressed = 0u64;
        let mut original_bytes = 0u64;
        let mut compressed_bytes = 0u64;

        for mut entry in self.store.iter_mut() {
            if entry.value().created_at < cutoff && entry.value().status == MemoryStatus::Active {
                let content_str = entry.value().content.to_string();
                original_bytes += content_str.len() as u64;

                // Simple compression: mark as compressed and store a summary.
                entry.value_mut().mark_compressed();
                compressed_bytes += content_str.len() as u64 / 3; // Approximate
                entries_compressed += 1;
            }
        }

        let record = CompressionRecord {
            timestamp: Utc::now(),
            entries_compressed,
            original_bytes,
            compressed_bytes,
        };

        self.compression_records.write().push(record.clone());

        Ok(record)
    }

    /// Take a snapshot of the current memory state.
    pub fn take_snapshot(&self, description: Option<String>) -> MemoryResult<MemorySnapshot> {
        let mut tier_counts = HashMap::new();
        for entry in self.store.iter() {
            let tier_name = entry.value().tier.to_string();
            *tier_counts.entry(tier_name).or_insert(0u64) += 1;
        }

        let snapshot = MemorySnapshot {
            id: uuid::Uuid::new_v4(),
            timestamp: Utc::now(),
            total_entries: self.total_entries.load(Ordering::SeqCst),
            tier_counts,
            description,
            size_bytes: self.total_bytes.load(Ordering::SeqCst),
        };

        let mut snapshots = self.snapshots.write();
        snapshots.push(snapshot.clone());
        if snapshots.len() > self.config.max_snapshots {
            snapshots.remove(0);
        }

        Ok(snapshot)
    }

    /// Get all snapshots.
    #[must_use]
    pub fn snapshots(&self) -> Vec<MemorySnapshot> {
        self.snapshots.read().clone()
    }

    /// Get compression history.
    #[must_use]
    pub fn compression_history(&self) -> Vec<CompressionRecord> {
        self.compression_records.read().clone()
    }

    /// Migrate entries from one tier to another.
    pub fn migrate_tier(
        &self,
        from: MemoryTier,
        to: MemoryTier,
        limit: usize,
    ) -> MemoryResult<u64> {
        let mut migrated = 0u64;

        let ids_to_migrate: Vec<MemoryId> = {
            self.tier_index
                .get(&from)
                .map(|ids| ids.iter().take(limit).copied().collect())
                .unwrap_or_default()
        };

        for id in ids_to_migrate {
            if let Some(mut entry) = self.store.get_mut(&id) {
                entry.value_mut().tier = to;
                entry.value_mut().touch_modified();
                entry.value_mut().consolidated = true;

                // Persist.
                if let Some(ref db) = self.db {
                    let key = id.0.as_bytes().to_vec();
                    if let Ok(value) = serde_json::to_vec(entry.value()) {
                        let _ = db.insert(key, value);
                    }
                }

                migrated += 1;
            }
        }

        // Rebuild tier indexes.
        if migrated > 0 {
            self.rebuild_tier_index()?;
        }

        Ok(migrated)
    }

    /// Rebuild tier index from store.
    fn rebuild_tier_index(&self) -> MemoryResult<()> {
        self.tier_index.clear();
        for entry in self.store.iter() {
            let tier = entry.value().tier;
            let id = entry.value().id;
            self.tier_index.entry(tier).or_default().push(id);
        }
        Ok(())
    }

    /// Recover from sled DB on startup.
    pub fn recover(&self) -> MemoryResult<u64> {
        let db = match self.db {
            Some(ref db) => db,
            None => return Ok(0),
        };

        let mut recovered = 0u64;
        for item in db.iter() {
            let (key, value) = item.map_err(|e| MemoryError::PersistenceError(e.to_string()))?;

            // Skip non-entry keys.
            if key.len() != 16 {
                continue;
            }

            if let Ok(entry) = serde_json::from_slice::<MemoryEntry>(&value) {
                let id = entry.id;
                let tier = entry.tier;
                let ns = entry.namespace.0.clone();

                self.tier_index.entry(tier).or_default().push(id);
                self.namespace_index.entry(ns).or_default().push(id);
                self.store.insert(id, entry);
                self.total_entries.fetch_add(1, Ordering::SeqCst);
                recovered += 1;
            }
        }

        Ok(recovered)
    }

    /// Backup all entries to a JSON file.
    pub fn backup(&self, path: &str) -> MemoryResult<()> {
        let entries: Vec<MemoryEntry> = self.store.iter().map(|e| e.value().clone()).collect();
        let json = serde_json::to_vec_pretty(&entries)
            .map_err(|e| MemoryError::SerializationError(e.to_string()))?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Restore entries from a JSON file.
    pub fn restore(&self, path: &str) -> MemoryResult<u64> {
        let data = std::fs::read(path)?;
        let entries: Vec<MemoryEntry> = serde_json::from_slice(&data)
            .map_err(|e| MemoryError::SerializationError(e.to_string()))?;

        let mut restored = 0u64;
        for entry in entries {
            self.store(entry)?;
            restored += 1;
        }

        Ok(restored)
    }

    /// Total number of entries.
    #[must_use]
    pub fn count(&self) -> u64 {
        self.total_entries.load(Ordering::SeqCst)
    }

    /// Total bytes stored.
    #[must_use]
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn make_entry(tier: MemoryTier) -> MemoryEntry {
        MemoryEntry::new(tier, serde_json::json!("data"), HashSet::new())
    }

    #[test]
    fn store_and_retrieve() {
        let ltm = LongTermMemory::new(LongTermMemoryConfig::default()).unwrap();
        let entry = make_entry(MemoryTier::LongTerm);
        let id = entry.id;
        ltm.store(entry).unwrap();

        let retrieved = ltm.get(id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, id);
    }

    #[test]
    fn tier_index() {
        let ltm = LongTermMemory::new(LongTermMemoryConfig::default()).unwrap();
        ltm.store(make_entry(MemoryTier::Episodic)).unwrap();
        ltm.store(make_entry(MemoryTier::Episodic)).unwrap();
        ltm.store(make_entry(MemoryTier::Semantic)).unwrap();

        assert_eq!(ltm.entries_by_tier(MemoryTier::Episodic).len(), 2);
        assert_eq!(ltm.entries_by_tier(MemoryTier::Semantic).len(), 1);
    }

    #[test]
    fn delete_entry() {
        let ltm = LongTermMemory::new(LongTermMemoryConfig::default()).unwrap();
        let entry = make_entry(MemoryTier::LongTerm);
        let id = entry.id;
        ltm.store(entry).unwrap();

        assert!(ltm.delete(id).unwrap());
        assert!(ltm.get(id).is_none());
        assert_eq!(ltm.count(), 0);
    }

    #[test]
    fn snapshot() {
        let ltm = LongTermMemory::new(LongTermMemoryConfig::default()).unwrap();
        ltm.store(make_entry(MemoryTier::LongTerm)).unwrap();

        let snap = ltm.take_snapshot(Some("test".to_string())).unwrap();
        assert_eq!(snap.total_entries, 1);
        assert_eq!(ltm.snapshots().len(), 1);
    }

    #[test]
    fn backup_and_restore() {
        let ltm = LongTermMemory::new(LongTermMemoryConfig::default()).unwrap();
        ltm.store(make_entry(MemoryTier::LongTerm)).unwrap();

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("backup.json");
        let path_str = path.to_str().unwrap();
        ltm.backup(path_str).unwrap();

        let ltm2 = LongTermMemory::new(LongTermMemoryConfig::default()).unwrap();
        let restored = ltm2.restore(path_str).unwrap();
        assert_eq!(restored, 1);
    }

    #[test]
    fn migrate_tier() {
        let ltm = LongTermMemory::new(LongTermMemoryConfig::default()).unwrap();
        let entry = make_entry(MemoryTier::Working);
        let id = entry.id;
        ltm.store(entry).unwrap();

        let migrated = ltm.migrate_tier(MemoryTier::Working, MemoryTier::LongTerm, 10).unwrap();
        assert_eq!(migrated, 1);

        let entry = ltm.get(id).unwrap();
        assert_eq!(entry.tier, MemoryTier::LongTerm);
    }

    #[test]
    fn namespace_index() {
        let ltm = LongTermMemory::new(LongTermMemoryConfig::default()).unwrap();
        let mut entry = make_entry(MemoryTier::LongTerm);
        entry.namespace = crate::types::MemoryNamespace::new("project_a");
        ltm.store(entry).unwrap();

        let results = ltm.entries_by_namespace("project_a");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn capacity_limit() {
        let config = LongTermMemoryConfig {
            max_entries: 2,
            ..LongTermMemoryConfig::default()
        };
        let ltm = LongTermMemory::new(config).unwrap();
        ltm.store(make_entry(MemoryTier::LongTerm)).unwrap();
        ltm.store(make_entry(MemoryTier::LongTerm)).unwrap();
        let result = ltm.store(make_entry(MemoryTier::LongTerm));
        assert!(result.is_err());
    }
}
