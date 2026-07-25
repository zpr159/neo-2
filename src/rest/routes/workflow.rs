use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::rest::error::RestError;
use crate::rest::NeoAppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStartRequest {
    pub workflow_type: String,
    pub parameters: Option<serde_json::Value>,
    pub trigger: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStartResponse {
    pub workflow_id: String,
    pub status: String,
    pub started_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCancelRequest {
    pub workflow_id: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStatusResponse {
    pub workflow_id: String,
    pub status: String,
    pub progress: f64,
    pub steps_completed: usize,
    pub steps_total: usize,
    pub started_at: String,
}

pub async fn start_workflow_handler(
    State(_state): State<NeoAppState>,
    Json(request): Json<WorkflowStartRequest>,
) -> Result<Json<WorkflowStartResponse>, RestError> {
    info!("Starting workflow of type: {}", request.workflow_type);

    Ok(Json(WorkflowStartResponse {
        workflow_id: uuid::Uuid::new_v4().to_string(),
        status: "started".to_string(),
        started_at: chrono::Utc::now().to_rfc3339(),
    }))
}

pub async fn cancel_workflow_handler(
    State(_state): State<NeoAppState>,
    Json(request): Json<WorkflowCancelRequest>,
) -> Result<Json<serde_json::Value>, RestError> {
    info!("Cancelling workflow: {} (reason: {:?})", request.workflow_id, request.reason);

    Ok(Json(serde_json::json!({
        "cancelled": true,
        "workflow_id": request.workflow_id,
    })))
}

pub async fn get_workflow_status_handler(
    State(_state): State<NeoAppState>,
) -> Result<Json<Vec<WorkflowStatusResponse>>, RestError> {
    info!("Getting workflow statuses");
    Ok(Json(vec![]))
}
