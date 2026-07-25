//! Distributed memory — replication, sharding, caching, snapshots, and
//! consistency modes for cluster-wide shared memory.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::{ConsistencyMode, MemoryConfiguration};
use std::sync::atomic::AtomicPtr;

use crate::error::{DistributedError, NeoResult};
use crate::types::NodeId;

// ---------------------------------------------------------------------------
// MemoryPartition
// ---------------------------------------------------------------------------

/// A shard/partition of the distributed memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPartition {
    /// Partition identifier.
    pub id: Uuid,
    /// Partition index.
    pub index: usize,
    /// Node responsible for this partition.
    pub primary_node: NodeId,
    /// Replica nodes.
    pub replica_nodes: Vec<NodeId>,
    /// Key count.
    pub key_count: usize,
    /// Size in bytes.
    pub size_bytes: u64,
}

// ---------------------------------------------------------------------------
// MemoryEntry
// ---------------------------------------------------------------------------

/// A single entry in distributed memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Key.
    pub key: String,
    /// Serialized value.
    pub value: Vec<u8>,
    /// Version counter (for conflict resolution).
    pub version: u64,
    /// When the entry was created.
    pub created_at: DateTime<Utc>,
    /// When the entry was last modified.
    pub modified_at: DateTime<Utc>,
    /// Time-to-live (optional).
    pub ttl: Option<u64>,
    /// Node that wrote this entry.
    pub writer: NodeId,
}

impl MemoryEntry {
    /// Check if the entry has expired.
    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl {
            let elapsed = Utc::now()
                .signed_duration_since(self.modified_at)
                .num_seconds() as u64;
            elapsed > ttl
        } else {
            false
        }
    }
}

// ---------------------------------------------------------------------------
// MemoryReplication
// ---------------------------------------------------------------------------

/// Manages replication of memory entries across nodes.
pub struct MemoryReplication {
    /// Replication factor.
    factor: usize,
    /// Per-key replica assignments.
    replicas: RwLock<HashMap<String, Vec<NodeId>>>,
    /// Replication lag per node (key → version difference).
    lag: RwLock<HashMap<NodeId, u64>>,
}

impl MemoryReplication {
    pub fn new(factor: usize) -> Self {
        Self {
            factor,
            replicas: RwLock::new(HashMap::new()),
            lag: RwLock::new(HashMap::new()),
        }
    }

    /// Get replica nodes for a key.
    pub fn replicas_for(&self, key: &str) -> Vec<NodeId> {
        self.replicas
            .read()
            .get(key)
            .cloned()
            .unwrap_or_default()
    }

    /// Assign replicas for a key.
    pub fn assign_replicas(&self, key: String, nodes: Vec<NodeId>) {
        self.replicas.write().insert(key, nodes);
    }

    /// Replication factor.
    pub fn factor(&self) -> usize {
        self.factor
    }

    /// Check if we have enough replicas for a key.
    pub fn has_quorum(&self, key: &str) -> bool {
        self.replicas_for(key).len() >= self.factor / 2 + 1
    }

    /// Record replication lag for a node.
    pub fn record_lag(&self, node_id: NodeId, lag: u64) {
        self.lag.write().insert(node_id, lag);
    }

    /// Get replication lag for a node.
    pub fn get_lag(&self, node_id: NodeId) -> u64 {
        self.lag.read().get(&node_id).copied().unwrap_or(0)
    }
}

// ---------------------------------------------------------------------------
// MemoryCache
// ---------------------------------------------------------------------------

/// Local cache for frequently accessed distributed memory entries.
pub struct MemoryCache {
    entries: DashMap<String, CacheEntry>,
    max_size: usize,
    hits: std::sync::atomic::AtomicU64,
    misses: std::sync::atomic::AtomicU64,
}

#[derive(Debug)]
struct CacheEntry {
    value: Vec<u8>,
    version: u64,
    inserted_at: std::time::Instant,
    ttl: Option<std::time::Duration>,
}

