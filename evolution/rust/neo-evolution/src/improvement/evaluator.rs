use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::{EvolutionId, RiskLevel};

use super::candidate::ImprovementCandidate;
use super::priority::ImprovementPriority;
use super::proposal::ImprovementProposal;

/// Result of evaluating a single improvement proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementEvaluation {
    /// ID of the proposal that was evaluated.
    pub proposal_id: EvolutionId,
    /// Composite score in `[0.0, 1.0]` — higher is better.
    pub score: f64,
    /// Estimated feasibility in `[0.0, 1.0]` — 1.0 means trivially feasible.
    pub feasibility: f64,
    /// Assessed risk level.
    pub risk_assessment: RiskLevel,
    /// Human-readable recommendation.
    pub recommendation: String,
    /// When this evaluation was performed.
    pub evaluated_at: DateTime<Utc>,
}

/// Evaluates improvement proposals and produces ranked recommendations.
#[derive(Debug, Clone)]
pub struct ImprovementEvaluator {
    /// Weight given to priority in the composite score.
    priority_weight: f64,
    /// Weight given to estimated impact.
    impact_weight: f64,
    /// Weight given to risk (inverse — higher risk lowers score).
    risk_weight: f64,
    /// Weight given to feasibility.
    feasibility_weight: f64,
}

impl ImprovementEvaluator {
    /// Create an evaluator with default weights.
    pub fn new() -> Self {
        Self {
            priority_weight: 0.25,
            impact_weight: 0.35,
            risk_weight: 0.25,
            feasibility_weight: 0.15,
        }
    }

    /// Create an evaluator with explicit weights (will be normalised to sum to 1.0).
    pub fn with_weights(
        priority_weight: f64,
        impact_weight: f64,
        risk_weight: f64,
        feasibility_weight: f64,
    ) -> Self {
        let total = priority_weight + impact_weight + risk_weight + feasibility_weight;
        let inv = if total > 0.0 { 1.0 / total } else { 1.0 };
        Self {
            priority_weight: priority_weight * inv,
            impact_weight: impact_weight * inv,
            risk_weight: risk_weight * inv,
            feasibility_weight: feasibility_weight * inv,
        }
    }

    /// Evaluate a single proposal and return an [`ImprovementEvaluation`].
    pub fn evaluate(&self, proposal: &ImprovementProposal) -> ImprovementEvaluation {
        let candidate = &proposal.candidate;

        let priority_score = self.priority_to_score(candidate.priority);
        let impact_score = candidate.estimated_impact;
        let risk_score = self.risk_to_score(candidate.risk_level);
        let feasibility_score = self.estimate_feasibility(candidate);

        let composite = self.priority_weight * priority_score
            + self.impact_weight * impact_score
            + self.risk_weight * risk_score
            + self.feasibility_weight * feasibility_score;

        let score = composite.clamp(0.0, 1.0);
        let feasibility = feasibility_score.clamp(0.0, 1.0);

        let recommendation = self.build_recommendation(score, candidate);

        ImprovementEvaluation {
            proposal_id: proposal.id,
            score,
            feasibility,
            risk_assessment: candidate.risk_level,
            recommendation,
            evaluated_at: Utc::now(),
        }
    }

