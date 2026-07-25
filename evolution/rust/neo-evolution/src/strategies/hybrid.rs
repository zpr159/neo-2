//! Hybrid strategy — composes multiple strategies with weighted blending.

use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::EvolutionResult;

use super::mutation::MutationStrategy;
use super::optimization::OptimizationStrategy;
use super::refinement::RefinementStrategy;
use super::strategy::{EvolutionStrategy, EvolutionStrategyTrait, StrategyInput, StrategyOutput};

/// Composes multiple child [`EvolutionStrategy`] instances and blends
/// their outputs according to configured weights.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridStrategy {
    /// Child strategies to compose.
    pub strategies: Vec<EvolutionStrategy>,
    /// Blending weights — one per child strategy.
    pub weights: Vec<f64>,
}

impl HybridStrategy {
    /// Create a new hybrid strategy from the given strategies and weights.
    ///
    /// # Panics
    ///
    /// Does not panic; if `strategies` and `weights` have different lengths
    /// the shorter list is padded with defaults.
    pub fn new(strategies: Vec<EvolutionStrategy>, weights: Vec<f64>) -> Self {
        let mut weights = weights;
        weights.resize(strategies.len(), 1.0 / strategies.len().max(1) as f64);
        Self {
            strategies,
            weights,
        }
    }

    /// Create a hybrid from category defaults.
    pub fn from_defaults() -> Self {
        Self::new(
            vec![
                EvolutionStrategy::Mutation { intensity: 0.3 },
                EvolutionStrategy::Optimization {
                    target_metric: "default".to_string(),
                },
                EvolutionStrategy::Refinement { iterations: 5 },
            ],
            vec![0.3, 0.4, 0.3],
        )
    }

    /// Compose multiple strategy outputs into a single blended result.
    ///
    /// Each parameter value is the weighted average of the values produced
    /// by the individual strategies.
    pub fn compose(&self, outputs: &[StrategyOutput]) -> StrategyOutput {
        let mut blended: HashMap<String, (f64, f64)> = HashMap::new();

        for (output, &weight) in outputs.iter().zip(&self.weights) {
            for (key, value) in &output.parameters {
                let entry = blended.entry(key.clone()).or_insert((0.0, 0.0));
                entry.0 += value * weight;
                entry.1 += weight;
            }
        }

        let parameters: HashMap<String, f64> = blended
            .into_iter()
            .map(|(k, (sum, w))| {
                let normalised = if w > 0.0 { sum / w } else { 0.0 };
                (k, normalised)
            })
            .collect();

        let avg_confidence = if outputs.is_empty() {
            0.0
        } else {
            let total_weight: f64 = self.weights.iter().take(outputs.len()).sum();
            if total_weight > 0.0 {
                outputs
                    .iter()
                    .zip(&self.weights)
                    .map(|(o, w)| o.confidence * w)
                    .sum::<f64>()
                    / total_weight
            } else {
                0.0
            }
        };

        let explanation = outputs
            .iter()
            .map(|o| o.explanation.as_str())
            .collect::<Vec<_>>()
            .join("; ");

        StrategyOutput {
            parameters,
            confidence: avg_confidence,
            explanation: format!(
                "hybrid composition of {} strategies: {explanation}",
                outputs.len(),
            ),
        }
    }

    /// Return the number of child strategies.
    pub fn get_strategy_count(&self) -> usize {
        self.strategies.len()
    }

    /// Adjust the weight of a child strategy by index.
    ///
    /// Returns `Err` if the index is out of bounds.
    pub fn adjust_weight(&mut self, index: usize, new_weight: f64) -> EvolutionResult<()> {
        if index >= self.weights.len() {
            return Err(crate::error::EvolutionError::ConfigError(format!(
                "weight index {} out of bounds (have {})",
                index,
                self.weights.len(),
            )));
        }
        self.weights[index] = new_weight;
        Ok(())
    }

    /// Normalize all weights so they sum to 1.0.
    pub fn normalize_weights(&mut self) {
        let sum: f64 = self.weights.iter().sum();
        if sum > 0.0 {
            for w in &mut self.weights {
                *w /= sum;
            }
        }
    }
}

#[async_trait]
impl EvolutionStrategyTrait for HybridStrategy {
    fn name(&self) -> &str {
        "hybrid"
    }

    async fn apply(&self, input: StrategyInput) -> EvolutionResult<StrategyOutput> {
        let mut outputs = Vec::with_capacity(self.strategies.len());

        for strategy in &self.strategies {
            let output = match strategy {
                EvolutionStrategy::Mutation { intensity } => {
                    let s = MutationStrategy::new(*intensity);
                    s.apply(input.clone()).await?
                }
                EvolutionStrategy::Optimization { target_metric } => {
                    let s = OptimizationStrategy::new(target_metric.clone());
                    s.apply(input.clone()).await?
                }
                EvolutionStrategy::Refinement { iterations } => {
                    let s = RefinementStrategy::new(*iterations);
                    s.apply(input.clone()).await?
                }
                EvolutionStrategy::Hybrid { .. } => {
                    return Err(crate::error::EvolutionError::UnsupportedOperation(
                        "nested hybrid strategies are not supported".to_string(),
                    ));
                }
            };
            outputs.push(output);
        }

        Ok(self.compose(&outputs))
    }
}
