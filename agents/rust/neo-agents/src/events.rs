use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::types::AgentId;

// ---------------------------------------------------------------------------
// AgentEvent
// ---------------------------------------------------------------------------

/// Events emitted by the agent framework.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentEvent {
    /// A new agent was created.
    AgentCreated {
        agent_id: AgentId,
        name: String,
        timestamp: DateTime<Utc>,
    },
    /// An agent started running.
    AgentStarted {
        agent_id: AgentId,
        timestamp: DateTime<Utc>,
    },
    /// An agent stopped.
    AgentStopped {
        agent_id: AgentId,
        timestamp: DateTime<Utc>,
    },
    /// An agent was paused.
    AgentPaused {
        agent_id: AgentId,
        timestamp: DateTime<Utc>,
    },
    /// An agent was resumed.
    AgentResumed {
        agent_id: AgentId,
        timestamp: DateTime<Utc>,
    },
    /// An agent failed.
    AgentFailed {
        agent_id: AgentId,
        error: String,
        timestamp: DateTime<Utc>,
    },
    /// An agent recovered from failure.
    AgentRecovered {
        agent_id: AgentId,
        timestamp: DateTime<Utc>,
    },
    /// A task was assigned to an agent.
    TaskAssigned {
        task_id: uuid::Uuid,
        agent_id: AgentId,
        timestamp: DateTime<Utc>,
    },
    /// A task was completed.
    TaskCompleted {
        task_id: uuid::Uuid,
        agent_id: AgentId,
        success: bool,
        duration_ms: u64,
        timestamp: DateTime<Utc>,
    },
    /// A task failed.
    TaskFailed {
        task_id: uuid::Uuid,
        agent_id: AgentId,
        error: String,
        timestamp: DateTime<Utc>,
    },
    /// A message was sent.
    MessageSent {
        from: AgentId,
        to: Option<AgentId>,
        message_type: String,
        timestamp: DateTime<Utc>,
    },
    /// A message was received.
    MessageReceived {
        from: AgentId,
        to: AgentId,
        message_type: String,
        timestamp: DateTime<Utc>,
    },
    /// A supervisor alert was raised.
    SupervisorAlert {
        agent_id: AgentId,
        severity: String,
        message: String,
        timestamp: DateTime<Utc>,
    },
}

impl AgentEvent {
    /// Get the timestamp of this event.
    #[must_use]
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::AgentCreated { timestamp, .. }
            | Self::AgentStarted { timestamp, .. }
            | Self::AgentStopped { timestamp, .. }
            | Self::AgentPaused { timestamp, .. }
            | Self::AgentResumed { timestamp, .. }
            | Self::AgentFailed { timestamp, .. }
            | Self::AgentRecovered { timestamp, .. }
            | Self::TaskAssigned { timestamp, .. }
            | Self::TaskCompleted { timestamp, .. }
            | Self::TaskFailed { timestamp, .. }
            | Self::MessageSent { timestamp, .. }
            | Self::MessageReceived { timestamp, .. }
            | Self::SupervisorAlert { timestamp, .. } => *timestamp,
        }
    }

    /// Get the event type as a string.
    #[must_use]
    pub fn event_type(&self) -> &str {
        match self {
            Self::AgentCreated { .. } => "agent_created",
            Self::AgentStarted { .. } => "agent_started",
            Self::AgentStopped { .. } => "agent_stopped",
            Self::AgentPaused { .. } => "agent_paused",
            Self::AgentResumed { .. } => "agent_resumed",
            Self::AgentFailed { .. } => "agent_failed",
            Self::AgentRecovered { .. } => "agent_recovered",
            Self::TaskAssigned { .. } => "task_assigned",
            Self::TaskCompleted { .. } => "task_completed",
            Self::TaskFailed { .. } => "task_failed",
            Self::MessageSent { .. } => "message_sent",
            Self::MessageReceived { .. } => "message_received",
            Self::SupervisorAlert { .. } => "supervisor_alert",
        }
    }
}

// ---------------------------------------------------------------------------
// AgentEventBus
// ---------------------------------------------------------------------------

/// Event bus for the agent framework.
///
/// Provides publish/subscribe capabilities for agent events, integrating
/// with the broader Neo event system.
pub struct AgentEventBus {
    /// Broadcast channel for events.
    sender: broadcast::Sender<AgentEvent>,
    /// Event history for replay.
    history: DashMap<String, Vec<AgentEvent>>,
    /// Maximum history size per event type.
    max_history_per_type: usize,
}

impl AgentEventBus {
    /// Create a new event bus.
    #[must_use]
    pub fn new(channel_capacity: usize, max_history_per_type: usize) -> Self {
        let (sender, _) = broadcast::channel(channel_capacity);
        Self {
            sender,
            history: DashMap::new(),
            max_history_per_type,
        }
    }

    /// Publish an event.
    pub fn publish(&self, event: AgentEvent) {
        let event_type = event.event_type().to_string();

        // Add to history
        let mut history = self.history.entry(event_type).or_default();
        if history.len() >= self.max_history_per_type {
            history.remove(0);
        }
        history.push(event.clone());

        // Broadcast
        let _ = self.sender.send(event);
    }

    /// Subscribe to events.
    pub fn subscribe(&self) -> broadcast::Receiver<AgentEvent> {
        self.sender.subscribe()
    }

    /// Get event history for a specific event type.
    pub fn get_history(&self, event_type: &str) -> Vec<AgentEvent> {
        self.history
            .get(event_type)
            .map(|h| h.clone())
            .unwrap_or_default()
    }

