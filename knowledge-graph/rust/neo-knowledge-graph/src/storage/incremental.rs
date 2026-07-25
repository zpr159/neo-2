use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::KnowledgeResult;

/// A single delta change record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaChange {
    /// Unique change id.
    pub id: String,
    /// The kind of change.
    pub change_kind: DeltaChangeKind,
    /// Timestamp.
    pub timestamp: DateTime<Utc>,
    /// Serialized entity/relation data.
    pub data: serde_json::Value,
}

/// Kind of delta change.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeltaChangeKind {
    EntityCreated,
    EntityUpdated,
    EntityRemoved,
    RelationCreated,
    RelationUpdated,
    RelationRemoved,
}

/// Record of accumulated delta changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaRecord {
    /// Sequential counter for the last applied delta.
    pub last_applied: u64,
    /// Pending changes.
    pub changes: Vec<DeltaChange>,
}

impl DeltaRecord {
    /// Create a new empty delta record.
    #[must_use]
    pub fn new() -> Self {
        Self {
            last_applied: 0,
            changes: Vec::new(),
        }
    }
}

impl Default for DeltaRecord {
    fn default() -> Self {
        Self::new()
    }
}

/// Manages incremental updates to the knowledge graph.
pub struct IncrementalUpdater {
    pending: parking_lot::RwLock<DeltaRecord>,
}

impl IncrementalUpdater {
    /// Create a new incremental updater.
    #[must_use]
    pub fn new() -> Self {
        Self {
            pending: parking_lot::RwLock::new(DeltaRecord::new()),
        }
    }

    /// Record a change.
    pub fn record_change(&self, change: DeltaChange) {
        self.pending.write().changes.push(change);
    }

    /// Get pending changes.
    #[must_use]
    pub fn pending_changes(&self) -> Vec<DeltaChange> {
        self.pending.read().changes.clone()
    }

    /// Get the count of pending changes.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.pending.read().changes.len()
    }

    /// Mark all pending changes as applied.
    pub fn flush(&self) {
        let mut pending = self.pending.write();
        pending.last_applied += pending.changes.len() as u64;
        pending.changes.clear();
    }

    /// Create a delta change for entity creation.
    pub fn entity_created(entity_id: &str, data: serde_json::Value) -> DeltaChange {
        DeltaChange {
            id: format!("delta-{}", chrono::Utc::now().timestamp_millis()),
            change_kind: DeltaChangeKind::EntityCreated,
            timestamp: Utc::now(),
            data,
        }
    }

    /// Create a delta change for entity update.
    pub fn entity_updated(entity_id: &str, data: serde_json::Value) -> DeltaChange {
        DeltaChange {
            id: format!("delta-{}", chrono::Utc::now().timestamp_millis()),
            change_kind: DeltaChangeKind::EntityUpdated,
            timestamp: Utc::now(),
            data,
        }
    }

    /// Create a delta change for entity removal.
    pub fn entity_removed(entity_id: &str, data: serde_json::Value) -> DeltaChange {
        DeltaChange {
            id: format!("delta-{}", chrono::Utc::now().timestamp_millis()),
            change_kind: DeltaChangeKind::EntityRemoved,
            timestamp: Utc::now(),
            data,
        }
    }

    /// Create a delta change for relation creation.
    pub fn relation_created(relation_id: &str, data: serde_json::Value) -> DeltaChange {
        DeltaChange {
            id: format!("delta-{}", chrono::Utc::now().timestamp_millis()),
            change_kind: DeltaChangeKind::RelationCreated,
            timestamp: Utc::now(),
            data,
        }
    }

    /// Create a delta change for relation update.
    pub fn relation_updated(relation_id: &str, data: serde_json::Value) -> DeltaChange {
        DeltaChange {
            id: format!("delta-{}", chrono::Utc::now().timestamp_millis()),
            change_kind: DeltaChangeKind::RelationUpdated,
            timestamp: Utc::now(),
            data,
        }
    }

    /// Create a delta change for relation removal.
    pub fn relation_removed(relation_id: &str, data: serde_json::Value) -> DeltaChange {
        DeltaChange {
            id: format!("delta-{}", chrono::Utc::now().timestamp_millis()),
            change_kind: DeltaChangeKind::RelationRemoved,
            timestamp: Utc::now(),
            data,
        }
    }
}

impl Default for IncrementalUpdater {
    fn default() -> Self {
        Self::new()
    }
}
