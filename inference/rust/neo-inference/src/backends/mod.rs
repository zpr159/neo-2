use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{InferenceError, InferenceResult};
use crate::model::{ModelId, ModelMetadata, ModelFormat};
use crate::generation::StreamChunk;

pub mod cpu;
pub mod neo_native;
pub mod cuda;
pub mod rocm;
pub mod metal;
pub mod llama_cpp;
pub mod onnx;
pub mod tensorrt;
pub mod openvino;
pub mod mlx;
pub mod coreml;
pub mod remote_http;
pub mod remote_grpc;
pub mod plugin;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BackendType {
    Cpu,
    Cuda,
    Rocm,
    Metal,
    LlamaCpp,
    Onnx,
    TensorRt,
    OpenVino,
    Mlx,
    CoreMl,
    RemoteHttp,
    RemoteGrpc,
    Plugin,
    NeoNative,
}

impl fmt::Display for BackendType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cpu => write!(f, "cpu"),
            Self::Cuda => write!(f, "cuda"),
            Self::Rocm => write!(f, "rocm"),
            Self::Metal => write!(f, "metal"),
            Self::LlamaCpp => write!(f, "llama_cpp"),
            Self::Onnx => write!(f, "onnx"),
            Self::TensorRt => write!(f, "tensorrt"),
            Self::OpenVino => write!(f, "openvino"),
            Self::Mlx => write!(f, "mlx"),
            Self::CoreMl => write!(f, "coreml"),
            Self::RemoteHttp => write!(f, "remote_http"),
            Self::RemoteGrpc => write!(f, "remote_grpc"),
            Self::Plugin => write!(f, "plugin"),
            Self::NeoNative => write!(f, "neo_native"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendInfo {
    pub backend_type: BackendType,
    pub name: String,
    pub version: String,
    pub is_available: bool,
    pub priority: u32,
    pub supported_formats: Vec<ModelFormat>,
    pub capabilities: Vec<String>,
    pub max_model_size: Option<u64>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    pub backend_type: BackendType,
    pub enabled: bool,
    pub device_id: Option<u32>,
    pub num_threads: usize,
    pub memory_limit_bytes: Option<u64>,
    pub config: HashMap<String, serde_json::Value>,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            backend_type: BackendType::Cpu,
            enabled: true,
            device_id: None,
            num_threads: 4,
            memory_limit_bytes: None,
            config: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InferenceInput {
    pub input_ids: Vec<u32>,
    pub attention_mask: Vec<u32>,
    pub position_ids: Option<Vec<u32>>,
    pub past_key_values: Option<Vec<(Vec<f32>, Vec<f32>)>>,
    pub parameters: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct InferenceOutput {
    pub logits: Vec<f32>,
    pub logits_shape: Vec<usize>,
    pub past_key_values: Option<Vec<(Vec<f32>, Vec<f32>)>>,
    pub hidden_states: Option<Vec<f32>>,
    pub attention_weights: Option<Vec<f32>>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[async_trait]
pub trait InferenceBackend: Send + Sync + fmt::Debug {
    fn info(&self) -> BackendInfo;

    fn is_available(&self) -> bool;

    async fn initialize(&mut self, config: &BackendConfig) -> InferenceResult<()>;

    async fn shutdown(&mut self) -> InferenceResult<()>;

    async fn load_model(&mut self, metadata: &ModelMetadata) -> InferenceResult<ModelId>;

    async fn unload_model(&mut self, model_id: ModelId) -> InferenceResult<()>;

    async fn inference(
        &self,
        model_id: ModelId,
        input: InferenceInput,
    ) -> InferenceResult<InferenceOutput>;

    async fn inference_stream(
        &self,
        model_id: ModelId,
        input: InferenceInput,
    ) -> InferenceResult<tokio::sync::mpsc::Receiver<InferenceResult<StreamChunk>>>;

    fn loaded_models(&self) -> Vec<ModelId>;

    fn model_memory_usage(&self, model_id: ModelId) -> Option<u64>;

    fn supported_formats(&self) -> Vec<ModelFormat>;
}

pub fn probe_available_backends() -> Vec<BackendInfo> {
    let mut backends = Vec::new();
    backends.push(BackendInfo {
        backend_type: BackendType::Cpu,
        name: "CPU Backend".to_string(),
        version: "1.0.0".to_string(),
        is_available: true,
        priority: 100,
        supported_formats: vec![ModelFormat::Bincode, ModelFormat::Json, ModelFormat::SafeTensors],
        capabilities: vec!["inference".to_string(), "quantization".to_string()],
        max_model_size: None,
        metadata: HashMap::new(),
    });
    backends.push(BackendInfo {
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
    });
    backends.push(BackendInfo {
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
    });
    backends.push(BackendInfo {
        backend_type: BackendType::Onnx,
        name: "ONNX Runtime Backend".to_string(),
        version: "1.0.0".to_string(),
        is_available: true,
        priority: 160,
        supported_formats: vec![ModelFormat::Onnx],
        capabilities: vec!["inference".to_string(), "batching".to_string()],
        max_model_size: None,
        metadata: HashMap::new(),
    });
    backends.push(BackendInfo {
        backend_type: BackendType::Cuda,
        name: "CUDA Backend".to_string(),
        version: "1.0.0".to_string(),
        is_available: cfg!(feature = "cuda"),
        priority: 300,
        supported_formats: vec![ModelFormat::SafeTensors, ModelFormat::Gguf, ModelFormat::TensorRt],
        capabilities: vec![
            "inference".to_string(),
            "streaming".to_string(),
            "batching".to_string(),
            "quantization".to_string(),
            "kv_cache".to_string(),
            "multi_gpu".to_string(),
        ],
        max_model_size: None,
        metadata: HashMap::new(),
    });
    backends.push(BackendInfo {
        backend_type: BackendType::Rocm,
        name: "ROCm Backend".to_string(),
        version: "1.0.0".to_string(),
        is_available: false,
        priority: 290,
        supported_formats: vec![ModelFormat::SafeTensors, ModelFormat::Gguf],
        capabilities: vec!["inference".to_string(), "batching".to_string()],
        max_model_size: None,
        metadata: HashMap::new(),
    });
    backends.push(BackendInfo {
        backend_type: BackendType::Metal,
        name: "Apple Metal Backend".to_string(),
        version: "1.0.0".to_string(),
        is_available: cfg!(target_os = "macos"),
        priority: 280,
        supported_formats: vec![ModelFormat::Mlx, ModelFormat::SafeTensors, ModelFormat::Gguf],
        capabilities: vec!["inference".to_string(), "streaming".to_string(), "batching".to_string()],
        max_model_size: None,
        metadata: HashMap::new(),
    });
    backends.push(BackendInfo {
        backend_type: BackendType::TensorRt,
        name: "TensorRT Backend".to_string(),
        version: "1.0.0".to_string(),
        is_available: false,
        priority: 310,
        supported_formats: vec![ModelFormat::TensorRt],
        capabilities: vec!["inference".to_string(), "optimization".to_string()],
        max_model_size: None,
        metadata: HashMap::new(),
    });
    backends.push(BackendInfo {
        backend_type: BackendType::OpenVino,
        name: "OpenVINO Backend".to_string(),
        version: "1.0.0".to_string(),
        is_available: false,
        priority: 200,
        supported_formats: vec![ModelFormat::OpenVino],
        capabilities: vec!["inference".to_string()],
        max_model_size: None,
        metadata: HashMap::new(),
    });
    backends.push(BackendInfo {
        backend_type: BackendType::Mlx,
        name: "Apple MLX Backend".to_string(),
        version: "1.0.0".to_string(),
        is_available: cfg!(target_os = "macos"),
        priority: 270,
        supported_formats: vec![ModelFormat::Mlx, ModelFormat::SafeTensors],
        capabilities: vec!["inference".to_string(), "streaming".to_string()],
        max_model_size: None,
        metadata: HashMap::new(),
    });
    backends.push(BackendInfo {
        backend_type: BackendType::CoreMl,
        name: "CoreML Backend".to_string(),
        version: "1.0.0".to_string(),
        is_available: cfg!(target_os = "macos"),
        priority: 250,
        supported_formats: vec![ModelFormat::CoreMl],
        capabilities: vec!["inference".to_string()],
        max_model_size: None,
        metadata: HashMap::new(),
    });
    backends.push(BackendInfo {
        backend_type: BackendType::RemoteHttp,
        name: "Remote HTTP Backend".to_string(),
        version: "1.0.0".to_string(),
        is_available: true,
        priority: 50,
        supported_formats: vec![],
        capabilities: vec!["inference".to_string(), "streaming".to_string()],
        max_model_size: None,
        metadata: HashMap::new(),
    });
    backends.push(BackendInfo {
        backend_type: BackendType::RemoteGrpc,
        name: "Remote gRPC Backend".to_string(),
        version: "1.0.0".to_string(),
        is_available: true,
        priority: 60,
        supported_formats: vec![],
        capabilities: vec!["inference".to_string(), "streaming".to_string()],
        max_model_size: None,
        metadata: HashMap::new(),
    });
    backends.push(BackendInfo {
        backend_type: BackendType::Plugin,
        name: "Plugin Backend".to_string(),
        version: "1.0.0".to_string(),
        is_available: true,
        priority: 10,
        supported_formats: vec![],
        capabilities: vec!["inference".to_string()],
        max_model_size: None,
        metadata: HashMap::new(),
    });
    backends
}
