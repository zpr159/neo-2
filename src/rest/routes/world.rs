use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::rest::error::RestError;
use crate::rest::NeoAppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldEntity {
    pub id: String,
    pub entity_type: String,
    pub properties: serde_json::Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityCreateRequest {
    pub entity_type: String,
    pub properties: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityUpdateRequest {
    pub properties: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldEvent {
    pub id: String,
    pub event_type: String,
    pub entity_id: String,
    pub data: serde_json::Value,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub entities: Vec<WorldEntity>,
    pub events: Vec<WorldEvent>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulateRequest {
    pub action: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulateResponse {
    pub result: serde_json::Value,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictRequest {
    pub query: String,
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictResponse {
    pub prediction: String,
    pub confidence: f64,
}

pub async fn list_entities_handler(
    State(_state): State<NeoAppState>,
) -> Result<Json<Vec<WorldEntity>>, RestError> {
    info!("Listing world entities");
    Ok(Json(vec![]))
}

pub async fn get_entity_handler(
    State(_state): State<NeoAppState>,
    Path(entity_id): Path<String>,
) -> Result<Json<WorldEntity>, RestError> {
    info!("Getting world entity: {}", entity_id);

    Ok(Json(WorldEntity {
        id: entity_id,
        entity_type: "unknown".to_string(),
        properties: serde_json::json!({}),
        created_at: chrono::Utc::now().to_rfc3339(),
    }))
}

pub async fn create_entity_handler(
    State(_state): State<NeoAppState>,
    Json(request): Json<EntityCreateRequest>,
) -> Result<Json<WorldEntity>, RestError> {
    info!("Creating world entity of type: {}", request.entity_type);

    Ok(Json(WorldEntity {
        id: uuid::Uuid::new_v4().to_string(),
        entity_type: request.entity_type,
        properties: request.properties,
        created_at: chrono::Utc::now().to_rfc3339(),
    }))
}

pub async fn update_entity_handler(
    State(_state): State<NeoAppState>,
    Path(entity_id): Path<String>,
    Json(request): Json<EntityUpdateRequest>,
) -> Result<Json<WorldEntity>, RestError> {
    info!("Updating world entity: {}", entity_id);

    Ok(Json(WorldEntity {
        id: entity_id,
        entity_type: "unknown".to_string(),
        properties: request.properties,
        created_at: chrono::Utc::now().to_rfc3339(),
    }))
}

pub async fn delete_entity_handler(
    State(_state): State<NeoAppState>,
    Path(entity_id): Path<String>,
) -> Result<Json<serde_json::Value>, RestError> {
    info!("Deleting world entity: {}", entity_id);

    Ok(Json(serde_json::json!({
        "deleted": true,
        "entity_id": entity_id,
    })))
}

pub async fn list_events_handler(
    State(_state): State<NeoAppState>,
) -> Result<Json<Vec<WorldEvent>>, RestError> {
    info!("Listing world events");
    Ok(Json(vec![]))
}

pub async fn get_snapshot_handler(
    State(_state): State<NeoAppState>,
) -> Result<Json<WorldSnapshot>, RestError> {
    info!("Getting world snapshot");

    Ok(Json(WorldSnapshot {
        entities: vec![],
        events: vec![],
        timestamp: chrono::Utc::now().to_rfc3339(),
    }))
}

pub async fn simulate_handler(
    State(_state): State<NeoAppState>,
    Json(request): Json<SimulateRequest>,
) -> Result<Json<SimulateResponse>, RestError> {
    info!("Simulating action: {}", request.action);

    Ok(Json(SimulateResponse {
        result: serde_json::json!({
            "status": "simulated",
            "action": request.action,
        }),
        duration_ms: 0,
    }))
}

pub async fn predict_handler(
    State(_state): State<NeoAppState>,
    Json(request): Json<PredictRequest>,
) -> Result<Json<PredictResponse>, RestError> {
    info!("Predicting for query: {}", &request.query[..request.query.len().min(50)]);

    Ok(Json(PredictResponse {
        prediction: format!("Prediction for: {}", request.query),
        confidence: 0.5,
    }))
}
