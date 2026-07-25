//! Provider implementations for the language engine.
//!
//! Each provider implements the `LanguageEngine` trait and handles
//! communication with its specific backend.

pub mod ollama;
pub mod llamacpp;
pub mod neolm;
pub mod openai;
pub mod anthropic;
pub mod deepseek;
pub mod custom;

pub use ollama::OllamaProvider;
pub use llamacpp::LlamaCppProvider;
pub use neolm::NeoLmProvider;
pub use openai::OpenAiProvider;
pub use anthropic::AnthropicProvider;
pub use deepseek::DeepSeekProvider;
pub use custom::CustomProvider;

use std::sync::Arc;

use crate::language::config::ProviderType;
use crate::language::engine::LanguageEngine;
use crate::language::error::LanguageResult;
use crate::language::registry::{ProviderDescriptor, ProviderRegistry};

/// Register all built-in providers with the registry.
pub async fn register_builtin_providers(registry: &ProviderRegistry) -> LanguageResult<()> {
    registry
        .register(
            ProviderDescriptor {
                provider_type: ProviderType::Ollama,
                display_name: "Ollama".to_string(),
                description: "Local LLM inference via Ollama".to_string(),
                default_endpoint: "http://localhost:11434".to_string(),
                requires_api_key: false,
            },
            |config| Arc::new(OllamaProvider::new(config.clone())) as Arc<dyn LanguageEngine>,
        )
        .await;

    registry
        .register(
            ProviderDescriptor {
                provider_type: ProviderType::LlamaCpp,
                display_name: "llama.cpp".to_string(),
                description: "Local GGUF model inference via llama.cpp".to_string(),
                default_endpoint: "http://localhost:8080".to_string(),
                requires_api_key: false,
            },
            |config| Arc::new(LlamaCppProvider::new(config.clone())) as Arc<dyn LanguageEngine>,
        )
        .await;

    registry
        .register(
            ProviderDescriptor {
                provider_type: ProviderType::NeoLm,
                display_name: "NeoLM".to_string(),
                description: "Neo's native language model (placeholder)".to_string(),
                default_endpoint: "http://localhost:8081".to_string(),
                requires_api_key: false,
            },
            |config| Arc::new(NeoLmProvider::new(config.clone())) as Arc<dyn LanguageEngine>,
        )
        .await;

    registry
        .register(
            ProviderDescriptor {
                provider_type: ProviderType::OpenAi,
                display_name: "OpenAI".to_string(),
                description: "OpenAI API compatible providers".to_string(),
                default_endpoint: "https://api.openai.com/v1".to_string(),
                requires_api_key: true,
            },
            |config| Arc::new(OpenAiProvider::new(config.clone())) as Arc<dyn LanguageEngine>,
        )
        .await;

    registry
        .register(
            ProviderDescriptor {
                provider_type: ProviderType::Anthropic,
                display_name: "Anthropic".to_string(),
                description: "Anthropic Claude API".to_string(),
                default_endpoint: "https://api.anthropic.com".to_string(),
                requires_api_key: true,
            },
            |config| Arc::new(AnthropicProvider::new(config.clone())) as Arc<dyn LanguageEngine>,
        )
        .await;

    registry
        .register(
            ProviderDescriptor {
                provider_type: ProviderType::DeepSeek,
                display_name: "DeepSeek".to_string(),
                description: "DeepSeek API".to_string(),
                default_endpoint: "https://api.deepseek.com".to_string(),
                requires_api_key: true,
            },
            |config| Arc::new(DeepSeekProvider::new(config.clone())) as Arc<dyn LanguageEngine>,
        )
        .await;

    Ok(())
}
