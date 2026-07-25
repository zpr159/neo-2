use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::conversation::error::ConversationResult;
use crate::conversation::types::ConversationContext;
use crate::id::AgentId;

/// Agent status.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Available,
    Busy,
    Error,
    Offline,
}

/// Information about a discovered agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: AgentId,
    pub name: String,
    pub description: String,
    pub status: AgentStatus,
    pub capabilities: Vec<String>,
    pub current_task: Option<String>,
}

/// Result from agent execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    pub agent_id: AgentId,
    pub status: AgentStatus,
    pub output: serde_json::Value,
    pub execution_time_ms: u64,
    pub confidence: f32,
}

/// Bridge between the Agent Framework subsystem and the Conversation layer.
#[async_trait]
pub trait AgentConversationBridge: Send + Sync {
    /// Discover available agents matching a capability query.
    async fn discover_agents(
        &self,
        context: &ConversationContext,
        capabilities: &[String],
    ) -> ConversationResult<Vec<AgentInfo>>;

    /// Assign an objective to an agent.
    async fn assign_objective(
        &self,
        context: &ConversationContext,
        agent_id: AgentId,
        objective: &str,
    ) -> ConversationResult<AgentResult>;

    /// Get agent status.
    async fn get_status(
        &self,
        context: &ConversationContext,
        agent_id: AgentId,
    ) -> ConversationResult<AgentInfo>;

    /// Coordinate multiple agents to accomplish a task.
    async fn coordinate_agents(
        &self,
        context: &ConversationContext,
        agent_ids: &[AgentId],
        objective: &str,
    ) -> ConversationResult<Vec<AgentResult>>;

    /// Aggregate results from multiple agents.
    async fn aggregate_results(
        &self,
        context: &ConversationContext,
        results: &[AgentResult],
    ) -> ConversationResult<serde_json::Value>;

    /// Monitor agent execution.
    async fn monitor_execution(
        &self,
        context: &ConversationContext,
        agent_id: AgentId,
    ) -> ConversationResult<AgentStatus>;
}

/// Mock implementation for testing.
pub struct MockAgentBridge;

#[async_trait]
impl AgentConversationBridge for MockAgentBridge {
    async fn discover_agents(
        &self,
        _context: &ConversationContext,
        _capabilities: &[String],
    ) -> ConversationResult<Vec<AgentInfo>> {
        Ok(Vec::new())
    }

    async fn assign_objective(
        &self,
        _context: &ConversationContext,
        agent_id: AgentId,
        _objective: &str,
    ) -> ConversationResult<AgentResult> {
        Ok(AgentResult {
            agent_id,
            status: AgentStatus::Available,
            output: serde_json::Value::Null,
            execution_time_ms: 0,
            confidence: 0.0,
        })
    }

    async fn get_status(
        &self,
        _context: &ConversationContext,
        agent_id: AgentId,
    ) -> ConversationResult<AgentInfo> {
        Ok(AgentInfo {
            id: agent_id,
            name: "mock-agent".to_string(),
            description: "Mock agent".to_string(),
            status: AgentStatus::Available,
            capabilities: Vec::new(),
            current_task: None,
        })
    }

    async fn coordinate_agents(
        &self,
        _context: &ConversationContext,
        agent_ids: &[AgentId],
        _objective: &str,
    ) -> ConversationResult<Vec<AgentResult>> {
        Ok(agent_ids
            .iter()
            .map(|id| AgentResult {
                agent_id: *id,
                status: AgentStatus::Available,
                output: serde_json::Value::Null,
                execution_time_ms: 0,
                confidence: 0.0,
            })
            .collect())
    }

    async fn aggregate_results(
        &self,
        _context: &ConversationContext,
        _results: &[AgentResult],
    ) -> ConversationResult<serde_json::Value> {
        Ok(serde_json::Value::Null)
    }

    async fn monitor_execution(
        &self,
        _context: &ConversationContext,
        _agent_id: AgentId,
    ) -> ConversationResult<AgentStatus> {
        Ok(AgentStatus::Available)
    }
}
