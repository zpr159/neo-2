use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{ApiError, PaginationParams};

/// Chat request payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub session_id: Option<String>,
    pub conversation_id: Option<String>,
    pub message: String,
    pub stream: bool,
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

/// Chat response payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub conversation_id: String,
    pub session_id: String,
    pub message: String,
    pub tool_calls: Option<Vec<crate::language::types::ToolCall>>,
    pub metadata: std::collections::HashMap<String, String>,
}

/// Session creation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub user_id: Option<String>,
}

/// Session information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub conversation_ids: Vec<String>,
    pub active_conversation: Option<String>,
    pub created_at: String,
}

/// Conversation history entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

/// Conversation API trait.
#[async_trait]
pub trait ConversationApi: Send + Sync {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ApiError>;
    async fn stream_chat(&self, request: ChatRequest) -> Result<tokio::sync::mpsc::Receiver<String>, ApiError>;
    async fn create_session(&self, request: CreateSessionRequest) -> Result<SessionInfo, ApiError>;
    async fn get_session(&self, session_id: &str) -> Result<SessionInfo, ApiError>;
    async fn delete_session(&self, session_id: &str) -> Result<(), ApiError>;
    async fn get_history(&self, conversation_id: &str, pagination: PaginationParams) -> Result<Vec<HistoryEntry>, ApiError>;
    async fn cancel_conversation(&self, session_id: &str, conversation_id: &str) -> Result<(), ApiError>;
}
