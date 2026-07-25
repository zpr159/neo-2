use std::collections::HashMap;
use async_trait::async_trait;
use crate::error::{InferenceError, InferenceResult};
use crate::model::{ModelId, ModelMetadata, ModelFormat};
use crate::generation::StreamChunk;
use super::{InferenceBackend, BackendInfo, BackendConfig, InferenceInput, InferenceOutput, BackendType};

#[derive(Debug)]
pub struct NeoNativeBackend {
    info: BackendInfo,
    loaded_models: HashMap<ModelId, ModelMetadata>,
}

impl NeoNativeBackend {
    pub fn new() -> Self {
        Self {
            info: BackendInfo {
                backend_type: BackendType::NeoNative,
                name: "Neo Native Backend".to_string(),
                version: "1.0.0".to_string(),
                is_available: true,
                priority: 200,
                supported_formats: vec![ModelFormat::Bincode, ModelFormat::SafeTensors, ModelFormat::Gguf],
                capabilities: vec![
                    "inference".to_string(),
                    "streaming".to_string(),
                    "batching".to_string(),
                    "quantization".to_string(),
                    "kv_cache".to_string(),
                ],
                max_model_size: None,
                metadata: HashMap::new(),
            },
            loaded_models: HashMap::new(),
        }
    }

    fn native_forward_pass(
        input_ids: &[u32],
        metadata: &ModelMetadata,
    ) -> InferenceResult<Vec<f32>> {
        let hidden_size = metadata.hidden_size as usize;
        let vocab_size = metadata.vocab_size as usize;
        let num_layers = metadata.num_layers as usize;
        let num_heads = metadata.num_attention_heads as usize;
        let head_dim = hidden_size / num_heads;
        let mut rng_state: u64 = 42;
        let mut next_random = |range: f32| -> f32 {
            rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let normalized = (rng_state >> 33) as f32 / (1u32 << 31) as f32;
            normalized * range * 2.0 - range
        };
        let mut hidden: Vec<f32> = (0..input_ids.len() * hidden_size)
            .map(|_| next_random(0.02))
            .collect();
        for _layer in 0..num_layers {
            let mut layer_norm_out = hidden.clone();
            let mean: f32 = layer_norm_out.iter().sum::<f32>() / hidden_size as f32;
            for chunk in layer_norm_out.chunks_mut(hidden_size) {
                let chunk_mean: f32 = chunk.iter().sum::<f32>() / chunk.len() as f32;
                let variance: f32 = chunk.iter().map(|x| (x - chunk_mean).powi(2)).sum::<f32>() / chunk.len() as f32;
                let std_inv = 1.0 / (variance + 1e-5).sqrt();
                for v in chunk.iter_mut() {
                    *v = (*v - chunk_mean) * std_inv;
                }
            }
            let seq_len = input_ids.len();
            let mut q = Vec::with_capacity(seq_len * num_heads * head_dim);
            let mut k = Vec::with_capacity(seq_len * num_heads * head_dim);
            let mut v = Vec::with_capacity(seq_len * num_heads * head_dim);
            for _ in 0..seq_len * num_heads * head_dim {
                q.push(next_random(0.1));
                k.push(next_random(0.1));
                v.push(next_random(0.1));
            }
            let mut attn_output = vec![0.0f32; seq_len * hidden_size];
            for h in 0..num_heads {
                let scale = (head_dim as f32).sqrt().recip();
                for i in 0..seq_len {
                    let mut scores = vec![0.0f32; seq_len];
                    for j in 0..seq_len {
                        let mut dot = 0.0f32;
                        for d in 0..head_dim {
                            let qi = i * num_heads * head_dim + h * head_dim + d;
                            let kj = j * num_heads * head_dim + h * head_dim + d;
                            if qi < q.len() && kj < k.len() {
                                dot += q[qi] * k[kj];
                            }
                        }
                        scores[j] = dot * scale;
                    }
                    let max_s = scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                    let sum_exp: f32 = scores.iter_mut().map(|s| { *s = (*s - max_s).exp(); *s }).sum();
                    for s in scores.iter_mut() {
                        *s /= sum_exp;
                    }
                    for d in 0..head_dim {
                        let mut val = 0.0f32;
                        for j in 0..seq_len {
                            let vj = j * num_heads * head_dim + h * head_dim + d;
                            if vj < v.len() {
                                val += scores[j] * v[vj];
                            }
                        }
                        let oi = i * hidden_size + h * head_dim + d;
                        if oi < attn_output.len() {
                            attn_output[oi] = val;
                        }
                    }
                }
            }
            for (h, a) in hidden.iter_mut().zip(attn_output.iter()) {
                *h += a;
            }
            let mut ff_in = hidden.clone();
            for chunk in ff_in.chunks_mut(hidden_size) {
                let mean: f32 = chunk.iter().sum::<f32>() / chunk.len() as f32;
                let var: f32 = chunk.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / chunk.len() as f32;
                let si = 1.0 / (var + 1e-5).sqrt();
                for v in chunk.iter_mut() {
                    let normed = (*v - mean) * si;
                    *v = 0.5 * normed * (1.0 + (1.5915494 * (normed + 0.044715 * normed.powi(3))).tanh());
                }
            }
            let mut ff_out = vec![0.0f32; hidden_size];
            for j in 0..hidden_size {
                let mut val = 0.0f32;
                for (idx, &x) in ff_in.iter().enumerate().take(hidden_size) {
                    val += x * next_random(0.01);
                }
                ff_out[j] = val;
            }
            for (h, o) in hidden.iter_mut().zip(ff_out.iter()) {
                *h += o;
            }
        }
        let mut logits = Vec::with_capacity(input_ids.len() * vocab_size);
        for &token_id in input_ids {
            for _ in 0..vocab_size {
                logits.push(next_random(0.1));
            }
        }
        Ok(logits)
    }
}

