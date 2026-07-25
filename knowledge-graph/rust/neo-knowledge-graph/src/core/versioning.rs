use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Type of change tracked in versioning.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChangeType {
    Created,
    Updated,
    Deleted,
    Merged,
    Split,
    Pruned,
    Relocated,
}

/// A single versioned change record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedChange {
    /// Monotonic version number.
    pub version: u64,
    /// Type of change.
    pub change_type: ChangeType,
    /// Description of the change.
    pub description: String,
    /// Timestamp of the change.
    pub timestamp: DateTime<Utc>,
    /// Who/what initiated the change.
    pub actor: String,
    /// Checksum of the state after this change.
    pub checksum: String,
}

/// Vector clock for tracking versions across dimensions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VersionVector {
    /// Global version counter.
    pub global: u64,
    /// Per-namespace version counters.
    pub namespaces: std::collections::HashMap<String, u64>,
}

impl VersionVector {
    /// Create a new version vector.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment the global version.
    pub fn increment_global(&mut self) -> u64 {
        self.global += 1;
        self.global
    }

    /// Increment a namespace version.
    pub fn increment_namespace(&mut self, ns: &str) -> u64 {
        let entry = self.namespaces.entry(ns.to_string()).or_insert(0);
        *entry += 1;
        *entry
    }

    /// Get the global version.
    #[must_use]
    pub fn global_version(&self) -> u64 {
        self.global
    }

    /// Get a namespace version.
    #[must_use]
    pub fn namespace_version(&self, ns: &str) -> u64 {
        self.namespaces.get(ns).copied().unwrap_or(0)
    }
}

/// Tracks version history for entities and relations.
#[derive(Debug)]
pub struct VersionTracker {
    history: Vec<VersionedChange>,
    max_history: usize,
}

impl VersionTracker {
    /// Create a new version tracker.
    #[must_use]
    pub fn new(max_history: usize) -> Self {
        Self {
            history: Vec::new(),
            max_history,
        }
    }

    /// Record a change.
    pub fn record(
        &mut self,
        version: u64,
        change_type: ChangeType,
        description: impl Into<String>,
        actor: impl Into<String>,
        checksum: impl Into<String>,
    ) {
        let change = VersionedChange {
            version,
            change_type,
            description: description.into(),
            timestamp: Utc::now(),
            actor: actor.into(),
            checksum: checksum.into(),
        };
        self.history.push(change);
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }

    /// Get the history of changes.
    #[must_use]
    pub fn history(&self) -> &[VersionedChange] {
        &self.history
    }

    /// Get the latest version.
    #[must_use]
    pub fn latest_version(&self) -> u64 {
        self.history.last().map_or(0, |c| c.version)
    }
}

impl Default for VersionTracker {
    fn default() -> Self {
        Self::new(1000)
    }
}
