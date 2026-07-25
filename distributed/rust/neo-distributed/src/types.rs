//! Core shared types for the distributed runtime.

use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// NodeId
// ---------------------------------------------------------------------------

/// Unique identifier for a node in the cluster.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord,
)]
pub struct NodeId(pub Uuid);

impl NodeId {
    /// Generate a new random node identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a `NodeId` from an existing UUID.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Get the inner UUID.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }

    /// Get the short representation (first 8 hex chars).
    pub fn short(&self) -> String {
        self.0.simple().to_string()[..8].to_string()
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for NodeId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

// ---------------------------------------------------------------------------
// NodeState
// ---------------------------------------------------------------------------

/// Lifecycle state of a node within the cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeState {
    /// Node is joining the cluster.
    Joining,
    /// Node is initializing local services.
    Initializing,
    /// Node is ready to accept work.
    Ready,
    /// Node is busy executing tasks.
    Busy,
    /// Node is being drained of work.
    Draining,
    /// Node is gracefully leaving.
    Leaving,
    /// Node is temporarily offline.
    Offline,
    /// Node has failed.
    Failed,
    /// Node is recovering from a failure.
    Recovering,
}

impl NodeState {
    /// Check whether a transition to `target` is valid.
    pub fn can_transition_to(self, target: NodeState) -> bool {
        matches!(
            (self, target),
            (Self::Joining, Self::Initializing)
                | (Self::Joining, Self::Failed)
                | (Self::Joining, Self::Offline)
                | (Self::Initializing, Self::Ready)
                | (Self::Initializing, Self::Failed)
                | (Self::Ready, Self::Busy)
                | (Self::Ready, Self::Draining)
                | (Self::Ready, Self::Leaving)
                | (Self::Ready, Self::Failed)
                | (Self::Ready, Self::Offline)
                | (Self::Busy, Self::Ready)
                | (Self::Busy, Self::Draining)
                | (Self::Busy, Self::Failed)
                | (Self::Busy, Self::Offline)
                | (Self::Draining, Self::Leaving)
                | (Self::Draining, Self::Ready)
                | (Self::Draining, Self::Failed)
                | (Self::Leaving, Self::Offline)
                | (Self::Offline, Self::Joining)
                | (Self::Offline, Self::Recovering)
                | (Self::Offline, Self::Failed)
                | (Self::Failed, Self::Recovering)
                | (Self::Recovering, Self::Initializing)
                | (Self::Recovering, Self::Failed)
        )
    }
}

impl fmt::Display for NodeState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Joining => write!(f, "joining"),
            Self::Initializing => write!(f, "initializing"),
            Self::Ready => write!(f, "ready"),
            Self::Busy => write!(f, "busy"),
            Self::Draining => write!(f, "draining"),
            Self::Leaving => write!(f, "leaving"),
            Self::Offline => write!(f, "offline"),
            Self::Failed => write!(f, "failed"),
            Self::Recovering => write!(f, "recovering"),
        }
    }
}

impl Default for NodeState {
    fn default() -> Self {
        Self::Joining
    }
}

// ---------------------------------------------------------------------------
// ClusterState
// ---------------------------------------------------------------------------

/// State of the cluster as a whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClusterState {
    /// Cluster is forming (not enough nodes yet).
    Forming,
    /// Cluster is fully operational.
    Active,
    /// Cluster has degraded (below min nodes or partial failure).
    Degraded,
    /// Cluster is experiencing a network partition.
    Partitioned,
    /// Cluster is rebalancing workloads.
    Rebalancing,
    /// Cluster is shutting down.
    Dissolving,
    /// Cluster has been dissolved.
    Dissolved,
}

impl ClusterState {
    /// Check whether the cluster can accept operations.
    pub fn accepts_operations(self) -> bool {
        matches!(self, Self::Active | Self::Degraded | Self::Rebalancing)
    }
}

impl fmt::Display for ClusterState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Forming => write!(f, "forming"),
            Self::Active => write!(f, "active"),
            Self::Degraded => write!(f, "degraded"),
            Self::Partitioned => write!(f, "partitioned"),
            Self::Rebalancing => write!(f, "rebalancing"),
            Self::Dissolving => write!(f, "dissolving"),
            Self::Dissolved => write!(f, "dissolved"),
        }
    }
}

impl Default for ClusterState {
    fn default() -> Self {
        Self::Forming
    }
}

// ---------------------------------------------------------------------------
// NodeType
// ---------------------------------------------------------------------------

/// Specialization of a node within the cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType {
    /// General-purpose coordinator node.
    Coordinator,
    /// CPU worker node.
    CpuWorker,
    /// GPU worker node.
    GpuWorker,
    /// Edge device node.
    Edge,
    /// Cloud-hosted node.
    Cloud,
    /// Specialized inference node.
    Inference,
    /// Storage node.
    Storage,
    /// Gateway / load-balancer node.
    Gateway,
}

