use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::WorkflowResult;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EventId(String);

impl EventId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EventType {
    WorkflowStarted,
    WorkflowCompleted,
    WorkflowFailed,
    WorkflowCancelled,
    NodeStarted,
    NodeCompleted,
    NodeFailed,
    NodeRetried,
    VariableChanged,
    CompensationTriggered,
    CheckpointCreated,
    CheckpointRestored,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEvent {
    pub id: EventId,
    pub event_type: EventType,
    pub workflow_id: crate::core::WorkflowId,
    pub execution_id: crate::core::ExecutionId,
    pub node_id: Option<crate::core::NodeId>,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

pub type EventHandler = Box<dyn Fn(&WorkflowEvent) + Send + Sync>;

/// Event system for workflow lifecycle events.
#[derive(Default)]
pub struct WorkflowEventSystem {
    handlers: HashMap<EventType, Vec<Box<dyn Fn(&WorkflowEvent) + Send + Sync>>>,
    history: Vec<WorkflowEvent>,
}

impl std::fmt::Debug for WorkflowEventSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkflowEventSystem")
            .field("handlers", &self.handlers.len())
            .field("history", &self.history.len())
            .finish()
    }
}

impl WorkflowEventSystem {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on(&mut self, event_type: EventType, handler: EventHandler) {
        self.handlers.entry(event_type).or_default().push(handler);
    }

    pub fn emit(&mut self, event: WorkflowEvent) {
        if let Some(handlers) = self.handlers.get(&event.event_type) {
            for handler in handlers.iter() {
                handler(&event);
            }
        }
        self.history.push(event);
    }

    #[must_use]
    pub fn history(&self) -> &[WorkflowEvent] {
        &self.history
    }

    #[must_use]
    pub fn history_for_workflow(
        &self,
        workflow_id: &crate::core::WorkflowId,
    ) -> Vec<&WorkflowEvent> {
        self.history
            .iter()
            .filter(|e| e.workflow_id == *workflow_id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{ExecutionId, WorkflowId};

    #[test]
    fn emit_and_history() {
        let mut system = WorkflowEventSystem::new();
        let wf_id = WorkflowId::new();
        let exec_id = ExecutionId::new();
        let event = WorkflowEvent {
            id: EventId::new(),
            event_type: EventType::WorkflowStarted,
            workflow_id: wf_id,
            execution_id: exec_id,
            node_id: None,
            payload: serde_json::Value::Null,
            timestamp: Utc::now(),
        };
        system.emit(event);
        assert_eq!(system.history().len(), 1);
    }

    #[test]
    fn handler_receives_events() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let mut system = WorkflowEventSystem::new();
        system.on(
            EventType::WorkflowStarted,
            Box::new(move |_| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            }),
        );

        for _ in 0..3 {
            system.emit(WorkflowEvent {
                id: EventId::new(),
                event_type: EventType::WorkflowStarted,
                workflow_id: WorkflowId::new(),
                execution_id: ExecutionId::new(),
                node_id: None,
                payload: serde_json::Value::Null,
                timestamp: Utc::now(),
            });
        }

        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }
}