impl MemoryCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: DashMap::new(),
            max_size,
            hits: std::sync::atomic::AtomicU64::new(0),
            misses: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Get a cached value.
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        if let Some(entry) = self.entries.get(key) {
            // Check TTL.
            if let Some(ttl) = entry.ttl {
                if entry.inserted_at.elapsed() > ttl {
                    drop(entry);
                    self.entries.remove(key);
                    self.misses
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    return None;
                }
            }
            self.hits
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Some(entry.value.clone())
        } else {
            self.misses
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            None
        }
    }

    /// Insert a value into the cache.
    pub fn insert(&self, key: String, value: Vec<u8>, version: u64, ttl: Option<std::time::Duration>) {
        if self.entries.len() >= self.max_size {
            // Evict oldest entries.
            let mut to_evict = Vec::new();
            for entry in self.entries.iter() {
                if entry.inserted_at.elapsed() > std::time::Duration::from_secs(60) {
                    to_evict.push(entry.key().clone());
                }
            }
            for key in to_evict {
                self.entries.remove(&key);
            }
        }

        self.entries.insert(
            key,
            CacheEntry {
                value,
                version,
                inserted_at: std::time::Instant::now(),
                ttl,
            },
        );
    }

    /// Invalidate a cached key.
    pub fn invalidate(&self, key: &str) -> bool {
        self.entries.remove(key).is_some()
    }

    /// Cache hit ratio.
    pub fn hit_ratio(&self) -> f64 {
        let h = self.hits.load(std::sync::atomic::Ordering::Relaxed) as f64;
        let m = self.misses.load(std::sync::atomic::Ordering::Relaxed) as f64;
        let total = h + m;
        if total > 0.0 {
            h / total
        } else {
            0.0
        }
    }

    /// Current cache size.
    pub fn size(&self) -> usize {
        self.entries.len()
    }

    /// Clear the cache.
    pub fn clear(&self) {
        self.entries.clear();
    }
}

// ---------------------------------------------------------------------------
// MemorySnapshot
// ---------------------------------------------------------------------------

/// A snapshot of distributed memory state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    /// Snapshot identifier.
    pub id: Uuid,
    /// When the snapshot was taken.
    pub timestamp: DateTime<Utc>,
    /// Number of entries.
    pub entry_count: usize,
    /// Total size in bytes.
    pub size_bytes: u64,
    /// Snapshot version.
    pub version: u64,
}

// ---------------------------------------------------------------------------
// MemoryConsistency
// ---------------------------------------------------------------------------

/// Enforces consistency modes for distributed memory operations.
pub struct MemoryConsistency {
    mode: ConsistencyMode,
    /// Pending writes awaiting acknowledgment.
    pending_writes: RwLock<HashMap<String, Vec<NodeId>>>,
    /// Last known version per key.
    versions: RwLock<HashMap<String, u64>>,
}

impl MemoryConsistency {
    pub fn new(mode: ConsistencyMode) -> Self {
        Self {
            mode,
            pending_writes: RwLock::new(HashMap::new()),
            versions: RwLock::new(HashMap::new()),
        }
    }

    /// Get the consistency mode.
    pub fn mode(&self) -> ConsistencyMode {
        self.mode
    }

    /// Check if a read is consistent.
    pub fn is_read_consistent(&self, key: &str, version: u64) -> bool {
        match self.mode {
            ConsistencyMode::Strong => {
                // For strong consistency, version must be latest.
                let latest = self.versions.read().get(key).copied().unwrap_or(0);
                version >= latest
            }
            ConsistencyMode::Eventual => true, // Always consistent for eventual.
            ConsistencyMode::Quorum => {
                // Check if quorum of replicas have acknowledged.
                let pending = self.pending_writes.read();
                if let Some(replicas) = pending.get(key) {
                    replicas.len() >= 2 // Simplified quorum check.
                } else {
                    true
                }
            }
        }
    }

    /// Record a write.
    pub fn record_write(&self, key: &str, version: u64) {
        self.versions
            .write()
            .insert(key.to_string(), version);
    }

    /// Record acknowledgment from a replica.
    pub fn record_ack(&self, key: &str, node_id: NodeId) {
        self.pending_writes
            .write()
            .entry(key.to_string())
            .or_default()
            .push(node_id);
    }

    /// Clear acknowledgments for a key.
    pub fn clear_acks(&self, key: &str) {
        self.pending_writes.write().remove(key);
    }
}

// ---------------------------------------------------------------------------
// DistributedMemory
// ---------------------------------------------------------------------------

/// The main distributed memory system.
pub struct DistributedMemory {
    /// Configuration.
    config: RwLock<MemoryConfiguration>,
    /// Local partition of data.
    data: DashMap<String, MemoryEntry>,
    /// Replication manager.
    replication: Arc<MemoryReplication>,
    /// Local cache.
    cache: Arc<MemoryCache>,
    /// Consistency enforcer.
    consistency: Arc<MemoryConsistency>,
    /// Partitions.
    partitions: RwLock<Vec<MemoryPartition>>,
    /// Snapshots.
    snapshots: RwLock<Vec<MemorySnapshot>>,
    /// Total reads.
    reads: std::sync::atomic::AtomicU64,
    /// Total writes.
    writes: std::sync::atomic::AtomicU64,
}

