use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use crate::error::{InferenceError, InferenceResult};
use crate::model::{ModelId, ModelMetadata};
use crate::backends::{InferenceBackend, BackendType, BackendInfo, BackendConfig, InferenceInput, InferenceOutput};
use crate::backends::cpu::CpuBackend;
use crate::backends::neo_native::NeoNativeBackend;
use crate::backends::llama_cpp::LlamaCppBackend;
use crate::backends::cuda::CudaBackend;
use crate::backends::rocm::RocmBackend;
use crate::backends::metal::MetalBackend;
use crate::backends::onnx::OnnxBackend;
use crate::backends::tensorrt::TensorRtBackend;
use crate::backends::openvino::OpenVinoBackend;
use crate::backends::mlx::MlxBackend;
use crate::backends::coreml::CoreMlBackend;
use crate::backends::remote_http::RemoteHttpBackend;
use crate::backends::remote_grpc::RemoteGrpcBackend;
use crate::backends::plugin::PluginBackend;
use crate::generation::{StreamChunk, GenerationParams, GenerationResult, FinishReason, TokenUsage};
use crate::scheduler::{InferenceScheduler, SchedulerConfig, ScheduledRequest, InferencePriority};
use crate::telemetry::{InferenceTelemetry, TelemetryConfig};
use crate::memory::MemoryOptimizer;
use crate::multi_gpu::MultiGpuManager;
use crate::distributed::DistributedInferenceManager;
use crate::api::ApiConfig;
use crate::context::ContextConfig;

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub max_batch_size: usize,
    pub max_queue_size: usize,
    pub device: String,
    pub model_cache_size: usize,
    pub timeout_ms: u64,
    pub max_concurrent_requests: usize,
    pub enable_streaming: bool,
    pub enable_dynamic_batching: bool,
    pub kv_cache_size_bytes: u64,
    pub scheduler: SchedulerConfig,
    pub telemetry: TelemetryConfig,
    pub api: ApiConfig,
    pub context: ContextConfig,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 32,
            max_queue_size: 4096,
            device: "cpu".to_string(),
            model_cache_size: 10,
            timeout_ms: 30_000,
            max_concurrent_requests: 64,
            enable_streaming: true,
            enable_dynamic_batching: true,
            kv_cache_size_bytes: 2 * 1024 * 1024 * 1024,
            scheduler: SchedulerConfig::default(),
            telemetry: TelemetryConfig::default(),
            api: ApiConfig::default(),
            context: ContextConfig::default(),
        }
    }
}

#[derive(Debug)]
pub struct InferenceEngine {
    config: EngineConfig,
    backends: RwLock<Vec<Box<dyn InferenceBackend>>>,
    model_backend_map: RwLock<HashMap<ModelId, BackendType>>,
    loaded_models: RwLock<HashMap<ModelId, ModelMetadata>>,
    scheduler: InferenceScheduler,
    telemetry: InferenceTelemetry,
    memory_optimizer: parking_lot::Mutex<MemoryOptimizer>,
    active_requests: AtomicUsize,
}

impl InferenceEngine {
    pub fn new(config: EngineConfig) -> Self {
        let scheduler = InferenceScheduler::new(config.scheduler.clone());
        let telemetry = InferenceTelemetry::new(config.telemetry.clone());
        let memory_optimizer = MemoryOptimizer::new(config.kv_cache_size_bytes);
        Self {
            config,
            backends: RwLock::new(Vec::new()),
            model_backend_map: RwLock::new(HashMap::new()),
            loaded_models: RwLock::new(HashMap::new()),
            scheduler,
            telemetry,
            memory_optimizer: parking_lot::Mutex::new(memory_optimizer),
            active_requests: AtomicUsize::new(0),
        }
    }

    pub fn development() -> Self {
        Self::new(EngineConfig::default())
    }

