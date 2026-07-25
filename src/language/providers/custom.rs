use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::language::config::ProviderConfig;
use crate::language::engine::{LanguageEngine, ProviderCapabilities};
use crate::language::error::{LanguageError, LanguageResult};
use crate::language::types::*;

/// Custom provider trait for user-defined implementations.
///
/// Users can implement this trait to add custom providers without
/// modifying the core language engine code.
pub struct CustomProvider {
    config: ProviderConfig,
}

impl CustomProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl LanguageEngine for CustomProvider {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn config(&self) -> &ProviderConfig {
        &self.config
    }

    async fn load_model(&self, _model_name: &str) -> LanguageResult<()> {
        Err(LanguageError::NotImplemented(
            "custom provider not configured".to_string(),
        ))
    }

    async fn unload_model(&self, _model_name: &str) -> LanguageResult<()> {
        Err(LanguageError::NotImplemented(
            "custom provider not configured".to_string(),
        ))
    }

    async fn health_check(&self) -> LanguageResult<ProviderHealth> {
        Ok(ProviderHealth::unhealthy("custom provider not configured"))
    }

    async fn generate(&self, _config: &GenerationConfig) -> LanguageResult<GenerationResponse> {
        Err(LanguageError::NotImplemented(
            "custom provider not configured".to_string(),
        ))
    }

    async fn stream(
        &self,
        _config: &GenerationConfig,
        _cancellation: CancellationToken,
    ) -> LanguageResult<mpsc::Receiver<StreamChunk>> {
        Err(LanguageError::NotImplemented(
            "custom provider not configured".to_string(),
        ))
    }

    async fn count_tokens(&self, _text: &str, _model: &str) -> LanguageResult<usize> {
        Ok(0)
    }

    async fn estimate_context_size(
        &self,
        messages: &[Message],
        _model: &str,
    ) -> LanguageResult<usize> {
        let estimator = crate::language::token::TokenEstimator::new();
        Ok(estimator.estimate_messages(messages))
    }

    async fn capabilities(&self) -> LanguageResult<ProviderCapabilities> {
        Ok(ProviderCapabilities::default())
    }

    async fn cancel_generation(&self, _id: GenerationId) -> LanguageResult<()> {
        Ok(())
    }

    async fn metrics(&self) -> LanguageResult<ProviderMetrics> {
        Ok(ProviderMetrics::default())
    }

    async fn list_models(&self) -> LanguageResult<Vec<ModelInfo>> {
        Ok(Vec::new())
    }

    async fn is_model_loaded(&self, _model_name: &str) -> LanguageResult<bool> {
        Ok(false)
    }
}
