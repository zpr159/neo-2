/// Audit logging subsystem for the Neo security layer.

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

/// The outcome of an audited action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuditOutcome {
    /// The action succeeded.
    Success,
    /// The action failed.
    Failure,
    /// The action was denied by policy.
    Denied,
}

impl std::fmt::Display for AuditOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Success => write!(f, "success"),
            Self::Failure => write!(f, "failure"),
            Self::Denied => write!(f, "denied"),
        }
    }
}

/// A single audit event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event identifier.
    pub id: String,
    /// ISO-8601 timestamp.
    pub timestamp: String,
    /// ID of the user who performed the action.
    pub user_id: String,
    /// The action that was performed.
    pub action: String,
    /// The resource the action targeted.
    pub resource: String,
    /// Whether the action succeeded, failed, or was denied.
    pub outcome: AuditOutcome,
    /// Optional human-readable details.
    pub details: Option<String>,
    /// Arbitrary metadata.
    pub metadata: std::collections::HashMap<String, String>,
}

/// Stores and queries audit events.
#[derive(Debug)]
pub struct AuditLogger {
    events: RwLock<Vec<AuditEvent>>,
}

impl AuditLogger {
    /// Create a new, empty AuditLogger.
    pub fn new() -> Self {
        Self {
            events: RwLock::new(Vec::new()),
        }
    }

    /// Log an audit event.
    pub async fn log_event(
        &self,
        user_id: &str,
        action: &str,
        resource: &str,
        outcome: AuditOutcome,
        details: Option<String>,
    ) {
        let event = AuditEvent {
            id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            user_id: user_id.to_string(),
            action: action.to_string(),
            resource: resource.to_string(),
            outcome: outcome.clone(),
            details,
            metadata: Default::default(),
        };

        tracing::info!(
            event_id = %event.id,
            user = %event.user_id,
            action = %event.action,
            resource = %event.resource,
            outcome = %event.outcome,
            "audit event logged"
        );

        let mut events = self.events.write().await;
        events.push(event);
    }

    /// Query events by user_id, returning all matching events.
    pub async fn query_events(&self, user_id: &str) -> Vec<AuditEvent> {
        let events = self.events.read().await;
        events
            .iter()
            .filter(|e| e.user_id == user_id)
            .cloned()
            .collect()
    }

    /// Export all events (for downstream processing / persistence).
    pub async fn export_events(&self) -> Vec<AuditEvent> {
        let events = self.events.read().await;
        events.clone()
    }

    /// Return the total number of logged events.
    pub async fn event_count(&self) -> usize {
        let events = self.events.read().await;
        events.len()
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}
