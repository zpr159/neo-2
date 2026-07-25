use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ReasoningResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionOption {
    pub id: Uuid,
    pub description: String,
    pub scores: HashMap<String, f64>,
    pub risk_score: f64,
    pub utility_score: f64,
    pub cost: f64,
    pub benefits: Vec<String>,
    pub drawbacks: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl DecisionOption {
    pub fn new(description: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            description,
            scores: HashMap::new(),
            risk_score: 0.0,
            utility_score: 0.0,
            cost: 0.0,
            benefits: Vec::new(),
            drawbacks: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_score(mut self, objective: String, score: f64) -> Self {
        self.scores.insert(objective, score);
        self
    }

    pub fn with_risk(mut self, risk: f64) -> Self {
        self.risk_score = risk.clamp(0.0, 1.0);
        self
    }

    pub fn with_utility(mut self, utility: f64) -> Self {
        self.utility_score = utility.clamp(0.0, 1.0);
        self
    }

    pub fn with_cost(mut self, cost: f64) -> Self {
        self.cost = cost;
        self
    }

    pub fn with_benefit(mut self, benefit: String) -> Self {
        self.benefits.push(benefit);
        self
    }

    pub fn with_drawback(mut self, drawback: String) -> Self {
        self.drawbacks.push(drawback);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectiveWeight {
    pub name: String,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredOption {
    pub option_id: Uuid,
    pub description: String,
    pub composite_score: f64,
    pub pareto_rank: usize,
    pub dominated_by: Vec<Uuid>,
    pub dominates: Vec<Uuid>,
    pub risk_adjusted_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionResult {
    pub selected_option_id: Uuid,
    pub selected_description: String,
    pub composite_score: f64,
    pub risk_adjusted_score: f64,
    pub confidence: f32,
    pub all_scored: Vec<ScoredOption>,
    pub explanation: String,
    pub alternative_ids: Vec<Uuid>,
}

#[derive(Debug)]
pub struct DecisionEngine {
    risk_aversion: f64,
    default_weights: Vec<ObjectiveWeight>,
}

impl DecisionEngine {
    pub fn new() -> Self {
        Self {
            risk_aversion: 0.5,
            default_weights: vec![
                ObjectiveWeight {
                    name: "utility".to_string(),
                    weight: 0.4,
                },
                ObjectiveWeight {
                    name: "risk".to_string(),
                    weight: 0.3,
                },
                ObjectiveWeight {
                    name: "cost".to_string(),
                    weight: 0.3,
                },
            ],
        }
    }

    pub fn with_risk_aversion(mut self, aversion: f64) -> Self {
        self.risk_aversion = aversion.clamp(0.0, 1.0);
        self
    }

    pub fn with_weights(mut self, weights: Vec<ObjectiveWeight>) -> Self {
        self.default_weights = weights;
        self
    }

    pub fn score_options(
        &self,
        options: &[DecisionOption],
        weights: Option<&[ObjectiveWeight]>,
    ) -> Vec<ScoredOption> {
        let w = weights.unwrap_or(&self.default_weights);

        let mut scored: Vec<ScoredOption> = options
            .iter()
            .map(|opt| {
                let composite = self.compute_composite(opt, w);
                let risk_adjusted = composite * (1.0 - self.risk_aversion * opt.risk_score);

                ScoredOption {
                    option_id: opt.id,
                    description: opt.description.clone(),
                    composite_score: composite,
                    pareto_rank: 0,
                    dominated_by: Vec::new(),
                    dominates: Vec::new(),
                    risk_adjusted_score: risk_adjusted,
                }
            })
            .collect();

        self.compute_pareto(&mut scored);
        scored
    }

    fn compute_composite(&self, option: &DecisionOption, weights: &[ObjectiveWeight]) -> f64 {
        let mut total = 0.0;
        let mut total_weight = 0.0;

        for weight in weights {
            let score = match weight.name.as_str() {
                "utility" => option.utility_score,
                "risk" => 1.0 - option.risk_score,
                "cost" => {
                    if option.cost > 0.0 {
                        1.0 / (1.0 + option.cost)
                    } else {
                        0.5
                    }
                }
                _ => option.scores.get(&weight.name).copied().unwrap_or(0.0),
            };
            total += score * weight.weight;
            total_weight += weight.weight;
        }

        if total_weight > 0.0 {
            total / total_weight
        } else {
            0.0
        }
    }

    fn compute_pareto(&self, scored: &mut [ScoredOption]) {
        let n = scored.len();
        for i in 0..n {
            for j in 0..n {
                if i == j {
                    continue;
                }
                let dominates_ij = scored[i].risk_adjusted_score > scored[j].risk_adjusted_score
                    && scored[i].composite_score >= scored[j].composite_score;
                let dominates_ji = scored[j].risk_adjusted_score > scored[i].risk_adjusted_score
                    && scored[j].composite_score >= scored[i].composite_score;

                if dominates_ij && !dominates_ji {
                    scored[i].dominates.push(scored[j].option_id);
                    scored[j].dominated_by.push(scored[i].option_id);
                }
            }
        }

        for opt in scored.iter_mut() {
            opt.pareto_rank = opt.dominated_by.len();
        }
    }

    pub fn select_best(
        &self,
        options: &[DecisionOption],
        weights: Option<&[ObjectiveWeight]>,
    ) -> ReasoningResult<DecisionResult> {
        if options.is_empty() {
            return Err(crate::error::ReasoningError::NoOptions(
                "no options to decide between".to_string(),
            ));
        }

        let scored = self.score_options(options, weights);

        let best = scored
            .iter()
            .max_by(|a, b| {
                a.risk_adjusted_score
                    .partial_cmp(&b.risk_adjusted_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .expect("at least one option");

        let best_id = best.option_id;
        let best_composite = best.composite_score;
        let best_risk_adj = best.risk_adjusted_score;
        let _best_desc = best.description.clone();

        let alternative_ids: Vec<Uuid> = scored
            .iter()
            .filter(|s| s.option_id != best_id)
            .map(|s| s.option_id)
            .collect();

        let confidence = (best_risk_adj as f32).clamp(0.1, 0.95);

        let best_option = options.iter().find(|o| o.id == best_id).unwrap();

        Ok(DecisionResult {
            selected_option_id: best_id,
            selected_description: best_option.description.clone(),
            composite_score: best_composite,
            risk_adjusted_score: best_risk_adj,
            confidence,
            all_scored: scored,
            explanation: format!(
                "Selected '{}' with risk-adjusted score {:.3} (composite: {:.3}, risk: {:.3})",
                best_option.description,
                best_risk_adj,
                best_composite,
                best_option.risk_score
            ),
            alternative_ids,
        })
    }

    pub fn estimate_risk(
        &self,
        option: &DecisionOption,
        context: &HashMap<String, serde_json::Value>,
    ) -> f64 {
        let mut risk = option.risk_score;

        let drawback_count = option.drawbacks.len() as f64;
        risk += drawback_count * 0.05;

        if context.contains_key("uncertainty") {
            risk += 0.1;
        }

        if option.cost > 100.0 {
            risk += 0.05;
        }

        risk.clamp(0.0, 1.0)
    }

    pub fn multi_objective_optimize(
        &self,
        options: &[DecisionOption],
        objectives: &[String],
    ) -> Vec<ScoredOption> {
        let weights: Vec<ObjectiveWeight> = objectives
            .iter()
            .map(|name| ObjectiveWeight {
                name: name.clone(),
                weight: 1.0 / objectives.len() as f64,
            })
            .collect();

        self.score_options(options, Some(&weights))
    }
}

impl Default for DecisionEngine {
    fn default() -> Self {
        Self::new()
    }
}
