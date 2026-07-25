use std::collections::HashMap;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

use crate::types::{EvolutionId, SubsystemTarget};

/// A single audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique id for this entry.
    pub id: EvolutionId,
    /// When the action occurred.
    pub timestamp: DateTime<Utc>,
    /// Description of the action performed.
    pub action: String,
    /// Who performed the action.
    pub actor: String,
    /// Subsystem that was targeted.
    pub target: SubsystemTarget,
    /// Arbitrary structured details.
    pub details: HashMap<String, Value>,
    /// Outcome of the action.
    pub result: String,
}

/// Append-only audit log with time-range and field queries.
#[derive(Debug)]
pub struct EvolutionAudit {
    entries: RwLock<Vec<AuditEntry>>,
}

impl EvolutionAudit {
    /// Create an empty audit log.
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
        }
    }

    /// Append an entry to the audit log.
    pub fn record(&self, entry: AuditEntry) {
        self.entries.write().push(entry);
    }

    /// Query entries by time range, actor, target, and action.
    ///
    /// All filters are optional — pass `None` to skip a filter.
    pub fn query(
        &self,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        actor: Option<&str>,
        target: Option<&SubsystemTarget>,
        action: Option<&str>,
    ) -> Vec<AuditEntry> {
        let entries = self.entries.read();
        entries
            .iter()
            .filter(|e| {
                if let Some(f) = from {
                    if e.timestamp < f {
                        return false;
                    }
                }
                if let Some(t) = to {
                    if e.timestamp > t {
                        return false;
                    }
                }
                if let Some(a) = actor {
                    if e.actor != a {
                        return false;
                    }
                }
                if let Some(tgt) = target {
                    if &e.target != tgt {
                        return false;
                    }
                }
                if let Some(act) = action {
                    if e.action != act {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect()
    }

    /// Return all recorded entries.
    pub fn get_entries(&self) -> Vec<AuditEntry> {
        self.entries.read().clone()
    }

    /// Return the total number of recorded entries.
    pub fn get_count(&self) -> usize {
        self.entries.read().len()
    }

    /// Export all entries.  Equivalent to [`get_entries`](Self::get_entries)
    /// but named for clarity in export contexts.
    pub fn export_entries(&self) -> Vec<AuditEntry> {
        self.get_entries()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(action: &str, actor: &str, target: SubsystemTarget) -> AuditEntry {
        AuditEntry {
            id: EvolutionId::new_v4(),
            timestamp: Utc::now(),
            action: action.to_string(),
            actor: actor.to_string(),
            target,
            details: HashMap::new(),
            result: "success".to_string(),
        }
    }

    #[test]
    fn record_and_count() {
        let audit = EvolutionAudit::new();
        assert_eq!(audit.get_count(), 0);
        audit.record(make_entry("deploy", "admin", SubsystemTarget::Core));
        audit.record(make_entry("rollback", "admin", SubsystemTarget::Memory));
        assert_eq!(audit.get_count(), 2);
    }

    #[test]
    fn query_by_actor() {
        let audit = EvolutionAudit::new();
        audit.record(make_entry("deploy", "alice", SubsystemTarget::Core));
        audit.record(make_entry("rollback", "bob", SubsystemTarget::Core));
        let results = audit.query(None, None, Some("alice"), None, None);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].actor, "alice");
    }

    #[test]
    fn query_by_target() {
        let audit = EvolutionAudit::new();
        audit.record(make_entry("a", "x", SubsystemTarget::Core));
        audit.record(make_entry("b", "x", SubsystemTarget::Memory));
        let results = audit.query(None, None, None, Some(&SubsystemTarget::Memory), None);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn export_returns_all() {
        let audit = EvolutionAudit::new();
        audit.record(make_entry("a", "x", SubsystemTarget::Core));
        audit.record(make_entry("b", "y", SubsystemTarget::Agents));
        assert_eq!(audit.export_entries().len(), 2);
    }

    #[test]
    fn query_by_time_range() {
        let audit = EvolutionAudit::new();
        let before = Utc::now();
        audit.record(make_entry("a", "x", SubsystemTarget::Core));
        let after = Utc::now();
        let results = audit.query(Some(before), Some(after), None, None, None);
        assert_eq!(results.len(), 1);
    }
}