impl DistributedMemory {
    /// Create a new distributed memory system.
    pub fn new(config: MemoryConfiguration) -> Self {
        let replication = Arc::new(MemoryReplication::new(config.replication_factor));
        let cache = Arc::new(MemoryCache::new(config.cache_size_bytes / 1024));
        let consistency = Arc::new(MemoryConsistency::new(config.consistency));

        tracing::info!(
            replication_factor = config.replication_factor,
            consistency = ?config.consistency,
            max_partitions = config.max_partitions,
            "distributed memory created"
        );

        Self {
            config: RwLock::new(config),
            data: DashMap::new(),
            replication,
            cache,
            consistency,
            partitions: RwLock::new(Vec::new()),
            snapshots: RwLock::new(Vec::new()),
            reads: std::sync::atomic::AtomicU64::new(0),
            writes: std::sync::atomic::AtomicU64::new(0),
        }
    }

    // -- Data operations --

    /// Get a value by key.
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        // Check cache first.
        if let Some(value) = self.cache.get(key) {
            self.reads
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return Some(value);
        }

        // Check local data.
        let result = self.data.get(key).map(|entry| {
            if entry.is_expired() {
                None
            } else {
                Some(entry.value.clone())
            }
        }).flatten();

        if let Some(value) = &result {
            self.cache
                .insert(key.to_string(), value.clone(), 1, None);
        }

        self.reads
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        result
    }

    /// Set a value.
    pub fn set(&self, key: String, value: Vec<u8>, writer: NodeId) -> NeoResult<()> {
        let now = Utc::now();
        let version = self.data.get(&key).map_or(1, |e| e.version + 1);

        let entry = MemoryEntry {
            key: key.clone(),
            value: value.clone(),
            version,
            created_at: now,
            modified_at: now,
            ttl: None,
            writer,
        };

        self.data.insert(key.clone(), entry);
        self.cache.insert(key.clone(), value, version, None);
        self.consistency.record_write(&key, version);

        self.writes
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        tracing::debug!(key = %key, version = version, "memory set");
        Ok(())
    }

    /// Set a value with TTL.
    pub fn set_with_ttl(
        &self,
        key: String,
        value: Vec<u8>,
        writer: NodeId,
        ttl_secs: u64,
    ) -> NeoResult<()> {
        let now = Utc::now();
        let version = self.data.get(&key).map_or(1, |e| e.version + 1);

        let entry = MemoryEntry {
            key: key.clone(),
            value: value.clone(),
            version,
            created_at: now,
            modified_at: now,
            ttl: Some(ttl_secs),
            writer,
        };

        self.data.insert(key.clone(), entry);
        self.cache.insert(
            key.clone(),
            value,
            version,
            Some(std::time::Duration::from_secs(ttl_secs)),
        );
        self.consistency.record_write(&key, version);

        self.writes
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        tracing::debug!(key = %key, version = version, ttl = ttl_secs, "memory set with TTL");
        Ok(())
    }

    /// Delete a value.
    pub fn delete(&self, key: &str) -> bool {
        let removed = self.data.remove(key).is_some();
        self.cache.invalidate(key);
        if removed {
            tracing::debug!(key = %key, "memory delete");
        }
        removed
    }

    /// Check if a key exists.
    pub fn contains_key(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    /// Get all keys.
    pub fn keys(&self) -> Vec<String> {
        self.data.iter().map(|r| r.key().clone()).collect()
    }

    /// Entry count.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    // -- Partitioning --

    /// Create partitions across nodes.
    pub fn create_partitions(&self, nodes: Vec<NodeId>) -> NeoResult<()> {
        let config = self.config.read();
        let count = nodes.len().min(config.max_partitions);
        let mut partitions = Vec::new();

        for (i, &node_id) in nodes.iter().take(count).enumerate() {
            partitions.push(MemoryPartition {
                id: Uuid::new_v4(),
                index: i,
                primary_node: node_id,
                replica_nodes: Vec::new(),
                key_count: 0,
                size_bytes: 0,
            });
        }

        *self.partitions.write() = partitions;
        tracing::info!(count = count, "memory partitions created");
        Ok(())
    }

    // -- Snapshots --

    /// Create a snapshot of the current state.
    pub fn snapshot(&self) -> MemorySnapshot {
        let snap = MemorySnapshot {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            entry_count: self.data.len(),
            size_bytes: 0, // Would compute actual size.
            version: self.writes.load(std::sync::atomic::Ordering::Relaxed),
        };

        self.snapshots.write().push(snap.clone());
        tracing::info!(
            snapshot_id = %snap.id,
            entry_count = snap.entry_count,
            "memory snapshot created"
        );
        snap
    }

    /// Get all snapshots.
    pub fn snapshots(&self) -> Vec<MemorySnapshot> {
        self.snapshots.read().clone()
    }

    // -- Queries --

    /// Get the replication manager.
    pub fn replication(&self) -> &Arc<MemoryReplication> {
        &self.replication
    }

    /// Get the consistency enforcer.
    pub fn consistency(&self) -> &Arc<MemoryConsistency> {
        &self.consistency
    }

    /// Get the cache.
    pub fn cache(&self) -> &Arc<MemoryCache> {
        &self.cache
    }

    /// Get memory statistics.
    pub fn stats(&self) -> MemoryStats {
        MemoryStats {
            total_entries: self.data.len(),
            total_reads: self.reads.load(std::sync::atomic::Ordering::Relaxed),
            total_writes: self.writes.load(std::sync::atomic::Ordering::Relaxed),
            cache_size: self.cache.size(),
            cache_hit_ratio: self.cache.hit_ratio(),
            partitions: self.partitions.read().len(),
            snapshots: self.snapshots.read().len(),
        }
    }

    /// Cleanup expired entries.
    pub fn cleanup_expired(&self) -> usize {
        let mut removed = 0;
        self.data.retain(|_, entry| {
            if entry.is_expired() {
                removed += 1;
                false
            } else {
                true
            }
        });
        if removed > 0 {
            tracing::info!(count = removed, "expired memory entries cleaned up");
        }
        removed
    }
}

