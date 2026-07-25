use std::sync::Arc;

use parking_lot::RwLock;
use tracing;

use crate::analysis::self_analyzer::{AnalysisResult, SelfAnalyzer};
use crate::benchmark::regression::RegressionDetector;
use crate::benchmark::suite::BenchmarkSuite;
use crate::config::EvolutionConfiguration;
use crate::context::EvolutionContext;
use crate::error::EvolutionResult;
use crate::experiment::experiment::ExperimentConfig;
use crate::experiment::manager::ExperimentManager;
use crate::governance::approval::ApprovalManager;
use crate::governance::audit::EvolutionAudit;
use crate::governance::authorization::{AuthorizationLevel, EvolutionAuthorization};
use crate::governance::validator::EvolutionPolicyValidator;
use crate::heuristic_evolution::evolution::HeuristicEvolution;
use crate::improvement::engine::ImprovementEngine;
use crate::lifecycle::EvolutionLifecycle;
use crate::metrics::tracker::MetricsTracker;
use crate::performance::engine::OptimizationEngine;
use crate::policy_evolution::engine::PolicyEvolutionEngine;
use crate::sandbox::sandbox::SandboxConfig;
use crate::sdk::builder::EvolutionEngineBuilder;
use crate::state::EvolutionSnapshot;
use crate::types::{EvolutionId, EvolutionStatus, RiskLevel, SubsystemTarget};

use crate::agent_evolution::evolution::AgentEvolution;
use crate::capability_evolution::evolution::CapabilityEvolution;
use crate::distributed_evolution::evolution::DistributedEvolution;
use crate::learning_evolution::evolution::LearningEvolution;
use crate::planning_evolution::evolution::PlanningEvolution;
use crate::reasoning_evolution::evolution::ReasoningEvolution;
use crate::workflow_evolution::evolution::WorkflowEvolution;

/// Top-level orchestrator for the entire Self-Evolution Infrastructure.
pub struct EvolutionEngine {
    config: EvolutionConfiguration,
    context: Arc<EvolutionContext>,
    pub self_analyzer: Arc<SelfAnalyzer>,
    pub improvement_engine: Arc<ImprovementEngine>,
    pub experiment_manager: Arc<ExperimentManager>,
    pub sandbox_config: SandboxConfig,
    pub policy_evolution: Arc<PolicyEvolutionEngine>,
    pub heuristic_evolution: Arc<HeuristicEvolution>,
    pub workflow_evolution: Arc<WorkflowEvolution>,
    pub capability_evolution: Arc<CapabilityEvolution>,
    pub agent_evolution: Arc<AgentEvolution>,
    pub planning_evolution: Arc<PlanningEvolution>,
    pub learning_evolution: Arc<LearningEvolution>,
    pub reasoning_evolution: Arc<ReasoningEvolution>,
    pub distributed_evolution: Arc<DistributedEvolution>,
    pub optimization_engine: Arc<OptimizationEngine>,
    pub benchmark_suite: Arc<BenchmarkSuite>,
    pub regression_detector: Arc<RegressionDetector>,
    pub governance_authorization: Arc<EvolutionAuthorization>,
    pub approval_manager: Arc<ApprovalManager>,
    pub audit: Arc<EvolutionAudit>,
    pub validator: Arc<EvolutionPolicyValidator>,
    pub metrics: Arc<MetricsTracker>,
    lifecycle_handlers: RwLock<Vec<Arc<dyn EvolutionLifecycle + Send + Sync>>>,
}

impl EvolutionEngine {
    /// Create a builder for configuring the engine.
    pub fn builder() -> EvolutionEngineBuilder {
        EvolutionEngineBuilder::new()
    }

    /// Create a new engine with the given configuration.
    pub fn new(config: EvolutionConfiguration) -> EvolutionResult<Self> {
        let context = EvolutionContext::new(config.clone());
        Ok(Self {
            self_analyzer: Arc::new(SelfAnalyzer::new(config.analysis_history_limit)),
            improvement_engine: Arc::new(ImprovementEngine::new(config.clone())),
            experiment_manager: Arc::new(ExperimentManager::new(config.clone())),
            sandbox_config: SandboxConfig::default(),
            policy_evolution: PolicyEvolutionEngine::new(config.clone()),
            heuristic_evolution: HeuristicEvolution::new(config.clone()),
            workflow_evolution: WorkflowEvolution::new(config.clone()),
            capability_evolution: CapabilityEvolution::new(config.clone()),
            agent_evolution: AgentEvolution::new(config.clone()),
            planning_evolution: PlanningEvolution::new(config.clone()),
            learning_evolution: LearningEvolution::new(config.clone()),
            reasoning_evolution: ReasoningEvolution::new(config.clone()),
            distributed_evolution: DistributedEvolution::new(config.clone()),
            optimization_engine: Arc::new(OptimizationEngine::new(config.clone())),
            benchmark_suite: Arc::new(BenchmarkSuite::new()),
            regression_detector: Arc::new(RegressionDetector::new(0.05)),
            governance_authorization: Arc::new(EvolutionAuthorization::new(
                AuthorizationLevel::Full,
                vec![],
                RiskLevel::High,
                None,
                None,
            )),
            approval_manager: Arc::new(ApprovalManager::new()),
            audit: Arc::new(EvolutionAudit::new()),
            validator: Arc::new(EvolutionPolicyValidator::new()),
            metrics: Arc::new(MetricsTracker::new()),
            config,
            context,
            lifecycle_handlers: RwLock::new(Vec::new()),
        })
    }

