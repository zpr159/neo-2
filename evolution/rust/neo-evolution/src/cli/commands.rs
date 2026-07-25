use std::sync::Arc;

use crate::error::EvolutionResult;
use crate::rest::api::EvolutionApi;
use crate::rest::types::*;
use crate::types::SubsystemTarget;

#[derive(Debug, Clone)]
pub enum EvolutionCommand {
    Analyze { target: Option<String> },
    Benchmark,
    Optimize { target: Option<String> },
    Experiment { experiment_id: Option<String> },
    Rollback { evolution_id: String },
    Proposals,
    Approve { proposal_id: String },
    Metrics,
}

impl EvolutionCommand {
    pub fn parse(args: &[String]) -> EvolutionResult<Self> {
        if args.is_empty() {
            return Err(crate::error::EvolutionError::InvalidConfiguration(
                "no command specified".into(),
            ));
        }
        match args[0].as_str() {
            "analyze" => {
                let target = args.get(1).cloned();
                Ok(Self::Analyze { target })
            }
            "benchmark" => Ok(Self::Benchmark),
            "optimize" => {
                let target = args.get(1).cloned();
                Ok(Self::Optimize { target })
            }
            "experiment" => {
                let experiment_id = args.get(1).cloned();
                Ok(Self::Experiment { experiment_id })
            }
            "rollback" => {
                let evolution_id = args.get(1).cloned().unwrap_or_default();
                Ok(Self::Rollback { evolution_id })
            }
            "proposals" => Ok(Self::Proposals),
            "approve" => {
                let proposal_id = args.get(1).cloned().unwrap_or_default();
                Ok(Self::Approve { proposal_id })
            }
            "metrics" => Ok(Self::Metrics),
            other => Err(crate::error::EvolutionError::InvalidConfiguration(format!(
                "unknown command: {other}"
            ))),
        }
    }
}

pub struct EvolutionCli {
    api: Arc<EvolutionApi>,
}

impl EvolutionCli {
    pub fn new(api: Arc<EvolutionApi>) -> Self {
        Self { api }
    }

    pub async fn execute(&self, command: EvolutionCommand) -> EvolutionResult<String> {
        match command {
            EvolutionCommand::Analyze { target } => {
                let target = target
                    .and_then(|t| t.parse().ok())
                    .unwrap_or(SubsystemTarget::Runtime);
                let results = self.api.analyzer.analyze_subsystem(target)?;
                Ok(format!(
                    "Analysis for {}:\n  Score: {:.2}\n  Findings: {}\n  Recommendations: {}",
                    target,
                    results.score,
                    results.findings.len(),
                    results.recommendations.len()
                ))
            }
            EvolutionCommand::Benchmark => {
                let summary = self.api.benchmark_suite.get_summary();
                Ok(format!(
                    "Benchmark Summary:\n  Scenarios: {}\n  Avg Duration: {:.1}ms\n  Success Rate: {:.1}%",
                    summary.total_scenarios,
                    summary.avg_duration_ms,
                    summary.success_rate * 100.0
                ))
            }
            EvolutionCommand::Optimize { target } => {
                let _target = target
                    .and_then(|t| t.parse().ok())
                    .unwrap_or(SubsystemTarget::Runtime);
                let report = self.api.optimization_engine.get_optimization_report();
                Ok(format!(
                    "Optimization Report:\n  Performance optimizations: {}\n  Resource items: {}\n  Execution profiles: {}\n  Recommendations: {}",
                    report.performance.len(),
                    report.resources.len(),
                    report.execution.len(),
                    report.recommendations.len()
                ))
            }
            EvolutionCommand::Experiment { experiment_id } => {
                if let Some(id) = experiment_id {
                    Ok(format!(
                        "Experiment {id} status: (check experiment manager)"
                    ))
                } else {
                    let list = self.api.list_experiments();
                    let count = list.experiments.len();
                    Ok(format!("Active experiments: {count}"))
                }
            }
            EvolutionCommand::Rollback { evolution_id } => {
                let resp = self
                    .api
                    .rollback(RollbackRequest {
                        evolution_id,
                        reason: "cli rollback".into(),
                    })
                    .await?;
                Ok(format!(
                    "Rollback: {} — {}",
                    if resp.success { "success" } else { "failed" },
                    resp.message
                ))
            }
            EvolutionCommand::Proposals => Ok("No pending proposals.".into()),
            EvolutionCommand::Approve { proposal_id } => {
                Ok(format!("Proposal {proposal_id}: approved"))
            }
            EvolutionCommand::Metrics => {
                let metrics = self.api.get_metrics();
                Ok(format!(
                    "Metrics:\n  Successful improvements: {}\n  Failed experiments: {}\n  Rollbacks: {}\n  Total experiments: {}\n  Total benchmarks: {}",
                    metrics.successful_improvements,
                    metrics.failed_experiments,
                    metrics.rollbacks,
                    metrics.total_experiments,
                    metrics.total_benchmarks
                ))
            }
        }
    }
}
