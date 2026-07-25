use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::ApiError;

/// Memory search request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySearchRequest {
    pub query: String,
    pub memory_type: Option<String>,
    pub limit: usize,
    pub min_relevance: f64,
}

/// Memory search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySearchResult {
    pub id: String,
    pub content: String,
    pub memory_type: String,
    pub relevance: f64,
    pub created_at: String,
    pub metadata: std::collections::HashMap<String, String>,
}

/// Memory store request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStoreRequest {
    pub content: String,
    pub memory_type: String,
    pub metadata: std::collections::HashMap<String, String>,
    pub importance: f64,
}

/// Memory statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStatistics {
    pub total_memories: usize,
    pub memories_by_type: std::collections::HashMap<String, usize>,
    pub total_size_bytes: u64,
    pub oldest_memory: Option<String>,
    pub newest_memory: Option<String>,
}

/// Memory API trait.
#[async_trait]
pub trait MemoryApi: Send + Sync {
    async fn search(&self, request: MemorySearchRequest) -> Result<Vec<MemorySearchResult>, ApiError>;
    async fn store(&self, request: MemoryStoreRequest) -> Result<String, ApiError>;
    async fn delete(&self, memory_id: &str) -> Result<(), ApiError>;
    async fn statistics(&self) -> Result<MemoryStatistics, ApiError>;
}
