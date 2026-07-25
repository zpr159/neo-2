use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::ApiError;

/// Planning task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanTask {
    pub id: String,
    pub description: String,
    pub dependencies: Vec<String>,
    pub estimated_cost: f64,
    pub status: String,
}

/// Plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub goal: String,
    pub tasks: Vec<PlanTask>,
    pub total_cost: f64,
    pub status: String,
    pub created_at: String,
}

/// Plan creation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePlanRequest {
    pub goal: String,
    pub constraints: Vec<String>,
    pub max_depth: Option<usize>,
}

/// Planning API trait.
#[async_trait]
pub trait PlanningApi: Send + Sync {
    async fn create_plan(&self, request: CreatePlanRequest) -> Result<Plan, ApiError>;
    async fn get_plan(&self, plan_id: &str) -> Result<Plan, ApiError>;
    async fn delete_plan(&self, plan_id: &str) -> Result<(), ApiError>;
}
