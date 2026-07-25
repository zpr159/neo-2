//! Planning events for the runtime event bus.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::error::{PlanningError, PlanningErrorCode, PlanningResult};
use crate::id::{PlanId, PlanningGoalId, StrategyId};

/// Types of planning events emitted on the event bus.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlanningEventType {
    PlanningStarted,
    GoalCreated,
    GoalDecomposed,
    PlanGenerated,
    StrategySelected,
    PlanOptimized,
    PlanValidated,
    ExecutionStarted,
    ExecutionCompleted,
    ExecutionFailed,
    PlanCancelled,
    PlanArchived,
    ReplanningTriggered,
}

impl std::fmt::Display for PlanningEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PlanningStarted => write!(f, "PlanningStarted"),
            Self::GoalCreated => write!(f, "GoalCreated"),
            Self::GoalDecomposed => write!(f, "GoalDecomposed"),
            Self::PlanGenerated => write!(f, "PlanGenerated"),
            Self::StrategySelected => write!(f, "StrategySelected"),
            Self::PlanOptimized => write!(f, "PlanOptimized"),
            Self::PlanValidated => write!(f, "PlanValidated"),
            Self::ExecutionStarted => write!(f, "ExecutionStarted"),
            Self::ExecutionCompleted => write!(f, "ExecutionCompleted"),
            Self::ExecutionFailed => write!(f, "ExecutionFailed"),
            Self::PlanCancelled => write!(f, "PlanCancelled"),
            Self::PlanArchived => write!(f, "PlanArchived"),
            Self::ReplanningTriggered => write!(f, "ReplanningTriggered"),
        }
    }
}

/// A single event on the planning event bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningEvent {
    /// The type of event.
    pub event_type: PlanningEventType,
    /// The associated plan, if any.
    pub plan_id: Option<PlanId>,
    /// The associated goal, if any.
    pub goal_id: Option<PlanningGoalId>,
    /// The associated strategy, if any.
    pub strategy_id: Option<StrategyId>,
    /// When the event occurred.
    pub timestamp: DateTime<Utc>,
    /// Arbitrary payload.
    pub payload: serde_json::Value,
    /// The component that produced the event.
    pub source: String,
}

impl PlanningEvent {
    /// Create a new event with the given type and source.
    pub fn new(event_type: PlanningEventType, source: impl Into<String>) -> Self {
        Self {
            event_type,
            plan_id: None,
            goal_id: None,
            strategy_id: None,
            timestamp: Utc::now(),
            payload: serde_json::Value::Null,
            source: source.into(),
        }
    }

    /// Attach a plan id.
    #[must_use]
    pub fn with_plan_id(mut self, plan_id: PlanId) -> Self {
        self.plan_id = Some(plan_id);
        self
    }

    /// Attach a goal id.
    #[must_use]
    pub fn with_goal_id(mut self, goal_id: PlanningGoalId) -> Self {
        self.goal_id = Some(goal_id);
        self
    }

    /// Attach a strategy id.
    #[must_use]
    pub fn with_strategy_id(mut self, strategy_id: StrategyId) -> Self {
        self.strategy_id = Some(strategy_id);
        self
    }

    /// Set the payload.
    #[must_use]
    pub fn with_payload(mut self, payload: serde_json::Value) -> Self {
        self.payload = payload;
        self
    }
}

/// Thread-safe event bus built on top of [`tokio::sync::broadcast`].
#[derive(Clone)]
pub struct EventBus {
    sender: Arc<broadcast::Sender<PlanningEvent>>,
}

