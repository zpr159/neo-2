use std::collections::HashMap;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::error::EvolutionResult;
use crate::types::{ImprovementCategory, RiskLevel, SubsystemTarget};

/// A single finding produced by the self analyser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Category of improvement this finding relates to.
    pub category: ImprovementCategory,
    /// Severity / risk level of the finding.
    pub severity: RiskLevel,
    /// Human-readable description of the finding.
    pub description: String,
    /// Optional location hint (e.g. module path, function name).
    pub location: Option<String>,
    /// Numeric metrics associated with this finding.
    pub metrics: HashMap<String, f64>,
}

/// Result of analysing a single subsystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// The subsystem that was analysed.
    pub subsystem: SubsystemTarget,
    /// When the analysis was performed.
    pub timestamp: DateTime<Utc>,
    /// Findings produced by the analysis.
    pub findings: Vec<Finding>,
    /// Aggregate health score in `[0.0, 1.0]` where 1.0 is perfectly healthy.
    pub score: f64,
    /// Actionable recommendations derived from the findings.
    pub recommendations: Vec<String>,
}

/// Performs self-analysis on individual subsystems and maintains a history
/// of past analyses.
pub struct SelfAnalyzer {
    history: RwLock<Vec<AnalysisResult>>,
    history_limit: usize,
}

impl SelfAnalyzer {
    /// Create a new `SelfAnalyzer` with the given history size limit.
    pub fn new(history_limit: usize) -> Self {
        Self {
            history: RwLock::new(Vec::new()),
            history_limit,
        }
    }

    /// Analyse a single subsystem and return deterministic findings.
    pub fn analyze_subsystem(&self, target: SubsystemTarget) -> EvolutionResult<AnalysisResult> {
        let findings = generate_findings_for(target);
        let score = compute_score(&findings);
        let recommendations = derive_recommendations(&findings);

        let result = AnalysisResult {
            subsystem: target,
            timestamp: Utc::now(),
            findings,
            score,
            recommendations,
        };

        {
            let mut history = self.history.write();
            if history.len() >= self.history_limit {
                history.remove(0);
            }
            history.push(result.clone());
        }

        Ok(result)
    }

    /// Analyse every known subsystem.
    pub fn analyze_all(&self) -> EvolutionResult<Vec<AnalysisResult>> {
        let targets = all_subsystem_targets();
        let mut results = Vec::with_capacity(targets.len());
        for target in targets {
            results.push(self.analyze_subsystem(target)?);
        }
        Ok(results)
    }

    /// Return a clone of the full analysis history.
    pub fn get_analysis_history(&self) -> Vec<AnalysisResult> {
        self.history.read().clone()
    }
}

fn all_subsystem_targets() -> Vec<SubsystemTarget> {
    vec![
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
    ]
}

fn empty_metrics() -> HashMap<String, f64> {
    HashMap::new()
}

