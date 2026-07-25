use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::policy_evolution::policy::Policy;
use crate::types::EvolutionId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEvaluation {
    pub policy_id: EvolutionId,
    pub score: f64,
    pub accuracy: f64,
    pub efficiency: f64,
    pub stability: f64,
    pub evaluated_at: DateTime<Utc>,
}

pub struct PolicyEvaluator;

impl PolicyEvaluator {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate_policy(&self, policy: &Policy) -> PolicyEvaluation {
        let rule_count = policy.rules.len() as f64;
        let enabled_count = policy.rules.iter().filter(|r| r.enabled).count() as f64;
        let avg_weight = if rule_count > 0.0 {
            policy.rules.iter().map(|r| r.weight).sum::<f64>() / rule_count
        } else {
            0.0
        };

        let accuracy = if rule_count > 0.0 {
            enabled_count / rule_count
        } else {
            0.0
        };
        let efficiency = avg_weight / 10.0;
        let stability = 1.0 / (1.0 + policy.version as f64 * 0.01);
        let score = accuracy * 0.4 + efficiency * 0.3 + stability * 0.3;

        PolicyEvaluation {
            policy_id: policy.id,
            score,
            accuracy,
            efficiency,
            stability,
            evaluated_at: Utc::now(),
        }
    }

    pub fn compare_policies(&self, a: &Policy, b: &Policy) -> f64 {
        let eval_a = self.evaluate_policy(a);
        let eval_b = self.evaluate_policy(b);
        eval_a.score - eval_b.score
    }

    pub fn rank_policies(&self, policies: &mut Vec<Policy>) {
        let evaluator = Self::new();
        policies.sort_by(|a, b| {
            let score_a = evaluator.evaluate_policy(a).score;
            let score_b = evaluator.evaluate_policy(b).score;
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
}

impl Default for PolicyEvaluator {
    fn default() -> Self {
        Self::new()
    }
}
