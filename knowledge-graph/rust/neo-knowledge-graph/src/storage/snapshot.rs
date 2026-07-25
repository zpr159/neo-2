use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::core::entity::Entity;
use crate::core::relation::Relation;
use crate::error::{KnowledgeError, KnowledgeResult};

/// Configuration for snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotConfig {
    /// Maximum number of snapshots to retain.
    pub max_snapshots: usize,
    /// Whether to compress snapshots.
    pub compress: bool,
}

impl Default for SnapshotConfig {
    fn default() -> Self {
        Self {
            max_snapshots: 10,
            compress: false,
        }
    }
}

/// A point-in-time snapshot of the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSnapshot {
    /// Snapshot identifier.
    pub id: String,
    /// When the snapshot was taken.
    pub timestamp: DateTime<Utc>,
    /// All entities at this point.
    pub entities: Vec<Entity>,
    /// All relations at this point.
    pub relations: Vec<Relation>,
    /// Entity count.
    pub entity_count: usize,
    /// Relation count.
    pub relation_count: usize,
    /// Optional description.
    pub description: String,
    /// Size in bytes (approximate).
    pub size_bytes: u64,
}

/// Manages graph snapshots for backup and recovery.
pub struct SnapshotManager {
    snapshots: RwLock<Vec<GraphSnapshot>>,
    config: SnapshotConfig,
}

impl SnapshotManager {
    /// Create a new snapshot manager.
    #[must_use]
    pub fn new(config: SnapshotConfig) -> Self {
        Self {
            snapshots: RwLock::new(Vec::new()),
            config,
        }
    }

    /// Create a snapshot from current graph state.
    pub fn create_snapshot(
        &self,
        entities: Vec<Entity>,
        relations: Vec<Relation>,
        description: impl Into<String>,
    ) -> GraphSnapshot {
        let entity_count = entities.len();
        let relation_count = relations.len();
        let desc = description.into();

        let snapshot = GraphSnapshot {
            id: format!("snap-{}", chrono::Utc::now().timestamp_millis()),
            timestamp: Utc::now(),
            entity_count,
            relation_count,
            description: desc,
            size_bytes: 0, // computed on demand
            entities,
            relations,
        };

        let mut snapshots = self.snapshots.write();
        snapshots.push(snapshot.clone());

        // Trim old snapshots
        if snapshots.len() > self.config.max_snapshots {
            let excess = snapshots.len() - self.config.max_snapshots;
            snapshots.drain(0..excess);
        }

        snapshot
    }

    /// Get the latest snapshot.
    #[must_use]
    pub fn latest(&self) -> Option<GraphSnapshot> {
        self.snapshots.read().last().cloned()
    }

    /// Get a snapshot by id.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<GraphSnapshot> {
        self.snapshots.read().iter().find(|s| s.id == id).cloned()
    }

    /// List all snapshot ids and timestamps.
    #[must_use]
    pub fn list(&self) -> Vec<(String, DateTime<Utc>)> {
        self.snapshots
            .read()
            .iter()
            .map(|s| (s.id.clone(), s.timestamp))
            .collect()
    }

    /// Restore from a snapshot, returning the entities and relations.
    #[must_use]
    pub fn restore(&self, id: &str) -> KnowledgeResult<(Vec<Entity>, Vec<Relation>)> {
        let snapshot = self
            .snapshots
            .read()
            .iter()
            .find(|s| s.id == id)
            .cloned()
            .ok_or_else(|| KnowledgeError::SnapshotError(format!("Snapshot '{}' not found", id)))?;

        Ok((snapshot.entities, snapshot.relations))
    }

    /// Delete a snapshot.
    pub fn delete(&self, id: &str) -> KnowledgeResult<bool> {
        let mut snapshots = self.snapshots.write();
        let len_before = snapshots.len();
        snapshots.retain(|s| s.id != id);
        Ok(snapshots.len() < len_before)
    }

    /// Number of stored snapshots.
    #[must_use]
    pub fn count(&self) -> usize {
        self.snapshots.read().len()
    }
}

impl Default for SnapshotManager {
    fn default() -> Self {
        Self::new(SnapshotConfig::default())
    }
}
