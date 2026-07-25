use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{ApiError, PaginationParams};

/// World entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldEntity {
    pub id: String,
    pub entity_type: String,
    pub name: String,
    pub properties: std::collections::HashMap<String, serde_json::Value>,
    pub relationships: Vec<EntityRelationship>,
    pub created_at: String,
    pub updated_at: String,
}

/// Relationship between entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRelationship {
    pub target_id: String,
    pub relationship_type: String,
    pub weight: f64,
    pub metadata: std::collections::HashMap<String, String>,
}

/// Temporal event in the world model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldEvent {
    pub id: String,
    pub event_type: String,
    pub entity_id: Option<String>,
    pub timestamp: String,
    pub data: serde_json::Value,
}

/// World state snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub entities: Vec<WorldEntity>,
    pub events: Vec<WorldEvent>,
    pub captured_at: String,
}

/// Prediction request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionRequest {
    pub entity_id: String,
    pub horizon_steps: usize,
    pub factors: Vec<String>,
}

/// Prediction result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionResult {
    pub entity_id: String,
    pub predictions: Vec<PredictionEntry>,
    pub confidence: f64,
}

/// Single prediction entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionEntry {
    pub step: usize,
    pub value: serde_json::Value,
    pub confidence: f64,
}

/// Simulation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationRequest {
    pub initial_state: WorldSnapshot,
    pub steps: usize,
    pub parameters: std::collections::HashMap<String, serde_json::Value>,
}

/// Simulation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub final_state: WorldSnapshot,
    pub trajectory: Vec<WorldSnapshot>,
    pub metrics: std::collections::HashMap<String, f64>,
}

/// World Model API trait.
#[async_trait]
pub trait WorldModelApi: Send + Sync {
    async fn list_entities(&self, pagination: PaginationParams) -> Result<Vec<WorldEntity>, ApiError>;
    async fn get_entity(&self, entity_id: &str) -> Result<WorldEntity, ApiError>;
    async fn create_entity(&self, entity: WorldEntity) -> Result<WorldEntity, ApiError>;
    async fn update_entity(&self, entity_id: &str, entity: WorldEntity) -> Result<WorldEntity, ApiError>;
    async fn delete_entity(&self, entity_id: &str) -> Result<(), ApiError>;
    async fn list_events(&self, pagination: PaginationParams) -> Result<Vec<WorldEvent>, ApiError>;
    async fn get_snapshot(&self) -> Result<WorldSnapshot, ApiError>;
    async fn simulate(&self, request: SimulationRequest) -> Result<SimulationResult, ApiError>;
    async fn predict(&self, request: PredictionRequest) -> Result<PredictionResult, ApiError>;
}
