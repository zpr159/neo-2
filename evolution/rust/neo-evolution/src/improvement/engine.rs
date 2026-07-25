use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::analysis::self_analyzer::{AnalysisResult, Finding};
use crate::config::EvolutionConfiguration;
use crate::error::{EvolutionError, EvolutionResult};
use crate::types::{EvolutionId, EvolutionStatus, ImprovementCategory, RiskLevel, SubsystemTarget};

use super::candidate::ImprovementCandidate;
use super::evaluator::{ImprovementEvaluation, ImprovementEvaluator};
use super::priority::ImprovementPriority;
use super::proposal::ImprovementProposal;
use super::repository::ImprovementRepository;

/// Aggregate statistics about the improvement pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementStats {
    /// Total candidates ever registered.
    pub total_candidates: usize,
    /// Total proposals ever created.
    pub total_proposals: usize,
    /// Number of approved proposals.
    pub approved_count: usize,
    /// Number of rejected proposals.
    pub rejected_count: usize,
    /// Number of proposals still pending.
    pub pending_count: usize,
    /// Average evaluation score across all evaluated proposals.
    pub avg_score: f64,
}

/// Orchestrates the improvement lifecycle: creation from analysis findings,
/// evaluation, ranking, approval, and tracking.
pub struct ImprovementEngine {
    repository: ImprovementRepository,
    evaluator: ImprovementEvaluator,
    config: EvolutionConfiguration,
}

impl ImprovementEngine {
    /// Create a new engine with default evaluator weights.
    pub fn new(config: EvolutionConfiguration) -> Self {
        Self {
            repository: ImprovementRepository::new(),
            evaluator: ImprovementEvaluator::new(),
            config,
        }
    }

    /// Create a new engine with custom evaluator weights.
    pub fn with_evaluator_weights(
        config: EvolutionConfiguration,
        priority_weight: f64,
        impact_weight: f64,
        risk_weight: f64,
        feasibility_weight: f64,
    ) -> Self {
        Self {
            repository: ImprovementRepository::new(),
            evaluator: ImprovementEvaluator::with_weights(
                priority_weight,
                impact_weight,
                risk_weight,
                feasibility_weight,
            ),
            config,
        }
    }

    /// Create improvement candidates from an analysis result and wrap each
    /// in a proposal. Returns the IDs of all created proposals.
    pub fn propose_improvement(
        &self,
        analysis: &AnalysisResult,
    ) -> EvolutionResult<Vec<EvolutionId>> {
        if analysis.findings.is_empty() {
            return Err(EvolutionError::AnalysisFailed(
                "analysis result contains no findings".into(),
            ));
        }

        let max = self.config.max_improvements_per_cycle;
        let mut proposal_ids = Vec::with_capacity(analysis.findings.len().min(max));

        for finding in analysis.findings.iter().take(max) {
            let candidate = self.finding_to_candidate(finding, analysis.subsystem);
            let proposal =
                self.candidate_to_proposal(candidate, finding, &analysis.recommendations);
            let id = self.repository.add_proposal(proposal);
            proposal_ids.push(id);
        }

        Ok(proposal_ids)
    }

    /// Evaluate all pending proposals and return their evaluations.
    pub fn evaluate_proposals(&self) -> Vec<ImprovementEvaluation> {
        let pending = self.repository.get_pending_proposals();
        pending.iter().map(|p| self.evaluator.evaluate(p)).collect()
    }

    /// Return the top `n` proposals ranked by evaluation score.
    pub fn get_top_proposals(&self, n: usize) -> Vec<(ImprovementProposal, ImprovementEvaluation)> {
        let mut pending = self.repository.get_pending_proposals();
        self.evaluator.rank_proposals(&mut pending);

        pending
            .into_iter()
            .take(n)
            .map(|p| {
                let eval = self.evaluator.evaluate(&p);
                (p, eval)
            })
            .collect()
    }

    /// Approve a proposal by ID.
    pub fn approve_proposal(&self, id: &EvolutionId, approver: &str) -> EvolutionResult<()> {
        self.repository.approve_proposal(id, approver)
    }

    /// Reject a proposal by ID.
    pub fn reject_proposal(&self, id: &EvolutionId) -> EvolutionResult<()> {
        self.repository.reject_proposal(id)
    }

