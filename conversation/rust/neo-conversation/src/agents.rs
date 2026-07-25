use crate::error::ConversationResult;

/// Interface to the agent framework.
///
/// Manages autonomous agents that can perform tasks
/// on behalf of the user during conversation.
pub trait AgentInterface: Send + Sync {
    /// List available agents.
    fn list_agents(&self) -> ConversationResult<Vec<AgentInfo>>;

    /// Dispatch a task to an agent.
    fn dispatch(&self, agent_id: &str, task: String) -> ConversationResult<String>;

    /// Get agent output.
    fn output(&self, task_id: &str) -> ConversationResult<Option<AgentOutput>>;
}

/// Information about an agent.
#[derive(Debug, Clone)]
pub struct AgentInfo {
    pub agent_id: String,
    pub name: String,
    pub capabilities: Vec<String>,
    pub status: String,
}

/// Output from an agent task.
#[derive(Debug, Clone)]
pub struct AgentOutput {
    pub task_id: String,
    pub agent_id: String,
    pub result: String,
    pub success: bool,
    pub completed_at: String,
}

/// Default agent interface.
pub struct DefaultAgentInterface;

impl AgentInterface for DefaultAgentInterface {
    fn list_agents(&self) -> ConversationResult<Vec<AgentInfo>> {
        Ok(Vec::new())
    }

    fn dispatch(&self, agent_id: &str, _task: String) -> ConversationResult<String> {
        let task_id = uuid::Uuid::new_v4().to_string();
        tracing::info!("Task dispatched to agent {agent_id}: task_id={task_id}");
        Ok(task_id)
    }

    fn output(&self, _task_id: &str) -> ConversationResult<Option<AgentOutput>> {
        Ok(None)
    }
}
