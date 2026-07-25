use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::rest::error::RestError;
use crate::rest::NeoAppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub agent_type: String,
    pub status: String,
    pub started_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStartRequest {
    pub agent_id: String,
    pub config: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStopRequest {
    pub agent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatusResponse {
    pub agent_id: String,
    pub status: String,
    pub uptime_secs: u64,
    pub metrics: Option<serde_json::Value>,
}

pub async fn list_agents_handler(
    State(_state): State<NeoAppState>,
) -> Result<Json<Vec<AgentInfo>>, RestError> {
    info!("Listing agents");
    Ok(Json(vec![]))
}

pub async fn start_agent_handler(
    State(_state): State<NeoAppState>,
    Json(request): Json<AgentStartRequest>,
) -> Result<Json<AgentInfo>, RestError> {
    info!("Starting agent: {}", request.agent_id);

    Ok(Json(AgentInfo {
        id: request.agent_id,
        name: "Agent".to_string(),
        agent_type: "default".to_string(),
        status: "running".to_string(),
        started_at: Some(chrono::Utc::now().to_rfc3339()),
    }))
}

pub async fn stop_agent_handler(
    State(_state): State<NeoAppState>,
    Json(request): Json<AgentStopRequest>,
) -> Result<Json<serde_json::Value>, RestError> {
    info!("Stopping agent: {}", request.agent_id);

    Ok(Json(serde_json::json!({
        "stopped": true,
        "agent_id": request.agent_id,
    })))
}

pub async fn get_agent_status_handler(
    State(_state): State<NeoAppState>,
) -> Result<Json<Vec<AgentStatusResponse>>, RestError> {
    info!("Getting agent statuses");
    Ok(Json(vec![]))
}
