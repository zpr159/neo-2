use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::config::EvolutionConfiguration;
use crate::types::SubsystemTarget;

use super::execution_optimizer::{ExecutionOptimizer, ExecutionProfile};
use super::optimizer::{OptimizationResult, PerformanceOptimizer};
use super::resource_optimizer::{ResourceOptimizer, ResourceUsage};

/// A comprehensive snapshot of the most recent full optimisation pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationReport {
    /// Per-subsystem performance optimisation results.
    pub performance: Vec<OptimizationResult>,
    /// Resource utilisation snapshot after optimisation.
    pub resources: Vec<ResourceUsage>,
    /// Execution profiles of tracked operations.
    pub execution: Vec<ExecutionProfile>,
    /// Aggregated recommendations from all sub-optimisers.
    pub recommendations: Vec<String>,
    /// When this report was generated.
    pub generated_at: DateTime<Utc>,
}

/// Coordinates the three specialised optimisers and produces consolidated
/// reports.
#[derive(Debug, Clone)]
pub struct OptimizationEngine {
    /// Subsystem performance metric tracking and optimisation.
    pub performance_optimizer: PerformanceOptimizer,
    /// System resource allocation analysis.
    pub resource_optimizer: ResourceOptimizer,
    /// Operation-level execution profiling.
    pub execution_optimizer: ExecutionOptimizer,
    /// Global evolution configuration.
    config: EvolutionConfiguration,
}

impl OptimizationEngine {
    /// Create a new engine with the given configuration.
    pub fn new(config: EvolutionConfiguration) -> Self {
        Self {
            performance_optimizer: PerformanceOptimizer::new(),
            resource_optimizer: ResourceOptimizer::new(),
            execution_optimizer: ExecutionOptimizer::new(),
            config,
        }
    }

    /// Run all optimisers across every subsystem and return a consolidated
    /// [`OptimizationReport`].
    ///
    /// Each subsystem listed in [`SubsystemTarget`] is optimised in turn.
    /// Resource allocation and execution profiling are run once.  All
    /// recommendations from the three sub-optimisers are merged into a single
    /// list.
    pub fn full_optimization(&self) -> OptimizationReport {
        let mut performance_results: Vec<OptimizationResult> = Vec::new();
        let subsystems = [
            SubsystemTarget::Core,
            SubsystemTarget::Agents,
            SubsystemTarget::Planning,
            SubsystemTarget::Memory,
            SubsystemTarget::KnowledgeGraph,
            SubsystemTarget::Reasoning,
            SubsystemTarget::Workflows,
            SubsystemTarget::Distributed,
            SubsystemTarget::Capabilities,
            SubsystemTarget::Executive,
            SubsystemTarget::Learning,
            SubsystemTarget::Tools,
            SubsystemTarget::Runtime,
        ];

        for target in subsystems {
            if self.config.verbose_logging {
                tracing::info!("Running performance optimisation for subsystem: {target}");
            }
            performance_results.push(self.performance_optimizer.optimize(target));
        }

        let resources = self.resource_optimizer.optimize_allocation();
        let execution = self.execution_optimizer.get_profiles();
        let mut recommendations = self.resource_optimizer.get_recommendations();
        recommendations.extend(self.execution_optimizer.optimize());

        OptimizationReport {
            performance: performance_results,
            resources,
            execution,
            recommendations,
            generated_at: Utc::now(),
        }
    }

    /// Return the most recent optimisation report without re-running
    /// optimisers.
    pub fn get_optimization_report(&self) -> OptimizationReport {
        let performance = self
            .performance_optimizer
            .get_history()
            .into_iter()
            .map(|m| OptimizationResult {
                target: SubsystemTarget::Core,
                before: m.clone(),
                after: m.clone(),
                improvement_percent: 0.0,
                description: "Historical snapshot".to_string(),
            })
            .collect();

        let resources = self.resource_optimizer.analyze_resources();
        let execution = self.execution_optimizer.get_profiles();
        let mut recommendations = self.resource_optimizer.get_recommendations();
        recommendations.extend(self.execution_optimizer.optimize());

        OptimizationReport {
            performance,
            resources,
            execution,
            recommendations,
            generated_at: Utc::now(),
        }
    }

    /// Return a reference to the global configuration.
    pub fn config(&self) -> &EvolutionConfiguration {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_creation() {
        let engine = OptimizationEngine::new(EvolutionConfiguration::default());
        assert!(engine.config().sandbox_mode);
    }

    #[test]
    fn full_optimization_produces_report() {
        let engine = OptimizationEngine::new(EvolutionConfiguration::default());
        let report = engine.full_optimization();
        assert_eq!(report.performance.len(), 13);
        assert!(!report.recommendations.is_empty());
    }

    #[test]
    fn get_optimization_report_works() {
        let engine = OptimizationEngine::new(EvolutionConfiguration::default());
        let report = engine.get_optimization_report();
        assert!(!report.recommendations.is_empty());
    }
}
