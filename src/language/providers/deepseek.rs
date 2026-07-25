use async_trait::async_trait;
use reqwest::Client;
use tokio::sync::mpsc;

use crate::language::config::ProviderConfig;
use crate::language::engine::{LanguageEngine, ProviderCapabilities};
use crate::language::error::{LanguageError, LanguageResult};
use crate::language::types::*;
use crate::time::Timestamp;

/// DeepSeek API provider.
pub struct DeepSeekProvider {
    config: ProviderConfig,
    client: Client,
}

impl DeepSeekProvider {
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
            headers.insert(
                "Authorization",
                format!("Bearer {}", api_key).parse().unwrap(),
            );
        }
        headers.insert("Content-Type", "application/json".parse().unwrap());
        headers
    }
}

#[async_trait]
impl LanguageEngine for DeepSeekProvider {
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
        let url = format!("{}/models", self.config.endpoint);
        let resp = self
            .client
            .get(&url)
            .headers(self.build_headers())
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() => Ok(ProviderHealth::healthy()),
            Ok(r) => Ok(ProviderHealth::unhealthy(format!("HTTP {}", r.status()))),
            Err(e) => Ok(ProviderHealth::unhealthy(e.to_string())),
        }
    }

    async fn generate(&self, config: &GenerationConfig) -> LanguageResult<GenerationResponse> {
        let model = config
            .model
            .as_deref()
            .unwrap_or(&self.config.model_name);

        let messages: Vec<serde_json::Value> = config
            .messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": m.role.to_string(),
                    "content": m.content
                })
            })
            .collect();

        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "max_tokens": config.max_tokens,
            "temperature": config.temperature,
            "top_p": config.top_p,
            "stream": false
        });

        let url = format!("{}/chat/completions", self.config.endpoint);
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
        let text = data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let prompt_tokens = data["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as usize;
        let completion_tokens = data["usage"]["completion_tokens"].as_u64().unwrap_or(0) as usize;

        Ok(GenerationResponse {
            id: uuid::Uuid::new_v4(),
            text,
            finish_reason: FinishReason::Stop,
            usage: TokenUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
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
            "DeepSeek streaming not yet implemented".to_string(),
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
            json_mode: true,
            vision: false,
            audio: false,
            embeddings: false,
            code_generation: true,
            reasoning: true,
            max_context: 64000,
            max_output_tokens: 8192,
            tokenizer: "deepseek".to_string(),
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
