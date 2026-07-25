//! Cluster storage — persistent repository for topology, node metadata,
//! execution state, workflows, memory indexes, and analytics.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{DistributedError, NeoResult};
use crate::types::NodeId;

// ---------------------------------------------------------------------------
// ClusterSnapshot
// ---------------------------------------------------------------------------

/// A full cluster state snapshot for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterSnapshot {
    /// Snapshot identifier.
    pub id: Uuid,
    /// When the snapshot was taken.
    pub timestamp: DateTime<Utc>,
    /// Cluster name.
    pub cluster_name: String,
    /// Cluster state.
    pub cluster_state: String,
    /// All node IDs and their states.
    pub nodes: HashMap<NodeId, String>,
    /// Leader node.
    pub leader: Option<NodeId>,
    /// Consensus term.
    pub term: u64,
    /// Consensus commit index.
    pub commit_index: u64,
    /// Custom key-value data.
    pub data: HashMap<String, Vec<u8>>,
    /// Snapshot version.
    pub version: u64,
}

// ---------------------------------------------------------------------------
// ClusterCheckpoint
// ---------------------------------------------------------------------------

/// A checkpoint for recovery purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterCheckpoint {
    /// Checkpoint identifier.
    pub id: Uuid,
    /// When the checkpoint was created.
    pub timestamp: DateTime<Utc>,
    /// Snapshot this checkpoint is based on.
    pub snapshot_id: Uuid,
    /// Incremental changes since snapshot.
    pub deltas: Vec<CheckpointDelta>,
    /// Checkpoint version.
    pub version: u64,
}

/// A single delta in a checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointDelta {
    /// Delta type.
    pub delta_type: DeltaType,
    /// Key.
    pub key: String,
    /// Value (if adding/updating).
    pub value: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeltaType {
    Set,
    Delete,
}

// ---------------------------------------------------------------------------
// DistributedRepository
// ---------------------------------------------------------------------------

/// Persistent key-value repository for cluster state.
pub struct DistributedRepository {
    /// In-memory backing store (would be sled/sqlite in production).
    store: DashMap<String, Vec<u8>>,
    /// Snapshots.
    snapshots: RwLock<Vec<ClusterSnapshot>>,
    /// Checkpoints.
    checkpoints: RwLock<Vec<ClusterCheckpoint>>,
    /// Total reads/writes.
    reads: std::sync::atomic::AtomicU64,
    writes: std::sync::atomic::AtomicU64,
}

impl DistributedRepository {
    /// Create a new repository.
    pub fn new() -> Self {
        tracing::info!("distributed repository created");
        Self {
            store: DashMap::new(),
            snapshots: RwLock::new(Vec::new()),
            checkpoints: RwLock::new(Vec::new()),
            reads: std::sync::atomic::AtomicU64::new(0),
            writes: std::sync::atomic::AtomicU64::new(0),
        }
    }

    // -- Key-value operations --

    /// Get a value by key.
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.reads.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.store.get(key).map(|r| r.value().clone())
    }

    /// Set a value.
    pub fn set(&self, key: String, value: Vec<u8>) {
        self.store.insert(key, value);
        self.writes.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Delete a value.
    pub fn delete(&self, key: &str) -> bool {
        self.store.remove(key).is_some()
    }

    /// Check if a key exists.
    pub fn contains(&self, key: &str) -> bool {
        self.store.contains_key(key)
    }

    /// Get all keys.
    pub fn keys(&self) -> Vec<String> {
        self.store.iter().map(|r| r.key().clone()).collect()
    }

    /// Entry count.
    pub fn count(&self) -> usize {
        self.store.len()
    }

    /// Get all entries as key-value pairs.
    pub fn entries(&self) -> Vec<(String, Vec<u8>)> {
        self.store
            .iter()
            .map(|r| (r.key().clone(), r.value().clone()))
            .collect()
    }

    /// Bulk set.
    pub fn bulk_set(&self, entries: Vec<(String, Vec<u8>)>) {
        for (key, value) in entries {
            self.store.insert(key, value);
        }
        self.writes
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    // -- Snapshots --

    /// Create a snapshot.
    pub fn create_snapshot(
        &self,
        cluster_name: String,
        cluster_state: String,
        nodes: HashMap<NodeId, String>,
        leader: Option<NodeId>,
        term: u64,
        commit_index: u64,
    ) -> ClusterSnapshot {
        let snapshot = ClusterSnapshot {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            cluster_name,
            cluster_state,
            nodes,
            leader,
            term,
            commit_index,
            data: HashMap::new(),
            version: self.snapshots.read().len() as u64 + 1,
        };

        self.snapshots.write().push(snapshot.clone());
        tracing::info!(snapshot_id = %snapshot.id, "cluster snapshot created");
        snapshot
    }

    /// Get all snapshots.
    pub fn snapshots(&self) -> Vec<ClusterSnapshot> {
        self.snapshots.read().clone()
    }

    /// Get the latest snapshot.
    pub fn latest_snapshot(&self) -> Option<ClusterSnapshot> {
        self.snapshots.read().last().cloned()
    }

    // -- Checkpoints --

    /// Create a checkpoint.
    pub fn create_checkpoint(&self, snapshot_id: Uuid, deltas: Vec<CheckpointDelta>) -> ClusterCheckpoint {
        let checkpoint = ClusterCheckpoint {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            snapshot_id,
            deltas,
            version: self.checkpoints.read().len() as u64 + 1,
        };

        self.checkpoints.write().push(checkpoint.clone());
        tracing::info!(checkpoint_id = %checkpoint.id, "checkpoint created");
        checkpoint
    }

    /// Get all checkpoints.
    pub fn checkpoints(&self) -> Vec<ClusterCheckpoint> {
        self.checkpoints.read().clone()
    }

    // -- Statistics --

    pub fn stats(&self) -> RepositoryStats {
        RepositoryStats {
            entries: self.count(),
            reads: self.reads.load(std::sync::atomic::Ordering::Relaxed),
            writes: self.writes.load(std::sync::atomic::Ordering::Relaxed),
            snapshots: self.snapshots.read().len(),
            checkpoints: self.checkpoints.read().len(),
        }
    }
}

impl Default for DistributedRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryStats {
    pub entries: usize,
    pub reads: u64,
    pub writes: u64,
    pub snapshots: usize,
    pub checkpoints: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_basic_ops() {
        let repo = DistributedRepository::new();
        repo.set("key1".to_string(), vec![1, 2, 3]);
        assert_eq!(repo.get("key1"), Some(vec![1, 2, 3]));
        assert!(repo.contains("key1"));
        assert_eq!(repo.count(), 1);

        assert!(repo.delete("key1"));
        assert!(!repo.contains("key1"));
    }

    #[test]
    fn repository_snapshot() {
        let repo = DistributedRepository::new();
        let snap = repo.create_snapshot(
            "test".to_string(),
            "active".to_string(),
            HashMap::new(),
            None,
            1,
            0,
        );
        assert_eq!(repo.snapshots().len(), 1);
        assert_eq!(repo.latest_snapshot().unwrap().id, snap.id);
    }

    #[test]
    fn repository_checkpoint() {
        let repo = DistributedRepository::new();
        let snap = repo.create_snapshot(
            "test".to_string(),
            "active".to_string(),
            HashMap::new(),
            None,
            1,
            0,
        );
        let cp = repo.create_checkpoint(snap.id, vec![]);
        assert_eq!(repo.checkpoints().len(), 1);
    }

    #[test]
    fn repository_stats() {
        let repo = DistributedRepository::new();
        let stats = repo.stats();
        assert_eq!(stats.entries, 0);
    }
}
