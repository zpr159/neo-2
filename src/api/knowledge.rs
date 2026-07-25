use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::ApiError;

/// Knowledge entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntity {
    pub id: String,
    pub entity_type: String,
    pub label: String,
    pub properties: std::collections::HashMap<String, serde_json::Value>,
}

/// Knowledge edge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEdge {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub relationship: String,
    pub weight: f64,
    pub properties: std::collections::HashMap<String, serde_json::Value>,
}

/// Knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    pub entities: Vec<KnowledgeEntity>,
    pub edges: Vec<KnowledgeEdge>,
}

/// Knowledge query request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeQueryRequest {
    pub query: String,
    pub query_type: String,
    pub max_depth: Option<usize>,
    pub limit: Option<usize>,
}

/// Knowledge search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSearchResult {
    pub entities: Vec<KnowledgeEntity>,
    pub edges: Vec<KnowledgeEdge>,
    pub relevance: f64,
}

/// Knowledge API trait.
#[async_trait]
pub trait KnowledgeApi: Send + Sync {
    async fn get_entity(&self, entity_id: &str) -> Result<KnowledgeEntity, ApiError>;
    async fn search(&self, query: &str, limit: usize) -> Result<KnowledgeSearchResult, ApiError>;
    async fn get_graph(&self, entity_id: &str, depth: usize) -> Result<KnowledgeGraph, ApiError>;
    async fn query(&self, request: KnowledgeQueryRequest) -> Result<KnowledgeSearchResult, ApiError>;
}
