//! Tool events for integration with the Neo runtime event bus.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::ToolVersion;

/// Tool-specific events emitted through the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolEvent {
    ToolRegistered {
        tool_name: String,
        tool_type: String,
        version: ToolVersion,
        timestamp: DateTime<Utc>,
    },
    ToolLoaded {
        tool_name: String,
        timestamp: DateTime<Utc>,
    },
    ToolStarted {
        tool_name: String,
        timestamp: DateTime<Utc>,
    },
    ToolFinished {
        tool_name: String,
        execution_id: String,
        success: bool,
        duration_ms: u64,
        timestamp: DateTime<Utc>,
    },
    ToolFailed {
        tool_name: String,
        execution_id: String,
        error: String,
        timestamp: DateTime<Utc>,
    },
    ToolUpdated {
        tool_name: String,
        old_version: ToolVersion,
        new_version: ToolVersion,
        timestamp: DateTime<Utc>,
    },
    ToolDisabled {
        tool_name: String,
        reason: Option<String>,
        timestamp: DateTime<Utc>,
    },
    ToolEnabled {
        tool_name: String,
        timestamp: DateTime<Utc>,
    },
    PermissionDenied {
        tool_name: String,
        caller_id: String,
        operation: String,
        timestamp: DateTime<Utc>,
    },
    SandboxViolation {
        tool_name: String,
        execution_id: String,
        violation_type: String,
        timestamp: DateTime<Utc>,
    },
    ExecutionCancelled {
        tool_name: String,
        execution_id: String,
        timestamp: DateTime<Utc>,
    },
}

impl ToolEvent {
    /// Convert to a generic JSON event payload.
    pub fn to_payload(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_else(|_| serde_json::json!({"type": "unknown"}))
    }
}

/// Event listener for tool events.
pub trait ToolEventListener: Send + Sync {
    fn on_event(&self, event: &ToolEvent);
}

/// Simple in-memory event log.
pub struct ToolEventLog {
    events: parking_lot::RwLock<Vec<ToolEvent>>,
    max_entries: usize,
}

impl std::fmt::Debug for ToolEventLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolEventLog")
            .field("max_entries", &self.max_entries)
            .field("event_count", &self.events.read().len())
            .finish()
    }
}

impl ToolEventLog {
    pub fn new(max_entries: usize) -> Self {
        Self {
            events: parking_lot::RwLock::new(Vec::new()),
            max_entries,
        }
    }

    pub fn push(&self, event: ToolEvent) {
        let mut events = self.events.write();
        events.push(event);
        if events.len() > self.max_entries {
            let drain_count = events.len() - self.max_entries;
            events.drain(..drain_count);
        }
    }

    pub fn recent(&self, n: usize) -> Vec<ToolEvent> {
        let events = self.events.read();
        events.iter().rev().take(n).cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.events.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.read().is_empty()
    }

    pub fn clear(&self) {
        self.events.write().clear();
    }
}

impl Default for ToolEventLog {
    fn default() -> Self {
        Self::new(10_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_log() {
        let log = ToolEventLog::new(5);
        for i in 0..10 {
            log.push(ToolEvent::ToolRegistered {
                tool_name: format!("tool_{i}"),
                tool_type: "test".into(),
                version: ToolVersion::new(1, 0, 0),
                timestamp: Utc::now(),
            });
        }
        assert_eq!(log.len(), 5);
        assert!(!log.is_empty());
    }
}