fn generate_findings_for(target: SubsystemTarget) -> Vec<Finding> {
    match target {
        SubsystemTarget::Core => vec![
            Finding {
                category: ImprovementCategory::Performance,
                severity: RiskLevel::Medium,
                description: "Event-loop tick latency exceeds target at p99".into(),
                location: Some("neo_core::event_loop".into()),
                metrics: [("p99_latency_ms".into(), 42.0), ("target_ms".into(), 20.0)].into(),
            },
            Finding {
                category: ImprovementCategory::Reliability,
                severity: RiskLevel::Low,
                description: "Graceful shutdown handler not registered for SIGTERM".into(),
                location: Some("neo_core::lifecycle".into()),
                metrics: empty_metrics(),
            },
            Finding {
                category: ImprovementCategory::Architecture,
                severity: RiskLevel::Low,
                description: "Circular module dependency between core and runtime".into(),
                location: Some("neo_core <-> neo_runtime".into()),
                metrics: empty_metrics(),
            },
        ],
        SubsystemTarget::Agents => vec![
            Finding {
                category: ImprovementCategory::Performance,
                severity: RiskLevel::High,
                description: "Agent spawn overhead is 3x above budget".into(),
                location: Some("neo_agents::factory".into()),
                metrics: [
                    ("spawn_overhead_us".into(), 1500.0),
                    ("budget_us".into(), 500.0),
                ]
                .into(),
            },
            Finding {
                category: ImprovementCategory::Scalability,
                severity: RiskLevel::Medium,
                description: "Agent registry uses a single global lock".into(),
                location: Some("neo_agents::registry".into()),
                metrics: empty_metrics(),
            },
            Finding {
                category: ImprovementCategory::CodeQuality,
                severity: RiskLevel::Low,
                description: "Duplicated retry logic across agent implementations".into(),
                location: Some("neo_agents::strategies".into()),
                metrics: empty_metrics(),
            },
        ],
        SubsystemTarget::Planning => vec![
            Finding {
                category: ImprovementCategory::Performance,
                severity: RiskLevel::Medium,
                description: "Task decomposition exceeds time budget for deep plans".into(),
                location: Some("neo_planning::decomposer".into()),
                metrics: [("avg_decomp_ms".into(), 320.0), ("budget_ms".into(), 100.0)].into(),
            },
            Finding {
                category: ImprovementCategory::Reliability,
                severity: RiskLevel::High,
                description: "Planner does not handle degenerate graph inputs".into(),
                location: Some("neo_planning::scheduler".into()),
                metrics: empty_metrics(),
            },
        ],
        SubsystemTarget::Memory => vec![
            Finding {
                category: ImprovementCategory::ResourceEfficiency,
                severity: RiskLevel::High,
                description: "Working set exceeds configured limit by 18%".into(),
                location: Some("neo_memory::store".into()),
                metrics: [
                    ("working_set_mb".into(), 1180.0),
                    ("limit_mb".into(), 1000.0),
                ]
                .into(),
            },
            Finding {
                category: ImprovementCategory::Performance,
                severity: RiskLevel::Medium,
                description: "Eviction rate spikes under concurrent access".into(),
                location: Some("neo_memory::eviction".into()),
                metrics: [("eviction_rate".into(), 0.35)].into(),
            },
            Finding {
                category: ImprovementCategory::Reliability,
                severity: RiskLevel::Low,
                description: "Cache hit rate drops below 80% threshold".into(),
                location: Some("neo_memory::cache".into()),
                metrics: [("hit_rate".into(), 0.74)].into(),
            },
        ],
        SubsystemTarget::KnowledgeGraph => vec![
            Finding {
                category: ImprovementCategory::Performance,
                severity: RiskLevel::Medium,
                description: "Graph traversal latency increases super-linearly with depth".into(),
                location: Some("neo_knowledge_graph::traversal".into()),
                metrics: [("depth".into(), 6.0), ("latency_ms".into(), 850.0)].into(),
            },
            Finding {
                category: ImprovementCategory::Reliability,
                severity: RiskLevel::High,
                description: "Stale entity count exceeds freshness threshold".into(),
                location: Some("neo_knowledge_graph::maintenance".into()),
                metrics: [("stale_ratio".into(), 0.12)].into(),
            },
        ],
        SubsystemTarget::Reasoning => vec![
            Finding {
                category: ImprovementCategory::Performance,
                severity: RiskLevel::High,
                description: "Inference chain depth causes exponential blow-up".into(),
                location: Some("neo_reasoning::inference".into()),
                metrics: [("avg_depth".into(), 8.0), ("max_depth".into(), 22.0)].into(),
            },
            Finding {
                category: ImprovementCategory::CodeQuality,
                severity: RiskLevel::Medium,
                description: "Heuristic scoring functions lack calibration data".into(),
                location: Some("neo_reasoning::heuristics".into()),
                metrics: empty_metrics(),
            },
        ],
        SubsystemTarget::Workflows => vec![
            Finding {
                category: ImprovementCategory::Performance,
                severity: RiskLevel::Medium,
                description: "Synchronous barriers block parallel branches".into(),
                location: Some("neo_workflows::executor".into()),
                metrics: [("blocked_pct".into(), 0.28)].into(),
            },
            Finding {
                category: ImprovementCategory::Architecture,
                severity: RiskLevel::Low,
                description: "Several workflow fragments are duplicated across pipelines".into(),
                location: Some("neo_workflows::definitions".into()),
                metrics: empty_metrics(),
            },
        ],
        SubsystemTarget::Distributed => vec![
            Finding {
                category: ImprovementCategory::Reliability,
                severity: RiskLevel::Critical,
                description: "Node failure detection timeout is too aggressive".into(),
                location: Some("neo_distributed::membership".into()),
                metrics: [
                    ("timeout_ms".into(), 500.0),
                    ("recommended_ms".into(), 2000.0),
                ]
                .into(),
            },
            Finding {
                category: ImprovementCategory::Scalability,
                severity: RiskLevel::High,
                description: "Consistent-hash ring rebalances cause traffic spikes".into(),
                location: Some("neo_distributed::sharding".into()),
                metrics: empty_metrics(),
            },
        ],
        SubsystemTarget::Capabilities => vec![
            Finding {
                category: ImprovementCategory::Architecture,
                severity: RiskLevel::Medium,
                description: "Capability registry has no versioning support".into(),
                location: Some("neo_capabilities::registry".into()),
                metrics: empty_metrics(),
            },
            Finding {
                category: ImprovementCategory::CodeQuality,
                severity: RiskLevel::Low,
                description: "Several registered capabilities are never invoked".into(),
                location: Some("neo_capabilities::usage".into()),
                metrics: [("unused_pct".into(), 0.22)].into(),
            },
        ],
        SubsystemTarget::Executive => vec![
            Finding {
                category: ImprovementCategory::Reliability,
                severity: RiskLevel::Medium,
                description: "Oversight heartbeat monitoring not resilient to transient failures"
                    .into(),
                location: Some("neo_executive::heartbeat".into()),
                metrics: empty_metrics(),
            },
            Finding {
                category: ImprovementCategory::Security,
                severity: RiskLevel::High,
                description: "Audit log does not capture all privilege escalation events".into(),
                location: Some("neo_executive::audit".into()),
                metrics: empty_metrics(),
            },
        ],
        SubsystemTarget::Learning => vec![
            Finding {
                category: ImprovementCategory::Performance,
                severity: RiskLevel::Medium,
                description: "Model update batch size not tuned for current throughput".into(),
                location: Some("neo_learning::trainer".into()),
                metrics: [
                    ("current_batch".into(), 32.0),
                    ("optimal_batch".into(), 128.0),
                ]
                .into(),
            },
            Finding {
                category: ImprovementCategory::Scalability,
                severity: RiskLevel::Low,
                description: "Gradient synchronisation is single-node bottleneck".into(),
                location: Some("neo_learning::distributed".into()),
                metrics: empty_metrics(),
            },
        ],
        SubsystemTarget::Tools => vec![
            Finding {
                category: ImprovementCategory::Security,
                severity: RiskLevel::High,
                description: "Tool invocation does not enforce least-privilege sandboxing".into(),
                location: Some("neo_tools::executor".into()),
                metrics: empty_metrics(),
            },
            Finding {
                category: ImprovementCategory::CodeQuality,
                severity: RiskLevel::Low,
                description: "Several tool wrappers lack structured error types".into(),
                location: Some("neo_tools::wrappers".into()),
                metrics: empty_metrics(),
            },
        ],
        SubsystemTarget::Runtime => vec![
            Finding {
                category: ImprovementCategory::Performance,
                severity: RiskLevel::Medium,
                description: "Tokio runtime worker count not aligned with CPU cores".into(),
                location: Some("neo_runtime::config".into()),
                metrics: [
                    ("configured_workers".into(), 4.0),
                    ("available_cores".into(), 16.0),
                ]
                .into(),
            },
            Finding {
                category: ImprovementCategory::Reliability,
                severity: RiskLevel::Medium,
                description: "Panic handler does not propagate structured diagnostics".into(),
                location: Some("neo_runtime::panic".into()),
                metrics: empty_metrics(),
            },
        ],
    }
}