impl std::fmt::Debug for DistributedMemory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DistributedMemory")
            .field("entries", &self.data.len())
            .field("partitions", &self.partitions.read().len())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// MemoryStats
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_entries: usize,
    pub total_reads: u64,
    pub total_writes: u64,
    pub cache_size: usize,
    pub cache_hit_ratio: f64,
    pub partitions: usize,
    pub snapshots: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::NodeId;

    #[test]
    fn distributed_memory_basic() {
        let mem = DistributedMemory::new(MemoryConfiguration::default());
        let writer = NodeId::new();

        mem.set("key1".to_string(), vec![1, 2, 3], writer).unwrap();
        assert_eq!(mem.get("key1"), Some(vec![1, 2, 3]));
        assert!(mem.contains_key("key1"));
        assert_eq!(mem.len(), 1);
    }

    #[test]
    fn distributed_memory_delete() {
        let mem = DistributedMemory::new(MemoryConfiguration::default());
        mem.set("k".to_string(), vec![1], NodeId::new()).unwrap();
        assert!(mem.delete("k"));
        assert!(!mem.contains_key("k"));
    }

    #[test]
    fn distributed_memory_ttl() {
        let mem = DistributedMemory::new(MemoryConfiguration::default());
        mem.set_with_ttl("k".to_string(), vec![1], NodeId::new(), 1).unwrap();
        assert!(mem.get("k").is_some());
    }

    #[test]
    fn memory_snapshot() {
        let mem = DistributedMemory::new(MemoryConfiguration::default());
        mem.set("k".to_string(), vec![1], NodeId::new()).unwrap();
        let snap = mem.snapshot();
        assert_eq!(snap.entry_count, 1);
        assert_eq!(mem.snapshots().len(), 1);
    }

    #[test]
    fn memory_stats() {
        let mem = DistributedMemory::new(MemoryConfiguration::default());
        let stats = mem.stats();
        assert_eq!(stats.total_entries, 0);
    }

    #[test]
    fn memory_cache() {
        let cache = MemoryCache::new(100);
        cache.insert("k".to_string(), vec![1, 2], 1, None);
        assert_eq!(cache.get("k"), Some(vec![1, 2]));
        assert_eq!(cache.size(), 1);
    }

    #[test]
    fn replication_quorum() {
        let repl = MemoryReplication::new(3);
        repl.assign_replicas("key1".to_string(), vec![NodeId::new(), NodeId::new(), NodeId::new()]);
        assert!(repl.has_quorum("key1"));
    }

    #[test]
    fn consistency_check() {
        let cons = MemoryConsistency::new(ConsistencyMode::Strong);
        cons.record_write("key1", 1);
        assert!(cons.is_read_consistent("key1", 1));
    }
}
