use async_trait::async_trait;

use crate::error::{ConversationError, ConversationResult};
use crate::language::{
    FinishReason, GenerateRequest, GenerateResponse, LanguageBackendType, LanguageEngine,
    LanguageEngineConfig, LanguageEngineInfo,
};
use crate::types::{StreamChunk, TokenUsage, SessionId};

/// DeepSeek API provider (OpenAI-compatible endpoint).
pub struct DeepSeekProvider {
    config: Option<LanguageEngineConfig>,
    client: Option<reqwest::Client>,
}

impl DeepSeekProvider {
    pub fn new() -> Self {
        Self {
            config: None,
            client: None,
        }
    }
}

impl Default for DeepSeekProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LanguageEngine for DeepSeekProvider {
    fn info(&self) -> LanguageEngineInfo {
        LanguageEngineInfo {
            backend_type: LanguageBackendType::Custom("deepseek".into()),
            model: self
                .config
                .as_ref()
                .map_or_else(|| "unknown".into(), |c| c.model.clone()),
            is_available: self.client.is_some(),
            version: None,
        }
    }

    async fn is_available(&self) -> bool {
        self.client.is_some() && self.config.is_some()
    }

    async fn initialize(&mut self, config: &LanguageEngineConfig) -> ConversationResult<()> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| ConversationError::LanguageEngineError(e.to_string()))?;

        self.config = Some(config.clone());
        self.client = Some(client);
        tracing::info!(
            "DeepSeek provider initialized: model={}",
            config.model
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

        let mut body = serde_json::json!({
            "model": config.model,
            "messages": messages,
            "temperature": request.temperature,
            "top_p": request.top_p,
            "max_tokens": request.max_tokens,
            "stream": false,
        });

        if !request.stop.is_empty() {
            body["stop"] = serde_json::json!(request.stop);
        }

        let url = format!("{}/v1/chat/completions", config.base_url);
        let mut req = client.post(&url).json(&body);
        if let Some(ref api_key) = config.api_key {
            req = req.header("Authorization", format!("Bearer {api_key}"));
        }

        let response = req
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
                "DeepSeek returned status {status}: {text}"
            )));
        }

        let parsed: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| ConversationError::LanguageEngineError(e.to_string()))?;

        let content = parsed["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        let prompt_tokens = parsed["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as usize;
        let completion_tokens =
            parsed["usage"]["completion_tokens"].as_u64().unwrap_or(0) as usize;

        let finish_reason = match parsed["choices"][0]["finish_reason"].as_str() {
            Some("stop") => FinishReason::Stop,
            Some("length") => FinishReason::Length,
            _ => FinishReason::Stop,
        };

        Ok(GenerateResponse {
            text: content,
            usage: TokenUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            },
            finish_reason,
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

        let mut body = serde_json::json!({
            "model": config.model,
            "messages": messages,
            "temperature": request.temperature,
            "top_p": request.top_p,
            "max_tokens": request.max_tokens,
            "stream": true,
        });

        if !request.stop.is_empty() {
            body["stop"] = serde_json::json!(request.stop);
        }

        let url = format!("{}/v1/chat/completions", config.base_url);
        let mut req = client.post(&url).json(&body);
        if let Some(ref api_key) = config.api_key {
            req = req.header("Authorization", format!("Bearer {api_key}"));
        }

        let response = req
            .send()
            .await
            .map_err(|e| ConversationError::LanguageEngineError(e.to_string()))?;

        if !response.status().is_success() {
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".into());
            return Err(ConversationError::LanguageEngineError(format!(
                "DeepSeek streaming error: {text}"
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

                while let Some(newline_pos) = buffer.find('\n') {
                    let line = buffer[..newline_pos].trim().to_string();
                    buffer = buffer[newline_pos + 1..].to_string();

                    if line.is_empty() || !line.starts_with("data: ") {
                        continue;
                    }

                    let json_str = &line[6..];
                    if json_str == "[DONE]" {
                        let _ = tx
                            .send(Ok(StreamChunk {
                                session_id: session_id.clone(),
                                text: String::new(),
                                done: true,
                                usage: None,
                            }))
                            .await;
                        return;
                    }

                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                        let text = parsed["choices"][0]["delta"]["content"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string();

                        let done = !parsed["choices"][0]["finish_reason"].is_null();

                        let _ = tx
                            .send(Ok(StreamChunk {
                                session_id: session_id.clone(),
                                text,
                                done,
                                usage: None,
                            }))
                            .await;
                    }
                }
            }
        });

        Ok(rx)
    }

    async fn count_tokens(&self, text: &str) -> ConversationResult<usize> {
        Ok(text.len() / 4)
    }

    async fn shutdown(&mut self) -> ConversationResult<()> {
        self.client = None;
        self.config = None;
        tracing::info!("DeepSeek provider shut down");
        Ok(())
    }
}
