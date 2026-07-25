use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::ApiError;

/// Workflow information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub steps_completed: usize,
    pub total_steps: usize,
    pub created_at: String,
}

/// Workflow status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStatus {
    pub id: String,
    pub status: String,
    pub progress: f64,
    pub current_step: Option<String>,
    pub error: Option<String>,
}

/// Workflow API trait.
#[async_trait]
pub trait WorkflowApi: Send + Sync {
    async fn start_workflow(&self, name: &str, parameters: std::collections::HashMap<String, serde_json::Value>) -> Result<WorkflowInfo, ApiError>;
    async fn cancel_workflow(&self, workflow_id: &str) -> Result<(), ApiError>;
    async fn get_status(&self, workflow_id: &str) -> Result<WorkflowStatus, ApiError>;
}
