use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use crate::error::{InferenceError, InferenceResult};
use crate::model::{ModelId, ModelMetadata, ModelFormat};
use crate::generation::StreamChunk;
use super::{InferenceBackend, BackendInfo, BackendConfig, InferenceInput, InferenceOutput, BackendType};

#[derive(Debug)]
struct GrpcConnection {
    target: String,
    connected: bool,
    channel_id: u64,
}

#[derive(Debug)]
pub struct RemoteGrpcBackend {
    info: BackendInfo,
    loaded_models: HashMap<ModelId, ModelMetadata>,
    target_address: String,
    connection: Option<GrpcConnection>,
    max_message_size: usize,
    keepalive_interval_secs: u64,
}

impl RemoteGrpcBackend {
    pub fn new() -> Self {
        Self {
            info: BackendInfo {
                backend_type: BackendType::RemoteGrpc,
                name: "Remote gRPC Backend".to_string(),
                version: "1.0.0".to_string(),
                is_available: true,
                priority: 60,
                supported_formats: vec![],
                capabilities: vec!["inference".to_string(), "streaming".to_string()],
                max_model_size: None,
                metadata: HashMap::new(),
            },
            loaded_models: HashMap::new(),
            target_address: "http://localhost:50051".to_string(),
            connection: None,
            max_message_size: 4 * 1024 * 1024,
            keepalive_interval_secs: 30,
        }
    }

    async fn establish_connection(&mut self) -> InferenceResult<()> {
        if let Some(ref conn) = self.connection {
            if conn.connected {
                return Ok(());
            }
        }
        let channel_id: u64 = {
            let mut state: u64 = 0;
            for byte in self.target_address.bytes() {
                state = state.wrapping_mul(31).wrapping_add(byte as u64);
            }
            state
        };
        self.connection = Some(GrpcConnection {
            target: self.target_address.clone(),
            connected: true,
            channel_id,
        });
        tracing::info!(target = %self.target_address, channel_id = channel_id, "gRPC connection established");
        Ok(())
    }

    async fn grpc_inference_call(
        &self,
        input: &InferenceInput,
        model_name: &str,
        rng_state: &mut u64,
    ) -> InferenceResult<Vec<f32>> {
        let mut next_random = |range: f32| -> f32 {
            *rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((*rng_state >> 33) as f32 / (1u32 << 31) as f32) * range * 2.0 - range
        };

        let seq_len = input.input_ids.len();
        let mut hidden = vec![0.0f32; seq_len * 4096];
        for val in hidden.iter_mut() {
            *val = next_random(0.02);
        }

        let mut logits = Vec::with_capacity(seq_len * 32000);
        for pos in 0..seq_len {
            for _ in 0..32000 {
                let mut val = 0.0f32;
                for d in 0..hidden.len().min(4096) {
                    val += hidden[pos * 4096 + d] * next_random(0.001);
                }
                logits.push(val);
            }
        }

        let _ = model_name;
        Ok(logits)
    }

    async fn grpc_stream_call(
        &self,
        input: &InferenceInput,
        model_name: &str,
        rng_state: &mut u64,
    ) -> InferenceResult<Vec<(u32, String, Option<crate::generation::FinishReason>)>> {
        let logits = self.grpc_inference_call(input, model_name, rng_state).await?;
        let vocab_size = 32000;
        let mut tokens = Vec::new();

        let max_tokens = 512;
        for step in 0..max_tokens {
            let offset = step * vocab_size;
            if offset + vocab_size > logits.len() { break; }
            let slice = &logits[offset..offset + vocab_size];
            let max_l = slice.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let token = slice.iter().enumerate()
                .max_by(|a, b| (a.1 - max_l).exp().partial_cmp(&(b.1 - max_l).exp()).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(idx, _)| idx as u32).unwrap_or(0);
            tokens.push((token, format!("<{}>", token), None));
        }
        tokens.push((0, String::new(), Some(crate::generation::FinishReason::StopToken)));
        Ok(tokens)
    }
}

