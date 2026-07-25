use std::collections::HashMap;
use std::path::Path;

use chrono::Utc;
use parking_lot::RwLock;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::error::{MemoryError, MemoryResult};
use crate::types::{MemoryEntry, MemoryId, MemoryTier};

/// Configuration for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceConfig {
    /// Storage backend type.
    pub backend: StorageBackend,
    /// Path for the storage file.
    pub path: String,
    /// Whether to enable WAL mode for SQLite.
    pub enable_wal: bool,
    /// Page size for SQLite.
    pub page_size: usize,
    /// Cache size for SQLite (in pages).
    pub cache_size: usize,
    /// Whether to enable synchronous mode.
    pub synchronous: bool,
    /// Backup interval in seconds.
    pub backup_interval_secs: u64,
    /// Maximum backup files to retain.
    pub max_backups: usize,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            backend: StorageBackend::Sled,
            path: "/tmp/neo-memory-db".to_string(),
            enable_wal: true,
            page_size: 4096,
            cache_size: 10_000,
            synchronous: true,
            backup_interval_secs: 3600,
            max_backups: 5,
        }
    }
}

/// Storage backend type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StorageBackend {
    /// Sled embedded database.
    Sled,
    /// SQLite database.
    Sqlite,
    /// Future distributed storage.
    Distributed,
}

/// Backup record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupRecord {
    /// When the backup was created.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Path to the backup file.
    pub path: String,
    /// Number of entries backed up.
    pub entry_count: u64,
    /// Size in bytes.
    pub size_bytes: u64,
}

/// Persistence layer for memory storage.
pub struct MemoryPersistence {
    config: PersistenceConfig,
    sled_db: RwLock<Option<sled::Db>>,
    sqlite_conn: RwLock<Option<Connection>>,
    backups: RwLock<Vec<BackupRecord>>,
}

impl MemoryPersistence {
    /// Create a new persistence layer.
    pub fn new(config: PersistenceConfig) -> MemoryResult<Self> {
        let sled_db = if config.backend == StorageBackend::Sled {
            Some(
                sled::open(&config.path)
                    .map_err(|e| MemoryError::PersistenceError(e.to_string()))?,
            )
        } else {
            None
        };

        let sqlite_conn = if config.backend == StorageBackend::Sqlite {
            let conn = Connection::open(&config.path)
                .map_err(|e| MemoryError::PersistenceError(e.to_string()))?;

            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS memories (
                    id TEXT PRIMARY KEY,
                    tier TEXT NOT NULL,
                    namespace TEXT NOT NULL,
                    content TEXT NOT NULL,
                    embedding BLOB,
                    tags TEXT NOT NULL,
                    access_count INTEGER NOT NULL DEFAULT 0,
                    created_at TEXT NOT NULL,
                    last_accessed TEXT NOT NULL,
                    importance REAL NOT NULL DEFAULT 0.5,
                    novelty REAL NOT NULL DEFAULT 0.5,
                    confidence REAL NOT NULL DEFAULT 0.5,
                    priority INTEGER NOT NULL DEFAULT 2,
                    status INTEGER NOT NULL DEFAULT 0,
                    ttl_secs INTEGER,
                    parent_id TEXT,
                    source TEXT,
                    estimated_tokens INTEGER NOT NULL DEFAULT 0,
                    consolidated INTEGER NOT NULL DEFAULT 0,
                    version INTEGER NOT NULL DEFAULT 1
                );",
            )
            .map_err(|e| MemoryError::PersistenceError(e.to_string()))?;

            if config.enable_wal {
                conn.execute_batch("PRAGMA journal_mode=WAL;")
                    .map_err(|e| MemoryError::PersistenceError(e.to_string()))?;
            }

            conn.execute_batch(&format!(
                "PRAGMA page_size={}; PRAGMA cache_size={};",
                config.page_size, config.cache_size
            ))
            .map_err(|e| MemoryError::PersistenceError(e.to_string()))?;

            Some(conn)
        } else {
            None
        };

