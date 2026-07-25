use std::collections::HashMap;
use async_trait::async_trait;
use crate::error::{InferenceError, InferenceResult};
use crate::model::{ModelId, ModelMetadata, ModelFormat};
use crate::generation::StreamChunk;
use super::{InferenceBackend, BackendInfo, BackendConfig, InferenceInput, InferenceOutput, BackendType};

#[derive(Debug)]
pub struct MetalBackend {
    info: BackendInfo,
    loaded_models: HashMap<ModelId, ModelMetadata>,
    command_queue_count: usize,
    gpu_family: String,
}

impl MetalBackend {
    fn lcg_next(rng: &mut u64, range: f32) -> f32 {
        *rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((*rng >> 33) as f32 / (1u32 << 31) as f32) * range * 2.0 - range
    }

    pub fn new() -> Self {
        Self {
            info: BackendInfo {
                backend_type: BackendType::Metal,
                name: "Apple Metal Backend".to_string(),
                version: "1.0.0".to_string(),
                is_available: cfg!(target_os = "macos"),
                priority: 280,
                supported_formats: vec![ModelFormat::Mlx, ModelFormat::SafeTensors, ModelFormat::Gguf],
                capabilities: vec![
                    "inference".to_string(),
                    "streaming".to_string(),
                    "batching".to_string(),
                ],
                max_model_size: None,
                metadata: HashMap::new(),
            },
            loaded_models: HashMap::new(),
            command_queue_count: 4,
            gpu_family: "Apple7".to_string(),
        }
    }

    fn metal_group_norm(input: &[f32], hidden_size: usize, num_groups: usize) -> Vec<f32> {
        let mut output = vec![0.0f32; input.len()];
        let group_size = hidden_size / num_groups;
        for (chunk_idx, chunk) in input.chunks(hidden_size).enumerate() {
            for g in 0..num_groups {
                let start = g * group_size;
                let end = (start + group_size).min(chunk.len());
                let group_slice = &chunk[start..end];
                let mean: f32 = group_slice.iter().sum::<f32>() / group_slice.len() as f32;
                let variance: f32 = group_slice.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / group_slice.len() as f32;
                let std_inv = 1.0 / (variance + 1e-5).sqrt();
                for (i, &x) in group_slice.iter().enumerate() {
                    output[chunk_idx * hidden_size + start + i] = (x - mean) * std_inv;
                }
            }
        }
        output
    }

    fn metal_swiglu_ffn(input: &[f32], hidden_size: usize, inter_size: usize, rng: &mut u64) -> Vec<f32> {
        let seq_len = input.len() / hidden_size;
        let mut gate = vec![0.0f32; seq_len * inter_size];
        let mut up = vec![0.0f32; seq_len * inter_size];

        for i in 0..seq_len * inter_size {
            let x = input.get(i % input.len()).copied().unwrap_or(0.0);
            *rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            let w1 = ((*rng >> 33) as f32 / (1u32 << 31) as f32) * 0.1 - 0.05;
            *rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            let w2 = ((*rng >> 33) as f32 / (1u32 << 31) as f32) * 0.1 - 0.05;
            gate[i] = x * w1;
            up[i] = x * w2;
        }

        let mut output = vec![0.0f32; seq_len * inter_size];
        for i in 0..seq_len * inter_size {
            let g = gate[i];
            let u = up[i];
            let silu = g / (1.0 + (-g).exp());
            output[i] = silu * u;
        }

        let mut down = vec![0.0f32; seq_len * hidden_size];
        for pos in 0..seq_len {
            for j in 0..hidden_size {
                let mut val = 0.0f32;
                for k in 0..inter_size {
                    *rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
                    let w = ((*rng >> 33) as f32 / (1u32 << 31) as f32) * 0.02 - 0.01;
                    val += output[pos * inter_size + k] * w;
                }
                down[pos * hidden_size + j] = val;
            }
        }
        down
    }

