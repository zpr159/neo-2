use std::time::{Duration, Instant};

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};

use crate::language::config::ProviderConfig;
use crate::language::engine::{LanguageEngine, ProviderCapabilities};
use crate::language::error::{LanguageError, LanguageResult};
use crate::language::types::*;
use crate::time::Timestamp;

/// Ollama API chat request.
#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<serde_json::Value>>,
}

/// Ollama API generate request.
#[derive(Debug, Serialize)]
struct OllamaGenerateRequest {
    model: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

/// Ollama message format.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<serde_json::Value>>,
}

/// Ollama generation options.
#[derive(Debug, Clone, Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repeat_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<u64>,
}

/// Ollama chat response.
#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: Option<OllamaMessage>,
    done: bool,
    total_duration: Option<u64>,
    eval_count: Option<u64>,
    eval_duration: Option<u64>,
}

/// Ollama streaming chunk.
#[derive(Debug, Deserialize)]
struct OllamaStreamChunk {
    message: Option<OllamaMessage>,
    done: bool,
    total_duration: Option<u64>,
    eval_count: Option<u64>,
    eval_duration: Option<u64>,
}

/// Ollama model info from /api/tags.
#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModel>,
}

/// Ollama model metadata.
#[derive(Debug, Clone, Deserialize)]
struct OllamaModel {
    name: String,
    model: String,
    size: Option<u64>,
    digest: Option<String>,
    modified_at: Option<String>,
    details: Option<OllamaModelDetails>,
}

/// Ollama model details.
#[derive(Debug, Clone, Deserialize)]
struct OllamaModelDetails {
    parent_model: Option<String>,
    format: Option<String>,
    family: Option<String>,
    parameter_size: Option<String>,
    quantization_level: Option<String>,
}

/// Ollama version response.
#[derive(Debug, Deserialize)]
struct OllamaVersionResponse {
    version: String,
}

/// Ollama provider implementation.
pub struct OllamaProvider {
    config: ProviderConfig,
    client: Client,
    loaded_models: RwLock<Vec<String>>,
}

impl OllamaProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("failed to create HTTP client");

        Self {
            config,
            client,
            loaded_models: RwLock::new(Vec::new()),
        }
    }

    fn base_url(&self) -> &str {
        &self.config.endpoint
    }

    async fn request<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        body: Option<&impl Serialize>,
    ) -> LanguageResult<T> {
        let url = format!("{}{}", self.base_url(), path);
        let mut req = self.client.get(&url);

        if let Some(body) = body {
            req = self.client.post(&url).json(body);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| LanguageError::ConnectionFailed(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".to_string());
            return Err(LanguageError::GenerationFailed(format!(
                "HTTP {}: {}",
                status, text
            )));
        }

        resp.json()
            .await
            .map_err(|e| LanguageError::SerializationFailed(e.to_string()))
    }

    fn build_options(&self, config: &GenerationConfig) -> OllamaOptions {
        OllamaOptions {
            temperature: Some(config.temperature),
            top_p: Some(config.top_p),
            top_k: config.top_k,
            repeat_penalty: Some(config.repeat_penalty),
            presence_penalty: Some(config.presence_penalty),
            frequency_penalty: Some(config.frequency_penalty),
            num_predict: Some(config.max_tokens),
            stop: config.stop.clone(),
            seed: config.seed,
        }
    }

    fn convert_messages(messages: &[Message]) -> Vec<OllamaMessage> {
        messages
            .iter()
            .map(|m| OllamaMessage {
                role: m.role.to_string(),
                content: m.content.clone(),
                tool_calls: m.tool_calls.as_ref().map(|calls| {
                    calls
                        .iter()
                        .map(|tc| {
                            serde_json::json!({
                                "function": {
                                    "name": tc.function.name,
                                    "arguments": tc.function.arguments
                                }
                            })
                        })
                        .collect()
                }),
            })
            .collect()
    }
}

#[async_trait]
impl LanguageEngine for OllamaProvider {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn config(&self) -> &ProviderConfig {
        &self.config
    }

