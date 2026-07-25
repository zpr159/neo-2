use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::chain::InternalReasoningState;
use crate::error::ReasoningResult;
use crate::reflection::ReflectionResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRef {
    pub source: String,
    pub description: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Explanation {
    pub id: Uuid,
    pub summary: String,
    pub detailed_reasoning: String,
    pub evidence_refs: Vec<EvidenceRef>,
    pub confidence: f32,
    pub alternative_explanations: Vec<String>,
    pub reasoning_depth: usize,
    pub strategy_used: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug)]
pub struct ExplanationEngine {
    include_alternatives: bool,
    max_alternative_count: usize,
}

impl ExplanationEngine {
    pub fn new() -> Self {
        Self {
            include_alternatives: true,
            max_alternative_count: 3,
        }
    }

    pub fn with_alternatives(mut self, include: bool) -> Self {
        self.include_alternatives = include;
        self
    }

    pub fn generate_explanation(
        &self,
        state: &InternalReasoningState,
        reflection: Option<&ReflectionResult>,
        _context: &HashMap<String, serde_json::Value>,
    ) -> ReasoningResult<Explanation> {
        let best_chain = state.best_chain();

        let (summary, detailed, depth, strategy, confidence) = match best_chain {
            Some(chain) => {
                let conclusion = chain
                    .get_conclusion()
                    .map(|s| s.content.clone())
                    .unwrap_or_else(|| "No conclusion reached".to_string());

                let premises: Vec<&str> = chain
                    .get_premises()
                    .iter()
                    .map(|s| s.content.as_str())
                    .collect();

                let inferences: Vec<&str> = chain
                    .get_inferences()
                    .iter()
                    .map(|s| s.content.as_str())
                    .collect();

                let detailed = format!(
                    "Based on {} premises and {} inference steps, the reasoning chain reached the conclusion: {}",
                    premises.len(),
                    inferences.len(),
                    conclusion
                );

                let conf = reflection
                    .map(|r| (chain.average_confidence() + r.confidence_adjustment).clamp(0.0, 1.0))
                    .unwrap_or_else(|| chain.average_confidence());

                (
                    conclusion,
                    detailed,
                    chain.step_count(),
                    chain.strategy.to_string(),
                    conf,
                )
            }
            None => (
                "No reasoning chain completed successfully".to_string(),
                "The reasoning process did not produce a conclusive result".to_string(),
                0,
                "none".to_string(),
                0.0,
            ),
        };

        let evidence_refs = self.extract_evidence_refs(state);
        let alternatives = if self.include_alternatives {
            self.generate_alternatives(state)
        } else {
            Vec::new()
        };

        Ok(Explanation {
            id: Uuid::new_v4(),
            summary,
            detailed_reasoning: detailed,
            evidence_refs,
            confidence,
            alternative_explanations: alternatives,
            reasoning_depth: depth,
            strategy_used: strategy,
            metadata: HashMap::new(),
        })
    }

    fn extract_evidence_refs(&self, state: &InternalReasoningState) -> Vec<EvidenceRef> {
        let mut refs = Vec::new();

        for evidence in &state.accumulated_evidence {
            refs.push(EvidenceRef {
                source: "reasoning_state".to_string(),
                description: evidence.clone(),
                confidence: 0.7,
            });
        }

        for chain in &state.chains {
            for step in &chain.steps {
                if let Some(ref source) = step.source {
                    refs.push(EvidenceRef {
                        source: source.clone(),
                        description: step.content.clone(),
                        confidence: step.confidence,
                    });
                }
            }
        }

        refs
    }

    fn generate_alternatives(&self, state: &InternalReasoningState) -> Vec<String> {
        let mut alternatives = Vec::new();

        for chain in state.all_finalized_chains() {
            if let Some(conclusion) = chain.get_conclusion() {
                let alt = format!(
                    "Alternative via {} strategy: {}",
                    chain.strategy, conclusion.content
                );
                alternatives.push(alt);

                if alternatives.len() >= self.max_alternative_count {
                    break;
                }
            }
        }

        for rejected in &state.rejected_paths {
            alternatives.push(format!("Rejected path: {rejected}"));
            if alternatives.len() >= self.max_alternative_count {
                break;
            }
        }

        alternatives.truncate(self.max_alternative_count);
        alternatives
    }

    pub fn generate_human_readable(&self, explanation: &Explanation) -> String {
        let mut parts = Vec::new();

        parts.push("Reasoning Explanation".to_string());
        parts.push("=".repeat(40));
        parts.push(format!("\nSummary: {}", explanation.summary));
        parts.push(format!("\nConfidence: {:.1}%", explanation.confidence * 100.0));
        parts.push(format!("Strategy: {}", explanation.strategy_used));
        parts.push(format!("Reasoning depth: {} steps", explanation.reasoning_depth));

        if !explanation.evidence_refs.is_empty() {
            parts.push("\nEvidence:".to_string());
            for (i, evidence) in explanation.evidence_refs.iter().enumerate() {
                parts.push(format!(
                    "  {}. [{}] {} (confidence: {:.1}%)",
                    i + 1,
                    evidence.source,
                    evidence.description,
                    evidence.confidence * 100.0
                ));
            }
        }

        parts.push(format!("\nDetailed: {}", explanation.detailed_reasoning));

        if !explanation.alternative_explanations.is_empty() {
            parts.push("\nAlternative explanations:".to_string());
            for alt in &explanation.alternative_explanations {
                parts.push(format!("  - {alt}"));
            }
        }

        parts.join("")
    }

    pub fn to_json(&self, explanation: &Explanation) -> ReasoningResult<String> {
        serde_json::to_string_pretty(explanation)
            .map_err(|e| crate::error::ReasoningError::ExplanationFailed(e.to_string()))
    }
}

impl Default for ExplanationEngine {
    fn default() -> Self {
        Self::new()
    }
}