    pub async fn initialize(&self) -> InferenceResult<()> {
        tracing::info!("Initializing Neo Universal Inference Engine");
        let mut backends: Vec<Box<dyn InferenceBackend>> = Vec::new();
        let mut cpu = CpuBackend::new();
        cpu.initialize(&BackendConfig::default()).await?;
        backends.push(Box::new(cpu));
        let mut native = NeoNativeBackend::new();
        native.initialize(&BackendConfig::default()).await?;
        backends.push(Box::new(native));
        let mut llama_cpp = LlamaCppBackend::new();
        llama_cpp.initialize(&BackendConfig::default()).await?;
        backends.push(Box::new(llama_cpp));
        let mut onnx = OnnxBackend::new();
        onnx.initialize(&BackendConfig::default()).await?;
        backends.push(Box::new(onnx));
        let mut cuda = CudaBackend::new();
        cuda.initialize(&BackendConfig::default()).await?;
        backends.push(Box::new(cuda));
        let mut rocm = RocmBackend::new();
        rocm.initialize(&BackendConfig::default()).await?;
        backends.push(Box::new(rocm));
        let mut metal = MetalBackend::new();
        metal.initialize(&BackendConfig::default()).await?;
        backends.push(Box::new(metal));
        let mut tensorrt = TensorRtBackend::new();
        tensorrt.initialize(&BackendConfig::default()).await?;
        backends.push(Box::new(tensorrt));
        let mut openvino = OpenVinoBackend::new();
        openvino.initialize(&BackendConfig::default()).await?;
        backends.push(Box::new(openvino));
        let mut mlx = MlxBackend::new();
        mlx.initialize(&BackendConfig::default()).await?;
        backends.push(Box::new(mlx));
        let mut coreml = CoreMlBackend::new();
        coreml.initialize(&BackendConfig::default()).await?;
        backends.push(Box::new(coreml));
        let mut http = RemoteHttpBackend::new();
        http.initialize(&BackendConfig::default()).await?;
        backends.push(Box::new(http));
        let mut grpc = RemoteGrpcBackend::new();
        grpc.initialize(&BackendConfig::default()).await?;
        backends.push(Box::new(grpc));
        let mut plugin = PluginBackend::new();
        plugin.initialize(&BackendConfig::default()).await?;
        backends.push(Box::new(plugin));
        *self.backends.write().await = backends;
        tracing::info!(backend_count = self.backends.read().await.len(), "Inference engine initialized with all backends");
        Ok(())
    }

    pub async fn shutdown(&self) -> InferenceResult<()> {
        tracing::info!("Shutting down Inference Engine");
        let mut backends = self.backends.write().await;
        for backend in backends.iter_mut() {
            let _ = backend.shutdown().await;
        }
        backends.clear();
        tracing::info!("Inference Engine shutdown complete");
        Ok(())
    }

    async fn select_backend(&self, metadata: &ModelMetadata) -> InferenceResult<usize> {
        let backends = self.backends.read().await;
        let mut best_idx = None;
        let mut best_priority = 0u32;
        for (idx, backend) in backends.iter().enumerate() {
            if !backend.is_available() {
                continue;
            }
            let info = backend.info();
            let format_compatible = info.supported_formats.contains(&metadata.format);
            let priority = info.priority;
            if format_compatible && priority > best_priority {
                best_priority = priority;
                best_idx = Some(idx);
            }
        }
        if best_idx.is_some() {
            return best_idx.ok_or_else(|| InferenceError::BackendNotAvailable {
                backend: "no suitable backend".to_string(),
            });
        }
        for (idx, backend) in backends.iter().enumerate() {
            if backend.is_available() {
                let priority = backend.info().priority;
                if priority > best_priority {
                    best_priority = priority;
                    best_idx = Some(idx);
                }
            }
        }
        best_idx.ok_or_else(|| InferenceError::BackendNotAvailable {
            backend: "no backends available".to_string(),
        })
    }

    pub async fn load_model(&self, metadata: ModelMetadata) -> InferenceResult<ModelId> {
        let model_id = metadata.id;
        if self.loaded_models.read().await.contains_key(&model_id) {
            return Ok(model_id);
        }
        let backend_idx = self.select_backend(&metadata).await?;
        let backend_type = self.backends.read().await[backend_idx].info().backend_type;
        let mut backends = self.backends.write().await;
        let loaded_id = backends[backend_idx].load_model(&metadata).await?;
        drop(backends);
        self.loaded_models.write().await.insert(model_id, metadata);
        self.model_backend_map.write().await.insert(model_id, backend_type);
        tracing::info!(model_id = %model_id, backend = ?backend_type, "Model loaded");
        Ok(loaded_id)
    }

