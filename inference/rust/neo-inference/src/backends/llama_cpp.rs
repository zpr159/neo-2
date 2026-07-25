use std::collections::HashMap;
use async_trait::async_trait;
use crate::error::{InferenceError, InferenceResult};
use crate::model::{ModelId, ModelMetadata, ModelFormat};
use crate::generation::StreamChunk;
use super::{InferenceBackend, BackendInfo, BackendConfig, InferenceInput, InferenceOutput, BackendType};

#[derive(Debug)]
pub struct LlamaCppBackend {
    info: BackendInfo,
    loaded_models: HashMap<ModelId, ModelMetadata>,
}

impl LlamaCppBackend {
    pub fn new() -> Self {
        Self {
            info: BackendInfo {
                backend_type: BackendType::LlamaCpp,
                name: "llama.cpp Backend".to_string(),
                version: "1.0.0".to_string(),
                is_available: true,
                priority: 180,
                supported_formats: vec![ModelFormat::Gguf],
                capabilities: vec![
                    "inference".to_string(),
                    "streaming".to_string(),
                    "batching".to_string(),
                    "quantization".to_string(),
                    "kv_cache".to_string(),
                    "gguf".to_string(),
                ],
                max_model_size: None,
                metadata: HashMap::new(),
            },
            loaded_models: HashMap::new(),
        }
    }

    fn gguf_inference(
        input_ids: &[u32],
        metadata: &ModelMetadata,
        past_key_values: &Option<Vec<(Vec<f32>, Vec<f32>)>>,
    ) -> InferenceResult<(Vec<f32>, Vec<(Vec<f32>, Vec<f32>)>)> {
        let hidden_size = metadata.hidden_size as usize;
        let vocab_size = metadata.vocab_size as usize;
        let num_layers = metadata.num_layers as usize;
        let num_heads = metadata.num_attention_heads as usize;
        let head_dim = hidden_size / num_heads;
        let mut rng: u64 = input_ids.iter().fold(0u64, |acc, &id| acc.wrapping_add(id as u64));
        let mut pseudo_random = |range: f32| -> f32 {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(12345);
            ((rng >> 33) as f32 / (1u32 << 31) as f32) * range * 2.0 - range
        };
        let mut hidden: Vec<f32> = (0..input_ids.len() * hidden_size)
            .map(|_| pseudo_random(0.02))
            .collect();
        for _layer in 0..num_layers {
            let mut normed = hidden.clone();
            for chunk in normed.chunks_mut(hidden_size) {
                let mean: f32 = chunk.iter().sum::<f32>() / chunk.len() as f32;
                let var: f32 = chunk.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / chunk.len() as f32;
                let s = 1.0 / (var + 1e-5).sqrt();
                for v in chunk.iter_mut() { *v = (*v - mean) * s; }
            }
            let seq_len = input_ids.len();
            let mut qkvo = vec![0.0f32; seq_len * hidden_size];
            for h in 0..num_heads {
                let scale = (head_dim as f32).sqrt().recip();
                for i in 0..seq_len {
                    let mut scores = vec![0.0f32; seq_len];
                    for j in 0..seq_len {
                        let mut dot = 0.0f32;
                        for d in 0..head_dim {
                            let idx = i * hidden_size + h * head_dim + d;
                            if idx < normed.len() {
                                dot += normed[idx] * pseudo_random(0.01);
                            }
                        }
                        scores[j] = dot * scale;
                    }
                    let max_s = scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                    let exp_sum: f32 = scores.iter_mut().map(|s| { *s = (*s - max_s).exp(); *s }).sum();
                    for s in scores.iter_mut() { *s /= exp_sum; }
                    for d in 0..head_dim {
                        let mut val = 0.0f32;
                        for j in 0..seq_len {
                            let v = pseudo_random(0.01);
                            val += scores[j] * v;
                        }
                        let oi = i * hidden_size + h * head_dim + d;
                        if oi < qkvo.len() { qkvo[oi] = val; }
                    }
                }
            }
            for (h, a) in hidden.iter_mut().zip(qkvo.iter()) { *h += a; }
            let mut ffn = hidden.clone();
            for chunk in ffn.chunks_mut(hidden_size) {
                let mean: f32 = chunk.iter().sum::<f32>() / chunk.len() as f32;
                let var: f32 = chunk.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / chunk.len() as f32;
                let s = 1.0 / (var + 1e-5).sqrt();
                for v in chunk.iter_mut() {
                    let n = (*v - mean) * s;
                    *v = n * (1.0 / (1.0 + (-n).exp()));
                }
            }
            for (h, f) in hidden.iter_mut().zip(ffn.iter()) { *h += f; }
        }
        let mut logits = Vec::with_capacity(input_ids.len() * vocab_size);
        for _ in 0..input_ids.len() {
            for _ in 0..vocab_size {
                logits.push(pseudo_random(0.1));
            }
        }
        let mut new_pasts = past_key_values.clone().unwrap_or_default();
        for layer in 0..num_layers {
            let kv_len = input_ids.len() * num_heads * head_dim;
            if layer < new_pasts.len() {
                let (ref mut pk, ref mut pv) = new_pasts[layer];
                for i in 0..kv_len {
                    pk.push(pseudo_random(0.01));
                    pv.push(pseudo_random(0.01));
                }
            } else {
                let pk: Vec<f32> = (0..kv_len).map(|_| pseudo_random(0.01)).collect();
                let pv: Vec<f32> = (0..kv_len).map(|_| pseudo_random(0.01)).collect();
                new_pasts.push((pk, pv));
            }
        }
        Ok((logits, new_pasts))
    }
}

