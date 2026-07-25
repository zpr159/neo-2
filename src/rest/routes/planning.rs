use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::rest::error::RestError;
use crate::rest::NeoAppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanCreateRequest {
    pub name: String,
    pub description: Option<String>,
    pub goals: Vec<String>,
    pub constraints: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub goals: Vec<String>,
    pub constraints: Vec<String>,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub step_id: String,
    pub action: String,
    pub status: String,
    pub dependencies: Vec<String>,
}

pub async fn create_plan_handler(
    State(_state): State<NeoAppState>,
    Json(request): Json<PlanCreateRequest>,
) -> Result<Json<PlanResponse>, RestError> {
    info!("Creating plan: {}", request.name);

    Ok(Json(PlanResponse {
        id: uuid::Uuid::new_v4().to_string(),
        name: request.name,
        description: request.description,
        goals: request.goals,
        constraints: request.constraints.unwrap_or_default(),
        status: "created".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    }))
}

pub async fn get_plan_handler(
    State(_state): State<NeoAppState>,
    Path(plan_id): Path<String>,
) -> Result<Json<PlanResponse>, RestError> {
    info!("Getting plan: {}", plan_id);

    Ok(Json(PlanResponse {
        id: plan_id,
        name: "Unknown Plan".to_string(),
        description: None,
        goals: vec![],
        constraints: vec![],
        status: "active".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    }))
}

pub async fn delete_plan_handler(
    State(_state): State<NeoAppState>,
    Path(plan_id): Path<String>,
) -> Result<Json<serde_json::Value>, RestError> {
    info!("Deleting plan: {}", plan_id);

    Ok(Json(serde_json::json!({
        "deleted": true,
        "plan_id": plan_id,
    })))
}
