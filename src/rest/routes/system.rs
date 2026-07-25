use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::rest::error::RestError;
use crate::rest::NeoAppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
    pub subsystems: std::collections::HashMap<String, SubsystemHealth>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsystemHealth {
    pub healthy: bool,
    pub latency_ms: Option<f64>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub requests_total: u64,
    pub requests_per_second: f64,
    pub active_sessions: usize,
    pub memory_usage_bytes: usize,
    pub cpu_usage_percent: f64,
}

pub async fn health_check_handler(
    State(_state): State<NeoAppState>,
) -> Result<Json<HealthResponse>, RestError> {
    info!("Health check requested");

    Ok(Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs: 0,
        subsystems: std::collections::HashMap::new(),
    }))
}

pub async fn metrics_handler(
    State(_state): State<NeoAppState>,
) -> Result<Json<MetricsResponse>, RestError> {
    info!("Metrics requested");

    Ok(Json(MetricsResponse {
        requests_total: 0,
        requests_per_second: 0.0,
        active_sessions: 0,
        memory_usage_bytes: 0,
        cpu_usage_percent: 0.0,
    }))
}