    /// Compute aggregate statistics.
    pub fn get_stats(&self) -> ImprovementStats {
        let candidates = self.repository.list_candidates();
        let proposals = self.repository.list_proposals();

        let total_candidates = candidates.len();
        let total_proposals = proposals.len();

        let mut approved_count = 0usize;
        let mut rejected_count = 0usize;
        let mut pending_count = 0usize;

        for prop in &proposals {
            if prop.approved {
                approved_count += 1;
            } else if matches!(
                prop.candidate.status,
                EvolutionStatus::Cancelled | EvolutionStatus::Failed
            ) {
                rejected_count += 1;
            } else {
                pending_count += 1;
            }
        }

        let avg_score = if proposals.is_empty() {
            0.0
        } else {
            let evaluations: Vec<_> = proposals
                .iter()
                .map(|p| self.evaluator.evaluate(p).score)
                .collect();
            evaluations.iter().sum::<f64>() / evaluations.len() as f64
        };

        ImprovementStats {
            total_candidates,
            total_proposals,
            approved_count,
            rejected_count,
            pending_count,
            avg_score,
        }
    }

    /// Access the underlying repository.
    pub fn repository(&self) -> &ImprovementRepository {
        &self.repository
    }

    // -- private helpers ------------------------------------------------

    /// Convert a single analysis [`Finding`] into an [`ImprovementCandidate`].
    fn finding_to_candidate(
        &self,
        finding: &Finding,
        target: SubsystemTarget,
    ) -> ImprovementCandidate {
        let priority = self.severity_to_priority(finding.severity);
        let impact = self.severity_to_impact(finding.severity);

        let implementation_plan = self.generate_implementation_plan(finding);

        ImprovementCandidate::new(
            self.truncate(&finding.description, 128),
            &finding.description,
            finding.category,
            target,
            priority,
            impact,
            finding.severity,
            implementation_plan,
        )
    }

    /// Wrap a candidate into a [`ImprovementProposal`].
    fn candidate_to_proposal(
        &self,
        candidate: ImprovementCandidate,
        finding: &Finding,
        recommendations: &[String],
    ) -> ImprovementProposal {
        let justification = format!(
            "Analysis of {} subsystem identified: {}",
            candidate.target, finding.description,
        );

        let expected_outcomes = self.derive_expected_outcomes(finding);
        let success_criteria = self.derive_success_criteria(finding);
        let rollback_plan = self.derive_rollback_plan(finding);

        let approval_required =
            matches!(candidate.risk_level, RiskLevel::High | RiskLevel::Critical)
                || candidate.priority <= ImprovementPriority::High;

        ImprovementProposal::new(
            candidate,
            justification,
            expected_outcomes,
            success_criteria,
            rollback_plan,
            approval_required,
        )
    }

    fn severity_to_priority(&self, severity: RiskLevel) -> ImprovementPriority {
        match severity {
            RiskLevel::Critical => ImprovementPriority::Critical,
            RiskLevel::High => ImprovementPriority::High,
            RiskLevel::Medium => ImprovementPriority::Medium,
            RiskLevel::Low => ImprovementPriority::Low,
            RiskLevel::None => ImprovementPriority::Informational,
        }
    }

    fn severity_to_impact(&self, severity: RiskLevel) -> f64 {
        match severity {
            RiskLevel::Critical => 0.95,
            RiskLevel::High => 0.80,
            RiskLevel::Medium => 0.55,
            RiskLevel::Low => 0.30,
            RiskLevel::None => 0.10,
        }
    }

    fn generate_implementation_plan(&self, finding: &Finding) -> String {
        let location = finding.location.as_deref().unwrap_or("unknown location");
        match finding.category {
            ImprovementCategory::Performance => {
                format!("Profile and optimise {location}; apply targeted fixes; benchmark before and after.")
            }
            ImprovementCategory::Reliability => {
                format!("Harden {location}; add error handling and retry logic; validate with fault injection.")
            }
            ImprovementCategory::Security => {
                format!("Audit {location}; apply least-privilege controls; run security scan.")
            }
            ImprovementCategory::Architecture => {
                format!("Redesign {location}; introduce abstraction layer; verify with integration tests.")
            }
            ImprovementCategory::CodeQuality => {
                format!("Refactor {location}; add documentation and tests; run linter and type checker.")
            }
            ImprovementCategory::ResourceEfficiency => {
                format!("Analyse resource usage at {location}; apply optimisations; monitor consumption.")
            }
            ImprovementCategory::Scalability => {
                format!("Load-test {location}; add sharding or partitioning; validate horizontal scaling.")
            }
            ImprovementCategory::Latency => {
                format!(
                    "Trace latency at {location}; apply async or caching; measure p99 improvement."
                )
            }
            ImprovementCategory::Throughput => {
                format!("Benchmark throughput at {location}; add batching or parallelism; verify gains.")
            }
            ImprovementCategory::DependencyManagement => {
                format!("Audit dependencies at {location}; update or replace outdated crates.")
            }
        }
    }

