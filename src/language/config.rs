use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported provider types.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    Ollama,
    LlamaCpp,
    NeoLm,
    OpenAi,
    Anthropic,
    DeepSeek,
    Custom(String),
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::Ollama => write!(f, "ollama"),
            ProviderType::LlamaCpp => write!(f, "llamacpp"),
            ProviderType::NeoLm => write!(f, "neolm"),
            ProviderType::OpenAi => write!(f, "openai"),
            ProviderType::Anthropic => write!(f, "anthropic"),
            ProviderType::DeepSeek => write!(f, "deepseek"),
            ProviderType::Custom(name) => write!(f, "custom_{}", name),
        }
    }
}

/// Load balancing policy.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadBalancingPolicy {
    RoundRobin,
    LeastLoaded,
    LatencyOptimized,
    Priority,
    Weighted,
    StickySession,
}

impl Default for LoadBalancingPolicy {
    fn default() -> Self {
        Self::RoundRobin
    }
}

/// Configuration for a single provider instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider_type: ProviderType,
    pub name: String,
    pub endpoint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default = "default_model_name")]
    pub model_name: String,
    #[serde(default)]
    pub priority: u32,
    #[serde(default = "default_weight")]
    pub weight: f64,
    #[serde(default)]
    pub enabled: bool,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, String>,
}

fn default_model_name() -> String {
    String::new()
}
fn default_weight() -> f64 {
    1.0
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            provider_type: ProviderType::Ollama,
            name: "default".to_string(),
            endpoint: "http://localhost:11434".to_string(),
            api_key: None,
            model_name: default_model_name(),
            priority: 0,
            weight: default_weight(),
            enabled: true,
            extra: HashMap::new(),
        }
    }
}

/// Configuration for model generation parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub model_name: String,
    #[serde(default = "default_max_context")]
    pub max_context: usize,
    #[serde(default = "default_max_output")]
    pub max_output: usize,
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
    #[serde(default = "default_stream_enabled")]
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_layers: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantization: Option<String>,
    #[serde(default)]
    pub offline_mode: bool,
}

fn default_max_context() -> usize {
    4096
}
fn default_max_output() -> usize {
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
fn default_stream_enabled() -> bool {
    true
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            model_name: String::new(),
            max_context: default_max_context(),
            max_output: default_max_output(),
            temperature: default_temperature(),
            top_p: default_top_p(),
            top_k: None,
            repeat_penalty: default_repeat_penalty(),
            presence_penalty: default_presence_penalty(),
            frequency_penalty: default_frequency_penalty(),
            stop: None,
            seed: None,
            stream: default_stream_enabled(),
            gpu_layers: None,
            thread_count: None,
            quantization: None,
            offline_mode: false,
        }
    }
}

/// Configuration for the language engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageEngineConfig {
    pub providers: Vec<ProviderConfig>,
    pub active_provider: String,
    pub model: ModelConfig,
    #[serde(default)]
    pub load_balancing: LoadBalancingPolicy,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_retry_count")]
    pub retry_count: u32,
    #[serde(default = "default_retry_backoff_ms")]
    pub retry_backoff_ms: u64,
    #[serde(default = "default_keep_alive_secs")]
    pub keep_alive_secs: u64,
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    #[serde(default)]
    pub enable_failover: bool,
    #[serde(default = "default_health_check_interval_secs")]
    pub health_check_interval_secs: u64,
}

fn default_timeout_ms() -> u64 {
    30000
}
fn default_retry_count() -> u32 {
    3
}
fn default_retry_backoff_ms() -> u64 {
    1000
}
fn default_keep_alive_secs() -> u64 {
    300
}
fn default_batch_size() -> usize {
    1
}
fn default_health_check_interval_secs() -> u64 {
    60
}

impl Default for LanguageEngineConfig {
    fn default() -> Self {
        Self {
            providers: vec![ProviderConfig::default()],
            active_provider: "default".to_string(),
            model: ModelConfig::default(),
            load_balancing: LoadBalancingPolicy::default(),
            timeout_ms: default_timeout_ms(),
            retry_count: default_retry_count(),
            retry_backoff_ms: default_retry_backoff_ms(),
            keep_alive_secs: default_keep_alive_secs(),
            batch_size: default_batch_size(),
            enable_failover: true,
            health_check_interval_secs: default_health_check_interval_secs(),
        }
    }
}

impl LanguageEngineConfig {
    pub fn ollama_default() -> Self {
        Self {
            providers: vec![ProviderConfig {
                provider_type: ProviderType::Ollama,
                name: "default".to_string(),
                endpoint: "http://localhost:11434".to_string(),
                api_key: None,
                model_name: "qwen2.5:latest".to_string(),
                priority: 0,
                weight: 1.0,
                enabled: true,
                extra: std::collections::HashMap::new(),
            }],
            active_provider: "default".to_string(),
            model: ModelConfig {
                model_name: "qwen2.5:latest".to_string(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    pub fn get_provider_config(&self, name: &str) -> Option<&ProviderConfig> {
        self.providers.iter().find(|p| p.name == name)
    }

    pub fn enabled_providers(&self) -> Vec<&ProviderConfig> {
        self.providers.iter().filter(|p| p.enabled).collect()
    }
}
