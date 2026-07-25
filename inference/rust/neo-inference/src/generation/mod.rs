use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SamplingStrategy {
    Greedy,
    TopK,
    TopP,
    Typical,
    Temperature,
    TopKTopP,
    BeamSearch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StopCondition {
    EosToken,
    MaxTokens,
    StopSequence,
    TimeLimit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationParams {
    pub max_tokens: usize,
    pub temperature: f64,
    pub top_k: Option<usize>,
    pub top_p: Option<f64>,
    pub typical_p: Option<f64>,
    pub repetition_penalty: f64,
    pub presence_penalty: f64,
    pub frequency_penalty: f64,
    pub beam_count: usize,
    pub stop_sequences: Vec<String>,
    pub stop_token_ids: Vec<u32>,
    pub seed: Option<u64>,
    pub logprobs: bool,
    pub top_logprobs: Option<usize>,
    pub echo: bool,
}

impl Default for GenerationParams {
    fn default() -> Self {
        Self {
            max_tokens: 512,
            temperature: 1.0,
            top_k: None,
            top_p: None,
            typical_p: None,
            repetition_penalty: 1.0,
            presence_penalty: 0.0,
            frequency_penalty: 0.0,
            beam_count: 1,
            stop_sequences: Vec::new(),
            stop_token_ids: Vec::new(),
            seed: None,
            logprobs: false,
            top_logprobs: None,
            echo: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRequest {
    pub prompt: String,
    pub params: GenerationParams,
    pub model_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResult {
    pub text: String,
    pub tokens: Vec<u32>,
    pub token_texts: Vec<String>,
    pub logprobs: Option<Vec<f64>>,
    pub finish_reason: FinishReason,
    pub usage: TokenUsage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FinishReason {
    StopToken,
    StopSequence,
    MaxTokens,
    Cancelled,
    Error,
}

impl std::fmt::Display for FinishReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StopToken => write!(f, "stop_token"),
            Self::StopSequence => write!(f, "stop_sequence"),
            Self::MaxTokens => write!(f, "max_tokens"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Error => write!(f, "error"),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub token_id: u32,
    pub token_text: String,
    pub logprob: Option<f64>,
    pub finish_reason: Option<FinishReason>,
}

pub mod engine;