impl EventBus {
    /// Create a new event bus with the given channel capacity.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender: Arc::new(sender),
        }
    }

    /// Publish an event on the bus.
    pub fn publish(&self, event: PlanningEvent) -> PlanningResult<()> {
        self.sender.send(event).map_err(|_| {
            PlanningError::new(
                PlanningErrorCode::InternalError,
                "event bus publish failed: no active receivers",
            )
        })?;
        Ok(())
    }

    /// Subscribe to events. Returns a receiver that will yield all future
    /// events published after the subscription is created.
    pub fn subscribe(&self) -> broadcast::Receiver<PlanningEvent> {
        self.sender.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_type_display() {
        assert_eq!(
            PlanningEventType::PlanningStarted.to_string(),
            "PlanningStarted"
        );
        assert_eq!(
            PlanningEventType::ReplanningTriggered.to_string(),
            "ReplanningTriggered"
        );
    }

    #[test]
    fn event_creation() {
        let event = PlanningEvent::new(PlanningEventType::GoalCreated, "test");
        assert_eq!(event.event_type, PlanningEventType::GoalCreated);
        assert!(event.plan_id.is_none());
        assert!(event.goal_id.is_none());
        assert!(event.strategy_id.is_none());
        assert_eq!(event.source, "test");
        assert_eq!(event.payload, serde_json::Value::Null);
    }

    #[test]
    fn event_builder_chain() {
        let plan_id = PlanId::new();
        let goal_id = PlanningGoalId::new();
        let strategy_id = StrategyId::new();

        let event = PlanningEvent::new(PlanningEventType::PlanGenerated, "planner")
            .with_plan_id(plan_id)
            .with_goal_id(goal_id)
            .with_strategy_id(strategy_id)
            .with_payload(serde_json::json!({"key": "value"}));

        assert_eq!(event.plan_id, Some(plan_id));
        assert_eq!(event.goal_id, Some(goal_id));
        assert_eq!(event.strategy_id, Some(strategy_id));
        assert_eq!(event.payload, serde_json::json!({"key": "value"}));
    }

    #[test]
    fn event_serialization_roundtrip() {
        let event = PlanningEvent::new(PlanningEventType::PlanOptimized, "optimizer")
            .with_plan_id(PlanId::new())
            .with_payload(serde_json::json!(42));

        let json = serde_json::to_string(&event).unwrap();
        let back: PlanningEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.event_type, PlanningEventType::PlanOptimized);
        assert_eq!(back.source, "optimizer");
        assert_eq!(back.payload, serde_json::json!(42));
    }

    #[test]
    fn event_bus_publish_subscribe() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let bus = EventBus::new(16);
            let mut rx = bus.subscribe();

            let event = PlanningEvent::new(PlanningEventType::ExecutionStarted, "executor");
            bus.publish(event.clone()).unwrap();

            let received = rx.recv().await.unwrap();
            assert_eq!(received.event_type, PlanningEventType::ExecutionStarted);
            assert_eq!(received.source, "executor");
        });
    }

    #[test]
    fn event_bus_multiple_subscribers() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let bus = EventBus::new(16);
            let mut rx1 = bus.subscribe();
            let mut rx2 = bus.subscribe();

            let event = PlanningEvent::new(PlanningEventType::PlanCancelled, "admin");
            bus.publish(event).unwrap();

            let r1 = rx1.recv().await.unwrap();
            let r2 = rx2.recv().await.unwrap();
            assert_eq!(r1.event_type, PlanningEventType::PlanCancelled);
            assert_eq!(r2.event_type, PlanningEventType::PlanCancelled);
        });
    }

    #[test]
    fn event_bus_publish_no_receivers() {
        let bus = EventBus::new(16);
        let event = PlanningEvent::new(PlanningEventType::PlanArchived, "cron");
        let result = bus.publish(event);
        assert!(result.is_err());
    }

    #[test]
    fn event_bus_clone_shares_state() {
        let bus1 = EventBus::new(16);
        let bus2 = bus1.clone();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut rx = bus2.subscribe();

            let event = PlanningEvent::new(PlanningEventType::PlanValidated, "validator");
            bus1.publish(event).unwrap();

            let received = rx.recv().await.unwrap();
            assert_eq!(received.event_type, PlanningEventType::PlanValidated);
        });
    }
}
