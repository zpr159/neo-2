use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::ApiError;

/// Agent information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub capabilities: Vec<String>,
    pub created_at: String,
}

/// Agent status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatusDetail {
    pub id: String,
    pub status: String,
    pub current_task: Option<String>,
    pub tasks_completed: usize,
    pub uptime_secs: u64,
}

/// Agent API trait.
#[async_trait]
pub trait AgentApi: Send + Sync {
    async fn list_agents(&self) -> Result<Vec<AgentInfo>, ApiError>;
    async fn start_agent(&self, agent_id: &str) -> Result<AgentInfo, ApiError>;
    async fn stop_agent(&self, agent_id: &str) -> Result<(), ApiError>;
    async fn get_status(&self, agent_id: &str) -> Result<AgentStatusDetail, ApiError>;
}