    async fn load_model(&self, model_name: &str) -> LanguageResult<()> {
        let url = format!("{}/api/generate", self.base_url());
        let body = serde_json::json!({
            "model": model_name,
            "prompt": "",
            "keep_alive": "5m"
        });

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| LanguageError::ConnectionFailed(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".to_string());
            return Err(LanguageError::ModelLoadingFailed(text));
        }

        let mut models = self.loaded_models.write().await;
        if !models.contains(&model_name.to_string()) {
            models.push(model_name.to_string());
        }

        Ok(())
    }

    async fn unload_model(&self, model_name: &str) -> LanguageResult<()> {
        let url = format!("{}/api/generate", self.base_url());
        let body = serde_json::json!({
            "model": model_name,
            "keep_alive": 0
        });

        let _ = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await;

        let mut models = self.loaded_models.write().await;
        models.retain(|m| m != model_name);

        Ok(())
    }

    async fn health_check(&self) -> LanguageResult<ProviderHealth> {
        let start = Instant::now();
        let _: OllamaVersionResponse = self.request("/api/version", None::<&()>).await?;
        let latency = start.elapsed().as_millis() as f64;

        let models = self.loaded_models.read().await;

        Ok(ProviderHealth {
            healthy: true,
            latency_ms: Some(latency),
            message: Some(format!("{} models loaded", models.len())),
            checked_at: Timestamp::now(),
            models_loaded: models.len(),
            memory_usage_bytes: None,
        })
    }

    async fn generate(&self, config: &GenerationConfig) -> LanguageResult<GenerationResponse> {
        let start = Instant::now();
        let model = config
            .model
            .as_deref()
            .unwrap_or(&self.config.model_name);

        let ollama_messages = Self::convert_messages(&config.messages);
        let options = self.build_options(config);

        let request = OllamaChatRequest {
            model: model.to_string(),
            messages: ollama_messages,
            stream: Some(false),
            options: Some(options),
            tools: config.tools.as_ref().map(|tools| {
                tools
                    .iter()
                    .map(|t| {
                        serde_json::json!({
                            "type": t.tool_type,
                            "function": {
                                "name": t.function.name,
                                "description": t.function.description,
                                "parameters": t.function.parameters
                            }
                        })
                    })
                    .collect()
            }),
        };

        let response: OllamaChatResponse = self.request("/api/chat", Some(&request)).await?;
        let latency = start.elapsed();

        let text = response
            .message
            .map(|m| m.content)
            .unwrap_or_default();

        let eval_count = response.eval_count.unwrap_or(0) as usize;
        let eval_duration_ns = response.eval_duration.unwrap_or(0);

        let tokens_per_second = if eval_duration_ns > 0 {
            eval_count as f64 / (eval_duration_ns as f64 / 1_000_000_000.0)
        } else {
            0.0
        };

        let finish_reason = if response.done {
            FinishReason::Stop
        } else {
            FinishReason::Length
        };

        Ok(GenerationResponse {
            id: uuid::Uuid::new_v4(),
            text,
            finish_reason,
            usage: TokenUsage {
                prompt_tokens: 0,
                completion_tokens: eval_count,
                total_tokens: eval_count,
            },
            latency,
            provider: self.config.name.clone(),
            model: model.to_string(),
            confidence: None,
            warnings: Vec::new(),
            tool_calls: None,
            metadata: {
                let mut m = std::collections::HashMap::new();
                m.insert(
                    "tokens_per_second".to_string(),
                    format!("{:.2}", tokens_per_second),
                );
                if let Some(total) = response.total_duration {
                    m.insert("total_duration_ns".to_string(), total.to_string());
                }
                m
            },
            created_at: Timestamp::now(),
        })
    }

