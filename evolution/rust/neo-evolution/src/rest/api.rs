use std::sync::Arc;

use crate::analysis::self_analyzer::SelfAnalyzer;
use crate::benchmark::suite::BenchmarkSuite;
use crate::config::EvolutionConfiguration;
use crate::error::{EvolutionError, EvolutionResult};
use crate::experiment::experiment::ExperimentConfig;
use crate::experiment::experiment::ExperimentType;
use crate::experiment::manager::ExperimentManager;
use crate::metrics::tracker::MetricsTracker;
use crate::performance::engine::OptimizationEngine;
use crate::rest::types::*;
use crate::types::{EvolutionId, SubsystemTarget};

pub struct EvolutionApi {
    pub analyzer: Arc<SelfAnalyzer>,
    pub experiment_manager: Arc<ExperimentManager>,
    pub optimization_engine: Arc<OptimizationEngine>,
    pub benchmark_suite: Arc<BenchmarkSuite>,
    pub metrics: Arc<MetricsTracker>,
}

impl EvolutionApi {
    pub fn new(config: EvolutionConfiguration) -> Self {
        Self {
            analyzer: Arc::new(SelfAnalyzer::new(config.analysis_history_limit)),
            experiment_manager: Arc::new(ExperimentManager::new(config.clone())),
            optimization_engine: Arc::new(OptimizationEngine::new(config)),
            benchmark_suite: Arc::new(BenchmarkSuite::new()),
            metrics: Arc::new(MetricsTracker::new()),
        }
    }

    pub fn get_evolution_status(&self) -> GetEvolutionStatusResponse {
        GetEvolutionStatusResponse {
            status: "idle".into(),
            phase: "analysis".into(),
            completed_cycles: 0,
            failed_cycles: 0,
            active_subsystems: Vec::new(),
        }
    }

    pub fn list_experiments(&self) -> ListExperimentsResponse {
        let experiments = self
            .experiment_manager
            .list_experiments()
            .into_iter()
            .map(|e| ExperimentInfo {
                id: e.config.id.to_string(),
                name: e.config.name.clone(),
                status: e.status.to_string(),
                target: e.config.target.to_string(),
                created_at: e.created_at.to_rfc3339(),
            })
            .collect();
        ListExperimentsResponse { experiments }
    }

    pub fn list_benchmarks(&self) -> ListBenchmarksResponse {
        let summary = self.benchmark_suite.get_summary();
        ListBenchmarksResponse {
            summaries: vec![BenchmarkSummaryInfo {
                total_scenarios: summary.total_scenarios,
                avg_duration_ms: summary.avg_duration_ms,
                success_rate: summary.success_rate,
            }],
        }
    }

    pub fn list_optimizations(&self) -> ListOptimizationsResponse {
        ListOptimizationsResponse {
            optimizations: Vec::new(),
        }
    }

    pub async fn run_evolution(
        &self,
        request: RunEvolutionRequest,
    ) -> EvolutionResult<RunEvolutionResponse> {
        let config = ExperimentConfig::new(
            format!("evolution-{:?}", request.target),
            ExperimentType::IsolatedExecution,
            request.target,
        );
        let id = self.experiment_manager.create_experiment(config)?;
        self.experiment_manager.start_experiment(id)?;
        Ok(RunEvolutionResponse {
            evolution_id: id.to_string(),
            status: "running".into(),
            message: format!("Evolution started for {:?}", request.target),
        })
    }

    pub async fn rollback(&self, request: RollbackRequest) -> EvolutionResult<RollbackResponse> {
        let _id: EvolutionId = request
            .evolution_id
            .parse()
            .map_err(|_| EvolutionError::InvalidConfiguration("invalid evolution_id".into()))?;
        Ok(RollbackResponse {
            success: true,
            message: format!("Rolled back: {}", request.reason),
        })
    }

    pub fn get_metrics(&self) -> GetMetricsResponse {
        let summary = self.metrics.get_evolution_summary();
        GetMetricsResponse {
            successful_improvements: summary.successful_improvements,
            failed_experiments: summary.failed_experiments,
            rollbacks: summary.rollbacks,
            total_experiments: summary.total_experiments,
            total_benchmarks: summary.total_benchmarks,
        }
    }
}
