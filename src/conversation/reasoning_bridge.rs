use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::conversation::error::ConversationResult;
use crate::conversation::evidence::Evidence;
use crate::conversation::types::ConversationContext;

/// Structured result from the reasoning subsystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningResult {
    pub conclusion: String,
    pub confidence: f32,
    pub reasoning_chain: Vec<ReasoningStep>,
    pub contradictions: Vec<Contradiction>,
    pub assumptions: Vec<String>,
    pub evidence: Vec<Evidence>,
    pub explanation: String,
    pub alternative_conclusions: Vec<AlternativeConclusion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStep {
    pub step_index: usize,
    pub operation: ReasoningOperation,
    pub input: String,
    pub output: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningOperation {
    Deduction,
    Induction,
    Abduction,
    Analogy,
    Classification,
    Comparison,
    CausalInference,
    ProbabilisticInference,
    ConstraintSatisfaction,
    HypothesisGeneration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contradiction {
    pub statement_a: String,
    pub statement_b: String,
    pub severity: ContradictionSeverity,
    pub resolution: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContradictionSeverity {
    Minor,
    Moderate,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternativeConclusion {
    pub conclusion: String,
    pub confidence: f32,
    pub supporting_reasoning: String,
}

/// Bridge between the Reasoning subsystem and the Conversation layer.
#[async_trait]
pub trait ReasoningConversationBridge: Send + Sync {
    /// Perform logical inference on a set of propositions.
    async fn logical_inference(
        &self,
        context: &ConversationContext,
        propositions: &[String],
    ) -> ConversationResult<ReasoningResult>;

    /// Perform symbolic reasoning.
    async fn symbolic_reasoning(
        &self,
        context: &ConversationContext,
        query: &str,
        knowledge: &[String],
    ) -> ConversationResult<ReasoningResult>;

    /// Perform probabilistic reasoning.
    async fn probabilistic_reasoning(
        &self,
        context: &ConversationContext,
        evidence: &[Evidence],
        hypothesis: &str,
    ) -> ConversationResult<ReasoningResult>;

    /// Check consistency of statements.
    async fn consistency_check(
        &self,
        context: &ConversationContext,
        statements: &[String],
    ) -> ConversationResult<ReasoningResult>;

    /// Detect contradictions between statements.
    async fn detect_contradictions(
        &self,
        context: &ConversationContext,
        statements: &[String],
    ) -> ConversationResult<Vec<Contradiction>>;

    /// Generate explanation for a conclusion.
    async fn generate_explanation(
        &self,
        context: &ConversationContext,
        conclusion: &str,
        evidence: &[Evidence],
    ) -> ConversationResult<String>;

    /// Estimate confidence in a conclusion.
    async fn estimate_confidence(
        &self,
        context: &ConversationContext,
        conclusion: &str,
        evidence: &[Evidence],
    ) -> ConversationResult<f32>;
}

/// Mock implementation for testing.
pub struct MockReasoningBridge;

#[async_trait]
impl ReasoningConversationBridge for MockReasoningBridge {
    async fn logical_inference(
        &self,
        _context: &ConversationContext,
        propositions: &[String],
    ) -> ConversationResult<ReasoningResult> {
        Ok(ReasoningResult {
            conclusion: propositions.join(" -> "),
            confidence: 0.8,
            reasoning_chain: Vec::new(),
            contradictions: Vec::new(),
            assumptions: Vec::new(),
            evidence: Vec::new(),
            explanation: "Mock reasoning: accepted all propositions".to_string(),
            alternative_conclusions: Vec::new(),
        })
    }

    async fn symbolic_reasoning(
        &self,
        _context: &ConversationContext,
        query: &str,
        _knowledge: &[String],
    ) -> ConversationResult<ReasoningResult> {
        Ok(ReasoningResult {
            conclusion: query.to_string(),
            confidence: 0.7,
            reasoning_chain: Vec::new(),
            contradictions: Vec::new(),
            assumptions: Vec::new(),
            evidence: Vec::new(),
            explanation: "Mock symbolic reasoning".to_string(),
            alternative_conclusions: Vec::new(),
        })
    }

    async fn probabilistic_reasoning(
        &self,
        _context: &ConversationContext,
        _evidence: &[Evidence],
        hypothesis: &str,
    ) -> ConversationResult<ReasoningResult> {
        Ok(ReasoningResult {
            conclusion: hypothesis.to_string(),
            confidence: 0.6,
            reasoning_chain: Vec::new(),
            contradictions: Vec::new(),
            assumptions: Vec::new(),
            evidence: Vec::new(),
            explanation: "Mock probabilistic reasoning".to_string(),
            alternative_conclusions: Vec::new(),
        })
    }

    async fn consistency_check(
        &self,
        _context: &ConversationContext,
        statements: &[String],
    ) -> ConversationResult<ReasoningResult> {
        Ok(ReasoningResult {
            conclusion: "Consistent".to_string(),
            confidence: 1.0,
            reasoning_chain: Vec::new(),
            contradictions: Vec::new(),
            assumptions: statements.iter().cloned().collect(),
            evidence: Vec::new(),
            explanation: "Mock consistency check: all statements accepted".to_string(),
            alternative_conclusions: Vec::new(),
        })
    }

    async fn detect_contradictions(
        &self,
        _context: &ConversationContext,
        _statements: &[String],
    ) -> ConversationResult<Vec<Contradiction>> {
        Ok(Vec::new())
    }

    async fn generate_explanation(
        &self,
        _context: &ConversationContext,
        conclusion: &str,
        _evidence: &[Evidence],
    ) -> ConversationResult<String> {
        Ok(format!("Mock explanation for: {}", conclusion))
    }

    async fn estimate_confidence(
        &self,
        _context: &ConversationContext,
        _conclusion: &str,
        _evidence: &[Evidence],
    ) -> ConversationResult<f32> {
        Ok(0.8)
    }
}