    /// Get all event types that have been published.
    #[must_use]
    pub fn event_types(&self) -> Vec<String> {
        self.history.iter().map(|e| e.key().clone()).collect()
    }

    /// Get total event count in history.
    #[must_use]
    pub fn total_history_count(&self) -> usize {
        self.history.iter().map(|e| e.value().len()).sum()
    }
}

impl Default for AgentEventBus {
    fn default() -> Self {
        Self::new(1024, 100)
    }
}

// ---------------------------------------------------------------------------
// EventRecorder
// ---------------------------------------------------------------------------

/// Records events with optional persistence callbacks.
pub struct EventRecorder {
    /// The event bus.
    pub bus: Arc<AgentEventBus>,
    /// Total events recorded.
    total_recorded: std::sync::atomic::AtomicU64,
}

impl EventRecorder {
    /// Create a new event recorder.
    #[must_use]
    pub fn new(bus: Arc<AgentEventBus>) -> Self {
        Self {
            bus,
            total_recorded: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Record an agent created event.
    pub fn agent_created(&self, agent_id: AgentId, name: String) {
        self.bus.publish(AgentEvent::AgentCreated {
            agent_id,
            name,
            timestamp: Utc::now(),
        });
        self.total_recorded
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record an agent started event.
    pub fn agent_started(&self, agent_id: AgentId) {
        self.bus.publish(AgentEvent::AgentStarted {
            agent_id,
            timestamp: Utc::now(),
        });
        self.total_recorded
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record an agent stopped event.
    pub fn agent_stopped(&self, agent_id: AgentId) {
        self.bus.publish(AgentEvent::AgentStopped {
            agent_id,
            timestamp: Utc::now(),
        });
        self.total_recorded
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record an agent failed event.
    pub fn agent_failed(&self, agent_id: AgentId, error: String) {
        self.bus.publish(AgentEvent::AgentFailed {
            agent_id,
            error,
            timestamp: Utc::now(),
        });
        self.total_recorded
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record an agent recovered event.
    pub fn agent_recovered(&self, agent_id: AgentId) {
        self.bus.publish(AgentEvent::AgentRecovered {
            agent_id,
            timestamp: Utc::now(),
        });
        self.total_recorded
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record a task assigned event.
    pub fn task_assigned(&self, task_id: uuid::Uuid, agent_id: AgentId) {
        self.bus.publish(AgentEvent::TaskAssigned {
            task_id,
            agent_id,
            timestamp: Utc::now(),
        });
        self.total_recorded
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record a task completed event.
    pub fn task_completed(
        &self,
        task_id: uuid::Uuid,
        agent_id: AgentId,
        success: bool,
        duration_ms: u64,
    ) {
        self.bus.publish(AgentEvent::TaskCompleted {
            task_id,
            agent_id,
            success,
            duration_ms,
            timestamp: Utc::now(),
        });
        self.total_recorded
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record a task failed event.
    pub fn task_failed(&self, task_id: uuid::Uuid, agent_id: AgentId, error: String) {
        self.bus.publish(AgentEvent::TaskFailed {
            task_id,
            agent_id,
            error,
            timestamp: Utc::now(),
        });
        self.total_recorded
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record a message sent event.
    pub fn message_sent(&self, from: AgentId, to: Option<AgentId>, message_type: String) {
        self.bus.publish(AgentEvent::MessageSent {
            from,
            to,
            message_type,
            timestamp: Utc::now(),
        });
        self.total_recorded
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record a message received event.
    pub fn message_received(&self, from: AgentId, to: AgentId, message_type: String) {
        self.bus.publish(AgentEvent::MessageReceived {
            from,
            to,
            message_type,
            timestamp: Utc::now(),
        });
        self.total_recorded
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record a supervisor alert event.
    pub fn supervisor_alert(&self, agent_id: AgentId, severity: String, message: String) {
        self.bus.publish(AgentEvent::SupervisorAlert {
            agent_id,
            severity,
            message,
            timestamp: Utc::now(),
        });
        self.total_recorded
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get the total number of events recorded.
    #[must_use]
    pub fn total_recorded(&self) -> u64 {
        self.total_recorded
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_bus_publish_subscribe() {
        let bus = AgentEventBus::new(64, 100);
        let mut rx = bus.subscribe();

        let agent = AgentId::new();
        bus.publish(AgentEvent::AgentCreated {
            agent_id: agent,
            name: "test".to_string(),
            timestamp: Utc::now(),
        });

        let event = rx.try_recv().unwrap();
        assert_eq!(event.event_type(), "agent_created");
    }

    #[test]
    fn test_event_history() {
        let bus = AgentEventBus::new(64, 10);
        let agent = AgentId::new();

        for _ in 0..5 {
            bus.publish(AgentEvent::AgentStarted {
                agent_id: agent,
                timestamp: Utc::now(),
            });
        }

        let history = bus.get_history("agent_started");
        assert_eq!(history.len(), 5);
    }

    #[test]
    fn test_event_recorder() {
        let bus = Arc::new(AgentEventBus::new(64, 100));
        let recorder = EventRecorder::new(bus.clone());

        let agent = AgentId::new();
        recorder.agent_created(agent, "test".to_string());
        recorder.agent_started(agent);
        recorder.task_completed(uuid::Uuid::new_v4(), agent, true, 100);

        assert_eq!(recorder.total_recorded(), 3);
        assert_eq!(bus.total_history_count(), 3);
    }
}
