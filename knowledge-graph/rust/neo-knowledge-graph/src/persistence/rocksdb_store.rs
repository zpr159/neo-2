use std::path::Path;

use tracing::info;

use crate::error::{KnowledgeError, KnowledgeResult};

/// RocksDB-backed persistent storage abstraction for the knowledge graph.
///
/// Uses sled as a Rust-native embedded key-value store alternative
/// (since rocksdb bindings have heavy C dependencies).
pub struct RocksDbStore {
    db: sled::Db,
}

impl RocksDbStore {
    /// Open or create a store at the given path.
    pub fn open(path: &Path) -> KnowledgeResult<Self> {
        let db = sled::open(path)
            .map_err(|e| KnowledgeError::StorageError(e.to_string()))?;
        info!("Key-value store opened at {}", path.display());
        Ok(Self { db })
    }

    /// Store a key-value pair.
    pub fn put(&self, key: &[u8], value: &[u8]) -> KnowledgeResult<()> {
        self.db
            .insert(key, value)
            .map_err(|e| KnowledgeError::StorageError(e.to_string()))?;
        Ok(())
    }

    /// Retrieve a value by key.
    #[must_use]
    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.db.get(key).ok().flatten().map(|v| v.to_vec())
    }

    /// Remove a key-value pair.
    pub fn remove(&self, key: &[u8]) -> KnowledgeResult<bool> {
        self.db
            .remove(key)
            .map(|r| r.is_some())
            .map_err(|e| KnowledgeError::StorageError(e.to_string()))
    }

    /// Iterate over all key-value pairs with a given prefix.
    #[must_use]
    pub fn scan_prefix(&self, prefix: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)> {
        self.db
            .scan_prefix(prefix)
            .filter_map(|r| r.ok())
            .map(|(k, v)| (k.to_vec(), v.to_vec()))
            .collect()
    }

    /// Count entries.
    #[must_use]
    pub fn count(&self) -> usize {
        self.db.len()
    }

    /// Flush to disk.
    pub fn flush(&self) -> KnowledgeResult<()> {
        self.db
            .flush()
            .map_err(|e| KnowledgeError::StorageError(e.to_string()))?;
        Ok(())
    }

    /// Get database size in bytes.
    #[must_use]
    pub fn size_on_disk(&self) -> u64 {
        self.db.size_on_disk().unwrap_or(0)
    }
}