    pub async fn unload_model(&self, model_id: ModelId) -> InferenceResult<()> {
        let backend_type = self.model_backend_map.write().await.remove(&model_id)
            .ok_or_else(|| InferenceError::ModelNotFound { model_id: model_id.to_string() })?;
        self.loaded_models.write().await.remove(&model_id);
        let mut backends = self.backends.write().await;
        for backend in backends.iter_mut() {
            if backend.info().backend_type == backend_type {
                backend.unload_model(model_id).await?;
                break;
            }
        }
        tracing::info!(model_id = %model_id, "Model unloaded");
        Ok(())
    }

    pub async fn hot_swap(&self, old_model_id: ModelId, new_metadata: ModelMetadata) -> InferenceResult<ModelId> {
        let new_id = self.load_model(new_metadata).await?;
        let _ = self.unload_model(old_model_id).await;
        tracing::info!(old = %old_model_id, new = %new_id, "Model hot-swapped");
        Ok(new_id)
    }

    pub async fn inference(
        &self,
        model_id: ModelId,
        input_ids: Vec<u32>,
        attention_mask: Vec<u32>,
        params: GenerationParams,
    ) -> InferenceResult<GenerationResult> {
        let start = Instant::now();
        self.telemetry.record_request_start();
        self.active_requests.fetch_add(1, Ordering::Relaxed);
        let result = async {
            let backend_type = self.model_backend_map.read().await.get(&model_id).copied()
                .ok_or_else(|| InferenceError::ModelNotFound { model_id: model_id.to_string() })?;
            let mut all_tokens = input_ids.clone();
            let mut all_token_texts = Vec::new();
            let mut total_input_tokens = input_ids.len() as u64;
            let mut past_kv = None;
            let mut generated_tokens = 0u64;
            let max_tokens = params.max_tokens;
            loop {
                let input = InferenceInput {
                    input_ids: all_tokens.clone(),
                    attention_mask: attention_mask.clone(),
                    position_ids: None,
                    past_key_values: past_kv.clone(),
                    parameters: HashMap::new(),
                };
                let backends = self.backends.read().await;
                let backend = backends.iter().find(|b| b.info().backend_type == backend_type)
                    .ok_or_else(|| InferenceError::BackendNotAvailable { backend: backend_type.to_string() })?;
                let output = backend.inference(model_id, input).await?;
                drop(backends);
                past_kv = output.past_key_values;
                let vocab_size = output.logits_shape.last().copied().unwrap_or(0);
                if vocab_size == 0 {
                    break;
                }
                let last_logits_start = (output.logits.len() / input_ids.len().max(1)) * (input_ids.len().saturating_sub(1));
                let logits_end = (last_logits_start + vocab_size).min(output.logits.len());
                if last_logits_start >= output.logits.len() || logits_end <= last_logits_start {
                    break;
                }
                let logits_slice = &output.logits[last_logits_start..logits_end];
                let mut processed_logits: Vec<f32> = logits_slice.to_vec();
                if params.temperature > 0.0 && params.temperature != 1.0 {
                    for l in processed_logits.iter_mut() {
                        *l /= params.temperature as f32;
                    }
                }
                if params.repetition_penalty != 1.0 {
                    for (i, l) in processed_logits.iter_mut().enumerate() {
                        if all_tokens.contains(&(i as u32)) {
                            *l /= params.repetition_penalty as f32;
                        }
                    }
                }
                let max_logit = processed_logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let mut probs: Vec<(u32, f32)> = processed_logits
                    .iter()
                    .enumerate()
                    .map(|(i, &p)| (i as u32, (p - max_logit).exp()))
                    .collect();
                let sum: f32 = probs.iter().map(|(_, p)| p).sum();
                for (_, p) in probs.iter_mut() { *p /= sum; }
                if let Some(top_k) = params.top_k {
                    probs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                    probs.truncate(top_k);
                    let new_sum: f32 = probs.iter().map(|(_, p)| p).sum();
                    for (_, p) in probs.iter_mut() { *p /= new_sum; }
                }
                if let Some(top_p) = params.top_p {
                    probs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                    let mut cumulative = 0.0f32;
                    let top_p_f32 = top_p as f32;
                    probs.retain(|&(_, p)| {
                        cumulative += p;
                        cumulative <= top_p_f32
                    });
                    if probs.is_empty() {
                        probs.push(probs[0]);
                    }
                    let new_sum: f32 = probs.iter().map(|(_, p)| p).sum();
                    for (_, p) in probs.iter_mut() { *p /= new_sum; }
                }
                let token_id = if let Some(seed) = params.seed {
                    let mut rng_state = seed;
                    let r = loop {
                        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                        let val = (rng_state >> 33) as f32 / (1u32 << 31) as f32;
                        if val > 0.0 && val < 1.0 { break val; }
                    };
                    let mut cumulative = 0.0;
                    let mut selected = probs[0].0;
                    for &(id, p) in &probs {
                        cumulative += p;
                        if r <= cumulative {
                            selected = id;
                            break;
                        }
                    }
                    selected
                } else if let Some(&(token_id, _)) = probs.first() {
                    token_id
                } else {
                    break;
                };
                all_tokens = vec![token_id];
                all_token_texts.push(format!("<{}>", token_id));
                generated_tokens += 1;
                if generated_tokens >= max_tokens as u64 {
                    break;
                }
                if params.stop_token_ids.contains(&token_id) {
                    break;
                }
            }
            let completion_tokens = generated_tokens;
            let finish_reason = if generated_tokens >= max_tokens as u64 {
                FinishReason::MaxTokens
            } else {
                FinishReason::StopToken
            };
            let text = all_token_texts.join(" ");
            Ok::<_, InferenceError>(GenerationResult {
                text,
                tokens: all_tokens,
                token_texts: all_token_texts,
                logprobs: None,
                finish_reason,
                usage: TokenUsage {
                    prompt_tokens: total_input_tokens,
                    completion_tokens,
                    total_tokens: total_input_tokens + completion_tokens,
                },
            })
        }.await;
        self.active_requests.fetch_sub(1, Ordering::Relaxed);
        let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
        match &result {
            Ok(gen) => {
                self.telemetry.record_request_complete(
                    latency_ms, true,
                    gen.usage.total_tokens,
                    gen.usage.prompt_tokens,
                    gen.usage.completion_tokens,
                );
            }
            Err(_) => {
                self.telemetry.record_request_complete(latency_ms, false, 0, 0, 0);
            }
        }
        result
    }

