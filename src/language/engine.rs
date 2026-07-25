use async_trait::async_trait;
use tokio::sync::mpsc;

use super::config::ProviderConfig;
use super::error::LanguageResult;
use super::types::*;

/// The core trait that all language model providers must implement.
///
/// This trait is provider-independent. No subsystem above this trait
/// may know which provider is being used.
#[async_trait]
pub trait LanguageEngine: Send + Sync {
    /// Returns the name of this provider.
    fn name(&self) -> &str;

    /// Returns the provider configuration.
    fn config(&self) -> &ProviderConfig;

    /// Load a model into memory.
    async fn load_model(&self, model_name: &str) -> LanguageResult<()>;

    /// Unload a model from memory.
    async fn unload_model(&self, model_name: &str) -> LanguageResult<()>;

    /// Check provider health and connectivity.
    async fn health_check(&self) -> LanguageResult<ProviderHealth>;

    /// Generate a completion response.
    async fn generate(&self, config: &GenerationConfig) -> LanguageResult<GenerationResponse>;

    /// Start a streaming generation. Returns a receiver for stream chunks.
    async fn stream(
        &self,
        config: &GenerationConfig,
        cancellation: CancellationToken,
    ) -> LanguageResult<mpsc::Receiver<StreamChunk>>;

    /// Count tokens in the given text using the provider's tokenizer.
    async fn count_tokens(&self, text: &str, model: &str) -> LanguageResult<usize>;

    /// Estimate the context size for the given messages.
    async fn estimate_context_size(
        &self,
        messages: &[Message],
        model: &str,
    ) -> LanguageResult<usize>;

    /// Report the capabilities of this provider.
    async fn capabilities(&self) -> LanguageResult<ProviderCapabilities>;

    /// Cancel an ongoing generation.
    async fn cancel_generation(&self, id: GenerationId) -> LanguageResult<()>;

    /// Get current provider metrics.
    async fn metrics(&self) -> LanguageResult<ProviderMetrics>;

    /// List available models from this provider.
    async fn list_models(&self) -> LanguageResult<Vec<ModelInfo>>;

    /// Check if a specific model is loaded.
    async fn is_model_loaded(&self, model_name: &str) -> LanguageResult<bool>;
}

/// Capabilities declared by a provider.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub streaming: bool,
    pub function_calling: bool,
    pub tool_calling: bool,
    pub json_mode: bool,
    pub vision: bool,
    pub audio: bool,
    pub embeddings: bool,
    pub code_generation: bool,
    pub reasoning: bool,
    pub max_context: usize,
    pub max_output_tokens: usize,
    pub tokenizer: String,
    pub offline_support: bool,
    pub supported_models: Vec<String>,
}

use serde::{Deserialize, Serialize};
