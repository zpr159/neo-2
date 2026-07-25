use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::rest::error::RestError;
use crate::rest::NeoAppState;

#[derive(Debug, Clone, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub memory_type: String,
    pub metadata: Option<serde_json::Value>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub entries: Vec<MemoryEntry>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreRequest {
    pub content: String,
    pub memory_type: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreResponse {
    pub id: String,
    pub stored: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStatistics {
    pub total_entries: usize,
    pub memory_types: std::collections::HashMap<String, usize>,
    pub total_size_bytes: usize,
}

pub async fn search_handler(
    State(_state): State<NeoAppState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<SearchResult>, RestError> {
    info!("Searching memory: {}", &query.query[..query.query.len().min(50)]);

    Ok(Json(SearchResult {
        entries: vec![],
        total: 0,
    }))
}

pub async fn store_handler(
    State(_state): State<NeoAppState>,
    Json(request): Json<StoreRequest>,
) -> Result<Json<StoreResponse>, RestError> {
    info!("Storing memory of type: {}", request.memory_type);

    Ok(Json(StoreResponse {
        id: uuid::Uuid::new_v4().to_string(),
        stored: true,
    }))
}

pub async fn delete_handler(
    State(_state): State<NeoAppState>,
    Path(memory_id): Path<String>,
) -> Result<Json<serde_json::Value>, RestError> {
    info!("Deleting memory: {}", memory_id);

    Ok(Json(serde_json::json!({
        "deleted": true,
        "id": memory_id,
    })))
}

pub async fn statistics_handler(
    State(_state): State<NeoAppState>,
) -> Result<Json<MemoryStatistics>, RestError> {
    info!("Getting memory statistics");

    Ok(Json(MemoryStatistics {
        total_entries: 0,
        memory_types: std::collections::HashMap::new(),
        total_size_bytes: 0,
    }))
}
