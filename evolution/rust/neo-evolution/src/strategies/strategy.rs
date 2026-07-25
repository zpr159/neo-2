//! Core strategy types and trait definition.

use std::collections::HashMap;
use std::fmt;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::EvolutionResult;
use crate::types::{ImprovementCategory, SubsystemTarget};

/// Input provided to a strategy when applying it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyInput {
    /// Current parameter map to be modified by the strategy.
    pub parameters: HashMap<String, f64>,
    /// The subsystem being targeted.
    pub target: SubsystemTarget,
    /// Arbitrary context key-value pairs (e.g. metric names, thresholds).
    pub context: HashMap<String, String>,
}

impl StrategyInput {
    /// Create a new strategy input with the given parameters and target.
    pub fn new(parameters: HashMap<String, f64>, target: SubsystemTarget) -> Self {
        Self {
            parameters,
            target,
            context: HashMap::new(),
        }
    }
}

/// Output produced by a strategy after applying it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyOutput {
    /// The resulting parameter map after strategy application.
    pub parameters: HashMap<String, f64>,
    /// Confidence score in the output (0.0–1.0).
    pub confidence: f64,
    /// Human-readable explanation of what the strategy did.
    pub explanation: String,
}

/// Trait that all evolution strategies must implement.
///
/// Strategies are evaluated asynchronously so they may perform I/O
/// (e.g. fetching historical data, running simulations).
#[async_trait]
pub trait EvolutionStrategyTrait: Send + Sync {
    /// Return the human-readable name of this strategy.
    fn name(&self) -> &str;

    /// Apply the strategy to the given input and return the output.
    async fn apply(&self, input: StrategyInput) -> EvolutionResult<StrategyOutput>;
}

/// The built-in evolution strategies available to the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvolutionStrategy {
    /// Apply stochastic mutations to parameters.
    Mutation {
        /// Mutation intensity in the range (0.0, 1.0].
        intensity: f64,
    },
    /// Iteratively optimise toward a target metric.
    Optimization {
        /// Name of the metric to optimise (e.g. "latency_p99").
        target_metric: String,
    },
    /// Iteratively refine parameters with convergence detection.
    Refinement {
        /// Maximum number of refinement iterations.
        iterations: usize,
    },
    /// Compose multiple strategies with weighted blending.
    Hybrid {
        /// Child strategies to compose.
        strategies: Vec<EvolutionStrategy>,
        /// Blending weights, one per child strategy.
        weights: Vec<f64>,
    },
}

impl EvolutionStrategy {
    /// Return the strategy name as a string slice.
    pub fn as_name(&self) -> &'static str {
        match self {
            Self::Mutation { .. } => "mutation",
            Self::Optimization { .. } => "optimization",
            Self::Refinement { .. } => "refinement",
            Self::Hybrid { .. } => "hybrid",
        }
    }

    /// Resolve the default strategy for a given improvement category.
    pub fn default_for_category(category: ImprovementCategory) -> Self {
        match category {
            ImprovementCategory::Performance | ImprovementCategory::Throughput => {
                Self::Optimization {
                    target_metric: category.to_string(),
                }
            }
            ImprovementCategory::Reliability | ImprovementCategory::Architecture => {
                Self::Refinement { iterations: 10 }
            }
            ImprovementCategory::ResourceEfficiency | ImprovementCategory::Scalability => {
                Self::Mutation { intensity: 0.3 }
            }
            ImprovementCategory::Latency => Self::Optimization {
                target_metric: "latency".to_string(),
            },
            ImprovementCategory::Security => Self::Refinement { iterations: 5 },
            ImprovementCategory::CodeQuality | ImprovementCategory::DependencyManagement => {
                Self::Mutation { intensity: 0.2 }
            }
        }
    }
}

impl fmt::Display for EvolutionStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mutation { intensity } => {
                write!(f, "Mutation(intensity={intensity:.2})")
            }
            Self::Optimization { target_metric } => {
                write!(f, "Optimization(target={target_metric})")
            }
            Self::Refinement { iterations } => {
                write!(f, "Refinement(iterations={iterations})")
            }
            Self::Hybrid {
                strategies,
                weights,
            } => {
                write!(
                    f,
                    "Hybrid(strategies={}, weights={})",
                    strategies.len(),
                    weights.len()
                )
            }
        }
    }
}
