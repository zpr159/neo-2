use crate::error::ConversationResult;

/// Interface to the executive subsystem.
///
/// Manages task scheduling, resource allocation, and
/// high-level decision-making for autonomous operations.
pub trait ExecutiveInterface: Send + Sync {
    /// Get executive decisions relevant to the current context.
    fn decisions(&self, context: &str) -> ConversationResult<Vec<ExecutiveDecision>>;

    /// Request an action from the executive.
    fn request_action(&self, action: ExecutiveAction) -> ConversationResult<ExecutiveDecision>;

    /// Get the current task queue status.
    fn task_status(&self) -> ConversationResult<Vec<TaskStatus>>;
}

/// A decision made by the executive.
#[derive(Debug, Clone)]
pub struct ExecutiveDecision {
    pub decision_id: String,
    pub action: String,
    pub reasoning: String,
    pub priority: u8,
    pub timestamp: String,
}

/// An action requested from the executive.
#[derive(Debug, Clone)]
pub struct ExecutiveAction {
    pub action_type: String,
    pub parameters: std::collections::HashMap<String, String>,
    pub priority: u8,
}

/// Status of a task.
#[derive(Debug, Clone)]
pub struct TaskStatus {
    pub task_id: String,
    pub description: String,
    pub status: String,
    pub progress: f64,
}

/// Default executive interface.
pub struct DefaultExecutive;

impl ExecutiveInterface for DefaultExecutive {
    fn decisions(&self, _context: &str) -> ConversationResult<Vec<ExecutiveDecision>> {
        Ok(Vec::new())
    }

    fn request_action(&self, action: ExecutiveAction) -> ConversationResult<ExecutiveDecision> {
        Ok(ExecutiveDecision {
            decision_id: uuid::Uuid::new_v4().to_string(),
            action: action.action_type,
            reasoning: "Default executive accepted action".into(),
            priority: action.priority,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }

    fn task_status(&self) -> ConversationResult<Vec<TaskStatus>> {
        Ok(Vec::new())
    }
}