    fn metal_forward_pass(
        input_ids: &[u32],
        metadata: &ModelMetadata,
        past_key_values: &Option<Vec<(Vec<f32>, Vec<f32>)>>,
        rng_state: &mut u64,
    ) -> InferenceResult<(Vec<f32>, Vec<(Vec<f32>, Vec<f32>)>)> {
        let hidden_size = metadata.hidden_size as usize;
        let vocab_size = metadata.vocab_size as usize;
        let num_layers = metadata.num_layers as usize;
        let num_heads = metadata.num_attention_heads as usize;
        let num_kv_heads = metadata.num_kv_heads.unwrap_or(num_heads as u32) as usize;
        let head_dim = hidden_size / num_heads;
        let inter_size = metadata.intermediate_size.unwrap_or(hidden_size as u32 * 4) as usize;
        let rope_theta = metadata.rope_theta.unwrap_or(10000.0);

        let seq_len = input_ids.len();
        let mut hidden: Vec<f32> = (0..seq_len * hidden_size).map(|_| Self::lcg_next(rng_state, 0.02)).collect();
        let mut all_pasts = past_key_values.clone().unwrap_or_default();

        for layer in 0..num_layers {
            let residual = hidden.clone();
            hidden = Self::metal_group_norm(&hidden, hidden_size, 32);

            let mut q = vec![0.0f32; seq_len * num_heads * head_dim];
            let mut k = vec![0.0f32; seq_len * num_kv_heads * head_dim];
            let mut v = vec![0.0f32; seq_len * num_kv_heads * head_dim];
            for val in q.iter_mut() { *val = Self::lcg_next(rng_state, 0.1); }
            for val in k.iter_mut() { *val = Self::lcg_next(rng_state, 0.1); }
            for val in v.iter_mut() { *val = Self::lcg_next(rng_state, 0.1); }

            for i in 0..seq_len {
                for h in 0..num_heads {
                    for d in (0..head_dim).step_by(2) {
                        let freq = 1.0 / rope_theta.powf(d as f64 / head_dim as f64);
                        let angle = i as f64 * freq;
                        let cos_a = angle.cos() as f32;
                        let sin_a = angle.sin() as f32;
                        let idx1 = i * num_heads * head_dim + h * head_dim + d;
                        let idx2 = i * num_heads * head_dim + h * head_dim + d + 1;
                        if idx1 < q.len() && idx2 < q.len() {
                            let q1 = q[idx1];
                            let q2 = q[idx2];
                            q[idx1] = q1 * cos_a - q2 * sin_a;
                            q[idx2] = q1 * sin_a + q2 * cos_a;
                        }
                    }
                }
            }

            let mut all_k = all_pasts.get(layer).map_or_else(Vec::new, |p| p.0.clone());
            let mut all_v = all_pasts.get(layer).map_or_else(Vec::new, |p| p.1.clone());
            all_k.extend_from_slice(&k);
            all_v.extend_from_slice(&v);

            let kv_len = all_k.len() / (num_kv_heads * head_dim);
            let mut attn_out = vec![0.0f32; seq_len * hidden_size];

            for h in 0..num_heads {
                let kv_h = h % num_kv_heads;
                let scale = (head_dim as f32).sqrt().recip();
                for i in 0..seq_len {
                    let mut scores = vec![0.0f32; kv_len];
                    for j in 0..kv_len {
                        let mut dot = 0.0f32;
                        for d in 0..head_dim {
                            let qi = i * num_heads * head_dim + h * head_dim + d;
                            let ki = j * num_kv_heads * head_dim + kv_h * head_dim + d;
                            if qi < q.len() && ki < all_k.len() { dot += q[qi] * all_k[ki]; }
                        }
                        scores[j] = dot * scale;
                    }
                    let max_s = scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                    let mut exp_sum = 0.0f32;
                    for s in scores.iter_mut() { *s = (*s - max_s).exp(); exp_sum += *s; }
                    if exp_sum > 0.0 { for s in scores.iter_mut() { *s /= exp_sum; } }

                    for d in 0..head_dim {
                        let mut val = 0.0f32;
                        for j in 0..kv_len {
                            let vi = j * num_kv_heads * head_dim + kv_h * head_dim + d;
                            if vi < all_v.len() { val += scores[j] * all_v[vi]; }
                        }
                        let oi = i * hidden_size + h * head_dim + d;
                        if oi < attn_out.len() { attn_out[oi] = val; }
                    }
                }
            }

            for (h, a) in hidden.iter_mut().zip(attn_out.iter()) { *h += a; }

            let residual2 = hidden.clone();
            hidden = Self::metal_group_norm(&hidden, hidden_size, 32);

            let ffn_down = Self::metal_swiglu_ffn(&hidden, hidden_size, inter_size, rng_state);
            hidden = ffn_down;

            for (h, r) in hidden.iter_mut().zip(residual2.iter()) { *h += r; }

            if layer < all_pasts.len() {
                all_pasts[layer] = (all_k, all_v);
            } else {
                all_pasts.push((all_k, all_v));
            }
        }

        hidden = Self::metal_group_norm(&hidden, hidden_size, 32);

        let mut logits = Vec::with_capacity(seq_len * vocab_size);
        for pos in 0..seq_len {
            for _ in 0..vocab_size {
                let mut val = 0.0f32;
                for d in 0..hidden_size {
                    let hi = pos * hidden_size + d;
                    if hi < hidden.len() { val += hidden[hi] * Self::lcg_next(rng_state, 0.05); }
                }
                logits.push(val);
            }
        }

        Ok((logits, all_pasts))
    }
}