    pub async fn inference_stream(
        &self,
        model_id: ModelId,
        input_ids: Vec<u32>,
        attention_mask: Vec<u32>,
        params: GenerationParams,
    ) -> InferenceResult<mpsc::Receiver<InferenceResult<StreamChunk>>> {
        let backend_type = self.model_backend_map.read().await.get(&model_id).copied()
            .ok_or_else(|| InferenceError::ModelNotFound { model_id: model_id.to_string() })?;
        let input = InferenceInput {
            input_ids,
            attention_mask,
            position_ids: None,
            past_key_values: None,
            parameters: HashMap::new(),
        };
        let backends = self.backends.read().await;
        let backend = backends.iter().find(|b| b.info().backend_type == backend_type)
            .ok_or_else(|| InferenceError::BackendNotAvailable { backend: backend_type.to_string() })?;
        backend.inference_stream(model_id, input).await
    }

    pub async fn list_backends(&self) -> Vec<BackendInfo> {
        self.backends.read().await.iter().map(|b| b.info()).collect()
    }

    pub async fn loaded_models(&self) -> Vec<ModelMetadata> {
        self.loaded_models.read().await.values().cloned().collect()
    }

    pub fn active_requests(&self) -> usize {
        self.active_requests.load(Ordering::Relaxed)
    }

    pub fn telemetry_snapshot(&self) -> crate::telemetry::TelemetrySnapshot {
        self.telemetry.snapshot()
    }

    pub fn scheduler_stats(&self) -> crate::scheduler::SchedulerStatistics {
        self.scheduler.statistics()
    }

    pub fn config(&self) -> &EngineConfig {
        &self.config
    }
}