impl fmt::Display for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Coordinator => write!(f, "coordinator"),
            Self::CpuWorker => write!(f, "cpu_worker"),
            Self::GpuWorker => write!(f, "gpu_worker"),
            Self::Edge => write!(f, "edge"),
            Self::Cloud => write!(f, "cloud"),
            Self::Inference => write!(f, "inference"),
            Self::Storage => write!(f, "storage"),
            Self::Gateway => write!(f, "gateway"),
        }
    }
}

// ---------------------------------------------------------------------------
// NodeCapabilities
// ---------------------------------------------------------------------------

/// Capabilities advertised by a node.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeCapabilities {
    /// CPU cores available.
    pub cpu_cores: u32,
    /// GPU devices available.
    pub gpu_count: u32,
    /// GPU model names (e.g. "A100", "H100").
    pub gpu_models: Vec<String>,
    /// Available RAM in bytes.
    pub memory_bytes: u64,
    /// Available disk in bytes.
    pub disk_bytes: u64,
    /// Network bandwidth in bits per second.
    pub network_bps: u64,
    /// Named capabilities (e.g. "inference", "ocr", "speech").
    pub capabilities: Vec<String>,
    /// Custom labels for filtering.
    pub labels: HashMap<String, String>,
    /// Maximum concurrent tasks the node can handle.
    pub max_concurrent_tasks: u32,
    /// Supported execution environments.
    pub supported_environments: Vec<String>,
}

impl NodeCapabilities {
    /// Returns `true` if the node has GPU resources.
    pub fn has_gpu(&self) -> bool {
        self.gpu_count > 0
    }

    /// Returns `true` if the node supports the named capability.
    pub fn supports_capability(&self, cap: &str) -> bool {
        self.capabilities.iter().any(|c| c == cap)
    }

    /// Returns `true` if the node matches all required labels.
    pub fn matches_labels(&self, required: &HashMap<String, String>) -> bool {
        required
            .iter()
            .all(|(k, v)| self.labels.get(k).map_or(false, |val| val == v))
    }
}

// ---------------------------------------------------------------------------
// NodeResources
// ---------------------------------------------------------------------------

/// Current resource utilization of a node.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeResources {
    /// CPU utilization 0.0 – 1.0.
    pub cpu_utilization: f32,
    /// GPU utilization 0.0 – 1.0.
    pub gpu_utilization: f32,
    /// Memory utilization 0.0 – 1.0.
    pub memory_utilization: f32,
    /// Disk utilization 0.0 – 1.0.
    pub disk_utilization: f32,
    /// Network utilization 0.0 – 1.0.
    pub network_utilization: f32,
    /// Power usage in watts (0 = unknown).
    pub power_watts: f32,
    /// Temperature in Celsius (0.0 = unknown).
    pub temperature_celsius: f32,
    /// Number of currently running tasks.
    pub active_tasks: u32,
    /// Number of queued tasks.
    pub queued_tasks: u32,
}

impl NodeResources {
    /// Composite load score 0.0 (idle) – 1.0 (fully loaded).
    pub fn load_score(&self) -> f32 {
        let cpu = self.cpu_utilization;
        let mem = self.memory_utilization;
        let gpu = if self.gpu_utilization > 0.0 {
            self.gpu_utilization
        } else {
            0.0
        };
        let task_score = if self.active_tasks > 0 {
            (self.active_tasks as f32 / 32.0).min(1.0)
        } else {
            0.0
        };
        // Weighted average.
        (cpu * 0.35 + gpu * 0.30 + mem * 0.20 + task_score * 0.15).min(1.0)
    }
}

// ---------------------------------------------------------------------------
// NodeInfo
// ---------------------------------------------------------------------------

/// Static identity information about a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Hostname.
    pub hostname: String,
    /// IP address.
    pub ip_address: String,
    /// Port for inter-node communication.
    pub port: u16,
    /// Node type / specialization.
    pub node_type: NodeType,
    /// Capabilities.
    pub capabilities: NodeCapabilities,
    /// Software version.
    pub version: String,
    /// Zone / availability-zone for locality-aware scheduling.
    pub zone: String,
    /// Rack identifier.
    pub rack: Option<String>,
}

// ---------------------------------------------------------------------------
// NodeHealth
// ---------------------------------------------------------------------------

/// Health report for a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeHealth {
    /// Overall health score 0.0 (dead) – 1.0 (perfect).
    pub score: f32,
    /// Current state.
    pub state: NodeState,
    /// Timestamp of last heartbeat received.
    pub last_heartbeat: DateTime<Utc>,
    /// Measured latency to this node in milliseconds.
    pub latency_ms: f64,
    /// Estimated clock drift in microseconds.
    pub clock_drift_us: i64,
    /// Whether the node is responsive.
    pub responsive: bool,
    /// Any health warnings.
    pub warnings: Vec<String>,
}

impl NodeHealth {
    /// Returns `true` if the node is considered healthy.
    pub fn is_healthy(&self) -> bool {
        self.score > 0.5 && self.responsive
    }
}

