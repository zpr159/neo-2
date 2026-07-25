use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::language::config::ProviderConfig;
use crate::language::engine::{LanguageEngine, ProviderCapabilities};
use crate::language::error::LanguageResult;
use crate::language::types::*;

/// NeoLM provider placeholder.
///
/// Initially exposes the same interface as every other provider.
/// When NeoLM is trained, it becomes a drop-in replacement.
pub struct NeoLmProvider {
    config: ProviderConfig,
}

impl NeoLmProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl LanguageEngine for NeoLmProvider {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn config(&self) -> &ProviderConfig {
        &self.config
    }

    async fn load_model(&self, _model_name: &str) -> LanguageResult<()> {
        tracing::info!("NeoLM: load_model placeholder");
        Ok(())
    }

    async fn unload_model(&self, _model_name: &str) -> LanguageResult<()> {
        tracing::info!("NeoLM: unload_model placeholder");
        Ok(())
    }

    async fn health_check(&self) -> LanguageResult<ProviderHealth> {
        Ok(ProviderHealth::unhealthy("NeoLM not yet trained"))
    }

    async fn generate(&self, _config: &GenerationConfig) -> LanguageResult<GenerationResponse> {
        Err(crate::language::error::LanguageError::NotImplemented(
            "NeoLM not yet trained".to_string(),
        ))
    }

    async fn stream(
        &self,
        _config: &GenerationConfig,
        _cancellation: CancellationToken,
    ) -> LanguageResult<mpsc::Receiver<StreamChunk>> {
        Err(crate::language::error::LanguageError::NotImplemented(
            "NeoLM not yet trained".to_string(),
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
        Ok(ProviderCapabilities {
            streaming: true,
            function_calling: true,
            tool_calling: true,
            json_mode: true,
            vision: false,
            audio: false,
            embeddings: false,
            code_generation: true,
            reasoning: true,
            max_context: 32000,
            max_output_tokens: 8192,
            tokenizer: "neolm".to_string(),
            offline_support: true,
            supported_models: Vec::new(),
        })
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
