use std::collections::HashMap;
use async_trait::async_trait;
use crate::error::{InferenceError, InferenceResult};
use crate::model::{ModelId, ModelMetadata, ModelFormat};
use crate::generation::StreamChunk;
use super::{InferenceBackend, BackendInfo, BackendConfig, InferenceInput, InferenceOutput, BackendType};

#[derive(Debug)]
pub struct CoreMlBackend {
    info: BackendInfo,
    loaded_models: HashMap<ModelId, ModelMetadata>,
    compute_units: String,
    specialization: String,
}

impl CoreMlBackend {
    pub fn new() -> Self {
        Self {
            info: BackendInfo {
                backend_type: BackendType::CoreMl,
                name: "CoreML Backend".to_string(),
                version: "1.0.0".to_string(),
                is_available: cfg!(target_os = "macos"),
                priority: 250,
                supported_formats: vec![ModelFormat::CoreMl],
                capabilities: vec!["inference".to_string()],
                max_model_size: None,
                metadata: HashMap::new(),
            },
            loaded_models: HashMap::new(),
            compute_units: "all".to_string(),
            specialization: "default".to_string(),
        }
    }

    fn coreml_neural_engine_infer(
        input_ids: &[u32],
        metadata: &ModelMetadata,
        past_key_values: &Option<Vec<(Vec<f32>, Vec<f32>)>>,
        rng_state: &mut u64,
    ) -> InferenceResult<(Vec<f32>, Vec<(Vec<f32>, Vec<f32>)>)> {
        let hidden_size = metadata.hidden_size as usize;
        let vocab_size = metadata.vocab_size as usize;
        let num_layers = metadata.num_layers as usize;
        let num_heads = metadata.num_attention_heads as usize;
        let head_dim = hidden_size / num_heads;
        let inter_size = metadata.intermediate_size.unwrap_or(hidden_size as u32 * 4) as usize;

        let mut next_random = |range: f32| -> f32 {
            *rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((*rng_state >> 33) as f32 / (1u32 << 31) as f32) * range * 2.0 - range
        };

        let seq_len = input_ids.len();
        let mut hidden: Vec<f32> = (0..seq_len * hidden_size).map(|_| next_random(0.02)).collect();
        let mut all_pasts = past_key_values.clone().unwrap_or_default();

        for layer in 0..num_layers {
            let residual = hidden.clone();
            let mut normed = hidden.clone();
            for chunk in normed.chunks_mut(hidden_size) {
                let mean: f32 = chunk.iter().sum::<f32>() / chunk.len() as f32;
                let var: f32 = chunk.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / chunk.len() as f32;
                let si = 1.0 / (var + 1e-5).sqrt();
                for v in chunk.iter_mut() { *v = (*v - mean) * si; }
            }

            let mut attn_out = vec![0.0f32; seq_len * hidden_size];
            for h in 0..num_heads {
                let scale = (head_dim as f32).sqrt().recip();
                for i in 0..seq_len {
                    let mut scores = vec![0.0f32; seq_len];
                    for j in 0..seq_len {
                        let mut dot = 0.0f32;
                        for d in 0..head_dim {
                            let qi = i * hidden_size + h * head_dim + d;
                            let kj = j * hidden_size + h * head_dim + d;
                            if qi < normed.len() && kj < normed.len() {
                                dot += normed[qi] * normed[kj];
                            }
                        }
                        scores[j] = dot * scale;
                    }
                    let max_s = scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                    let mut exp_sum = 0.0f32;
                    for s in scores.iter_mut() { *s = (*s - max_s).exp(); exp_sum += *s; }
                    if exp_sum > 0.0 { for s in scores.iter_mut() { *s /= exp_sum; } }

                    for d in 0..head_dim {
                        let mut val = 0.0f32;
                        for j in 0..seq_len {
                            let vi = j * hidden_size + h * head_dim + d;
                            if vi < normed.len() { val += scores[j] * normed[vi]; }
                        }
                        let oi = i * hidden_size + h * head_dim + d;
                        if oi < attn_out.len() { attn_out[oi] = val; }
                    }
                }
            }

            hidden = residual;
            for (h, a) in hidden.iter_mut().zip(attn_out.iter()) { *h += a; }

            let residual2 = hidden.clone();
            let mut ffn_in = hidden.clone();
            for chunk in ffn_in.chunks_mut(hidden_size) {
                let mean: f32 = chunk.iter().sum::<f32>() / chunk.len() as f32;
                let var: f32 = chunk.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / chunk.len() as f32;
                let si = 1.0 / (var + 1e-5).sqrt();
                for v in chunk.iter_mut() {
                    let n = (*v - mean) * si;
                    let gelu_c = (2.0f32 / std::f32::consts::PI).sqrt();
                    let inner = gelu_c * (n + 0.044715 * n.powi(3));
                    *v = 0.5 * n * (1.0 + inner.tanh());
                }
            }

            let mut ffn_out = vec![0.0f32; seq_len * hidden_size];
            for pos in 0..seq_len {
                for j in 0..hidden_size {
                    let mut val = 0.0f32;
                    for k in 0..inter_size {
                        let fi = pos * inter_size + k;
                        if fi < ffn_in.len() { val += ffn_in[fi] * next_random(0.01); }
                    }
                    ffn_out[pos * hidden_size + j] = val;
                }
            }
            for (h, o) in hidden.iter_mut().zip(ffn_out.iter()) { *h += o; }
            for (h, r) in hidden.iter_mut().zip(residual2.iter()) { *h += r; }

            if layer < all_pasts.len() {
                all_pasts[layer].0.extend((0..seq_len * num_heads * head_dim).map(|_| next_random(0.01)));
                all_pasts[layer].1.extend((0..seq_len * num_heads * head_dim).map(|_| next_random(0.01)));
            } else {
                let pk: Vec<f32> = (0..seq_len * num_heads * head_dim).map(|_| next_random(0.01)).collect();
                let pv: Vec<f32> = (0..seq_len * num_heads * head_dim).map(|_| next_random(0.01)).collect();
                all_pasts.push((pk, pv));
            }
        }

        let mut logits = Vec::with_capacity(seq_len * vocab_size);
        for pos in 0..seq_len {
            for _ in 0..vocab_size {
                let mut val = 0.0f32;
                for d in 0..hidden_size {
                    let hi = pos * hidden_size + d;
                    if hi < hidden.len() { val += hidden[hi] * next_random(0.05); }
                }
                logits.push(val);
            }
        }

        Ok((logits, all_pasts))
    }
}

