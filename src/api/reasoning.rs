use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::ApiError;

/// Reasoning step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStep {
    pub step_type: String,
    pub input: String,
    pub output: String,
    pub confidence: f64,
}

/// Reasoning result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningResult {
    pub conclusion: String,
    pub steps: Vec<ReasoningStep>,
    pub confidence: f64,
    pub contradictions: Vec<String>,
}

/// Reasoning request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningRequest {
    pub query: String,
    pub depth: String,
    pub context: Vec<String>,
}

/// Reasoning API trait.
#[async_trait]
pub trait ReasoningApi: Send + Sync {
    async fn reason(&self, request: ReasoningRequest) -> Result<ReasoningResult, ApiError>;
    async fn check_consistency(&self, statements: Vec<String>) -> Result<bool, ApiError>;
    async fn detect_contradictions(&self, statements: Vec<String>) -> Result<Vec<String>, ApiError>;
}
