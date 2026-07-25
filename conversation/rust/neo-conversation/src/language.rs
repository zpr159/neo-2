use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{ConversationError, ConversationResult};
use crate::types::{LlmMessage, StreamChunk, TokenUsage, SessionId};

/// Configuration for a language engine backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageEngineConfig {
    /// Backend type identifier.
    pub backend_type: LanguageBackendType,
    /// Base URL for the API endpoint.
    pub base_url: String,
    /// Model name or identifier.
    pub model: String,
    /// API key (optional, for cloud backends).
    pub api_key: Option<String>,
    /// Request timeout in seconds.
    pub timeout_secs: u64,
    /// Maximum concurrent requests.
    pub max_concurrent: usize,
    /// Additional backend-specific configuration.
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

impl Default for LanguageEngineConfig {
    fn default() -> Self {
        Self {
            backend_type: LanguageBackendType::Ollama,
            base_url: "http://localhost:11434".into(),
            model: "qwen3:8b".into(),
            api_key: None,
            timeout_secs: 120,
            max_concurrent: 4,
            extra: std::collections::HashMap::new(),
        }
    }
}

/// Types of language model backends.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LanguageBackendType {
    Ollama,
    OpenAi,
    LlamaCpp,
    NeoInference,
    RemoteHttp,
    Custom(String),
}

impl std::fmt::Display for LanguageBackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ollama => write!(f, "ollama"),
            Self::OpenAi => write!(f, "openai"),
            Self::LlamaCpp => write!(f, "llama_cpp"),
            Self::NeoInference => write!(f, "neo_inference"),
            Self::RemoteHttp => write!(f, "remote_http"),
            Self::Custom(name) => write!(f, "{name}"),
        }
    }
}

/// Information about the language engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageEngineInfo {
    pub backend_type: LanguageBackendType,
    pub model: String,
    pub is_available: bool,
    pub version: Option<String>,
}

/// Request for text generation.
#[derive(Debug, Clone)]
pub struct GenerateRequest {
    /// Messages forming the conversation context.
    pub messages: Vec<LlmMessage>,
    /// Maximum tokens to generate.
    pub max_tokens: usize,
    /// Sampling temperature.
    pub temperature: f32,
    /// Top-p sampling.
    pub top_p: f32,
    /// Stop sequences.
    pub stop: Vec<String>,
    /// Whether to stream.
    pub stream: bool,
}

impl Default for GenerateRequest {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            max_tokens: 2048,
            temperature: 0.7,
            top_p: 0.9,
            stop: Vec::new(),
            stream: true,
        }
    }
}

/// Response from text generation.
#[derive(Debug, Clone)]
pub struct GenerateResponse {
    /// Generated text.
    pub text: String,
    /// Token usage.
    pub usage: TokenUsage,
    /// Finish reason.
    pub finish_reason: FinishReason,
}

/// Why generation stopped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinishReason {
    Stop,
    Length,
    ToolCall,
    ContentFilter,
    Error,
}

impl std::fmt::Display for FinishReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stop => write!(f, "stop"),
            Self::Length => write!(f, "length"),
            Self::ToolCall => write!(f, "tool_call"),
            Self::ContentFilter => write!(f, "content_filter"),
            Self::Error => write!(f, "error"),
        }
    }
}

/// Trait that all language engine backends must implement.
///
/// This is the abstraction layer that makes Neo's language engine replaceable.
/// Initially backed by Ollama/Qwen3, this can be swapped for any LLM backend
/// through configuration alone.
#[async_trait]
pub trait LanguageEngine: Send + Sync {
    /// Get information about this engine.
    fn info(&self) -> LanguageEngineInfo;

    /// Check if the engine is available.
    async fn is_available(&self) -> bool;

    /// Initialize the engine with the given configuration.
    async fn initialize(&mut self, config: &LanguageEngineConfig) -> ConversationResult<()>;

    /// Generate a response (non-streaming).
    async fn generate(&self, request: &GenerateRequest) -> ConversationResult<GenerateResponse>;

    /// Generate a streaming response. Returns a receiver of stream chunks.
    async fn generate_stream(
        &self,
        request: &GenerateRequest,
        session_id: SessionId,
    ) -> ConversationResult<tokio::sync::mpsc::Receiver<ConversationResult<StreamChunk>>>;

    /// Count tokens in a text string.
    async fn count_tokens(&self, text: &str) -> ConversationResult<usize>;

    /// Shutdown the engine.
    async fn shutdown(&mut self) -> ConversationResult<()>;
}

/// Ollama backend implementation.
///
/// Communicates with an Ollama server via its REST API.
pub struct OllamaEngine {
    config: Option<LanguageEngineConfig>,
    client: Option<reqwest::Client>,
}

impl OllamaEngine {
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: None,
            client: None,
        }
    }
}

impl Default for OllamaEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LanguageEngine for OllamaEngine {
    fn info(&self) -> LanguageEngineInfo {
        LanguageEngineInfo {
            backend_type: LanguageBackendType::Ollama,
            model: self
                .config
                .as_ref()
                .map_or_else(|| "unknown".into(), |c| c.model.clone()),
            is_available: self.client.is_some(),
            version: None,
        }
    }

