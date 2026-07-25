use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Timestamp of the action.
    pub timestamp: DateTime<Utc>,
    /// The action performed.
    pub action: AuditAction,
    /// Actor performing the action.
    pub actor: String,
    /// Target entity or relation id.
    pub target_id: String,
    /// Namespace.
    pub namespace: String,
    /// Whether the action was permitted.
    pub permitted: bool,
    /// Additional details.
    pub details: Option<String>,
}

/// Types of audited actions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuditAction {
    EntityCreate,
    EntityRead,
    EntityUpdate,
    EntityDelete,
    RelationCreate,
    RelationRead,
    RelationUpdate,
    RelationDelete,
    Search,
    Traverse,
    Export,
    Import,
    SnapshotCreate,
    SnapshotRestore,
}

/// Maintains an audit trail for the knowledge graph.
pub struct AuditTrail {
    entries: parking_lot::RwLock<Vec<AuditEntry>>,
    max_entries: usize,
}

impl AuditTrail {
    /// Create a new audit trail.
    #[must_use]
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: parking_lot::RwLock::new(Vec::new()),
            max_entries,
        }
    }

    /// Log an action.
    pub fn log(
        &self,
        action: AuditAction,
        actor: impl Into<String>,
        target_id: impl Into<String>,
        namespace: impl Into<String>,
        permitted: bool,
        details: Option<String>,
    ) {
        let entry = AuditEntry {
            timestamp: Utc::now(),
            action,
            actor: actor.into(),
            target_id: target_id.into(),
            namespace: namespace.into(),
            permitted,
            details,
        };

        let mut entries = self.entries.write();
        entries.push(entry);
        if entries.len() > self.max_entries {
            entries.remove(0);
        }
    }

    /// Get all audit entries.
    #[must_use]
    pub fn entries(&self) -> Vec<AuditEntry> {
        self.entries.read().clone()
    }

    /// Get entries for a specific target.
    #[must_use]
    pub fn entries_for_target(&self, target_id: &str) -> Vec<AuditEntry> {
        self.entries
            .read()
            .iter()
            .filter(|e| e.target_id == target_id)
            .cloned()
            .collect()
    }

    /// Get entries for a specific actor.
    #[must_use]
    pub fn entries_for_actor(&self, actor: &str) -> Vec<AuditEntry> {
        self.entries
            .read()
            .iter()
            .filter(|e| e.actor == actor)
            .cloned()
            .collect()
    }

    /// Get total entry count.
    #[must_use]
    pub fn count(&self) -> usize {
        self.entries.read().len()
    }
}

impl Default for AuditTrail {
    fn default() -> Self {
        Self::new(10_000)
    }
}