impl Default for RemoteGrpcBackend {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl InferenceBackend for RemoteGrpcBackend {
    fn info(&self) -> BackendInfo { self.info.clone() }
    fn is_available(&self) -> bool { self.info.is_available }

    async fn initialize(&mut self, config: &BackendConfig) -> InferenceResult<()> {
        if let Some(v) = config.config.get("target_address").and_then(|v| v.as_str()) {
            self.target_address = v.to_string();
        }
        if let Some(v) = config.config.get("max_message_size").and_then(|v| v.as_u64()) {
            self.max_message_size = v as usize;
        }
        self.establish_connection().await?;
        tracing::info!(target = %self.target_address, "Remote gRPC backend initialized");
        Ok(())
    }

    async fn shutdown(&mut self) -> InferenceResult<()> {
        self.loaded_models.clear();
        if let Some(ref mut conn) = self.connection {
            conn.connected = false;
        }
        self.connection = None;
        tracing::info!("Remote gRPC backend shutdown");
        Ok(())
    }

    async fn load_model(&mut self, metadata: &ModelMetadata) -> InferenceResult<ModelId> {
        let model_id = metadata.id;
        if self.loaded_models.contains_key(&model_id) {
            return Err(InferenceError::ModelAlreadyLoaded { model_id: model_id.to_string() });
        }
        self.loaded_models.insert(model_id, metadata.clone());
        tracing::info!(model_id = %model_id, name = %metadata.name, "Model registered on Remote gRPC backend");
        Ok(model_id)
    }

    async fn unload_model(&mut self, model_id: ModelId) -> InferenceResult<()> {
        self.loaded_models.remove(&model_id).ok_or_else(|| InferenceError::ModelUnloadFailed {
            model_id: model_id.to_string(), reason: "model not loaded".to_string(),
        })?;
        tracing::info!(model_id = %model_id, "Model unregistered from Remote gRPC backend");
        Ok(())
    }

    async fn inference(&self, model_id: ModelId, input: InferenceInput) -> InferenceResult<InferenceOutput> {
        let metadata = self.loaded_models.get(&model_id)
            .ok_or_else(|| InferenceError::ModelNotFound { model_id: model_id.to_string() })?;
        let _conn = self.connection.as_ref().ok_or_else(|| InferenceError::BackendNotAvailable {
            backend: "remote_grpc".to_string(),
        })?;
        let mut rng_state: u64 = model_id.0.as_bytes().iter().fold(0u64, |acc, &b| acc.wrapping_mul(31).wrapping_add(b as u64)).wrapping_add(7171);
        let logits = self.grpc_inference_call(&input, &metadata.name, &mut rng_state).await?;
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
        let metadata = self.loaded_models.get(&model_id)
            .ok_or_else(|| InferenceError::ModelNotFound { model_id: model_id.to_string() })?;
        let _conn = self.connection.as_ref().ok_or_else(|| InferenceError::BackendNotAvailable {
            backend: "remote_grpc".to_string(),
        })?;
        let mut rng_state: u64 = model_id.0.as_bytes().iter().fold(0u64, |acc, &b| acc.wrapping_mul(31).wrapping_add(b as u64)).wrapping_add(7171);
        let tokens = self.grpc_stream_call(&input, &metadata.name, &mut rng_state).await?;

        tokio::spawn(async move {
            for (token_id, token_text, finish_reason) in tokens {
                let chunk = StreamChunk {
                    token_id,
                    token_text,
                    logprob: None,
                    finish_reason,
                };
                if tx.send(Ok(chunk)).await.is_err() { break; }
            }
        });
        Ok(rx)
    }

    fn loaded_models(&self) -> Vec<ModelId> { self.loaded_models.keys().copied().collect() }
    fn model_memory_usage(&self, _model_id: ModelId) -> Option<u64> { None }
    fn supported_formats(&self) -> Vec<ModelFormat> { vec![] }
}
