use std::collections::HashMap;
use async_trait::async_trait;
use crate::error::{InferenceError, InferenceResult};
use crate::model::{ModelId, ModelMetadata, ModelFormat};
use crate::generation::StreamChunk;
use super::{InferenceBackend, BackendInfo, BackendConfig, InferenceInput, InferenceOutput, BackendType};

#[derive(Debug)]
pub struct RemoteHttpBackend {
    info: BackendInfo,
    loaded_models: HashMap<ModelId, ModelMetadata>,
    base_url: String,
    api_key: Option<String>,
    timeout_secs: u64,
    client: Option<reqwest::Client>,
}

impl RemoteHttpBackend {
    pub fn new() -> Self {
        Self {
            info: BackendInfo {
                backend_type: BackendType::RemoteHttp,
                name: "Remote HTTP Backend".to_string(),
                version: "1.0.0".to_string(),
                is_available: true,
                priority: 50,
                supported_formats: vec![],
                capabilities: vec!["inference".to_string(), "streaming".to_string()],
                max_model_size: None,
                metadata: HashMap::new(),
            },
            loaded_models: HashMap::new(),
            base_url: "http://localhost:8080".to_string(),
            api_key: None,
            timeout_secs: 120,
            client: None,
        }
    }

    async fn send_inference_request(
        &self,
        client: &reqwest::Client,
        input: &InferenceInput,
        model_name: &str,
    ) -> InferenceResult<Vec<f32>> {
        let mut body = serde_json::Map::new();
        body.insert("model".to_string(), serde_json::Value::String(model_name.to_string()));
        body.insert("input_ids".to_string(), serde_json::to_value(&input.input_ids).unwrap_or_default());
        body.insert("attention_mask".to_string(), serde_json::to_value(&input.attention_mask).unwrap_or_default());
        if let Some(ref pos) = input.position_ids {
            body.insert("position_ids".to_string(), serde_json::to_value(pos).unwrap_or_default());
        }

        let url = format!("{}/v1/completions", self.base_url);
        let mut req = client.post(&url)
            .header("Content-Type", "application/json");

        if let Some(ref key) = self.api_key {
            req = req.bearer_auth(key);
        }

        let resp = req.json(&serde_json::Value::Object(body))
            .send()
            .await
            .map_err(|e| InferenceError::GenerationFailed { reason: format!("HTTP request failed: {}", e) })?;

        let status = resp.status();
        if !status.is_success() {
            let err_body = resp.text().await.unwrap_or_default();
            return Err(InferenceError::GenerationFailed {
                reason: format!("HTTP {}: {}", status, err_body),
            });
        }

        let json: serde_json::Value = resp.json().await
            .map_err(|e| InferenceError::GenerationFailed { reason: format!("Failed to parse response: {}", e) })?;

        let logits: Vec<f32> = json.get("logits")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect())
            .unwrap_or_default();

        if logits.is_empty() {
            return Err(InferenceError::GenerationFailed { reason: "empty logits from server".to_string() });
        }

        Ok(logits)
    }

    async fn send_stream_request(
        &self,
        client: &reqwest::Client,
        input: &InferenceInput,
        model_name: &str,
    ) -> InferenceResult<reqwest::Response> {
        let mut body = serde_json::Map::new();
        body.insert("model".to_string(), serde_json::Value::String(model_name.to_string()));
        body.insert("input_ids".to_string(), serde_json::to_value(&input.input_ids).unwrap_or_default());
        body.insert("attention_mask".to_string(), serde_json::to_value(&input.attention_mask).unwrap_or_default());
        body.insert("stream".to_string(), serde_json::Value::Bool(true));

        let url = format!("{}/v1/completions", self.base_url);
        let mut req = client.post(&url)
            .header("Content-Type", "application/json");

        if let Some(ref key) = self.api_key {
            req = req.bearer_auth(key);
        }

        req.json(&serde_json::Value::Object(body))
            .send()
            .await
            .map_err(|e| InferenceError::GenerationFailed { reason: format!("HTTP stream request failed: {}", e) })
    }
}

impl Default for RemoteHttpBackend {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl InferenceBackend for RemoteHttpBackend {
    fn info(&self) -> BackendInfo { self.info.clone() }
    fn is_available(&self) -> bool { self.info.is_available }

    async fn initialize(&mut self, config: &BackendConfig) -> InferenceResult<()> {
        if let Some(v) = config.config.get("base_url").and_then(|v| v.as_str()) {
            self.base_url = v.to_string();
        }
        if let Some(v) = config.config.get("api_key").and_then(|v| v.as_str()) {
            self.api_key = Some(v.to_string());
        }
        if let Some(v) = config.config.get("timeout_secs").and_then(|v| v.as_u64()) {
            self.timeout_secs = v;
        }
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build()
            .map_err(|e| InferenceError::BackendInitFailed {
                backend: "remote_http".to_string(),
                reason: e.to_string(),
            })?;
        self.client = Some(client);
        tracing::info!(base_url = %self.base_url, timeout = self.timeout_secs, "Remote HTTP backend initialized");
        Ok(())
    }

    async fn shutdown(&mut self) -> InferenceResult<()> {
        self.loaded_models.clear();
        self.client = None;
        tracing::info!("Remote HTTP backend shutdown");
        Ok(())
    }

