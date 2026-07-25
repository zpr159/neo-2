use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// A globally unique identifier for evolution entities.
pub type EvolutionId = uuid::Uuid;

/// Subsystems that can be targeted for evolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubsystemTarget {
    /// Core infrastructure.
    Core,
    /// Agent framework.
    Agents,
    /// Task planning.
    Planning,
    /// Memory subsystem.
    Memory,
    /// Knowledge graph.
    KnowledgeGraph,
    /// Reasoning engine.
    Reasoning,
    /// Workflow orchestration.
    Workflows,
    /// Distributed coordination.
    Distributed,
    /// Capability registry.
    Capabilities,
    /// Executive oversight.
    Executive,
    /// Learning subsystem.
    Learning,
    /// Tool integrations.
    Tools,
    /// Runtime environment.
    Runtime,
}

impl fmt::Display for SubsystemTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Core => "core",
            Self::Agents => "agents",
            Self::Planning => "planning",
            Self::Memory => "memory",
            Self::KnowledgeGraph => "knowledge_graph",
            Self::Reasoning => "reasoning",
            Self::Workflows => "workflows",
            Self::Distributed => "distributed",
            Self::Capabilities => "capabilities",
            Self::Executive => "executive",
            Self::Learning => "learning",
            Self::Tools => "tools",
            Self::Runtime => "runtime",
        };
        write!(f, "{label}")
    }
}

impl FromStr for SubsystemTarget {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "core" => Ok(Self::Core),
            "agents" => Ok(Self::Agents),
            "planning" => Ok(Self::Planning),
            "memory" => Ok(Self::Memory),
            "knowledge_graph" | "knowledgegraph" | "knowledge" => Ok(Self::KnowledgeGraph),
            "reasoning" => Ok(Self::Reasoning),
            "workflows" | "workflow" => Ok(Self::Workflows),
            "distributed" => Ok(Self::Distributed),
            "capabilities" | "capability" => Ok(Self::Capabilities),
            "executive" => Ok(Self::Executive),
            "learning" => Ok(Self::Learning),
            "tools" => Ok(Self::Tools),
            "runtime" => Ok(Self::Runtime),
            _ => Err(format!("unknown subsystem: {s}")),
        }
    }
}

/// Categories of improvements that evolution can propose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImprovementCategory {
    /// Performance optimisation.
    Performance,
    /// Reliability and fault tolerance.
    Reliability,
    /// Security hardening.
    Security,
    /// Architectural restructuring.
    Architecture,
    /// Code quality and maintainability.
    CodeQuality,
    /// Resource efficiency.
    ResourceEfficiency,
    /// Scalability improvements.
    Scalability,
    /// Latency reduction.
    Latency,
    /// Throughput increase.
    Throughput,
    /// Dependency management.
    DependencyManagement,
}

impl fmt::Display for ImprovementCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Performance => "performance",
            Self::Reliability => "reliability",
            Self::Security => "security",
            Self::Architecture => "architecture",
            Self::CodeQuality => "code_quality",
            Self::ResourceEfficiency => "resource_efficiency",
            Self::Scalability => "scalability",
            Self::Latency => "latency",
            Self::Throughput => "throughput",
            Self::DependencyManagement => "dependency_management",
        };
        write!(f, "{label}")
    }
}

/// Status of an evolution cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvolutionStatus {
    /// Not yet started.
    Pending,
    /// Currently running.
    Running,
    /// Completed successfully.
    Completed,
    /// Failed.
    Failed,
    /// Cancelled by user or policy.
    Cancelled,
    /// Waiting for approval.
    AwaitingApproval,
    /// Rolling back a previous change.
    RollingBack,
}

impl fmt::Display for EvolutionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::AwaitingApproval => "awaiting_approval",
            Self::RollingBack => "rolling_back",
        };
        write!(f, "{label}")
    }
}

/// Risk level associated with a finding or proposed change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    /// No risk.
    None,
    /// Low risk — unlikely to cause issues.
    Low,
    /// Medium risk — may require mitigation.
    Medium,
    /// High risk — likely to cause issues without mitigation.
    High,
    /// Critical risk — immediate attention required.
    Critical,
}

impl fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::None => "none",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        };
        write!(f, "{label}")
    }
}

/// Phase of the evolution lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvolutionPhase {
    /// Self-analysis phase.
    Analysis,
    /// Planning improvements.
    Planning,
    /// Implementing changes.
    Implementation,
    /// Running tests.
    Testing,
    /// Deploying changes.
    Deployment,
    /// Monitoring post-deployment.
    Monitoring,
    /// Rolling back if needed.
    Rollback,
}

impl fmt::Display for EvolutionPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Analysis => "analysis",
            Self::Planning => "planning",
            Self::Implementation => "implementation",
            Self::Testing => "testing",
            Self::Deployment => "deployment",
            Self::Monitoring => "monitoring",
            Self::Rollback => "rollback",
        };
        write!(f, "{label}")
    }
}
