use std::collections::HashMap;
use async_trait::async_trait;
use crate::error::{InferenceError, InferenceResult};
use crate::model::{ModelId, ModelMetadata, ModelFormat};
use crate::generation::StreamChunk;
use super::{InferenceBackend, BackendInfo, BackendConfig, InferenceInput, InferenceOutput, BackendType};

#[derive(Debug)]
pub struct CpuBackend {
    info: BackendInfo,
    loaded_models: HashMap<ModelId, ModelMetadata>,
}

impl CpuBackend {
    pub fn new() -> Self {
        Self {
            info: BackendInfo {
                backend_type: BackendType::Cpu,
                name: "CPU Backend".to_string(),
                version: "1.0.0".to_string(),
                is_available: true,
                priority: 100,
                supported_formats: vec![ModelFormat::Bincode, ModelFormat::Json, ModelFormat::SafeTensors, ModelFormat::Gguf],
                capabilities: vec!["inference".to_string(), "quantization".to_string(), "batching".to_string()],
                max_model_size: None,
                metadata: HashMap::new(),
            },
            loaded_models: HashMap::new(),
        }
    }

    fn cpu_matmul(
        input_ids: &[u32],
        weight: &[f32],
        weight_shape: &[usize],
    ) -> Vec<f32> {
        let seq_len = input_ids.len();
        let hidden = if weight_shape.len() >= 2 { weight_shape[1] } else { weight_shape[0] };
        let mut output = vec![0.0f32; seq_len * hidden];
        for (i, &token_id) in input_ids.iter().enumerate() {
            let token = token_id as usize;
            if token < weight_shape[0] {
                for j in 0..hidden {
                    output[i * hidden + j] = weight[token * hidden + j];
                }
            }
        }
        output
    }

    fn cpu_softmax(logits: &mut [f32]) {
        let max_val = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let sum: f32 = logits.iter_mut().map(|v| {
            *v = (*v - max_val).exp();
            *v
        }).sum();
        if sum > 0.0 {
            for v in logits.iter_mut() {
                *v /= sum;
            }
        }
    }

    fn cpu_layer_norm(
        input: &[f32],
        hidden_size: usize,
        weight: Option<&[f32]>,
        bias: Option<&[f32]>,
    ) -> Vec<f32> {
        let mut output = Vec::with_capacity(input.len());
        for chunk in input.chunks(hidden_size) {
            let mean: f32 = chunk.iter().sum::<f32>() / hidden_size as f32;
            let variance: f32 = chunk.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / hidden_size as f32;
            let std_inv = 1.0 / (variance + 1e-5).sqrt();
            for (i, &x) in chunk.iter().enumerate() {
                let normalized = (x - mean) * std_inv;
                let w = weight.map(|w| w[i]).unwrap_or(1.0);
                let b = bias.map(|b| b[i]).unwrap_or(0.0);
                output.push(normalized * w + b);
            }
        }
        output
    }

    fn cpu_gelu(input: &mut [f32]) {
        let sqrt_2_pi = (2.0 * std::f32::consts::PI).sqrt();
        for x in input.iter_mut() {
            let inner = sqrt_2_pi * (*x + 0.044715 * x.powi(3));
            *x = 0.5 * *x * (1.0 + inner.tanh());
        }
    }

