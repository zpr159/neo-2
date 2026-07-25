use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

use crate::time::Timestamp;

/// Unique identifier for a generation request.
pub type GenerationId = uuid::Uuid;

/// Unique identifier for a streaming session.
pub type StreamId = uuid::Uuid;

/// Role of a message in a conversation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

impl fmt::Display for MessageRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageRole::System => write!(f, "system"),
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
            MessageRole::Tool => write!(f, "tool"),
        }
    }
}

/// A single message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn tool(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Tool,
            content: content.into(),
            name: None,
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

/// A tool call requested by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

/// A function call within a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// Definition of a tool available to the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDefinition,
}

/// Definition of a function tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Configuration for a generation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationConfig {
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_top_p")]
    pub top_p: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(default = "default_repeat_penalty")]
    pub repeat_penalty: f32,
    #[serde(default = "default_presence_penalty")]
    pub presence_penalty: f32,
    #[serde(default = "default_frequency_penalty")]
    pub frequency_penalty: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    #[serde(default = "default_stream")]
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

fn default_max_tokens() -> usize {
    2048
}
fn default_temperature() -> f32 {
    0.7
}
fn default_top_p() -> f32 {
    1.0
}
fn default_repeat_penalty() -> f32 {
    1.1
}
fn default_presence_penalty() -> f32 {
    0.0
}
fn default_frequency_penalty() -> f32 {
    0.0
}
fn default_stream() -> bool {
    false
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            model: None,
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            top_p: default_top_p(),
            top_k: None,
            repeat_penalty: default_repeat_penalty(),
            presence_penalty: default_presence_penalty(),
            frequency_penalty: default_frequency_penalty(),
            stop: None,
            seed: None,
            stream: false,
            tools: None,
            response_format: None,
            metadata: HashMap::new(),
        }
    }
}

/// Response format specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseFormat {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "json_object")]
    JsonObject,
}

/// Token usage statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

/// Reason the generation finished.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
    Error,
    Cancelled,
}

impl fmt::Display for FinishReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FinishReason::Stop => write!(f, "stop"),
            FinishReason::Length => write!(f, "length"),
            FinishReason::ToolCalls => write!(f, "tool_calls"),
            FinishReason::ContentFilter => write!(f, "content_filter"),
            FinishReason::Error => write!(f, "error"),
            FinishReason::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Complete response from a generation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResponse {
    pub id: GenerationId,
    pub text: String,
    pub finish_reason: FinishReason,
    pub usage: TokenUsage,
    pub latency: Duration,
    pub provider: String,
    pub model: String,
    pub confidence: Option<f32>,
    pub warnings: Vec<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
    pub created_at: Timestamp,
}

impl GenerationResponse {
    pub fn new(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            text: String::new(),
            finish_reason: FinishReason::Stop,
            usage: TokenUsage::default(),
            latency: Duration::ZERO,
            provider: provider.into(),
            model: model.into(),
            confidence: None,
            warnings: Vec::new(),
            tool_calls: None,
            metadata: HashMap::new(),
            created_at: Timestamp::now(),
        }
    }
}

/// A single chunk in a streaming response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub token: String,
    pub accumulated: String,
    pub finished: bool,
    pub timestamp: Timestamp,
    pub sequence: u64,
    pub finish_reason: Option<FinishReason>,
    pub usage: Option<TokenUsage>,
}

impl StreamChunk {
    pub fn new(token: impl Into<String>, accumulated: impl Into<String>, sequence: u64) -> Self {
        Self {
            token: token.into(),
            accumulated: accumulated.into(),
            finished: false,
            timestamp: Timestamp::now(),
            sequence,
            finish_reason: None,
            usage: None,
        }
    }

    pub fn done(accumulated: impl Into<String>, sequence: u64, finish_reason: FinishReason) -> Self {
        Self {
            token: String::new(),
            accumulated: accumulated.into(),
            finished: true,
            timestamp: Timestamp::now(),
            sequence,
            finish_reason: Some(finish_reason),
            usage: None,
        }
    }
}

/// Model metadata discovered from a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub display_name: Option<String>,
    pub version: Option<String>,
    pub context_length: Option<usize>,
    pub max_output_tokens: Option<usize>,
    pub quantization: Option<String>,
    pub parameter_count: Option<String>,
    pub license: Option<String>,
    pub memory_requirements: Option<String>,
    pub capabilities: ModelCapabilities,
}

/// Capabilities declared by a specific model.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub supports_streaming: bool,
    pub supports_function_calling: bool,
    pub supports_tool_calling: bool,
    pub supports_json_mode: bool,
    pub supports_vision: bool,
    pub supports_audio: bool,
    pub supports_embeddings: bool,
    pub supports_code_generation: bool,
    pub supports_reasoning: bool,
    pub max_context_length: Option<usize>,
    pub max_output_tokens: Option<usize>,
    pub supports_offline: bool,
}

/// Health status of a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealth {
    pub healthy: bool,
    pub latency_ms: Option<f64>,
    pub message: Option<String>,
    pub checked_at: Timestamp,
    pub models_loaded: usize,
    pub memory_usage_bytes: Option<u64>,
}

impl ProviderHealth {
    pub fn healthy() -> Self {
        Self {
            healthy: true,
            latency_ms: None,
            message: None,
            checked_at: Timestamp::now(),
            models_loaded: 0,
            memory_usage_bytes: None,
        }
    }

    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self {
            healthy: false,
            latency_ms: None,
            message: Some(message.into()),
            checked_at: Timestamp::now(),
            models_loaded: 0,
            memory_usage_bytes: None,
        }
    }
}

/// Performance metrics for a provider.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderMetrics {
    pub request_latency_ms: f64,
    pub first_token_latency_ms: f64,
    pub tokens_per_second: f64,
    pub total_tokens_generated: u64,
    pub average_completion_length: f64,
    pub provider_uptime_secs: u64,
    pub retry_count: u64,
    pub failure_count: u64,
    pub active_requests: u64,
    pub queued_requests: u64,
    pub total_requests: u64,
}

/// Cancellation token for generation requests.
#[derive(Debug, Clone)]
pub struct CancellationToken {
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled
            .load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}
