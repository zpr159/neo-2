//! Refinement strategy — iterative refinement with convergence detection.

use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{EvolutionError, EvolutionResult};

use super::strategy::{EvolutionStrategyTrait, StrategyInput, StrategyOutput};

/// A single iteration record in the refinement history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefinementRecord {
    /// Parameters at the start of this iteration.
    pub parameters: HashMap<String, f64>,
    /// Quality metrics observed during this iteration.
    pub quality_metrics: HashMap<String, f64>,
}

/// Performs iterative refinement on parameters, monitoring quality metrics
/// and detecting convergence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefinementStrategy {
    /// Maximum number of refinement iterations.
    pub max_iterations: usize,
    /// Minimum required quality improvement per iteration.
    pub min_improvement: f64,
    /// Smoothing factor for the quality score (0.0–1.0).
    pub smoothing_factor: f64,
    /// Internal iteration counter.
    iteration: usize,
    /// Smoothed quality score from previous iterations.
    smoothed_quality: f64,
    /// Quality metric history for convergence analysis.
    history: Vec<RefinementRecord>,
}

impl RefinementStrategy {
    /// Create a new refinement strategy with the given iteration budget.
    pub fn new(max_iterations: usize) -> Self {
        Self {
            max_iterations,
            min_improvement: 1e-6,
            smoothing_factor: 0.3,
            iteration: 0,
            smoothed_quality: 0.0,
            history: Vec::new(),
        }
    }

    /// Set the minimum improvement threshold for convergence.
    pub fn with_min_improvement(mut self, threshold: f64) -> Self {
        self.min_improvement = threshold;
        self
    }

    /// Perform one refinement pass on the current state.
    ///
    /// The `quality_metrics` map should contain at least one entry.
    /// Parameters are adjusted proportionally to their deviation from the
    /// quality-weighted ideal.
    pub fn refine(
        &mut self,
        parameters: &HashMap<String, f64>,
        quality_metrics: &HashMap<String, f64>,
    ) -> EvolutionResult<HashMap<String, f64>> {
        if parameters.is_empty() {
            return Err(EvolutionError::InvalidStateTransition(
                "cannot refine empty parameter set".to_string(),
            ));
        }
        if self.iteration >= self.max_iterations {
            return Err(EvolutionError::ResourceExhausted(format!(
                "refinement reached maximum iterations ({})",
                self.max_iterations,
            )));
        }

        let avg_quality = if quality_metrics.is_empty() {
            0.5
        } else {
            quality_metrics.values().sum::<f64>() / quality_metrics.len() as f64
        };

        self.smoothed_quality = self.smoothing_factor * avg_quality
            + (1.0 - self.smoothing_factor) * self.smoothed_quality;

        let factor = (self.smoothed_quality - 0.5).abs() * 0.05;
        let direction = if self.smoothed_quality >= 0.5 {
            1.0 + factor
        } else {
            1.0 - factor
        };

        let mut refined = HashMap::with_capacity(parameters.len());
        for (key, value) in parameters {
            refined.insert(key.clone(), value * direction);
        }

        self.history.push(RefinementRecord {
            parameters: refined.clone(),
            quality_metrics: quality_metrics.clone(),
        });
        self.iteration += 1;

        Ok(refined)
    }

    /// Check whether the refinement process has converged.
    ///
    /// Convergence is declared when the smoothed quality improvement over
    /// the last few iterations falls below `min_improvement`.
    pub fn check_convergence(&self) -> bool {
        if self.history.len() < 3 {
            return false;
        }

        let recent = &self.history[self.history.len().saturating_sub(3)..];
        let qualities: Vec<f64> = recent
            .iter()
            .map(|r| {
                if r.quality_metrics.is_empty() {
                    0.5
                } else {
                    r.quality_metrics.values().sum::<f64>() / r.quality_metrics.len() as f64
                }
            })
            .collect();

        if qualities.len() < 2 {
            return false;
        }

        let max_diff = qualities
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0_f64, f64::max);

        max_diff < self.min_improvement
    }

    /// Return the current iteration count.
    pub fn get_iteration_count(&self) -> usize {
        self.iteration
    }

    /// Return a reference to the refinement history.
    pub fn history(&self) -> &[RefinementRecord] {
        &self.history
    }

    /// Return the current smoothed quality score.
    pub fn smoothed_quality(&self) -> f64 {
        self.smoothed_quality
    }

    /// Reset the refinement state.
    pub fn reset(&mut self) {
        self.iteration = 0;
        self.smoothed_quality = 0.0;
        self.history.clear();
    }
}

#[async_trait]
impl EvolutionStrategyTrait for RefinementStrategy {
    fn name(&self) -> &str {
        "refinement"
    }

    async fn apply(&self, input: StrategyInput) -> EvolutionResult<StrategyOutput> {
        let mut this = self.clone();

        let quality_metrics = extract_quality_metrics(&input.context);
        let refined = this.refine(&input.parameters, &quality_metrics)?;
        let converged = this.check_convergence();

        Ok(StrategyOutput {
            parameters: refined,
            confidence: if converged { 0.9 } else { 0.5 },
            explanation: format!(
                "refinement iteration {}/{} (converged={converged}, quality={:.4})",
                this.get_iteration_count(),
                self.max_iterations,
                this.smoothed_quality(),
            ),
        })
    }
}

/// Extract quality metric values from the strategy context.
///
/// Context keys prefixed with `quality:` are treated as quality metrics
/// with their value parsed as `f64`.
fn extract_quality_metrics(context: &HashMap<String, String>) -> HashMap<String, f64> {
    context
        .iter()
        .filter_map(|(k, v)| {
            if k.starts_with("quality:") {
                let parsed: f64 = v.parse().ok()?;
                Some((k[8..].to_string(), parsed))
            } else {
                None
            }
        })
        .collect()
}
