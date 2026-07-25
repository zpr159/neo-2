use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::chain::InternalReasoningState;
use crate::error::ReasoningResult;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReflectionType {
    SelfEvaluation,
    AnswerVerification,
    ConsistencyCheck,
    ConflictDetection,
    ConfidenceEstimation,
}

impl std::fmt::Display for ReflectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SelfEvaluation => write!(f, "self_evaluation"),
            Self::AnswerVerification => write!(f, "answer_verification"),
            Self::ConsistencyCheck => write!(f, "consistency_check"),
            Self::ConflictDetection => write!(f, "conflict_detection"),
            Self::ConfidenceEstimation => write!(f, "confidence_estimation"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionEntry {
    pub id: Uuid,
    pub reflection_type: ReflectionType,
    pub input_summary: String,
    pub output: String,
    pub score: f32,
    pub issues_found: Vec<String>,
    pub suggestions: Vec<String>,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionResult {
    pub overall_score: f32,
    pub is_consistent: bool,
    pub conflicts_detected: Vec<String>,
    pub confidence_adjustment: f32,
    pub entries: Vec<ReflectionEntry>,
    pub recommendations: Vec<String>,
}

impl ReflectionResult {
    pub fn passed(&self, threshold: f32) -> bool {
        self.overall_score >= threshold && self.is_consistent
    }
}

#[derive(Debug)]
pub struct ReflectionEngine {
    consistency_threshold: f32,
    _conflict_threshold: f32,
}

impl ReflectionEngine {
    pub fn new() -> Self {
        Self {
            consistency_threshold: 0.5,
            _conflict_threshold: 0.3,
        }
    }

    pub fn with_consistency_threshold(mut self, threshold: f32) -> Self {
        self.consistency_threshold = threshold;
        self
    }

    pub fn reflect(
        &self,
        state: &InternalReasoningState,
        _context: &HashMap<String, serde_json::Value>,
    ) -> ReasoningResult<ReflectionResult> {
        let mut entries = Vec::new();
        let mut total_score = 0.0f32;
        let mut all_conflicts = Vec::new();
        let mut all_recommendations = Vec::new();

        let eval = self.evaluate_self(state);
        total_score += eval.score;
        all_recommendations.extend(eval.suggestions.clone());
        entries.push(eval);

        let verif = self.verify_answer(state);
        total_score += verif.score;
        all_recommendations.extend(verif.suggestions.clone());
        entries.push(verif);

        let consist = self.check_consistency(state);
        total_score += consist.score;
        all_conflicts.extend(consist.issues_found.clone());
        entries.push(consist);

        let conflicts = self.detect_conflicts(state);
        total_score += conflicts.score;
        all_conflicts.extend(conflicts.issues_found.clone());
        entries.push(conflicts);

        let conf_est = self.estimate_confidence(state);
        total_score += conf_est.score;
        all_recommendations.extend(conf_est.suggestions.clone());
        entries.push(conf_est);

        let count = entries.len() as f32;
        let overall_score = if count > 0.0 {
            total_score / count
        } else {
            0.0
        };

        let is_consistent = !all_conflicts.iter().any(|c| {
            c.to_lowercase().contains("critical") || c.to_lowercase().contains("contradiction")
        });

        let confidence_adjustment = if is_consistent && overall_score > 0.7 {
            0.1
        } else if !is_consistent {
            -0.2
        } else {
            0.0
        };

        Ok(ReflectionResult {
            overall_score,
            is_consistent,
            conflicts_detected: all_conflicts,
            confidence_adjustment,
            entries,
            recommendations: all_recommendations,
        })
    }

    fn evaluate_self(&self, state: &InternalReasoningState) -> ReflectionEntry {
        let mut issues = Vec::new();
        let mut suggestions = Vec::new();
        let mut score: f32 = 0.7;

        for chain in state.all_finalized_chains() {
            let avg_conf = chain.average_confidence();
            if avg_conf < self.consistency_threshold {
                issues.push(format!(
                    "Chain {} has low average confidence: {:.2}",
                    chain.id, avg_conf
                ));
                suggestions.push("Consider gathering more evidence before concluding".to_string());
                score -= 0.1;
            }

            let min_conf = chain.min_confidence();
            if min_conf < 0.2 {
                issues.push(format!(
                    "Chain {} has a very weak step with confidence {:.2}",
                    chain.id, min_conf
                ));
                suggestions.push("Review weakest reasoning step for validity".to_string());
                score -= 0.1;
            }
        }

        if state.rejected_paths.len() > state.chains.len() {
            suggestions.push("Many paths were rejected - consider alternative strategies".to_string());
            score -= 0.05;
        }

        score = score.clamp(0.0, 1.0);

        ReflectionEntry {
            id: Uuid::new_v4(),
            reflection_type: ReflectionType::SelfEvaluation,
            input_summary: format!("{} chains, {} rejected paths", state.chains.len(), state.rejected_paths.len()),
            output: "Self-evaluation completed".to_string(),
            score,
            issues_found: issues,
            suggestions,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    fn verify_answer(&self, state: &InternalReasoningState) -> ReflectionEntry {
        let mut issues = Vec::new();
        let mut suggestions = Vec::new();
        let mut score: f32 = 0.7;

        let best = state.best_chain();
        match best {
            Some(chain) => {
                if let Some(conclusion) = chain.get_conclusion() {
                    let premises = chain.get_premises();
                    if premises.is_empty() {
                        issues.push("Conclusion reached without explicit premises".to_string());
                        suggestions.push("Add explicit premises to support conclusion".to_string());
                        score -= 0.15;
                    }

                    let inferences = chain.get_inferences();
                    if inferences.is_empty() && chain.step_count() > 2 {
                        issues.push("No intermediate inferences found in chain".to_string());
                        suggestions.push("Add explicit inference steps between premises and conclusion".to_string());
                        score -= 0.1;
                    }

                    if conclusion.confidence < 0.4 {
                        issues.push("Final conclusion has low confidence".to_string());
                        suggestions.push("Gather additional evidence to strengthen conclusion".to_string());
                        score -= 0.15;
                    }
                }
            }
            None => {
                issues.push("No finalized chain with conclusion found".to_string());
                suggestions.push("Complete reasoning before verification".to_string());
                score -= 0.3;
            }
        }

        score = score.clamp(0.0, 1.0);

        ReflectionEntry {
            id: Uuid::new_v4(),
            reflection_type: ReflectionType::AnswerVerification,
            input_summary: "Verifying reasoning output".to_string(),
            output: "Answer verification completed".to_string(),
            score,
            issues_found: issues,
            suggestions,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    fn check_consistency(&self, state: &InternalReasoningState) -> ReflectionEntry {
        let mut issues = Vec::new();
        let mut score: f32 = 0.8;

        let chains = state.all_finalized_chains();
        if chains.len() >= 2 {
            let conclusions: Vec<&str> = chains
                .iter()
                .filter_map(|c| c.get_conclusion())
                .map(|s| s.content.as_str())
                .collect();

            let unique: std::collections::HashSet<&str> = conclusions.iter().copied().collect();
            if unique.len() > 1 && conclusions.len() > 1 {
                issues.push("Multiple chains reach different conclusions".to_string());
                score -= 0.15 * (unique.len() as f32 - 1.0);
            }
        }

        score = score.clamp(0.0, 1.0);

        ReflectionEntry {
            id: Uuid::new_v4(),
            reflection_type: ReflectionType::ConsistencyCheck,
            input_summary: format!("Checking {} finalized chains", chains.len()),
            output: "Consistency check completed".to_string(),
            score,
            issues_found: issues,
            suggestions: Vec::new(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    fn detect_conflicts(&self, state: &InternalReasoningState) -> ReflectionEntry {
        let mut issues = Vec::new();
        let mut score: f32 = 0.8;

        let evidence_set: std::collections::HashSet<&str> =
            state.accumulated_evidence.iter().map(|s| s.as_str()).collect();

        for chain in &state.chains {
            for step in &chain.steps {
                for evidence in &evidence_set {
                    if step.content.to_lowercase().contains("not")
                        && evidence.to_lowercase().contains(&step.content.to_lowercase())
                    {
                        issues.push(format!(
                            "Potential conflict between evidence '{}' and step '{}'",
                            evidence, step.content
                        ));
                        score -= 0.05;
                    }
                }
            }
        }

        score = score.clamp(0.0, 1.0);

        ReflectionEntry {
            id: Uuid::new_v4(),
            reflection_type: ReflectionType::ConflictDetection,
            input_summary: format!(
                "Scanning {} chains and {} evidence items",
                state.chains.len(),
                state.accumulated_evidence.len()
            ),
            output: "Conflict detection completed".to_string(),
            score,
            issues_found: issues,
            suggestions: Vec::new(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    fn estimate_confidence(&self, state: &InternalReasoningState) -> ReflectionEntry {
        let mut suggestions = Vec::new();
        let mut score: f32 = 0.6;

        if let Some(best) = state.best_chain() {
            let chain_conf = best.average_confidence();
            score = chain_conf;

            if chain_conf < 0.5 {
                suggestions.push("Overall confidence is low - consider more evidence".to_string());
            }
            if chain_conf > 0.8 {
                suggestions.push("High confidence - result is likely reliable".to_string());
            }

            let step_count = best.step_count();
            if step_count < 3 {
                suggestions.push("Short reasoning chain - consider deeper analysis".to_string());
                score -= 0.05;
            }
        }

        score = score.clamp(0.0, 1.0);

        ReflectionEntry {
            id: Uuid::new_v4(),
            reflection_type: ReflectionType::ConfidenceEstimation,
            input_summary: "Estimating overall confidence".to_string(),
            output: format!("Estimated confidence: {:.2}", score),
            score,
            issues_found: Vec::new(),
            suggestions,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }
}

impl Default for ReflectionEngine {
    fn default() -> Self {
        Self::new()
    }
}