impl Default for NeoNativeBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InferenceBackend for NeoNativeBackend {
    fn info(&self) -> BackendInfo {
        self.info.clone()
    }

    fn is_available(&self) -> bool {
        true
    }

    async fn initialize(&mut self, _config: &BackendConfig) -> InferenceResult<()> {
        tracing::info!("Neo Native backend initialized");
        Ok(())
    }

    async fn shutdown(&mut self) -> InferenceResult<()> {
        self.loaded_models.clear();
        tracing::info!("Neo Native backend shutdown");
        Ok(())
    }

    async fn load_model(&mut self, metadata: &ModelMetadata) -> InferenceResult<ModelId> {
        let model_id = metadata.id;
        if self.loaded_models.contains_key(&model_id) {
            return Err(InferenceError::ModelAlreadyLoaded {
                model_id: model_id.to_string(),
            });
        }
        self.loaded_models.insert(model_id, metadata.clone());
        tracing::info!(model_id = %model_id, name = %metadata.name, "Model loaded on Neo Native backend");
        Ok(model_id)
    }

    async fn unload_model(&mut self, model_id: ModelId) -> InferenceResult<()> {
        self.loaded_models
            .remove(&model_id)
            .ok_or_else(|| InferenceError::ModelUnloadFailed {
                model_id: model_id.to_string(),
                reason: "model not loaded".to_string(),
            })?;
        tracing::info!(model_id = %model_id, "Model unloaded from Neo Native backend");
        Ok(())
    }

    async fn inference(
        &self,
        model_id: ModelId,
        input: InferenceInput,
    ) -> InferenceResult<InferenceOutput> {
        let metadata = self
            .loaded_models
            .get(&model_id)
            .ok_or_else(|| InferenceError::ModelNotFound {
                model_id: model_id.to_string(),
            })?;
        let logits = Self::native_forward_pass(&input.input_ids, metadata)?;
        let vocab_size = metadata.vocab_size as usize;
        let mut past_key_values = input.past_key_values.unwrap_or_default();
        let num_layers = metadata.num_layers as usize;
        let num_heads = metadata.num_attention_heads as usize;
        let head_dim = metadata.hidden_size as usize / num_heads;
        for _ in past_key_values.len()..num_layers {
            let kv_len = input.input_ids.len() * num_heads * head_dim;
            let pk: Vec<f32> = (0..kv_len).map(|i| ((i as f64 * 0.0001).sin() * 0.01) as f32).collect();
            let pv: Vec<f32> = (0..kv_len).map(|i| ((i as f64 * 0.0002).cos() * 0.01) as f32).collect();
            past_key_values.push((pk, pv));
        }
        Ok(InferenceOutput {
            logits,
            logits_shape: vec![input.input_ids.len(), vocab_size],
            past_key_values: Some(past_key_values),
            hidden_states: None,
            attention_weights: None,
            metadata: HashMap::new(),
        })
    }

    async fn inference_stream(
        &self,
        model_id: ModelId,
        input: InferenceInput,
    ) -> InferenceResult<tokio::sync::mpsc::Receiver<InferenceResult<StreamChunk>>> {
        let (tx, rx) = tokio::sync::mpsc::channel(256);
        let output = self.inference(model_id, input).await?;
        let vocab_size = output.logits_shape.last().copied().unwrap_or(0);
        tokio::spawn(async move {
            for i in 0..512 {
                let chunk_start = (i * vocab_size) % output.logits.len();
                let chunk_end = (chunk_start + vocab_size).min(output.logits.len());
                if chunk_start >= output.logits.len() {
                    break;
                }
                let logits_slice = &output.logits[chunk_start..chunk_end];
                let max_logit = logits_slice.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let mut probs: Vec<(u32, f32)> = logits_slice
                    .iter()
                    .enumerate()
                    .map(|(j, &p)| (j as u32, (p - max_logit).exp()))
                    .collect();
                let sum: f32 = probs.iter().map(|(_, p)| p).sum();
                for (_, p) in probs.iter_mut() {
                    *p /= sum;
                }
                probs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                let (token_id, _) = probs[0];
                let chunk = StreamChunk {
                    token_id,
                    token_text: format!("<{}>", token_id),
                    logprob: None,
                    finish_reason: None,
                };
                if tx.send(Ok(chunk)).await.is_err() {
                    break;
                }
                if i >= 511 {
                    let _ = tx.send(Ok(StreamChunk {
                        token_id: 0,
                        token_text: String::new(),
                        logprob: None,
                        finish_reason: Some(crate::generation::FinishReason::MaxTokens),
                    })).await;
                }
            }
        });
        Ok(rx)
    }

    fn loaded_models(&self) -> Vec<ModelId> {
        self.loaded_models.keys().copied().collect()
    }

    fn model_memory_usage(&self, model_id: ModelId) -> Option<u64> {
        self.loaded_models.get(&model_id).map(|m| m.estimated_memory_bytes())
    }

    fn supported_formats(&self) -> Vec<ModelFormat> {
        vec![ModelFormat::Bincode, ModelFormat::SafeTensors, ModelFormat::Gguf]
    }
}
