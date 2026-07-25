//! Mutation strategy — applies controlled stochastic perturbations to parameters.

use std::collections::HashMap;

use async_trait::async_trait;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::error::{EvolutionError, EvolutionResult};

use super::strategy::{EvolutionStrategyTrait, StrategyInput, StrategyOutput};

/// Mutation mode controls how perturbations are computed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationMode {
    /// Additive gaussian noise scaled by `intensity`.
    Gaussian,
    /// Uniform random perturbation within [-intensity, +intensity].
    Uniform,
    /// Adaptive intensity that shrinks when fitness is improving.
    Adaptive,
}

impl Default for MutationMode {
    fn default() -> Self {
        Self::Gaussian
    }
}

/// Applies stochastic mutations to a parameter map.
///
/// The mutation `intensity` (0.0–1.0) controls the magnitude of changes.
/// A seeded RNG ensures reproducibility when the same seed is reused.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationStrategy {
    /// Base intensity of mutations (0.0, 1.0].
    pub intensity: f64,
    /// Current mutation mode.
    pub mode: MutationMode,
    /// Optional seed for reproducible mutations.
    pub seed: Option<u64>,
    /// Running estimate of mutation rate based on fitness history.
    mutation_rate: f64,
    /// Fitness history (most-recent last).
    fitness_history: Vec<f64>,
}

impl MutationStrategy {
    /// Create a new mutation strategy with the given intensity.
    pub fn new(intensity: f64) -> Self {
        Self {
            intensity: intensity.clamp(0.01, 1.0),
            mode: MutationMode::Gaussian,
            seed: None,
            mutation_rate: intensity,
            fitness_history: Vec::new(),
        }
    }

    /// Create a new mutation strategy with a specific mode.
    pub fn with_mode(intensity: f64, mode: MutationMode) -> Self {
        Self {
            intensity: intensity.clamp(0.01, 1.0),
            mode,
            seed: None,
            mutation_rate: intensity,
            fitness_history: Vec::new(),
        }
    }

    /// Set the seed for reproducible mutations.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Apply a mutation to the given parameter map and return the mutated map.
    pub fn apply_mutation(
        &self,
        parameters: &HashMap<String, f64>,
    ) -> EvolutionResult<HashMap<String, f64>> {
        let seed = self.seed.unwrap_or_else(rand::random);
        let mut rng = StdRng::seed_from_u64(seed);

        let mut mutated = HashMap::with_capacity(parameters.len());
        for (key, value) in parameters {
            let perturbation = match self.mode {
                MutationMode::Gaussian => {
                    let std_dev = self.mutation_rate * value.abs().max(1.0) * 0.1;
                    rng.gen_range(-std_dev..=std_dev)
                }
                MutationMode::Uniform => {
                    let range = self.mutation_rate * value.abs().max(1.0);
                    rng.gen_range(-range..=range)
                }
                MutationMode::Adaptive => {
                    let adaptive_rate = self.calculate_adaptive_rate();
                    let range = adaptive_rate * value.abs().max(1.0) * 0.1;
                    rng.gen_range(-range..=range)
                }
            };
            mutated.insert(key.clone(), value + perturbation);
        }
        Ok(mutated)
    }

    /// Compute the adaptive mutation rate from fitness history.
    ///
    /// When fitness is improving (trend is positive) the rate shrinks;
    /// when it stalls or regresses the rate increases.
    pub fn calculate_mutation_rate(&self) -> f64 {
        self.calculate_adaptive_rate()
    }

    /// Record a fitness observation and recompute the mutation rate.
    pub fn record_fitness(&mut self, fitness: f64) {
        self.fitness_history.push(fitness);
        if self.fitness_history.len() > 100 {
            self.fitness_history.remove(0);
        }
        self.mutation_rate = self.calculate_adaptive_rate();
    }

    /// Internal adaptive-rate calculation based on recent fitness trend.
    fn calculate_adaptive_rate(&self) -> f64 {
        if self.fitness_history.len() < 2 {
            return self.intensity;
        }

        let window = &self.fitness_history[self.fitness_history.len().saturating_sub(10)..];
        let mut improving_count = 0usize;
        for pair in window.windows(2) {
            if pair[1] > pair[0] {
                improving_count += 1;
            }
        }
        let improvement_ratio =
            f64::from(improving_count as u32) / f64::from((window.len() - 1) as u32);

        // Shrinking when improving, expanding when stagnating.
        let factor = 1.0 + (0.5 - improvement_ratio);
        (self.intensity * factor).clamp(0.01, 1.0)
    }
}

#[async_trait]
impl EvolutionStrategyTrait for MutationStrategy {
    fn name(&self) -> &str {
        "mutation"
    }

    async fn apply(&self, input: StrategyInput) -> EvolutionResult<StrategyOutput> {
        if input.parameters.is_empty() {
            return Err(EvolutionError::InvalidStateTransition(
                "cannot mutate empty parameter set".to_string(),
            ));
        }

        let mutated = self.apply_mutation(&input.parameters)?;

        let changed_count = mutated
            .iter()
            .filter(|(k, v)| {
                input
                    .parameters
                    .get(*k)
                    .map_or(true, |orig| (orig - *v).abs() > 1e-12)
            })
            .count();

        Ok(StrategyOutput {
            parameters: mutated,
            confidence: self.mutation_rate,
            explanation: format!(
                "applied {mode:?} mutation to {changed}/{total} parameters (rate={rate:.4})",
                mode = self.mode,
                changed = changed_count,
                total = input.parameters.len(),
                rate = self.mutation_rate,
            ),
        })
    }
}