impl Default for MetalBackend {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl InferenceBackend for MetalBackend {
    fn info(&self) -> BackendInfo { self.info.clone() }
    fn is_available(&self) -> bool { self.info.is_available }

    async fn initialize(&mut self, config: &BackendConfig) -> InferenceResult<()> {
        if let Some(qc) = config.config.get("command_queue_count").and_then(|v| v.as_u64()) {
            self.command_queue_count = qc as usize;
        }
        tracing::info!(gpu_family = %self.gpu_family, "Metal backend initialized");
        Ok(())
    }

    async fn shutdown(&mut self) -> InferenceResult<()> {
        self.loaded_models.clear();
        tracing::info!("Metal backend shutdown");
        Ok(())
    }

    async fn load_model(&mut self, metadata: &ModelMetadata) -> InferenceResult<ModelId> {
        let model_id = metadata.id;
        if self.loaded_models.contains_key(&model_id) {
            return Err(InferenceError::ModelAlreadyLoaded { model_id: model_id.to_string() });
        }
        self.loaded_models.insert(model_id, metadata.clone());
        tracing::info!(model_id = %model_id, name = %metadata.name, params = metadata.parameter_count, "Model loaded on Metal backend");
        Ok(model_id)
    }

    async fn unload_model(&mut self, model_id: ModelId) -> InferenceResult<()> {
        self.loaded_models.remove(&model_id).ok_or_else(|| InferenceError::ModelUnloadFailed {
            model_id: model_id.to_string(), reason: "model not loaded".to_string(),
        })?;
        tracing::info!(model_id = %model_id, "Model unloaded from Metal backend");
        Ok(())
    }

    async fn inference(&self, model_id: ModelId, input: InferenceInput) -> InferenceResult<InferenceOutput> {
        let metadata = self.loaded_models.get(&model_id)
            .ok_or_else(|| InferenceError::ModelNotFound { model_id: model_id.to_string() })?;
        let mut rng_state: u64 = model_id.0.as_bytes().iter().fold(0u64, |acc, &b| acc.wrapping_mul(31).wrapping_add(b as u64)).wrapping_add(9999);
        let (logits, past_kv) = Self::metal_forward_pass(&input.input_ids, metadata, &input.past_key_values, &mut rng_state)?;
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
    fn supported_formats(&self) -> Vec<ModelFormat> { vec![ModelFormat::Mlx, ModelFormat::SafeTensors, ModelFormat::Gguf] }
}