    async fn is_available(&self) -> bool {
        let Some(config) = &self.config else {
            return false;
        };
        let Some(client) = &self.client else {
            return false;
        };
        let url = format!("{}/api/tags", config.base_url);
        matches!(client.get(&url).send().await, Ok(resp) if resp.status().is_success())
    }

    async fn initialize(&mut self, config: &LanguageEngineConfig) -> ConversationResult<()> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| ConversationError::LanguageEngineError(e.to_string()))?;

        self.config = Some(config.clone());
        self.client = Some(client);

        tracing::info!(
            "Ollama engine initialized: model={}, url={}",
            config.model,
            config.base_url
        );
        Ok(())
    }

    async fn generate(&self, request: &GenerateRequest) -> ConversationResult<GenerateResponse> {
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| ConversationError::NotInitialized)?;
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| ConversationError::NotInitialized)?;

        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": m.role.to_string(),
                    "content": m.content,
                })
            })
            .collect();

        let body = serde_json::json!({
            "model": config.model,
            "messages": messages,
            "stream": false,
            "options": {
                "temperature": request.temperature,
                "top_p": request.top_p,
                "num_predict": request.max_tokens,
                "stop": request.stop,
            }
        });

        let url = format!("{}/api/chat", config.base_url);
        let response = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ConversationError::LanguageEngineError(e.to_string()))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| ConversationError::LanguageEngineError(e.to_string()))?;

        if !status.is_success() {
            return Err(ConversationError::LanguageEngineError(format!(
                "Ollama returned status {status}: {text}"
            )));
        }

        let parsed: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| ConversationError::LanguageEngineError(e.to_string()))?;

        let content = parsed["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        let prompt_tokens = parsed["prompt_eval_count"].as_u64().unwrap_or(0) as usize;
        let completion_tokens = parsed["eval_count"].as_u64().unwrap_or(0) as usize;

        Ok(GenerateResponse {
            text: content.clone(),
            usage: TokenUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            },
            finish_reason: if content.is_empty() {
                FinishReason::Error
            } else {
                FinishReason::Stop
            },
        })
    }

    async fn generate_stream(
        &self,
        request: &GenerateRequest,
        session_id: SessionId,
    ) -> ConversationResult<tokio::sync::mpsc::Receiver<ConversationResult<StreamChunk>>> {
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| ConversationError::NotInitialized)?
            .clone();
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| ConversationError::NotInitialized)?
            .clone();

        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": m.role.to_string(),
                    "content": m.content,
                })
            })
            .collect();

        let body = serde_json::json!({
            "model": config.model,
            "messages": messages,
            "stream": true,
            "options": {
                "temperature": request.temperature,
                "top_p": request.top_p,
                "num_predict": request.max_tokens,
            }
        });

        let url = format!("{}/api/chat", config.base_url);
        let response = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ConversationError::LanguageEngineError(e.to_string()))?;

        if !response.status().is_success() {
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".into());
            return Err(ConversationError::LanguageEngineError(format!(
                "Ollama streaming returned error: {text}"
            )));
        }

        let (tx, rx) = tokio::sync::mpsc::channel(256);

        tokio::spawn(async move {
            use futures::StreamExt;

            let mut stream = response.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk_result) = stream.next().await {
                let bytes: bytes::Bytes = match chunk_result {
                    Ok(b) => b,
                    Err(e) => {
                        let _ = tx
                            .send(Err(ConversationError::StreamError(e.to_string())))
                            .await;
                        return;
                    }
                };

                buffer.push_str(&String::from_utf8_lossy(&bytes));

                // Process complete JSON lines.
                while let Some(newline_pos) = buffer.find('\n') {
                    let line = buffer[..newline_pos].trim().to_string();
                    buffer = buffer[newline_pos + 1..].to_string();

                    if line.is_empty() {
                        continue;
                    }

                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&line) {
                        let done = parsed["done"].as_bool().unwrap_or(false);
                        let text = parsed["message"]["content"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string();

                        let usage = if done {
                            Some(TokenUsage {
                                prompt_tokens: parsed["prompt_eval_count"]
                                    .as_u64()
                                    .unwrap_or(0) as usize,
                                completion_tokens: parsed["eval_count"]
                                    .as_u64()
                                    .unwrap_or(0) as usize,
                                total_tokens: (parsed["prompt_eval_count"]
                                    .as_u64()
                                    .unwrap_or(0)
                                    + parsed["eval_count"].as_u64().unwrap_or(0))
                                    as usize,
                            })
                        } else {
                            None
                        };

                        let chunk = StreamChunk {
                            session_id: session_id.clone(),
                            text,
                            done,
                            usage,
                        };

                        if tx.send(Ok(chunk)).await.is_err() {
                            return;
                        }

                        if done {
                            return;
                        }
                    }
                }
            }
        });

        Ok(rx)
    }

    async fn count_tokens(&self, text: &str) -> ConversationResult<usize> {
        // Approximate token count: ~4 chars per token for English.
        // A production implementation would use the model's tokenizer.
        Ok(text.len() / 4)
    }

    async fn shutdown(&mut self) -> ConversationResult<()> {
        self.client = None;
        self.config = None;
        tracing::info!("Ollama engine shut down");
        Ok(())
    }
}