    async fn stream(
        &self,
        config: &GenerationConfig,
        cancellation: CancellationToken,
    ) -> LanguageResult<mpsc::Receiver<StreamChunk>> {
        let model = config
            .model
            .as_deref()
            .unwrap_or(&self.config.model_name)
            .to_string();

        let ollama_messages = Self::convert_messages(&config.messages);
        let options = self.build_options(config);
        let endpoint = self.base_url().to_string();
        let client = self.client.clone();

        let request = OllamaChatRequest {
            model: model.clone(),
            messages: ollama_messages,
            stream: Some(true),
            options: Some(options),
            tools: None,
        };

        let url = format!("{}/api/chat", endpoint);

        let (tx, rx) = mpsc::channel(256);

        tokio::spawn(async move {
            let resp = match client.post(&url).json(&request).send().await {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx
                        .send(StreamChunk::done(
                            String::new(),
                            0,
                            FinishReason::Error,
                        ))
                        .await;
                    tracing::error!("ollama stream connection failed: {}", e);
                    return;
                }
            };

            if !resp.status().is_success() {
                let _ = tx
                    .send(StreamChunk::done(
                        String::new(),
                        0,
                        FinishReason::Error,
                    ))
                    .await;
                return;
            }

            let mut accumulated = String::new();
            let mut sequence: u64 = 0;

            let bytes_stream = resp.bytes_stream();
            tokio::pin!(bytes_stream);

            let mut buffer = String::new();

            use futures::StreamExt;
            while let Some(chunk_result) = bytes_stream.next().await {
                if cancellation.is_cancelled() {
                    let _ = tx
                        .send(StreamChunk::done(
                            &accumulated,
                            sequence,
                            FinishReason::Cancelled,
                        ))
                        .await;
                    break;
                }

                let bytes = match chunk_result {
                    Ok(b) => b,
                    Err(e) => {
                        tracing::error!("ollama stream read error: {}", e);
                        break;
                    }
                };

                buffer.push_str(&String::from_utf8_lossy(&bytes));

                while let Some(newline_pos) = buffer.find('\n') {
                    let line = buffer[..newline_pos].to_string();
                    buffer = buffer[newline_pos + 1..].to_string();

                    if line.trim().is_empty() {
                        continue;
                    }

                    match serde_json::from_str::<OllamaStreamChunk>(&line) {
                        Ok(chunk) => {
                            if let Some(msg) = chunk.message {
                                accumulated.push_str(&msg.content);
                                sequence += 1;

                                let stream_chunk =
                                    StreamChunk::new(msg.content, &accumulated, sequence);

                                if chunk.done {
                                    let mut final_chunk = StreamChunk::done(
                                        &accumulated,
                                        sequence,
                                        FinishReason::Stop,
                                    );

                                    if let Some(eval_count) = chunk.eval_count {
                                        final_chunk.usage = Some(TokenUsage {
                                            prompt_tokens: 0,
                                            completion_tokens: eval_count as usize,
                                            total_tokens: eval_count as usize,
                                        });
                                    }

                                    let _ = tx.send(final_chunk).await;
                                    return;
                                }

                                let _ = tx.send(stream_chunk).await;
                            }
                        }
                        Err(e) => {
                            tracing::warn!("failed to parse ollama chunk: {}", e);
                        }
                    }
                }
            }

            let _ = tx
                .send(StreamChunk::done(&accumulated, sequence, FinishReason::Stop))
                .await;
        });

        Ok(rx)
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
            max_context: 128000,
            max_output_tokens: 4096,
            tokenizer: "ollama".to_string(),
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
        let response: OllamaTagsResponse = self.request("/api/tags", None::<&()>).await?;

        Ok(response
            .models
            .into_iter()
            .map(|m| {
                let details = m.details.clone();
                ModelInfo {
                    name: m.name.clone(),
                    display_name: Some(m.name),
                    version: details.as_ref().and_then(|d| d.parent_model.clone()),
                    context_length: None,
                    max_output_tokens: None,
                    quantization: details.as_ref().and_then(|d| d.quantization_level.clone()),
                    parameter_count: details.as_ref().and_then(|d| d.parameter_size.clone()),
                    license: None,
                    memory_requirements: m.size.map(|s| format_bytes(s)),
                    capabilities: ModelCapabilities {
                        supports_streaming: true,
                        supports_function_calling: true,
                        supports_tool_calling: true,
                        supports_json_mode: false,
                        supports_vision: true,
                        supports_audio: false,
                        supports_embeddings: false,
                        supports_code_generation: true,
                        supports_reasoning: true,
                        max_context_length: None,
                        max_output_tokens: None,
                        supports_offline: true,
                    },
                }
            })
            .collect())
    }

    async fn is_model_loaded(&self, model_name: &str) -> LanguageResult<bool> {
        let models = self.loaded_models.read().await;
        Ok(models.iter().any(|m| m == model_name))
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