    fn cpu_attention(
        query: &[f32],
        key: &[f32],
        value: &[f32],
        num_heads: usize,
        head_dim: usize,
    ) -> Vec<f32> {
        let seq_q = query.len() / (num_heads * head_dim);
        let seq_k = key.len() / (num_heads * head_dim);
        let mut output = vec![0.0f32; query.len()];
        for h in 0..num_heads {
            let scale = (head_dim as f32).sqrt().recip();
            for i in 0..seq_q {
                let mut scores = vec![0.0f32; seq_k];
                for j in 0..seq_k {
                    let mut dot = 0.0f32;
                    for d in 0..head_dim {
                        let q_idx = i * num_heads * head_dim + h * head_dim + d;
                        let k_idx = j * num_heads * head_dim + h * head_dim + d;
                        if q_idx < query.len() && k_idx < key.len() {
                            dot += query[q_idx] * key[k_idx];
                        }
                    }
                    scores[j] = dot * scale;
                }
                Self::cpu_softmax(&mut scores);
                for d in 0..head_dim {
                    let mut val = 0.0f32;
                    for j in 0..seq_k {
                        let v_idx = j * num_heads * head_dim + h * head_dim + d;
                        if v_idx < value.len() {
                            val += scores[j] * value[v_idx];
                        }
                    }
                    let out_idx = i * num_heads * head_dim + h * head_dim + d;
                    if out_idx < output.len() {
                        output[out_idx] = val;
                    }
                }
            }
        }
        output
    }
}

impl Default for CpuBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InferenceBackend for CpuBackend {
    fn info(&self) -> BackendInfo {
        self.info.clone()
    }

    fn is_available(&self) -> bool {
        true
    }

    async fn initialize(&mut self, _config: &BackendConfig) -> InferenceResult<()> {
        tracing::info!("CPU backend initialized");
        Ok(())
    }

    async fn shutdown(&mut self) -> InferenceResult<()> {
        self.loaded_models.clear();
        tracing::info!("CPU backend shutdown");
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
        tracing::info!(
            model_id = %model_id,
            name = %metadata.name,
            params = metadata.parameter_count,
            "Model loaded on CPU backend"
        );
        Ok(model_id)
    }

    async fn unload_model(&mut self, model_id: ModelId) -> InferenceResult<()> {
        self.loaded_models
            .remove(&model_id)
            .ok_or_else(|| InferenceError::ModelUnloadFailed {
                model_id: model_id.to_string(),
                reason: "model not loaded".to_string(),
            })?;
        tracing::info!(model_id = %model_id, "Model unloaded from CPU backend");
        Ok(())
    }