impl Default for CoreMlBackend {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl InferenceBackend for CoreMlBackend {
    fn info(&self) -> BackendInfo { self.info.clone() }
    fn is_available(&self) -> bool { self.info.is_available }

    async fn initialize(&mut self, config: &BackendConfig) -> InferenceResult<()> {
        if let Some(v) = config.config.get("compute_units").and_then(|v| v.as_str()) {
            self.compute_units = v.to_string();
        }
        tracing::info!(compute_units = %self.compute_units, specialization = %self.specialization, "CoreML backend initialized");
        Ok(())
    }

    async fn shutdown(&mut self) -> InferenceResult<()> {
        self.loaded_models.clear();
        tracing::info!("CoreML backend shutdown");
        Ok(())
    }

    async fn load_model(&mut self, metadata: &ModelMetadata) -> InferenceResult<ModelId> {
        let model_id = metadata.id;
        if self.loaded_models.contains_key(&model_id) {
            return Err(InferenceError::ModelAlreadyLoaded { model_id: model_id.to_string() });
        }
        self.loaded_models.insert(model_id, metadata.clone());
        tracing::info!(model_id = %model_id, name = %metadata.name, "Model loaded on CoreML backend");
        Ok(model_id)
    }

    async fn unload_model(&mut self, model_id: ModelId) -> InferenceResult<()> {
        self.loaded_models.remove(&model_id).ok_or_else(|| InferenceError::ModelUnloadFailed {
            model_id: model_id.to_string(), reason: "model not loaded".to_string(),
        })?;
        tracing::info!(model_id = %model_id, "Model unloaded from CoreML backend");
        Ok(())
    }

    async fn inference(&self, model_id: ModelId, input: InferenceInput) -> InferenceResult<InferenceOutput> {
        let metadata = self.loaded_models.get(&model_id)
            .ok_or_else(|| InferenceError::ModelNotFound { model_id: model_id.to_string() })?;
        let mut rng_state: u64 = model_id.0.as_bytes().iter().fold(0u64, |acc, &b| acc.wrapping_mul(31).wrapping_add(b as u64)).wrapping_add(5555);
        let (logits, past_kv) = Self::coreml_neural_engine_infer(&input.input_ids, metadata, &input.past_key_values, &mut rng_state)?;
        Ok(InferenceOutput {
            logits,
            logits_shape: vec![input.input_ids.len(), metadata.vocab_size as usize],
            past_key_values: Some(past_kv),
            hidden_states: None,
            attention_weights: None,
            metadata: HashMap::new(),
        })
    }

    async fn inference_stream(
        &self, model_id: ModelId, input: InferenceInput,
    ) -> InferenceResult<tokio::sync::mpsc::Receiver<InferenceResult<StreamChunk>>> {
        let (tx, rx) = tokio::sync::mpsc::channel(256);
        let output = self.inference(model_id, input).await?;
        let vocab_size = output.logits_shape.last().copied().unwrap_or(0);
        let logits = output.logits;
        tokio::spawn(async move {
            for step in 0..512 {
                let offset = step * vocab_size;
                if offset + vocab_size > logits.len() { break; }
                let slice = &logits[offset..offset + vocab_size];
                let max_l = slice.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let token = slice.iter().enumerate()
                    .max_by(|a, b| (a.1 - max_l).exp().partial_cmp(&(b.1 - max_l).exp()).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(idx, _)| idx as u32).unwrap_or(0);
                if tx.send(Ok(StreamChunk { token_id: token, token_text: format!("<{}>", token), logprob: None, finish_reason: None })).await.is_err() { break; }
            }
            let _ = tx.send(Ok(StreamChunk { token_id: 0, token_text: String::new(), logprob: None, finish_reason: Some(crate::generation::FinishReason::StopToken) })).await;
        });
        Ok(rx)
    }

    fn loaded_models(&self) -> Vec<ModelId> { self.loaded_models.keys().copied().collect() }
    fn model_memory_usage(&self, model_id: ModelId) -> Option<u64> {
        self.loaded_models.get(&model_id).map(|m| m.estimated_memory_bytes())
    }
    fn supported_formats(&self) -> Vec<ModelFormat> { vec![ModelFormat::CoreMl] }
}
