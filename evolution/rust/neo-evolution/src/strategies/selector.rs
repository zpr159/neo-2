//! Strategy selector that maps improvement categories and subsystem targets
//! to concrete evolution strategies.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::types::{ImprovementCategory, SubsystemTarget};

use super::strategy::EvolutionStrategy;

/// Selects an appropriate [`EvolutionStrategy`] based on the improvement
/// category and subsystem target, using configurable weights.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategySelector {
    /// Weights for each improvement category.
    category_weights: HashMap<ImprovementCategory, f64>,
    /// Strategy overrides per (category, target) pair.
    overrides: HashMap<(ImprovementCategory, SubsystemTarget), EvolutionStrategy>,
}

impl Default for StrategySelector {
    fn default() -> Self {
        let mut category_weights = HashMap::new();
        category_weights.insert(ImprovementCategory::Performance, 1.0);
        category_weights.insert(ImprovementCategory::Reliability, 1.0);
        category_weights.insert(ImprovementCategory::Security, 0.8);
        category_weights.insert(ImprovementCategory::Architecture, 0.9);
        category_weights.insert(ImprovementCategory::CodeQuality, 0.7);
        category_weights.insert(ImprovementCategory::ResourceEfficiency, 0.9);
        category_weights.insert(ImprovementCategory::Scalability, 0.85);
        category_weights.insert(ImprovementCategory::Latency, 1.0);
        category_weights.insert(ImprovementCategory::Throughput, 0.95);
        category_weights.insert(ImprovementCategory::DependencyManagement, 0.6);

        Self {
            category_weights,
            overrides: HashMap::new(),
        }
    }
}

impl StrategySelector {
    /// Create a new selector with default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Select the best strategy for the given subsystem and improvement category.
    ///
    /// Checks overrides first, then falls back to the default mapping.
    pub fn select_strategy(
        &self,
        target: SubsystemTarget,
        category: ImprovementCategory,
    ) -> EvolutionStrategy {
        if let Some(strategy) = self.overrides.get(&(category, target)) {
            return strategy.clone();
        }
        EvolutionStrategy::default_for_category(category)
    }

    /// Return all available strategy variants.
    pub fn get_available_strategies() -> Vec<EvolutionStrategy> {
        vec![
            EvolutionStrategy::Mutation { intensity: 0.3 },
            EvolutionStrategy::Optimization {
                target_metric: "default".to_string(),
            },
            EvolutionStrategy::Refinement { iterations: 10 },
            EvolutionStrategy::Hybrid {
                strategies: Vec::new(),
                weights: Vec::new(),
            },
        ]
    }

    /// Update the weight for a given improvement category.
    pub fn configure_strategy_weight(&mut self, category: ImprovementCategory, weight: f64) {
        self.category_weights.insert(category, weight);
    }

    /// Set a per (category, target) strategy override.
    pub fn set_override(
        &mut self,
        category: ImprovementCategory,
        target: SubsystemTarget,
        strategy: EvolutionStrategy,
    ) {
        self.overrides.insert((category, target), strategy);
    }

    /// Retrieve the weight for a given improvement category.
    pub fn get_weight(&self, category: &ImprovementCategory) -> f64 {
        self.category_weights.get(category).copied().unwrap_or(1.0)
    }

    /// Return a reference to the full category weight map.
    pub fn category_weights(&self) -> &HashMap<ImprovementCategory, f64> {
        &self.category_weights
    }
}