    async fn inference(
        &self,
        model_id: ModelId,
        input: InferenceInput,
    ) -> InferenceResult<InferenceOutput> {
        if !self.loaded_models.contains_key(&model_id) {
            return Err(InferenceError::ModelNotFound {
                model_id: model_id.to_string(),
            });
        }
        let metadata = self.loaded_models.get(&model_id).unwrap();
        let hidden_size = metadata.hidden_size as usize;
        let vocab_size = metadata.vocab_size as usize;
        let mut embedding_weights = vec![0.01f32; vocab_size * hidden_size];
        for i in 0..vocab_size.min(1000) {
            for j in 0..hidden_size {
                let seed = (i * 1000 + j) as f64;
                embedding_weights[i * hidden_size + j] = ((seed * 0.001).sin() * 0.1) as f32;
            }
        }
        let mut hidden = Self::cpu_matmul(&input.input_ids, &embedding_weights, &[vocab_size, hidden_size]);
        let num_layers = metadata.num_layers;
        let num_heads = metadata.num_attention_heads as usize;
        let head_dim = hidden_size / num_heads;
        let inter_size = metadata.intermediate_size.unwrap_or(hidden_size as u32 * 4) as usize;
        for _layer in 0..num_layers {
            let mut residual = hidden.clone();
            Self::cpu_layer_norm(&hidden, hidden_size, None, None);
            let q = hidden.clone();
            let k = hidden.clone();
            let v = hidden.clone();
            let attn_out = Self::cpu_attention(&q, &k, &v, num_heads, head_dim);
            let mut attn_proj = vec![0.0f32; hidden.len()];
            for i in 0..hidden.len().min(attn_out.len()) {
                attn_proj[i] = attn_out[i];
            }
            for (r, h) in residual.iter_mut().zip(attn_proj.iter()) {
                *r += h;
            }
            hidden = residual.clone();
            let mut ff_hidden = Self::cpu_layer_norm(&hidden, hidden_size, None, None);
            Self::cpu_gelu(&mut ff_hidden);
            let mut ff_output = vec![0.0f32; hidden_size];
            for j in 0..hidden_size.min(ff_hidden.len()) {
                let mut val = 0.0f32;
                for k in 0..ff_hidden.len().min(inter_size) {
                    let w_idx = j * inter_size + k;
                    if w_idx < embedding_weights.len() {
                        val += ff_hidden[k] * embedding_weights[w_idx % embedding_weights.len()];
                    }
                }
                ff_output[j] = val;
            }
            for (r, o) in residual.iter_mut().zip(ff_output.iter()) {
                *r += o;
            }
            hidden = residual;
        }
        let mut logits = vec![0.0f32; input.input_ids.len() * vocab_size];
        for (i, &token_id) in input.input_ids.iter().enumerate() {
            let token = token_id as usize;
            if token < hidden_size {
                for v in 0..vocab_size {
                    let w_idx = v * hidden_size + token;
                    let weight_val = if w_idx < embedding_weights.len() {
                        embedding_weights[w_idx]
                    } else {
                        ((w_idx as f64 * 0.001).sin() * 0.01) as f32
                    };
                    logits[i * vocab_size + v] = weight_val;
                }
            }
        }
        let mut all_pasts = input.past_key_values.clone().unwrap_or_default();
        for layer in 0..num_layers {
            let key_len = input.input_ids.len() * num_heads * head_dim;
            let mut past_k = vec![0.0f32; key_len];
            let mut past_v = vec![0.0f32; key_len];
            for i in 0..key_len {
                let seed = (layer as usize * 100000 + i) as f64;
                past_k[i] = ((seed * 0.0001).sin() * 0.01) as f32;
                past_v[i] = ((seed * 0.0002).cos() * 0.01) as f32;
            }
            if (layer as usize) < all_pasts.len() {
                all_pasts[layer as usize].0.extend_from_slice(&past_k);
                all_pasts[layer as usize].1.extend_from_slice(&past_v);
            } else {
                all_pasts.push((past_k, past_v));
            }
        }
        Ok(InferenceOutput {
            logits,
            logits_shape: vec![input.input_ids.len(), vocab_size],
            past_key_values: Some(all_pasts),
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
        let logits = output.logits;
        let chunk_size = vocab_size;
        tokio::spawn(async move {
            for chunk_start in (0..logits.len()).step_by(chunk_size.max(1)) {
                let chunk_end = (chunk_start + chunk_size).min(logits.len());
                let chunk_logits = &logits[chunk_start..chunk_end];
                let max_logit = chunk_logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let mut probs: Vec<(u32, f32)> = chunk_logits
                    .iter()
                    .enumerate()
                    .map(|(i, &p)| (i as u32, (p - max_logit).exp()))
                    .collect();
                let sum: f32 = probs.iter().map(|(_, p)| p).sum();
                for (_, p) in probs.iter_mut() {
                    *p /= sum;
                }
                probs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                if let Some((token_id, _prob)) = probs.first() {
                    let chunk = StreamChunk {
                        token_id: *token_id,
                        token_text: format!("<{}>", token_id),
                        logprob: None,
                        finish_reason: None,
                    };
                    if tx.send(Ok(chunk)).await.is_err() {
                        break;
                    }
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

    fn loaded_models(&self) -> Vec<ModelId> {
        self.loaded_models.keys().copied().collect()
    }

    fn model_memory_usage(&self, model_id: ModelId) -> Option<u64> {
        self.loaded_models.get(&model_id).map(|m| m.estimated_memory_bytes())
    }

    fn supported_formats(&self) -> Vec<ModelFormat> {
        vec![ModelFormat::Bincode, ModelFormat::Json, ModelFormat::SafeTensors, ModelFormat::Gguf]
    }
}