    /// Sort proposals in-place by descending evaluation score.
    pub fn rank_proposals(&self, proposals: &mut [ImprovementProposal]) {
        proposals.sort_by(|a, b| {
            let sa = self.evaluate(a).score;
            let sb = self.evaluate(b).score;
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    // -- private helpers ------------------------------------------------

    fn priority_to_score(&self, priority: ImprovementPriority) -> f64 {
        // Invert: Critical (rank 1) → 1.0, Informational (rank 5) → 0.2
        let rank = priority.rank() as f64;
        (6.0 - rank) / 5.0
    }

    fn risk_to_score(&self, risk: RiskLevel) -> f64 {
        match risk {
            RiskLevel::None => 1.0,
            RiskLevel::Low => 0.8,
            RiskLevel::Medium => 0.5,
            RiskLevel::High => 0.25,
            RiskLevel::Critical => 0.05,
        }
    }

    /// Estimate feasibility based on the number of prerequisites and risk level.
    fn estimate_feasibility(&self, candidate: &ImprovementCandidate) -> f64 {
        let prereq_penalty = (candidate.prerequisites.len() as f64 * 0.1).min(0.5);
        let risk_penalty = match candidate.risk_level {
            RiskLevel::None => 0.0,
            RiskLevel::Low => 0.05,
            RiskLevel::Medium => 0.15,
            RiskLevel::High => 0.30,
            RiskLevel::Critical => 0.50,
        };
        (1.0 - prereq_penalty - risk_penalty).clamp(0.0, 1.0)
    }

    fn build_recommendation(&self, score: f64, candidate: &ImprovementCandidate) -> String {
        if score >= 0.8 {
            format!(
                "Strongly recommend implementing \"{}\" — high impact, low risk.",
                candidate.title
            )
        } else if score >= 0.6 {
            format!(
                "Recommend implementing \"{}\" — good value, manageable risk.",
                candidate.title
            )
        } else if score >= 0.4 {
            format!(
                "Consider implementing \"{}\" — moderate value or moderate risk.",
                candidate.title
            )
        } else if score >= 0.2 {
            format!(
                "Low priority: \"{}\" — limited impact or elevated risk.",
                candidate.title
            )
        } else {
            format!(
                "Not recommended: \"{}\" — insufficient impact to justify risk.",
                candidate.title
            )
        }
    }
}

impl Default for ImprovementEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::improvement::candidate::ImprovementCandidate;
    use crate::improvement::priority::ImprovementPriority;
    use crate::types::{EvolutionStatus, ImprovementCategory, RiskLevel, SubsystemTarget};

    fn proposal_with(
        priority: ImprovementPriority,
        impact: f64,
        risk: RiskLevel,
    ) -> ImprovementProposal {
        let candidate = ImprovementCandidate::new(
            "Test",
            "desc",
            ImprovementCategory::Performance,
            SubsystemTarget::Core,
            priority,
            impact,
            risk,
            "plan",
        );
        ImprovementProposal::new(candidate, "justification", vec![], vec![], "rollback", true)
    }

    #[test]
    fn evaluate_high_quality() {
        let eval = ImprovementEvaluator::new();
        let prop = proposal_with(ImprovementPriority::Critical, 0.9, RiskLevel::Low);
        let result = eval.evaluate(&prop);
        assert!(result.score > 0.7, "score was {}", result.score);
    }

    #[test]
    fn evaluate_low_quality() {
        let eval = ImprovementEvaluator::new();
        let prop = proposal_with(ImprovementPriority::Informational, 0.1, RiskLevel::Critical);
        let result = eval.evaluate(&prop);
        assert!(result.score < 0.3, "score was {}", result.score);
    }

    #[test]
    fn rank_sorts_descending() {
        let eval = ImprovementEvaluator::new();
        let mut props = vec![
            proposal_with(ImprovementPriority::Low, 0.2, RiskLevel::High),
            proposal_with(ImprovementPriority::Critical, 0.9, RiskLevel::None),
            proposal_with(ImprovementPriority::Medium, 0.5, RiskLevel::Medium),
        ];
        eval.rank_proposals(&mut props);

        // The first element should have the highest priority + impact
        assert_eq!(props[0].candidate.priority, ImprovementPriority::Critical);
        assert_eq!(props[2].candidate.priority, ImprovementPriority::Low);
    }

    #[test]
    fn score_clamped() {
        let eval = ImprovementEvaluator::new();
        let prop = proposal_with(ImprovementPriority::Critical, 1.0, RiskLevel::None);
        let result = eval.evaluate(&prop);
        assert!(result.score <= 1.0);
        assert!(result.score >= 0.0);
    }
}