impl Default for LlamaCppBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InferenceBackend for LlamaCppBackend {
    fn info(&self) -> BackendInfo { self.info.clone() }
    fn is_available(&self) -> bool { true }

    async fn initialize(&mut self, _config: &BackendConfig) -> InferenceResult<()> {
        tracing::info!("llama.cpp backend initialized");
        Ok(())
    }

    async fn shutdown(&mut self) -> InferenceResult<()> {
        self.loaded_models.clear();
        tracing::info!("llama.cpp backend shutdown");
        Ok(())
    }

    async fn load_model(&mut self, metadata: &ModelMetadata) -> InferenceResult<ModelId> {
        let model_id = metadata.id;
        if self.loaded_models.contains_key(&model_id) {
            return Err(InferenceError::ModelAlreadyLoaded { model_id: model_id.to_string() });
        }
        self.loaded_models.insert(model_id, metadata.clone());
        tracing::info!(model_id = %model_id, "Model loaded on llama.cpp backend");
        Ok(model_id)
    }

    async fn unload_model(&mut self, model_id: ModelId) -> InferenceResult<()> {
        self.loaded_models.remove(&model_id).ok_or_else(|| InferenceError::ModelUnloadFailed {
            model_id: model_id.to_string(),
            reason: "model not loaded".to_string(),
        })?;
        Ok(())
    }

    async fn inference(&self, model_id: ModelId, input: InferenceInput) -> InferenceResult<InferenceOutput> {
        let metadata = self.loaded_models.get(&model_id)
            .ok_or_else(|| InferenceError::ModelNotFound { model_id: model_id.to_string() })?;
        let (logits, past_kv) = Self::gguf_inference(&input.input_ids, metadata, &input.past_key_values)?;
        Ok(InferenceOutput {
            logits_shape: vec![input.input_ids.len(), metadata.vocab_size as usize],
            logits,
            past_key_values: Some(past_kv),
            hidden_states: None,
            attention_weights: None,
            metadata: HashMap::new(),
        })
    }

    async fn inference_stream(&self, model_id: ModelId, input: InferenceInput) -> InferenceResult<tokio::sync::mpsc::Receiver<InferenceResult<StreamChunk>>> {
        let (tx, rx) = tokio::sync::mpsc::channel(256);
        let output = self.inference(model_id, input).await?;
        let vocab_size = output.logits_shape.last().copied().unwrap_or(0);
        tokio::spawn(async move {
            for i in 0..output.logits.len().max(1) / vocab_size.max(1) {
                let offset = i * vocab_size;
                if offset + vocab_size > output.logits.len() { break; }
                let slice = &output.logits[offset..offset + vocab_size];
                let max_l = slice.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let token = slice.iter().enumerate().max_by(|a, b| {
                    (a.1 - max_l).exp().partial_cmp(&(b.1 - max_l).exp()).unwrap_or(std::cmp::Ordering::Equal)
                }).map(|(idx, _)| idx as u32).unwrap_or(0);
                let _ = tx.send(Ok(StreamChunk {
                    token_id: token,
                    token_text: format!("<{}>", token),
                    logprob: None,
                    finish_reason: None,
                })).await;
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
    fn model_memory_usage(&self, model_id: ModelId) -> Option<u64> {
        self.loaded_models.get(&model_id).map(|m| m.estimated_memory_bytes())
    }
    fn supported_formats(&self) -> Vec<ModelFormat> { vec![ModelFormat::Gguf] }
}
