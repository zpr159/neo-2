use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::ApiError;
use crate::language::types::ModelInfo;

/// Provider status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub name: String,
    pub healthy: bool,
    pub models_loaded: Vec<String>,
    pub latency_ms: Option<f64>,
}

/// Language API trait.
#[async_trait]
pub trait LanguageApi: Send + Sync {
    async fn list_providers(&self) -> Result<Vec<ProviderStatus>, ApiError>;
    async fn list_models(&self, provider: &str) -> Result<Vec<ModelInfo>, ApiError>;
    async fn health_check(&self, provider: &str) -> Result<ProviderStatus, ApiError>;
    async fn generate(&self, provider: &str, model: &str, prompt: &str) -> Result<String, ApiError>;
}