// ---------------------------------------------------------------------------
// TaskPriority
// ---------------------------------------------------------------------------

/// Priority level for distributed tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TaskPriority(pub u8);

impl TaskPriority {
    pub const CRITICAL: Self = Self(0);
    pub const HIGH: Self = Self(1);
    pub const NORMAL: Self = Self(5);
    pub const LOW: Self = Self(8);
    pub const BACKGROUND: Self = Self(10);

    pub fn is_critical(self) -> bool {
        self.0 <= 2
    }
}

impl Default for TaskPriority {
    fn default() -> Self {
        Self::NORMAL
    }
}

impl fmt::Display for TaskPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            0..=2 => write!(f, "critical"),
            3..=4 => write!(f, "high"),
            5..=7 => write!(f, "normal"),
            8..=9 => write!(f, "low"),
            _ => write!(f, "background"),
        }
    }
}

// ---------------------------------------------------------------------------
// ClusterMetadata
// ---------------------------------------------------------------------------

/// Cluster-wide metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMetadata {
    /// Cluster name.
    pub name: String,
    /// Cluster unique identifier.
    pub id: Uuid,
    /// When the cluster was created.
    pub created_at: DateTime<Utc>,
    /// Current leader node.
    pub leader: Option<NodeId>,
    /// Current cluster state.
    pub state: ClusterState,
    /// Total number of nodes.
    pub node_count: usize,
    /// Number of healthy nodes.
    pub healthy_node_count: usize,
    /// Software version running on the cluster.
    pub version: String,
}

// ---------------------------------------------------------------------------
// Duration helpers
// ---------------------------------------------------------------------------

/// Serialize/deserialize `Duration` as milliseconds.
pub fn duration_ms(duration: Duration) -> u64 {
    duration.as_millis() as u64
}

/// Create a `Duration` from milliseconds.
pub fn ms_to_duration(ms: u64) -> Duration {
    Duration::from_millis(ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_id_short() {
        let id = NodeId::new();
        assert_eq!(id.short().len(), 8);
    }

    #[test]
    fn node_state_transitions() {
        assert!(NodeState::Joining.can_transition_to(NodeState::Initializing));
        assert!(NodeState::Initializing.can_transition_to(NodeState::Ready));
        assert!(NodeState::Ready.can_transition_to(NodeState::Busy));
        assert!(NodeState::Busy.can_transition_to(NodeState::Ready));
        assert!(NodeState::Ready.can_transition_to(NodeState::Failed));
        assert!(NodeState::Failed.can_transition_to(NodeState::Recovering));
        assert!(!NodeState::Failed.can_transition_to(NodeState::Ready));
        assert!(!NodeState::Dissolved.can_transition_to(NodeState::Active));
    }

    #[test]
    fn cluster_state_operations() {
        assert!(ClusterState::Active.accepts_operations());
        assert!(ClusterState::Degraded.accepts_operations());
        assert!(!ClusterState::Forming.accepts_operations());
        assert!(!ClusterState::Dissolved.accepts_operations());
    }

    #[test]
    fn node_capabilities() {
        let caps = NodeCapabilities {
            gpu_count: 4,
            capabilities: vec!["inference".to_string(), "ocr".to_string()],
            labels: HashMap::from([("zone".to_string(), "us-east-1".to_string())]),
            ..Default::default()
        };
        assert!(caps.has_gpu());
        assert!(caps.supports_capability("inference"));
        assert!(!caps.supports_capability("speech"));
        assert!(caps.matches_labels(&HashMap::from([(
            "zone".to_string(),
            "us-east-1".to_string()
        )])));
        assert!(!caps.matches_labels(&HashMap::from([(
            "zone".to_string(),
            "eu-west-1".to_string()
        )])));
    }

    #[test]
    fn node_resources_load_score() {
        let mut res = NodeResources::default();
        res.cpu_utilization = 0.5;
        let score = res.load_score();
        assert!(score > 0.0 && score <= 1.0);
    }

    #[test]
    fn task_priority_ordering() {
        assert!(TaskPriority::CRITICAL < TaskPriority::HIGH);
        assert!(TaskPriority::HIGH < TaskPriority::NORMAL);
        assert!(TaskPriority::NORMAL < TaskPriority::LOW);
        assert!(TaskPriority::LOW < TaskPriority::BACKGROUND);
        assert!(TaskPriority::CRITICAL.is_critical());
        assert!(!TaskPriority::NORMAL.is_critical());
    }

    #[test]
    fn node_health_check() {
        let healthy = NodeHealth {
            score: 0.9,
            state: NodeState::Ready,
            last_heartbeat: Utc::now(),
            latency_ms: 1.0,
            clock_drift_us: 0,
            responsive: true,
            warnings: vec![],
        };
        assert!(healthy.is_healthy());

        let unhealthy = NodeHealth {
            score: 0.2,
            responsive: false,
            ..healthy
        };
        assert!(!unhealthy.is_healthy());
    }
}
