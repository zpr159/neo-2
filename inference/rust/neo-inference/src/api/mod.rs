use std::collections::HashMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestConfig {
    pub bind_address: String,
    pub port: u16,
    pub max_request_size: usize,
    pub enable_cors: bool,
    pub request_timeout: Duration,
    pub enable_auth: bool,
    pub api_key_header: String,
    pub rate_limit_per_second: u64,
}

impl Default for RestConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            port: 8080,
            max_request_size: 10 * 1024 * 1024,
            enable_cors: true,
            request_timeout: Duration::from_secs(120),
            enable_auth: false,
            api_key_header: "X-API-Key".to_string(),
            rate_limit_per_second: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcConfig {
    pub bind_address: String,
    pub port: u16,
    pub max_message_size: usize,
    pub concurrency_limit: usize,
    pub keepalive_interval: Duration,
    pub request_timeout: Duration,
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            port: 50051,
            max_message_size: 64 * 1024 * 1024,
            concurrency_limit: 100,
            keepalive_interval: Duration::from_secs(30),
            request_timeout: Duration::from_secs(120),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub rest: RestConfig,
    pub grpc: GrpcConfig,
    pub enable_metrics_endpoint: bool,
    pub enable_health_endpoint: bool,
    pub openapi_path: Option<String>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            rest: RestConfig::default(),
            grpc: GrpcConfig::default(),
            enable_metrics_endpoint: true,
            enable_health_endpoint: true,
            openapi_path: Some("/api/openapi.json".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub models_loaded: usize,
    pub active_requests: usize,
    pub gpu_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfoResponse {
    pub id: String,
    pub name: String,
    pub version: String,
    pub architecture: String,
    pub format: String,
    pub quantization: String,
    pub parameter_count: u64,
    pub context_length: u32,
    pub loaded: bool,
    pub memory_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferRequest {
    pub model: String,
    pub messages: Option<Vec<ChatMessage>>,
    pub prompt: Option<String>,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub top_k: Option<usize>,
    pub stream: Option<bool>,
    pub stop: Option<Vec<String>>,
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferResponse {
    pub id: String,
    pub choices: Vec<InferChoice>,
    pub usage: InferUsage,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferChoice {
    pub index: usize,
    pub message: ChatMessage,
    pub finish_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamDelta {
    pub role: Option<String>,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChoice {
    pub index: usize,
    pub delta: StreamDelta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamResponse {
    pub id: String,
    pub choices: Vec<StreamChoice>,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedRequest {
    pub model: String,
    pub input: Vec<String>,
    pub dimensions: Option<usize>,
    pub normalize: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedResponse {
    pub data: Vec<EmbedData>,
    pub model: String,
    pub usage: InferUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedData {
    pub index: usize,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub message: String,
    pub code: String,
    pub status: u16,
    pub details: Option<HashMap<String, serde_json::Value>>,
}

impl ErrorResponse {
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            error: ErrorDetail {
                message: message.into(),
                code: "not_found".to_string(),
                status: 404,
                details: None,
            },
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            error: ErrorDetail {
                message: message.into(),
                code: "bad_request".to_string(),
                status: 400,
                details: None,
            },
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            error: ErrorDetail {
                message: message.into(),
                code: "internal_error".to_string(),
                status: 500,
                details: None,
            },
        }
    }

    pub fn rate_limited() -> Self {
        Self {
            error: ErrorDetail {
                message: "rate limit exceeded".to_string(),
                code: "rate_limited".to_string(),
                status: 429,
                details: None,
            },
        }
    }
}