fn compute_score(findings: &[Finding]) -> f64 {
    if findings.is_empty() {
        return 1.0;
    }

    let severity_weight = |r: &RiskLevel| -> f64 {
        match r {
            RiskLevel::None => 0.0,
            RiskLevel::Low => 0.05,
            RiskLevel::Medium => 0.15,
            RiskLevel::High => 0.30,
            RiskLevel::Critical => 0.50,
        }
    };

    let total_deduction: f64 = findings.iter().map(|f| severity_weight(&f.severity)).sum();
    (1.0 - total_deduction).clamp(0.0, 1.0)
}

fn derive_recommendations(findings: &[Finding]) -> Vec<String> {
    let mut recs = Vec::new();
    for finding in findings {
        match finding.category {
            ImprovementCategory::Performance => {
                recs.push(format!(
                    "Investigate performance in {}: {}",
                    finding.location.as_deref().unwrap_or("unknown"),
                    finding.description,
                ));
            }
            ImprovementCategory::Reliability => {
                recs.push(format!("Improve reliability: {}", finding.description,));
            }
            ImprovementCategory::Security => {
                recs.push(format!("Address security concern: {}", finding.description,));
            }
            ImprovementCategory::Architecture => {
                recs.push(format!("Refactor architecture: {}", finding.description,));
            }
            ImprovementCategory::CodeQuality => {
                recs.push(format!("Improve code quality: {}", finding.description,));
            }
            ImprovementCategory::ResourceEfficiency => {
                recs.push(format!("Optimise resource usage: {}", finding.description,));
            }
            ImprovementCategory::Scalability => {
                recs.push(format!("Improve scalability: {}", finding.description,));
            }
            ImprovementCategory::Latency => {
                recs.push(format!("Reduce latency: {}", finding.description,));
            }
            ImprovementCategory::Throughput => {
                recs.push(format!("Increase throughput: {}", finding.description,));
            }
            ImprovementCategory::DependencyManagement => {
                recs.push(format!("Clean up dependencies: {}", finding.description,));
            }
        }
    }
    recs
}
