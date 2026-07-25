use async_trait::async_trait;
use reqwest::Client;
use tokio::sync::mpsc;

use crate::language::config::ProviderConfig;
use crate::language::engine::{LanguageEngine, ProviderCapabilities};
use crate::language::error::{LanguageError, LanguageResult};
use crate::language::types::*;
use crate::time::Timestamp;

/// Anthropic Claude API provider.
pub struct AnthropicProvider {
    config: ProviderConfig,
    client: Client,
}

impl AnthropicProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("failed to create HTTP client");

        Self { config, client }
    }

    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(ref api_key) = self.config.api_key {
            headers.insert("x-api-key", api_key.parse().unwrap());
        }
        headers.insert("anthropic-version", "2023-06-01".parse().unwrap());
        headers.insert("Content-Type", "application/json".parse().unwrap());
        headers
    }
}

#[async_trait]
impl LanguageEngine for AnthropicProvider {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn config(&self) -> &ProviderConfig {
        &self.config
    }

    async fn load_model(&self, _model_name: &str) -> LanguageResult<()> {
        Ok(())
    }

    async fn unload_model(&self, _model_name: &str) -> LanguageResult<()> {
        Ok(())
    }

    async fn health_check(&self) -> LanguageResult<ProviderHealth> {
        let url = format!("{}/v1/messages", self.config.endpoint);
        let body = serde_json::json!({
            "model": self.config.model_name,
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "hi"}]
        });

        let resp = self
            .client
            .post(&url)
            .headers(self.build_headers())
            .json(&body)
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() || r.status().as_u16() == 400 => {
                Ok(ProviderHealth::healthy())
            }
            Ok(r) => Ok(ProviderHealth::unhealthy(format!("HTTP {}", r.status()))),
            Err(e) => Ok(ProviderHealth::unhealthy(e.to_string())),
        }
    }

    async fn generate(&self, config: &GenerationConfig) -> LanguageResult<GenerationResponse> {
        let model = config
            .model
            .as_deref()
            .unwrap_or(&self.config.model_name);

        let mut system_prompt = String::new();
        let mut messages = Vec::new();

        for msg in &config.messages {
            match msg.role {
                MessageRole::System => {
                    system_prompt.push_str(&msg.content);
                    system_prompt.push('\n');
                }
                _ => {
                    messages.push(serde_json::json!({
                        "role": msg.role.to_string(),
                        "content": msg.content
                    }));
                }
            }
        }

        let mut body = serde_json::json!({
            "model": model,
            "max_tokens": config.max_tokens,
            "messages": messages
        });

        if !system_prompt.is_empty() {
            body["system"] = serde_json::Value::String(system_prompt);
        }

        if config.temperature > 0.0 {
            body["temperature"] = serde_json::json!(config.temperature);
        }

        let url = format!("{}/v1/messages", self.config.endpoint);
        let start = std::time::Instant::now();

        let resp = self
            .client
            .post(&url)
            .headers(self.build_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| LanguageError::ConnectionFailed(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(LanguageError::GenerationFailed(text));
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| LanguageError::SerializationFailed(e.to_string()))?;

        let latency = start.elapsed();

        let text = data["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let input_tokens = data["usage"]["input_tokens"].as_u64().unwrap_or(0) as usize;
        let output_tokens = data["usage"]["output_tokens"].as_u64().unwrap_or(0) as usize;

        Ok(GenerationResponse {
            id: uuid::Uuid::new_v4(),
            text,
            finish_reason: FinishReason::Stop,
            usage: TokenUsage {
                prompt_tokens: input_tokens,
                completion_tokens: output_tokens,
                total_tokens: input_tokens + output_tokens,
            },
            latency,
            provider: self.config.name.clone(),
            model: model.to_string(),
            confidence: None,
            warnings: Vec::new(),
            tool_calls: None,
            metadata: std::collections::HashMap::new(),
            created_at: Timestamp::now(),
        })
    }

    async fn stream(
        &self,
        _config: &GenerationConfig,
        _cancellation: CancellationToken,
    ) -> LanguageResult<mpsc::Receiver<StreamChunk>> {
        let (_tx, _rx) = mpsc::channel::<StreamChunk>(1);
        Err(LanguageError::NotImplemented(
            "Anthropic streaming not yet implemented".to_string(),
        ))
    }

    async fn count_tokens(&self, text: &str, _model: &str) -> LanguageResult<usize> {
        let estimator = crate::language::token::TokenEstimator::new();
        Ok(estimator.estimate(text))
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
            json_mode: false,
            vision: true,
            audio: false,
            embeddings: false,
            code_generation: true,
            reasoning: true,
            max_context: 200000,
            max_output_tokens: 8192,
            tokenizer: "anthropic".to_string(),
            offline_support: false,
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
