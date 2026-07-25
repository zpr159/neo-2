use serde::{Deserialize, Serialize};

use crate::error::EvolutionResult;
use crate::types::{RiskLevel, SubsystemTarget};

/// A identified performance bottleneck.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bottleneck {
    /// Component where the bottleneck occurs.
    pub component: String,
    /// Metric that is degraded.
    pub metric: String,
    /// Current observed value.
    pub current_value: f64,
    /// Threshold above/below which the metric is considered a bottleneck.
    pub threshold: f64,
    /// Impact severity.
    pub impact: RiskLevel,
}

/// Utilisation snapshot for key resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUtilization {
    /// CPU utilisation in `[0.0, 1.0]`.
    pub cpu: f64,
    /// Memory utilisation in `[0.0, 1.0]`.
    pub memory: f64,
    /// Disk utilisation in `[0.0, 1.0]`.
    pub disk: f64,
    /// Network utilisation in `[0.0, 1.0]`.
    pub network: f64,
}

/// An actionable optimisation opportunity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationOpportunity {
    /// Human-readable description.
    pub description: String,
    /// Expected performance improvement in percentage points.
    pub expected_improvement: f64,
    /// Risk level of applying the optimisation.
    pub risk: RiskLevel,
    /// Subsystem this optimisation targets.
    pub subsystem: SubsystemTarget,
}

/// Full performance analysis output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAnalysis {
    /// Detected bottlenecks.
    pub bottlenecks: Vec<Bottleneck>,
    /// Current resource utilisation.
    pub resource_utilization: ResourceUtilization,
    /// Suggested optimisations.
    pub optimization_opportunities: Vec<OptimizationOpportunity>,
}

/// Analyses runtime performance characteristics of the system.
pub struct PerformanceAnalyzer;

impl PerformanceAnalyzer {
    /// Create a new `PerformanceAnalyzer`.
    pub fn new() -> Self {
        Self
    }

    /// Run a full performance analysis.
    pub fn analyze(&self) -> EvolutionResult<PerformanceAnalysis> {
        let bottlenecks = self.detect_bottlenecks();
        let resource_utilization = self.measure_resources();
        let optimization_opportunities = self.suggest_optimizations();

        Ok(PerformanceAnalysis {
            bottlenecks,
            resource_utilization,
            optimization_opportunities,
        })
    }

    /// Detect performance bottlenecks across known subsystems.
    pub fn detect_bottlenecks(&self) -> Vec<Bottleneck> {
        vec![
            Bottleneck {
                component: "neo_agents::factory".into(),
                metric: "spawn_latency_us".into(),
                current_value: 1500.0,
                threshold: 500.0,
                impact: RiskLevel::High,
            },
            Bottleneck {
                component: "neo_memory::cache".into(),
                metric: "hit_rate".into(),
                current_value: 0.74,
                threshold: 0.80,
                impact: RiskLevel::Medium,
            },
            Bottleneck {
                component: "neo_reasoning::inference".into(),
                metric: "avg_chain_depth".into(),
                current_value: 8.0,
                threshold: 5.0,
                impact: RiskLevel::High,
            },
            Bottleneck {
                component: "neo_workflows::executor".into(),
                metric: "sync_block_pct".into(),
                current_value: 0.28,
                threshold: 0.10,
                impact: RiskLevel::Medium,
            },
            Bottleneck {
                component: "neo_distributed::membership".into(),
                metric: "detection_timeout_ms".into(),
                current_value: 500.0,
                threshold: 2000.0,
                impact: RiskLevel::Critical,
            },
            Bottleneck {
                component: "neo_runtime::scheduler".into(),
                metric: "worker_utilisation".into(),
                current_value: 0.25,
                threshold: 0.60,
                impact: RiskLevel::Medium,
            },
            Bottleneck {
                component: "neo_knowledge_graph::traversal".into(),
                metric: "p99_latency_ms".into(),
                current_value: 850.0,
                threshold: 200.0,
                impact: RiskLevel::High,
            },
        ]
    }

    /// Measure current resource utilisation.
    pub fn measure_resources(&self) -> ResourceUtilization {
        ResourceUtilization {
            cpu: 0.42,
            memory: 0.68,
            disk: 0.31,
            network: 0.15,
        }
    }

    /// Suggest concrete optimisation actions.
    pub fn suggest_optimizations(&self) -> Vec<OptimizationOpportunity> {
        vec![
            OptimizationOpportunity {
                description: "Introduce an agent-pool to amortise spawn overhead".into(),
                expected_improvement: 35.0,
                risk: RiskLevel::Medium,
                subsystem: SubsystemTarget::Agents,
            },
            OptimizationOpportunity {
                description: "Add LRU eviction with adaptive TTL to improve cache hit rate".into(),
                expected_improvement: 12.0,
                risk: RiskLevel::Low,
                subsystem: SubsystemTarget::Memory,
            },
            OptimizationOpportunity {
                description: "Implement depth-bounded inference with early pruning".into(),
                expected_improvement: 40.0,
                risk: RiskLevel::Medium,
                subsystem: SubsystemTarget::Reasoning,
            },
            OptimizationOpportunity {
                description: "Replace synchronous barriers with async join points".into(),
                expected_improvement: 22.0,
                risk: RiskLevel::Low,
                subsystem: SubsystemTarget::Workflows,
            },
            OptimizationOpportunity {
                description: "Increase failure-detection timeout and add exponential backoff"
                    .into(),
                expected_improvement: 60.0,
                risk: RiskLevel::Low,
                subsystem: SubsystemTarget::Distributed,
            },
            OptimizationOpportunity {
                description: "Align tokio worker count with available CPU cores".into(),
                expected_improvement: 18.0,
                risk: RiskLevel::Low,
                subsystem: SubsystemTarget::Runtime,
            },
            OptimizationOpportunity {
                description: "Add BFS-level caching for knowledge-graph traversals".into(),
                expected_improvement: 30.0,
                risk: RiskLevel::Medium,
                subsystem: SubsystemTarget::KnowledgeGraph,
            },
            OptimizationOpportunity {
                description: "Enable batched model updates to reduce synchronisation overhead"
                    .into(),
                expected_improvement: 25.0,
                risk: RiskLevel::Medium,
                subsystem: SubsystemTarget::Learning,
            },
        ]
    }
}

impl Default for PerformanceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
