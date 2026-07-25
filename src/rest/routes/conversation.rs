use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::rest::error::RestError;
use crate::rest::NeoAppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub session_id: Option<String>,
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub response: String,
    pub session_id: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamRequest {
    pub message: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamResponse {
    pub session_id: String,
    pub stream_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCreateRequest {
    pub agent_id: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResponse {
    pub session_id: String,
    pub created_at: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryResponse {
    pub session_id: String,
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

pub async fn chat_handler(
    State(_state): State<NeoAppState>,
    Json(request): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, RestError> {
    info!("Received chat request: {}", &request.message[..request.message.len().min(50)]);

    let session_id = request.session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    Ok(Json(ChatResponse {
        response: format!("Echo: {}", request.message),
        session_id,
        timestamp: chrono::Utc::now().to_rfc3339(),
    }))
}

pub async fn stream_handler(
    State(_state): State<NeoAppState>,
    Json(request): Json<StreamRequest>,
) -> Result<Json<StreamResponse>, RestError> {
    info!("Received stream request for session: {:?}", request.session_id);

    let session_id = request.session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    Ok(Json(StreamResponse {
        session_id,
        stream_id: uuid::Uuid::new_v4().to_string(),
    }))
}

pub async fn create_session_handler(
    State(_state): State<NeoAppState>,
    Json(request): Json<SessionCreateRequest>,
) -> Result<Json<SessionResponse>, RestError> {
    info!("Creating new session for agent: {:?}", request.agent_id);

    Ok(Json(SessionResponse {
        session_id: uuid::Uuid::new_v4().to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        status: "active".to_string(),
    }))
}

pub async fn get_session_handler(
    State(_state): State<NeoAppState>,
    Path(session_id): Path<String>,
) -> Result<Json<SessionResponse>, RestError> {
    info!("Getting session: {}", session_id);

    Ok(Json(SessionResponse {
        session_id,
        created_at: chrono::Utc::now().to_rfc3339(),
        status: "active".to_string(),
    }))
}

pub async fn delete_session_handler(
    State(_state): State<NeoAppState>,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, RestError> {
    info!("Deleting session: {}", session_id);

    Ok(Json(serde_json::json!({
        "deleted": true,
        "session_id": session_id,
    })))
}

pub async fn get_history_handler(
    State(_state): State<NeoAppState>,
    Path(session_id): Path<String>,
) -> Result<Json<HistoryResponse>, RestError> {
    info!("Getting history for session: {}", session_id);

    Ok(Json(HistoryResponse {
        session_id,
        messages: vec![],
    }))
}