        Ok(Self {
            config,
            sled_db: RwLock::new(sled_db),
            sqlite_conn: RwLock::new(sqlite_conn),
            backups: RwLock::new(Vec::new()),
        })
    }

    /// Persist a memory entry.
    pub fn persist(&self, entry: &MemoryEntry) -> MemoryResult<()> {
        match self.config.backend {
            StorageBackend::Sled => self.persist_sled(entry),
            StorageBackend::Sqlite => self.persist_sqlite(entry),
            StorageBackend::Distributed => {
                Err(MemoryError::NotImplemented("Distributed backend not yet implemented".to_string()))
            }
        }
    }

    /// Load a memory entry by id.
    pub fn load(&self, id: MemoryId) -> MemoryResult<Option<MemoryEntry>> {
        match self.config.backend {
            StorageBackend::Sled => self.load_sled(id),
            StorageBackend::Sqlite => self.load_sqlite(id),
            StorageBackend::Distributed => {
                Err(MemoryError::NotImplemented("Distributed backend not yet implemented".to_string()))
            }
        }
    }

    /// Load all entries.
    pub fn load_all(&self) -> MemoryResult<Vec<MemoryEntry>> {
        match self.config.backend {
            StorageBackend::Sled => self.load_all_sled(),
            StorageBackend::Sqlite => self.load_all_sqlite(),
            StorageBackend::Distributed => {
                Err(MemoryError::NotImplemented("Distributed backend not yet implemented".to_string()))
            }
        }
    }

    /// Delete a memory entry.
    pub fn delete(&self, id: MemoryId) -> MemoryResult<bool> {
        match self.config.backend {
            StorageBackend::Sled => {
                let db = self.sled_db.read();
                if let Some(ref db) = *db {
                    let key = id.0.as_bytes().to_vec();
                    let removed = db.remove(key)
                        .map_err(|e| MemoryError::PersistenceError(e.to_string()))?;
                    Ok(removed.is_some())
                } else {
                    Ok(false)
                }
            }
            StorageBackend::Sqlite => {
                let conn = self.sqlite_conn.read();
                if let Some(ref conn) = *conn {
                    let deleted = conn
                        .execute("DELETE FROM memories WHERE id = ?1", [id.0.to_string()])
                        .map_err(|e| MemoryError::PersistenceError(e.to_string()))?;
                    Ok(deleted > 0)
                } else {
                    Ok(false)
                }
            }
            StorageBackend::Distributed => {
                Err(MemoryError::NotImplemented("Distributed backend not yet implemented".to_string()))
            }
        }
    }

    /// Count all entries.
    pub fn count(&self) -> MemoryResult<u64> {
        match self.config.backend {
            StorageBackend::Sled => {
                let db = self.sled_db.read();
                Ok(db.as_ref().map_or(0, |db| db.len() as u64))
            }
            StorageBackend::Sqlite => {
                let conn = self.sqlite_conn.read();
                if let Some(ref conn) = *conn {
                    let count: i64 = conn
                        .query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))
                        .map_err(|e| MemoryError::PersistenceError(e.to_string()))?;
                    Ok(count as u64)
                } else {
                    Ok(0)
                }
            }
            StorageBackend::Distributed => {
                Err(MemoryError::NotImplemented("Distributed backend not yet implemented".to_string()))
            }
        }
    }

    /// Create a backup.
    pub fn backup(&self, backup_path: &str) -> MemoryResult<BackupRecord> {
        let entries = self.load_all()?;
        let count = entries.len() as u64;

        let json = serde_json::to_vec_pretty(&entries)
            .map_err(|e| MemoryError::SerializationError(e.to_string()))?;

        std::fs::write(backup_path, &json)?;

        let record = BackupRecord {
            timestamp: Utc::now(),
            path: backup_path.to_string(),
            entry_count: count,
            size_bytes: json.len() as u64,
        };

        self.backups.write().push(record.clone());
        Ok(record)
    }

    /// Restore from backup.
    pub fn restore(&self, backup_path: &str) -> MemoryResult<u64> {
        let data = std::fs::read(backup_path)?;
        let entries: Vec<MemoryEntry> = serde_json::from_slice(&data)
            .map_err(|e| MemoryError::SerializationError(e.to_string()))?;

        let mut count = 0;
        for entry in entries {
            self.persist(&entry)?;
            count += 1;
        }

        Ok(count)
    }

    /// Get backup records.
    #[must_use]
    pub fn backups(&self) -> Vec<BackupRecord> {
        self.backups.read().clone()
    }

    // Sled backend methods.

    fn persist_sled(&self, entry: &MemoryEntry) -> MemoryResult<()> {
        let db = self.sled_db.read();
        if let Some(ref db) = *db {
            let key = entry.id.0.as_bytes().to_vec();
            let value = serde_json::to_vec(entry)
                .map_err(|e| MemoryError::SerializationError(e.to_string()))?;
            db.insert(key, value)
                .map_err(|e| MemoryError::PersistenceError(e.to_string()))?;
        }
        Ok(())
    }

    fn load_sled(&self, id: MemoryId) -> MemoryResult<Option<MemoryEntry>> {
        let db = self.sled_db.read();
        if let Some(ref db) = *db {
            let key = id.0.as_bytes().to_vec();
            if let Some(value) = db.get(key)
                .map_err(|e| MemoryError::PersistenceError(e.to_string()))?
            {
                let entry: MemoryEntry = serde_json::from_slice(&value)
                    .map_err(|e| MemoryError::SerializationError(e.to_string()))?;
                return Ok(Some(entry));
            }
        }
        Ok(None)
    }

    fn load_all_sled(&self) -> MemoryResult<Vec<MemoryEntry>> {
        let db = self.sled_db.read();
        let mut entries = Vec::new();
        if let Some(ref db) = *db {
            for item in db.iter() {
                let (_, value) = item.map_err(|e| MemoryError::PersistenceError(e.to_string()))?;
                if let Ok(entry) = serde_json::from_slice::<MemoryEntry>(&value) {
                    entries.push(entry);
                }
            }
        }
        Ok(entries)
    }

    // SQLite backend methods.

    fn persist_sqlite(&self, entry: &MemoryEntry) -> MemoryResult<()> {
        let conn = self.sqlite_conn.read();
        if let Some(ref conn) = *conn {
            let tags_json = serde_json::to_string(&entry.tags)
                .map_err(|e| MemoryError::SerializationError(e.to_string()))?;
            let content_json = entry.content.to_string();

            conn.execute(
                "INSERT OR REPLACE INTO memories (
                    id, tier, namespace, content, tags, access_count,
                    created_at, last_accessed, importance, novelty,
                    confidence, priority, status, ttl_secs, parent_id,
                    source, estimated_tokens, consolidated, version
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                          ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
                rusqlite::params![
                    entry.id.0.to_string(),
                    entry.tier.to_string(),
                    entry.namespace.0,
                    content_json,
                    tags_json,
                    entry.access_count.load(std::sync::atomic::Ordering::SeqCst) as i64,
                    entry.created_at.to_rfc3339(),
                    entry.last_accessed.lock().map_or_else(
                        |_| entry.created_at.to_rfc3339(),
                        |l| l.to_rfc3339()
                    ),
                    entry.importance as f64,
                    entry.novelty as f64,
                    entry.confidence as f64,
                    entry.priority as i32,
                    entry.status as i32,
                    entry.ttl.map(|d| d.as_secs() as i64),
                    entry.parent_id.map(|id| id.0.to_string()),
                    entry.source,
                    entry.estimated_tokens as i64,
                    entry.consolidated as i32,
                    entry.version as i64,
                ],
            )
            .map_err(|e| MemoryError::PersistenceError(e.to_string()))?;
        }
        Ok(())
    }

    fn load_sqlite(&self, id: MemoryId) -> MemoryResult<Option<MemoryEntry>> {
        let conn = self.sqlite_conn.read();
        if let Some(ref conn) = *conn {
            let result = conn.query_row(
                "SELECT id, tier, namespace, content, tags, access_count,
                        created_at, last_accessed, importance, novelty,
                        confidence, priority, status, ttl_secs, parent_id,
                        source, estimated_tokens, consolidated, version
                 FROM memories WHERE id = ?1",
                [id.0.to_string()],
                |row| {
                    Ok(MemoryEntry {
                        id: MemoryId(uuid::Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default()),
                        tier: parse_tier(&row.get::<_, String>(1)?),
                        namespace: crate::types::MemoryNamespace::new(row.get::<_, String>(2)?),
                        content: serde_json::from_str(&row.get::<_, String>(3)?)
                            .unwrap_or(serde_json::json!(null)),
                        embedding: None,
                        tags: serde_json::from_str(&row.get::<_, String>(4)?)
                            .unwrap_or_default(),
                        access_count: std::sync::atomic::AtomicU64::new(row.get::<_, i64>(5)? as u64),
                        created_at: chrono::DateTime::parse_from_rfc3339(
                            &row.get::<_, String>(6)?,
                        )
                        .map_or_else(|_| Utc::now(), |dt| dt.with_timezone(&Utc)),
                        last_accessed: std::sync::Mutex::new(
                            chrono::DateTime::parse_from_rfc3339(
                                &row.get::<_, String>(7)?,
                            )
                            .map_or_else(|_| Utc::now(), |dt| dt.with_timezone(&Utc)),
                        ),
                        last_modified: std::sync::Mutex::new(Utc::now()),
                        importance: row.get::<_, f64>(8)? as f32,
                        novelty: row.get::<_, f64>(9)? as f32,
                        confidence: row.get::<_, f64>(10)? as f32,
                        priority: parse_priority(row.get::<_, i32>(11)?),
                        status: parse_status(row.get::<_, i32>(12)?),
                        ttl: row.get::<_, Option<i64>>(13)?
                            .map(|s| std::time::Duration::from_secs(s as u64)),
                        parent_id: row.get::<_, Option<String>>(14)?
                            .and_then(|s| uuid::Uuid::parse_str(&s).ok())
                            .map(MemoryId),
                        source: row.get::<_, Option<String>>(15)?,
                        estimated_tokens: row.get::<_, i64>(16)? as usize,
                        consolidated: row.get::<_, i32>(17)? != 0,
                        version: row.get::<_, i64>(18)? as u64,
                    })
                },
            );

            match result {
                Ok(entry) => Ok(Some(entry)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(MemoryError::PersistenceError(e.to_string())),
            }
        } else {
            Ok(None)
        }
    }

    fn load_all_sqlite(&self) -> MemoryResult<Vec<MemoryEntry>> {
        let conn = self.sqlite_conn.read();
        let mut entries = Vec::new();
        if let Some(ref conn) = *conn {
            let mut stmt = conn
                .prepare("SELECT id FROM memories")
                .map_err(|e| MemoryError::PersistenceError(e.to_string()))?;
            let ids: Vec<String> = stmt
                .query_map([], |row| row.get(0))
                .map_err(|e| MemoryError::PersistenceError(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect();
            drop(stmt);

            for id_str in ids {
                if let Ok(uuid) = uuid::Uuid::parse_str(&id_str) {
                    if let Some(entry) = self.load_sqlite(MemoryId(uuid))? {
                        entries.push(entry);
                    }
                }
            }
        }
        Ok(entries)
    }
}

fn parse_tier(s: &str) -> MemoryTier {
    match s {
        "working" => MemoryTier::Working,
        "episodic" => MemoryTier::Episodic,
        "semantic" => MemoryTier::Semantic,
        "procedural" => MemoryTier::Procedural,
        _ => MemoryTier::LongTerm,
    }
}

fn parse_priority(v: i32) -> crate::types::MemoryPriority {
    match v {
        0 => crate::types::MemoryPriority::Background,
        1 => crate::types::MemoryPriority::Low,
        2 => crate::types::MemoryPriority::Normal,
        3 => crate::types::MemoryPriority::High,
        4 => crate::types::MemoryPriority::Critical,
        _ => crate::types::MemoryPriority::Normal,
    }
}

fn parse_status(v: i32) -> crate::types::MemoryStatus {
    match v {
        0 => crate::types::MemoryStatus::Active,
        1 => crate::types::MemoryStatus::Compressed,
        2 => crate::types::MemoryStatus::Archived,
        3 => crate::types::MemoryStatus::Deleted,
        4 => crate::types::MemoryStatus::Pinned,
        _ => crate::types::MemoryStatus::Active,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn sled_persist_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let config = PersistenceConfig {
            backend: StorageBackend::Sled,
            path: dir.path().join("test-sled").to_str().unwrap().to_string(),
            ..PersistenceConfig::default()
        };
        let persistence = MemoryPersistence::new(config).unwrap();

        let entry = MemoryEntry::new(
            MemoryTier::LongTerm,
            serde_json::json!("test"),
            HashSet::new(),
        );
        let id = entry.id;

        persistence.persist(&entry).unwrap();
        let loaded = persistence.load(id).unwrap();
        assert!(loaded.is_some());
    }

    #[test]
    fn sled_delete() {
        let dir = tempfile::tempdir().unwrap();
        let config = PersistenceConfig {
            backend: StorageBackend::Sled,
            path: dir.path().join("test-del").to_str().unwrap().to_string(),
            ..PersistenceConfig::default()
        };
        let persistence = MemoryPersistence::new(config).unwrap();

        let entry = MemoryEntry::new(
            MemoryTier::LongTerm,
            serde_json::json!("test"),
            HashSet::new(),
        );
        let id = entry.id;

        persistence.persist(&entry).unwrap();
        assert!(persistence.delete(id).unwrap());
        assert!(persistence.load(id).unwrap().is_none());
    }

    #[test]
    fn backup_and_restore() {
        let dir = tempfile::tempdir().unwrap();
        let config = PersistenceConfig {
            backend: StorageBackend::Sled,
            path: dir.path().join("test-bk").to_str().unwrap().to_string(),
            ..PersistenceConfig::default()
        };
        let persistence = MemoryPersistence::new(config).unwrap();

        let entry = MemoryEntry::new(
            MemoryTier::LongTerm,
            serde_json::json!("backup test"),
            HashSet::new(),
        );
        persistence.persist(&entry).unwrap();

        let backup_path = dir.path().join("backup.json");
        let record = persistence.backup(backup_path.to_str().unwrap()).unwrap();
        assert_eq!(record.entry_count, 1);

        // Restore to a fresh persistence.
        let config2 = PersistenceConfig {
            backend: StorageBackend::Sled,
            path: dir.path().join("test-restore").to_str().unwrap().to_string(),
            ..PersistenceConfig::default()
        };
        let persistence2 = MemoryPersistence::new(config2).unwrap();
        let restored = persistence2.restore(backup_path.to_str().unwrap()).unwrap();
        assert_eq!(restored, 1);
    }
}