    fn derive_expected_outcomes(&self, finding: &Finding) -> Vec<String> {
        let mut outcomes = Vec::new();
        outcomes.push(format!("Resolve: {}", finding.description));
        if let Some(location) = &finding.location {
            outcomes.push(format!("Improve code health at {location}"));
        }
        if let Some((metric, value)) = finding.metrics.iter().next() {
            outcomes.push(format!("Improve {metric} (currently {value})"));
        }
        outcomes
    }

    fn derive_success_criteria(&self, finding: &Finding) -> Vec<String> {
        let mut criteria = Vec::new();
        criteria.push(format!(
            "Finding \"{}\" is no longer present in subsequent analysis",
            self.truncate(&finding.description, 64),
        ));
        for (metric, value) in &finding.metrics {
            criteria.push(format!("{metric} improves beyond current value of {value}"));
        }
        criteria
    }

    fn derive_rollback_plan(&self, finding: &Finding) -> String {
        format!(
            "Revert changes related to {}; restore previous configuration and verify system stability.",
            finding
                .location
                .as_deref()
                .unwrap_or("the affected module")
        )
    }

    fn truncate(&self, s: &str, max_chars: usize) -> String {
        if s.len() <= max_chars {
            s.to_string()
        } else {
            format!("{}…", &s[..max_chars])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_analysis() -> AnalysisResult {
        AnalysisResult {
            subsystem: SubsystemTarget::Core,
            timestamp: Utc::now(),
            findings: vec![
                Finding {
                    category: ImprovementCategory::Performance,
                    severity: RiskLevel::High,
                    description: "Event-loop tick latency exceeds target at p99".into(),
                    location: Some("neo_core::event_loop".into()),
                    metrics: [("p99_latency_ms".into(), 42.0), ("target_ms".into(), 20.0)].into(),
                },
                Finding {
                    category: ImprovementCategory::Reliability,
                    severity: RiskLevel::Low,
                    description: "Graceful shutdown handler not registered".into(),
                    location: Some("neo_core::lifecycle".into()),
                    metrics: HashMap::new(),
                },
            ],
            score: 0.65,
            recommendations: vec!["Optimise event loop".into()],
        }
    }

    #[test]
    fn propose_from_analysis() {
        let engine = ImprovementEngine::new(EvolutionConfiguration::default());
        let analysis = make_analysis();
        let ids = engine.propose_improvement(&analysis).unwrap();
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn propose_empty_findings_errors() {
        let engine = ImprovementEngine::new(EvolutionConfiguration::default());
        let analysis = AnalysisResult {
            subsystem: SubsystemTarget::Core,
            timestamp: Utc::now(),
            findings: vec![],
            score: 1.0,
            recommendations: vec![],
        };
        assert!(engine.propose_improvement(&analysis).is_err());
    }

    #[test]
    fn evaluate_and_rank() {
        let engine = ImprovementEngine::new(EvolutionConfiguration::default());
        let analysis = make_analysis();
        engine.propose_improvement(&analysis).unwrap();

        let evals = engine.evaluate_proposals();
        assert_eq!(evals.len(), 2);

        let top = engine.get_top_proposals(1);
        assert_eq!(top.len(), 1);
        // The high-severity finding should rank first
        assert_eq!(top[0].0.candidate.risk_level, RiskLevel::High);
    }

    #[test]
    fn approve_reject_flow() {
        let engine = ImprovementEngine::new(EvolutionConfiguration::default());
        let analysis = make_analysis();
        let ids = engine.propose_improvement(&analysis).unwrap();

        engine.approve_proposal(&ids[0], "admin").unwrap();
        engine.reject_proposal(&ids[1]).unwrap();

        let stats = engine.get_stats();
        assert_eq!(stats.approved_count, 1);
        assert_eq!(stats.rejected_count, 1);
    }

    #[test]
    fn stats() {
        let engine = ImprovementEngine::new(EvolutionConfiguration::default());
        let analysis = make_analysis();
        engine.propose_improvement(&analysis).unwrap();

        let stats = engine.get_stats();
        assert_eq!(stats.total_proposals, 2);
        assert!(stats.avg_score > 0.0);
    }

    #[test]
    fn respect_max_per_cycle() {
        let mut config = EvolutionConfiguration::default();
        config.max_improvements_per_cycle = 1;
        let engine = ImprovementEngine::new(config);
        let analysis = make_analysis();
        let ids = engine.propose_improvement(&analysis).unwrap();
        assert_eq!(ids.len(), 1);
    }
}