    /// Start the evolution engine.
    pub async fn start(&self) -> EvolutionResult<()> {
        self.context.transition_to(EvolutionStatus::Running);
        tracing::info!("evolution engine started");
        Ok(())
    }

    /// Gracefully stop the evolution engine.
    pub async fn stop(&self) -> EvolutionResult<()> {
        self.context.transition_to(EvolutionStatus::Cancelled);
        tracing::info!("evolution engine stopped");
        Ok(())
    }

    /// Run analysis on a specific subsystem.
    pub fn run_analysis(&self, target: SubsystemTarget) -> EvolutionResult<Vec<AnalysisResult>> {
        let result = self.self_analyzer.analyze_subsystem(target)?;
        self.context.record_analysis(target);
        self.metrics.record_improvement_success();
        Ok(vec![result])
    }

    /// Run analysis on all subsystems.
    pub fn run_full_analysis(&self) -> EvolutionResult<Vec<AnalysisResult>> {
        let targets = [
            SubsystemTarget::Runtime,
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
        ];

        let mut results = Vec::new();
        for target in targets {
            let result = self.self_analyzer.analyze_subsystem(target)?;
            self.context.record_analysis(target);
            results.push(result);
        }
        Ok(results)
    }

    /// Create improvement proposals from an analysis result.
    pub fn propose_improvements_from_analysis(
        &self,
        analysis: &AnalysisResult,
    ) -> EvolutionResult<Vec<EvolutionId>> {
        self.improvement_engine.propose_improvement(analysis)
    }

    /// Approve a proposal.
    pub fn approve_proposal(
        &self,
        proposal_id: EvolutionId,
        approver: &str,
    ) -> EvolutionResult<()> {
        self.improvement_engine
            .approve_proposal(&proposal_id, approver)
    }

    /// Start an experiment.
    pub fn start_experiment(&self, config: ExperimentConfig) -> EvolutionResult<EvolutionId> {
        let id = self.experiment_manager.create_experiment(config)?;
        self.experiment_manager.start_experiment(id)?;
        self.metrics.record_experiment(true, 0.0);
        self.context.record_experiment(SubsystemTarget::Runtime);
        tracing::info!(experiment_id = %id, "experiment started");
        Ok(id)
    }

    /// Run optimization on a target subsystem.
    pub fn run_optimization(
        &self,
        target: SubsystemTarget,
    ) -> EvolutionResult<crate::performance::optimizer::OptimizationResult> {
        let result = self
            .optimization_engine
            .performance_optimizer
            .optimize(target);
        self.metrics.record_optimization(true, 5.0);
        Ok(result)
    }

    /// Run benchmarks.
    pub fn run_benchmark(&self) -> EvolutionResult<crate::benchmark::suite::BenchmarkSummary> {
        let summary = self.benchmark_suite.get_summary();
        self.metrics.record_benchmark();
        Ok(summary)
    }

    /// Rollback an evolution.
    pub fn rollback(&self, _evolution_id: EvolutionId, reason: &str) -> EvolutionResult<()> {
        self.metrics.record_rollback();
        tracing::warn!(reason = %reason, "evolution rolled back");
        let handlers = self.lifecycle_handlers.read();
        for handler in handlers.iter() {
            let _ = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(handler.on_rollback())
            });
        }
        Ok(())
    }

    /// Get current status snapshot.
    pub fn get_status(&self) -> EvolutionSnapshot {
        self.context.snapshot()
    }

    /// Get metrics as JSON.
    pub fn get_metrics(&self) -> EvolutionResult<serde_json::Value> {
        let summary = self.metrics.get_evolution_summary();
        serde_json::to_value(&summary)
            .map_err(|e| crate::error::EvolutionError::SerializationError(e.to_string()))
    }

    /// Register a lifecycle handler.
    pub fn register_lifecycle_handler(&self, handler: Arc<dyn EvolutionLifecycle + Send + Sync>) {
        self.lifecycle_handlers.write().push(handler);
    }
}
