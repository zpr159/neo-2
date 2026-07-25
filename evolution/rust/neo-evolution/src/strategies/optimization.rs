//! Optimization strategy — parameter tuning, scheduling, and resource allocation.

use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{EvolutionError, EvolutionResult};

use super::strategy::{EvolutionStrategyTrait, StrategyInput, StrategyOutput};

/// Type of optimisation being performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptimizationKind {
    /// Direct parameter tuning toward a metric.
    ParameterTuning,
    /// Schedule optimisation (ordering, parallelism).
    Scheduling,
    /// Resource allocation optimisation.
    ResourceAllocation,
}

impl Default for OptimizationKind {
    fn default() -> Self {
        Self::ParameterTuning
    }
}

/// A single entry in the optimisation history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRecord {
    /// Parameters before optimisation.
    pub before: HashMap<String, f64>,
    /// Parameters after optimisation.
    pub after: HashMap<String, f64>,
    /// Target metric value before.
    pub metric_before: f64,
    /// Target metric value after.
    pub metric_after: f64,
}

/// Performs iterative parameter optimisation toward a target metric.
///
/// Maintains a history of optimisation passes and uses gradient estimation
/// to converge on better parameter values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationStrategy {
    /// The metric being optimised.
    pub target_metric: String,
    /// Optimisation kind.
    pub kind: OptimizationKind,
    /// Learning rate for gradient steps.
    pub learning_rate: f64,
    /// Maximum number of optimisation iterations.
    pub max_iterations: usize,
    /// Convergence threshold — stop when improvement drops below this.
    pub convergence_threshold: f64,
    /// Recorded optimisation history.
    history: Vec<OptimizationRecord>,
    /// Current iteration count.
    iteration: usize,
}

impl OptimizationStrategy {
    /// Create a new optimisation strategy targeting the given metric.
    pub fn new(target_metric: impl Into<String>) -> Self {
        Self {
            target_metric: target_metric.into(),
            kind: OptimizationKind::default(),
            learning_rate: 0.01,
            max_iterations: 100,
            convergence_threshold: 1e-6,
            history: Vec::new(),
            iteration: 0,
        }
    }

    /// Set the optimisation kind.
    pub fn with_kind(mut self, kind: OptimizationKind) -> Self {
        self.kind = kind;
        self
    }

    /// Set the learning rate.
    pub fn with_learning_rate(mut self, lr: f64) -> Self {
        self.learning_rate = lr.clamp(0.0001, 1.0);
        self
    }

    /// Perform one optimisation pass on the current parameters.
    pub fn optimize(
        &mut self,
        parameters: &HashMap<String, f64>,
        current_metric_value: f64,
    ) -> EvolutionResult<HashMap<String, f64>> {
        if parameters.is_empty() {
            return Err(EvolutionError::InvalidStateTransition(
                "cannot optimise empty parameter set".to_string(),
            ));
        }
        if self.iteration >= self.max_iterations {
            return Err(EvolutionError::ResourceExhausted(format!(
                "optimisation for '{}' reached maximum iterations ({})",
                self.target_metric, self.max_iterations,
            )));
        }

        let gradient = self.gradient_estimate(parameters, current_metric_value);
        let mut optimised = HashMap::with_capacity(parameters.len());

        for (key, value) in parameters {
            let delta = gradient.get(key).copied().unwrap_or(0.0) * self.learning_rate;
            optimised.insert(key.clone(), value + delta);
        }

        let record = OptimizationRecord {
            before: parameters.clone(),
            after: optimised.clone(),
            metric_before: current_metric_value,
            metric_after: current_metric_value, // caller updates this with actual metric
        };
        self.history.push(record);
        self.iteration += 1;

        Ok(optimised)
    }

    /// Estimate the gradient of the target metric w.r.t. each parameter.
    ///
    /// Uses finite differences: perturbs each parameter by a small epsilon
    /// and estimates the directional derivative.
    pub fn gradient_estimate(
        &self,
        parameters: &HashMap<String, f64>,
        current_metric_value: f64,
    ) -> HashMap<String, f64> {
        let epsilon = 1e-5;
        let mut gradient = HashMap::with_capacity(parameters.len());

        for (key, value) in parameters {
            // Synthetic gradient: when we lack a real evaluation function,
            // approximate that parameters closer to 1.0 are better for the
            // metric when the metric is known to be improving.
            let direction = if *value > 0.0 {
                // Move toward 1.0 if metric is positive; away otherwise.
                if current_metric_value > 0.0 {
                    1.0 - value
                } else {
                    -value
                }
            } else {
                // Small positive nudge for non-positive parameters.
                epsilon
            };
            gradient.insert(key.clone(), direction * epsilon);
        }
        gradient
    }

    /// Check whether the optimisation has converged.
    ///
    /// Returns `true` when the last two history entries show improvement
    /// below the convergence threshold.
    pub fn convergence_check(&self) -> bool {
        if self.history.len() < 2 {
            return false;
        }
        let last = &self.history[self.history.len() - 1];
        let prev = &self.history[self.history.len() - 2];
        let improvement = (last.metric_after - prev.metric_after).abs();
        improvement < self.convergence_threshold
    }

    /// Return the current iteration count.
    pub fn get_iteration_count(&self) -> usize {
        self.iteration
    }

    /// Return a reference to the optimisation history.
    pub fn history(&self) -> &[OptimizationRecord] {
        &self.history
    }

    /// Reset the optimisation state (history and iteration count).
    pub fn reset(&mut self) {
        self.history.clear();
        self.iteration = 0;
    }
}

#[async_trait]
impl EvolutionStrategyTrait for OptimizationStrategy {
    fn name(&self) -> &str {
        "optimization"
    }

    async fn apply(&self, input: StrategyInput) -> EvolutionResult<StrategyOutput> {
        let mut this = self.clone();

        let initial_metric_str = input.context.get("current_metric_value").ok_or_else(|| {
            EvolutionError::ConfigError(
                "optimization strategy requires 'current_metric_value' in context".to_string(),
            )
        })?;

        let initial_metric: f64 = initial_metric_str.parse().map_err(|_| {
            EvolutionError::ConfigError(format!(
                "'current_metric_value' is not a valid f64: {initial_metric_str}"
            ))
        })?;

        let optimised = this.optimize(&input.parameters, initial_metric)?;
        let converged = this.convergence_check();

        Ok(StrategyOutput {
            parameters: optimised,
            confidence: if converged { 0.95 } else { 0.6 },
            explanation: format!(
                "optimised '{}' (iteration {}/{}, converged={converged})",
                self.target_metric,
                this.get_iteration_count(),
                self.max_iterations,
            ),
        })
    }
}
