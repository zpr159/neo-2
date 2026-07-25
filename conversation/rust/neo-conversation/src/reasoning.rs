use crate::error::ConversationResult;
use crate::types::CognitiveContext;

/// Interface to the reasoning subsystem.
///
/// Provides chain-of-thought reasoning, logical inference,
/// and evidence-based conclusion generation.
pub trait ReasoningInterface: Send + Sync {
    /// Perform reasoning on the given query with context.
    fn reason(&self, query: &str, context: &CognitiveContext) -> ConversationResult<ReasoningResult>;

    /// Validate a claim against available evidence.
    fn validate_claim(
        &self,
        claim: &str,
        evidence: &[String],
    ) -> ConversationResult<ValidationResult>;

    /// Get the reasoning chain for a previous result.
    fn get_chain(&self, result_id: &str) -> ConversationResult<ReasoningChain>;
}

/// Result of a reasoning operation.
#[derive(Debug, Clone)]
pub struct ReasoningResult {
    pub result_id: String,
    pub chain: ReasoningChain,
    pub conclusion: String,
    pub confidence: f64,
}

/// A chain of reasoning steps.
#[derive(Debug, Clone)]
pub struct ReasoningChain {
    pub steps: Vec<ReasoningStep>,
    pub summary: String,
}

/// A single step in a reasoning chain.
#[derive(Debug, Clone)]
pub struct ReasoningStep {
    pub index: usize,
    pub premise: String,
    pub inference: String,
    pub supports: bool,
}

/// Result of a claim validation.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub claim: String,
    pub supported: bool,
    pub confidence: f64,
    pub reasoning: String,
}

/// Default reasoner using simple chain-of-thought.
pub struct DefaultReasoner;

impl ReasoningInterface for DefaultReasoner {
    fn reason(
        &self,
        query: &str,
        _context: &CognitiveContext,
    ) -> ConversationResult<ReasoningResult> {
        let result_id = uuid::Uuid::new_v4().to_string();
        let chain = ReasoningChain {
            steps: vec![ReasoningStep {
                index: 0,
                premise: format!("User asked: {query}"),
                inference: "Analyzing the request and formulating a response.".into(),
                supports: true,
            }],
            summary: "Basic reasoning applied.".into(),
        };

        Ok(ReasoningResult {
            result_id,
            chain,
            conclusion: format!("Based on analysis of the query: {query}"),
            confidence: 0.7,
        })
    }

    fn validate_claim(
        &self,
        claim: &str,
        evidence: &[String],
    ) -> ConversationResult<ValidationResult> {
        let supported = !evidence.is_empty();
        Ok(ValidationResult {
            claim: claim.to_string(),
            supported,
            confidence: if supported { 0.8 } else { 0.2 },
            reasoning: format!(
                "Claim evaluated against {} pieces of evidence.",
                evidence.len()
            ),
        })
    }

    fn get_chain(&self, _result_id: &str) -> ConversationResult<ReasoningChain> {
        Ok(ReasoningChain {
            steps: Vec::new(),
            summary: "Chain not found".into(),
        })
    }
}
