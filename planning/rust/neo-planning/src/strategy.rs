use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use deriving_more::Display;
use crate::types::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Strategy {
    pub id: StrategyId,
    pub plan_id: Option<PlanId>,
    pub name: String,
    pub description: Option<String>,
    pub algorithm: AlgorithmType,
    pub evaluation: StrategyComparison,
    pub created_at: DateTime<Utc>,
    pub confidence_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyComparison {
    pub cost: f64,
    pub duration_ms: u64,
    pub probability_of_success: f32,
    pub resource_consumption: HashMap<String, f64>,
    pub risk_score: f32,
    pub complexity_score: f32,
    pub scalability_score: f32,
    pub reliability_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StrategyPolicy {
    MinCost,
    MinDuration,
    MaxSuccess,
    MinRisk,
    MaxReliability,
    Balanced,
}

pub struct StrategyGenerator;

impl StrategyGenerator {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_candidates(&self, goal: &Goal, context: &PlanContext) -> Vec<Strategy> {
        let mut candidates = Vec::new();

        let htn_strategy = Strategy {
            id: StrategyId::new(),
            plan_id: None,
            name: "Hierarchical Task Network".to_string(),
            description: Some("Decomposes goals into hierarchical subtasks".to_string()),
            algorithm: AlgorithmType::HierarchicalTaskNetwork,
            evaluation: StrategyComparison {
                cost: goal.budget.max_cost * 1.2,
                duration_ms: (goal.budget.max_time_seconds * 1000) as u64,
                probability_of_success: 0.85,
                resource_consumption: HashMap::new(),
                risk_score: 0.2,
                complexity_score: 0.6,
                scalability_score: 0.8,
                reliability_score: 0.7,
            },
            created_at: Utc::now(),
            confidence_score: 0.8,
        };
        candidates.push(htn_strategy);

        let goap_strategy = Strategy {
            id: StrategyId::new(),
            plan_id: None,
            name: "Goal-Oriented Action Planning".to_string(),
            description: Some("Backward-chains from goal conditions".to_string()),
            algorithm: AlgorithmType::GoalOrientedActionPlanning,
            evaluation: StrategyComparison {
                cost: goal.budget.max_cost * 1.1,
                duration_ms: (goal.budget.max_time_seconds * 800) as u64,
                probability_of_success: 0.75,
                resource_consumption: HashMap::new(),
                risk_score: 0.3,
                complexity_score: 0.9,
                scalability_score: 0.6,
                reliability_score: 0.5,
            },
            created_at: Utc::now(),
            confidence_score: 0.7,
        };
        candidates.push(goap_strategy);

        let a_star_strategy = Strategy {
            id: StrategyId::new(),
            plan_id: None,
            name: "A* Search".to_string(),
            description: Some("Heuristic search with optimal pathfinding".to_string()),
            algorithm: AlgorithmType::AStar,
            evaluation: StrategyComparison {
                cost: goal.budget.max_cost * 1.3,
                duration_ms: (goal.budget.max_time_seconds * 1200) as u64,
                probability_of_success: 0.9,
                resource_consumption: HashMap::new(),
                risk_score: 0.15,
                complexity_score: 0.7,
                scalability_score: 0.7,
                reliability_score: 0.8,
            },
            created_at: Utc::now(),
            confidence_score: 0.85,
        };
        candidates.push(a_star_strategy);

        candidates
    }
}

pub struct StrategyEvaluator;

impl StrategyEvaluator {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(&self, strategy: &mut Strategy, context: &PlanContext) -> StrategyComparison {
        let mut comparison = strategy.evaluation.clone();

        let base_cost = context.variables.get("max_cost")
            .and_then(|v| v.as_f64())
            .unwrap_or(1000.0);

        comparison.cost = base_cost * (1.0 + (rand::random::<f64>() * 0.5 - 0.25));
        comparison.duration_ms = (base_cost / 100.0 * 1000.0) as u64;
        comparison.probability_of_success = 0.5 + (rand::random::<f64>() * 0.5);
        comparison.risk_score = 1.0 - comparison.probability_of_success;

        comparison.resource_consumption = HashMap::from([
            ("cpu".to_string(), rand::random::<f64>() * 10.0),
            ("memory".to_string(), rand::random::<f64>() * 5.0),
            ("network".to_string(), rand::random::<f64>() * 2.0),
        ]);

        comparison.complexity_score = 1.0 - comparison.probability_of_success;
        comparison.scalability_score = 0.5 + (rand::random::<f64>() * 0.5);
        comparison.reliability_score = comparison.probability_of_success;

        comparison
    }
}

pub struct StrategySelector;

impl StrategySelector {
    pub fn new() -> Self {
        Self
    }

    pub fn select_best(&self, candidates: Vec<Strategy>, policy: StrategyPolicy) -> Option<Strategy> {
        match policy {
            StrategyPolicy::MinCost => candidates.into_iter().min_by(|a, b| {
                a.evaluation.cost.partial_cmp(&b.evaluation.cost).unwrap_or(std::cmp::Ordering::Equal)
            }),
            StrategyPolicy::MinDuration => candidates.into_iter().min_by(|a, b| {
                a.evaluation.duration_ms.cmp(&b.evaluation.duration_ms)
            }),
            StrategyPolicy::MaxSuccess => candidates.into_iter().max_by(|a, b| {
                a.evaluation.probability_of_success.partial_cmp(&b.evaluation.probability_of_success)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            StrategyPolicy::MinRisk => candidates.into_iter().min_by(|a, b| {
                a.evaluation.risk_score.partial_cmp(&b.evaluation.risk_score).unwrap_or(std::cmp::Ordering::Equal)
            }),
            StrategyPolicy::MaxReliability => candidates.into_iter().max_by(|a, b| {
                a.evaluation.reliability_score.partial_cmp(&b.evaluation.reliability_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            StrategyPolicy::Balanced => {
                let mut best: Option<Strategy> = None;
                let mut best_score = f32::MIN;

                for candidate in candidates {
                    let score = candidate.evaluation.probability_of_success
                        * 0.4
                        - candidate.evaluation.risk_score * 0.3
                        - candidate.evaluation.complexity_score * 0.2
                        + candidate.evaluation.reliability_score * 0.1;

                    if score > best_score {
                        best_score = score;
                        best = Some(candidate);
                    }
                }

                best
            }
        }
    }
}