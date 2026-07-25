use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelId(pub Uuid);

impl ModelId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        Uuid::parse_str(s).ok().map(Self)
    }
}

impl Default for ModelId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ModelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for ModelId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl ModelVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    pub fn as_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl std::fmt::Display for ModelVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Default for ModelVersion {
    fn default() -> Self {
        Self::new(1, 0, 0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelFormat {
    SafeTensors,
    Gguf,
    Gptq,
    Awq,
    Onnx,
    TensorRt,
    OpenVino,
    CoreMl,
    Mlx,
    Bincode,
    Json,
    Raw,
}

impl std::fmt::Display for ModelFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SafeTensors => write!(f, "safetensors"),
            Self::Gguf => write!(f, "gguf"),
            Self::Gptq => write!(f, "gptq"),
            Self::Awq => write!(f, "awq"),
            Self::Onnx => write!(f, "onnx"),
            Self::TensorRt => write!(f, "tensorrt"),
            Self::OpenVino => write!(f, "openvino"),
            Self::CoreMl => write!(f, "coreml"),
            Self::Mlx => write!(f, "mlx"),
            Self::Bincode => write!(f, "bincode"),
            Self::Json => write!(f, "json"),
            Self::Raw => write!(f, "raw"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QuantizationType {
    Fp32,
    Fp16,
    Bf16,
    Int8,
    Int4,
    Gptq4Bit,
    Gptq3Bit,
    GgufQ4_0,
    GgufQ4_1,
    GgufQ5_0,
    GgufQ5_1,
    GgufQ8_0,
    GgufQ2_K,
    GgufQ3_K,
    GgufQ4_K,
    GgufQ5_K,
    GgufQ6_K,
    Awq4Bit,
    Dynamic,
}

impl QuantizationType {
    #[must_use]
    pub fn bits_per_weight(&self) -> f64 {
        match self {
            Self::Fp32 => 32.0,
            Self::Fp16 | Self::Bf16 => 16.0,
            Self::Int8 => 8.0,
            Self::Int4 | Self::Gptq4Bit | Self::Awq4Bit | Self::GgufQ4_0 | Self::GgufQ4_1
            | Self::GgufQ4_K | Self::GgufQ5_K => 4.0,
            Self::Gptq3Bit | Self::GgufQ3_K | Self::GgufQ2_K => 3.0,
            Self::GgufQ5_0 | Self::GgufQ5_1 => 5.0,
            Self::GgufQ8_0 => 8.0,
            Self::GgufQ6_K => 6.0,
            Self::Dynamic => 8.0,
        }
    }
}

impl std::fmt::Display for QuantizationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fp32 => write!(f, "fp32"),
            Self::Fp16 => write!(f, "fp16"),
            Self::Bf16 => write!(f, "bf16"),
            Self::Int8 => write!(f, "int8"),
            Self::Int4 => write!(f, "int4"),
            Self::Gptq4Bit => write!(f, "gptq-4bit"),
            Self::Gptq3Bit => write!(f, "gptq-3bit"),
            Self::GgufQ4_0 => write!(f, "gguf-q4_0"),
            Self::GgufQ4_1 => write!(f, "gguf-q4_1"),
            Self::GgufQ5_0 => write!(f, "gguf-q5_0"),
            Self::GgufQ5_1 => write!(f, "gguf-q5_1"),
            Self::GgufQ8_0 => write!(f, "gguf-q8_0"),
            Self::GgufQ2_K => write!(f, "gguf-q2_k"),
            Self::GgufQ3_K => write!(f, "gguf-q3_k"),
            Self::GgufQ4_K => write!(f, "gguf-q4_k"),
            Self::GgufQ5_K => write!(f, "gguf-q5_k"),
            Self::GgufQ6_K => write!(f, "gguf-q6_k"),
            Self::Awq4Bit => write!(f, "awq-4bit"),
            Self::Dynamic => write!(f, "dynamic"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelArchitecture {
    TransformerDecoder,
    TransformerEncoder,
    EncoderDecoder,
    Gpt,
    Llama,
    Mistral,
    Qwen,
    Phi,
    Gemma,
    Mamba,
    Rwkv,
    mixture_of_experts,
    diffusion,
    Vae,
    Clip,
    Whisper,
    custom(String),
}

impl std::fmt::Display for ModelArchitecture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TransformerDecoder => write!(f, "transformer-decoder"),
            Self::TransformerEncoder => write!(f, "transformer-encoder"),
            Self::EncoderDecoder => write!(f, "encoder-decoder"),
            Self::Gpt => write!(f, "gpt"),
            Self::Llama => write!(f, "llama"),
            Self::Mistral => write!(f, "mistral"),
            Self::Qwen => write!(f, "qwen"),
            Self::Phi => write!(f, "phi"),
            Self::Gemma => write!(f, "gemma"),
            Self::Mamba => write!(f, "mamba"),
            Self::Rwkv => write!(f, "rwkv"),
            Self::mixture_of_experts => write!(f, "mixture-of-experts"),
            Self::diffusion => write!(f, "diffusion"),
            Self::Vae => write!(f, "vae"),
            Self::Clip => write!(f, "clip"),
            Self::Whisper => write!(f, "whisper"),
            Self::custom(name) => write!(f, "custom-{}", name),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub id: ModelId,
    pub name: String,
    pub version: ModelVersion,
    pub architecture: ModelArchitecture,
    pub format: ModelFormat,
    pub quantization: QuantizationType,
    pub path: String,
    pub sha256: Option<String>,
    pub file_size: u64,
    pub parameter_count: u64,
    pub num_layers: u32,
    pub hidden_size: u32,
    pub num_attention_heads: u32,
    pub num_kv_heads: Option<u32>,
    pub intermediate_size: Option<u32>,
    pub vocab_size: u32,
    pub max_position_embeddings: u32,
    pub context_length: u32,
    pub rope_theta: Option<f64>,
    pub eos_token_id: Option<u32>,
    pub bos_token_id: Option<u32>,
    pub pad_token_id: Option<u32>,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
    pub dependencies: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ModelMetadata {
    #[must_use]
    pub fn estimated_memory_bytes(&self) -> u64 {
        let bits = self.quantization.bits_per_weight();
        let bytes_per_param = bits / 8.0;
        (self.parameter_count as f64 * bytes_per_param) as u64
    }
}

#[derive(Debug)]
pub struct ModelSlot {
    pub metadata: ModelMetadata,
    ref_count: AtomicU64,
    pub loaded_at: Option<DateTime<Utc>>,
    memory_allocated: AtomicU64,
    pub is_hot_swapped: bool,
}

impl ModelSlot {
    pub fn new(metadata: ModelMetadata) -> Self {
        Self {
            metadata,
            ref_count: AtomicU64::new(0),
            loaded_at: None,
            memory_allocated: AtomicU64::new(0),
            is_hot_swapped: false,
        }
    }

    pub fn increment_ref(&self) -> u64 {
        self.ref_count.fetch_add(1, Ordering::SeqCst)
    }

    pub fn decrement_ref(&self) -> u64 {
        self.ref_count.fetch_sub(1, Ordering::SeqCst)
    }

    #[must_use]
    pub fn ref_count(&self) -> u64 {
        self.ref_count.load(Ordering::SeqCst)
    }

    pub fn set_memory_allocated(&self, bytes: u64) {
        self.memory_allocated.store(bytes, Ordering::SeqCst);
    }

    #[must_use]
    pub fn memory_allocated(&self) -> u64 {
        self.memory_allocated.load(Ordering::SeqCst)
    }
}

pub mod manager;
pub mod repository;
