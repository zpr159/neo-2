use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::strategy::ReasoningStrategy;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelRole {
    Primary,
    Verifier,
    Specialist,
    Fallback,
    Critic,
}

impl std::fmt::Display for ModelRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primary => write!(f, "primary"),
            Self::Verifier => write!(f, "verifier"),
            Self::Specialist => write!(f, "specialist"),
            Self::Fallback => write!(f, "fallback"),
            Self::Critic => write!(f, "critic"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelBackend {
    pub id: Uuid,
    pub name: String,
    pub role: ModelRole,
    pub strategies: Vec<ReasoningStrategy>,
    pub reliability: f32,
    pub latency_ms: u64,
    pub cost_per_call: f64,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ModelBackend {
    pub fn new(name: String, role: ModelRole) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            role,
            strategies: Vec::new(),
            reliability: 0.9,
            latency_ms: 100,
            cost_per_call: 0.001,
            metadata: HashMap::new(),
        }
    }

    pub fn with_reliability(mut self, reliability: f32) -> Self {
        self.reliability = reliability.clamp(0.0, 1.0);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelResponse {
    pub model_id: Uuid,
    pub model_name: String,
    pub output: String,
    pub confidence: f32,
    pub strategy_used: ReasoningStrategy,
    pub latency_ms: u64,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusMethod {
    MajorityVote,
    WeightedAverage,
    HighestConfidence,
    MedianOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusResult {
    pub agreed_output: String,
    pub confidence: f32,
    pub agreement_ratio: f32,
    pub method: ConsensusMethod,
    pub dissenting_views: Vec<ModelResponse>,
    pub participating_models: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiModelResult {
    pub consensus: Option<ConsensusResult>,
    pub individual_responses: Vec<ModelResponse>,
    pub fallback_used: bool,
    pub total_latency_ms: u64,
}

#[derive(Debug)]
pub struct MultiModelReasoner {
    models: Vec<ModelBackend>,
    consensus_method: ConsensusMethod,
    consensus_threshold: f32,
    _enable_voting: bool,
}

impl MultiModelReasoner {
    pub fn new() -> Self {
        Self {
            models: Vec::new(),
            consensus_method: ConsensusMethod::WeightedAverage,
            consensus_threshold: 0.6,
            _enable_voting: true,
        }
    }

    pub fn with_consensus_method(mut self, method: ConsensusMethod) -> Self {
        self.consensus_method = method;
        self
    }

    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.consensus_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    pub fn register_model(&mut self, model: ModelBackend) {
        self.models.push(model);
    }

    pub fn models(&self) -> &[ModelBackend] {
        &self.models
    }

    pub fn route_work(
        &self,
        strategy: &ReasoningStrategy,
    ) -> Vec<&ModelBackend> {
        self.models
            .iter()
            .filter(|m| m.strategies.is_empty() || m.strategies.contains(strategy))
            .collect()
    }

    pub fn select_primary(&self, strategy: &ReasoningStrategy) -> Option<&ModelBackend> {
        self.route_work(strategy)
            .into_iter()
            .max_by(|a, b| {
                a.reliability
                    .partial_cmp(&b.reliability)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    pub fn compute_consensus(
        &self,
        responses: &[ModelResponse],
    ) -> Option<ConsensusResult> {
        if responses.is_empty() {
            return None;
        }

        if responses.len() == 1 {
            return Some(ConsensusResult {
                agreed_output: responses[0].output.clone(),
                confidence: responses[0].confidence,
                agreement_ratio: 1.0,
                method: self.consensus_method.clone(),
                dissenting_views: Vec::new(),
                participating_models: responses
                    .iter()
                    .map(|r| r.model_name.clone())
                    .collect(),
            });
        }

        match self.consensus_method {
            ConsensusMethod::HighestConfidence => {
                let best = responses
                    .iter()
                    .max_by(|a, b| {
                        a.confidence
                            .partial_cmp(&b.confidence)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .unwrap();

                let agreement_ratio = responses
                    .iter()
                    .filter(|r| r.output == best.output)
                    .count() as f32
                    / responses.len() as f32;

                let dissenting: Vec<ModelResponse> = responses
                    .iter()
                    .filter(|r| r.output != best.output)
                    .cloned()
                    .collect();

                Some(ConsensusResult {
                    agreed_output: best.output.clone(),
                    confidence: best.confidence,
                    agreement_ratio,
                    method: self.consensus_method.clone(),
                    dissenting_views: dissenting,
                    participating_models: responses
                        .iter()
                        .map(|r| r.model_name.clone())
                        .collect(),
                })
            }
            ConsensusMethod::WeightedAverage => {
                let total_weight: f32 = responses.iter().map(|r| r.confidence).sum();
                let avg_confidence = if total_weight > 0.0 {
                    responses
                        .iter()
                        .map(|r| r.confidence * r.confidence)
                        .sum::<f32>()
                        / total_weight
                } else {
                    0.0
                };

                let majority_output = responses
                    .iter()
                    .max_by_key(|r| {
                        responses
                            .iter()
                            .filter(|r2| r2.output == r.output)
                            .count()
                    })
                    .map(|r| r.output.clone())
                    .unwrap_or_default();

                let agreement_ratio = responses
                    .iter()
                    .filter(|r| r.output == majority_output)
                    .count() as f32
                    / responses.len() as f32;

                let dissenting: Vec<ModelResponse> = responses
                    .iter()
                    .filter(|r| r.output != majority_output)
                    .cloned()
                    .collect();

                Some(ConsensusResult {
                    agreed_output: majority_output,
                    confidence: avg_confidence.clamp(0.0, 1.0),
                    agreement_ratio,
                    method: self.consensus_method.clone(),
                    dissenting_views: dissenting,
                    participating_models: responses
                        .iter()
                        .map(|r| r.model_name.clone())
                        .collect(),
                })
            }
            ConsensusMethod::MajorityVote => {
                let majority_output = responses
                    .iter()
                    .max_by_key(|r| {
                        responses
                            .iter()
                            .filter(|r2| r2.output == r.output)
                            .count()
                    })
                    .map(|r| r.output.clone())
                    .unwrap_or_default();

                let majority_count = responses
                    .iter()
                    .filter(|r| r.output == majority_output)
                    .count();

                let agreement_ratio = majority_count as f32 / responses.len() as f32;
                let consensus_reached = agreement_ratio > self.consensus_threshold;

                if !consensus_reached {
                    return None;
                }

                let avg_conf: f32 = responses
                    .iter()
                    .filter(|r| r.output == majority_output)
                    .map(|r| r.confidence)
                    .sum::<f32>()
                    / majority_count as f32;

                let dissenting: Vec<ModelResponse> = responses
                    .iter()
                    .filter(|r| r.output != majority_output)
                    .cloned()
                    .collect();

                Some(ConsensusResult {
                    agreed_output: majority_output,
                    confidence: avg_conf,
                    agreement_ratio,
                    method: self.consensus_method.clone(),
                    dissenting_views: dissenting,
                    participating_models: responses
                        .iter()
                        .map(|r| r.model_name.clone())
                        .collect(),
                })
            }
            ConsensusMethod::MedianOutput => {
                let mut sorted = responses.to_vec();
                sorted.sort_by(|a, b| {
                    a.confidence
                        .partial_cmp(&b.confidence)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

                let median_idx = sorted.len() / 2;
                let median = &sorted[median_idx];

                Some(ConsensusResult {
                    agreed_output: median.output.clone(),
                    confidence: median.confidence,
                    agreement_ratio: 1.0 / responses.len() as f32,
                    method: self.consensus_method.clone(),
                    dissenting_views: Vec::new(),
                    participating_models: responses
                        .iter()
                        .map(|r| r.model_name.clone())
                        .collect(),
                })
            }
        }
    }

    pub fn find_fallback(&self, failed_model_id: Uuid) -> Option<&ModelBackend> {
        self.models
            .iter()
            .find(|m| m.role == ModelRole::Fallback && m.id != failed_model_id)
    }

    pub fn select_specialist(
        &self,
        task_description: &str,
    ) -> Option<&ModelBackend> {
        let lower = task_description.to_lowercase();
        self.models
            .iter()
            .filter(|m| m.role == ModelRole::Specialist)
            .max_by_key(|m| {
                let mut score = 0;
                for strategy in &m.strategies {
                    if lower.contains(&strategy.to_string()) {
                        score += 10;
                    }
                }
                score + (m.reliability * 10.0) as u32
            })
    }

    pub fn all_available_models(&self) -> Vec<&ModelBackend> {
        self.models.iter().collect()
    }
}

impl Default for MultiModelReasoner {
    fn default() -> Self {
        Self::new()
    }
}