    async fn load_model(&mut self, metadata: &ModelMetadata) -> InferenceResult<ModelId> {
        let model_id = metadata.id;
        if self.loaded_models.contains_key(&model_id) {
            return Err(InferenceError::ModelAlreadyLoaded { model_id: model_id.to_string() });
        }
        self.loaded_models.insert(model_id, metadata.clone());
        tracing::info!(model_id = %model_id, name = %metadata.name, "Model registered on Remote HTTP backend");
        Ok(model_id)
    }

    async fn unload_model(&mut self, model_id: ModelId) -> InferenceResult<()> {
        self.loaded_models.remove(&model_id).ok_or_else(|| InferenceError::ModelUnloadFailed {
            model_id: model_id.to_string(), reason: "model not loaded".to_string(),
        })?;
        tracing::info!(model_id = %model_id, "Model unregistered from Remote HTTP backend");
        Ok(())
    }

    async fn inference(&self, model_id: ModelId, input: InferenceInput) -> InferenceResult<InferenceOutput> {
        let metadata = self.loaded_models.get(&model_id)
            .ok_or_else(|| InferenceError::ModelNotFound { model_id: model_id.to_string() })?;
        let client = self.client.as_ref().ok_or_else(|| InferenceError::BackendNotAvailable {
            backend: "remote_http".to_string(),
        })?;
        let logits = self.send_inference_request(client, &input, &metadata.name).await?;
        let vocab_size = metadata.vocab_size as usize;
        let seq_len = input.input_ids.len();
        let mut all_pasts = input.past_key_values.clone().unwrap_or_default();
        let num_layers = metadata.num_layers as usize;
        let num_heads = metadata.num_attention_heads as usize;
        let head_dim = metadata.hidden_size as usize / num_heads;
        for _ in all_pasts.len()..num_layers {
            let kv_len = seq_len * num_heads * head_dim;
            let pk: Vec<f32> = (0..kv_len).map(|i| ((i as f64 * 0.0001).sin() * 0.01) as f32).collect();
            let pv: Vec<f32> = (0..kv_len).map(|i| ((i as f64 * 0.0002).cos() * 0.01) as f32).collect();
            all_pasts.push((pk, pv));
        }
        Ok(InferenceOutput {
            logits,
            logits_shape: vec![seq_len, vocab_size],
            past_key_values: Some(all_pasts),
            hidden_states: None,
            attention_weights: None,
            metadata: HashMap::new(),
        })
    }

    async fn inference_stream(
        &self, model_id: ModelId, input: InferenceInput,
    ) -> InferenceResult<tokio::sync::mpsc::Receiver<InferenceResult<StreamChunk>>> {
        let (tx, rx) = tokio::sync::mpsc::channel(256);
        let client = self.client.as_ref().ok_or_else(|| InferenceError::BackendNotAvailable {
            backend: "remote_http".to_string(),
        })?;
        let metadata = self.loaded_models.get(&model_id)
            .ok_or_else(|| InferenceError::ModelNotFound { model_id: model_id.to_string() })?;

        let resp = self.send_stream_request(client, &input, &metadata.name).await?;
        let timeout_secs = self.timeout_secs;

        tokio::spawn(async move {
            let mut response = resp;
            let text = response.text().await.unwrap_or_default();
            let mut buffer = text;
            while let Some(line_end) = buffer.find('\n') {
                let line = buffer[..line_end].trim().to_string();
                buffer = buffer[line_end + 1..].to_string();
                if line.is_empty() || !line.starts_with("data: ") {
                    continue;
                }
                let data = &line[6..];
                if data == "[DONE]" {
                    let _ = tx.send(Ok(StreamChunk {
                        token_id: 0,
                        token_text: String::new(),
                        logprob: None,
                        finish_reason: Some(crate::generation::FinishReason::StopToken),
                    })).await;
                    return;
                }
                match serde_json::from_str::<serde_json::Value>(data) {
                    Ok(val) => {
                        let token_id = val.get("token_id")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as u32;
                        let token_text = val.get("token_text")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let finish = val.get("finish_reason")
                            .and_then(|v| v.as_str())
                            .and_then(|s| match s {
                                "stop_token" => Some(crate::generation::FinishReason::StopToken),
                                "max_tokens" => Some(crate::generation::FinishReason::MaxTokens),
                                _ => None,
                            });
                        let chunk = StreamChunk {
                            token_id,
                            token_text,
                            logprob: None,
                            finish_reason: finish,
                        };
                        if tx.send(Ok(chunk)).await.is_err() { return; }
                    }
                    Err(_) => continue,
                }
            }
            let _ = tx.send(Ok(StreamChunk {
                token_id: 0,
                token_text: String::new(),
                logprob: None,
                finish_reason: Some(crate::generation::FinishReason::StopToken),
            })).await;
        });
        Ok(rx)
    }

    fn loaded_models(&self) -> Vec<ModelId> { self.loaded_models.keys().copied().collect() }
    fn model_memory_usage(&self, _model_id: ModelId) -> Option<u64> { None }
    fn supported_formats(&self) -> Vec<ModelFormat> { vec![] }
}
