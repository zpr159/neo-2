//! Configuration types for the distributed runtime.

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::types::NodeType;

// ---------------------------------------------------------------------------
// ClusterConfiguration
// ---------------------------------------------------------------------------

/// Top-level configuration for the distributed runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfiguration {
    /// Cluster name.
    pub name: String,
    /// Minimum number of nodes to maintain quorum.
    pub min_nodes: usize,
    /// Maximum number of nodes allowed.
    pub max_nodes: usize,
    /// Heartbeat interval.
    pub heartbeat_interval: Duration,
    /// Heartbeat timeout before marking a node suspect.
    pub heartbeat_timeout: Duration,
    /// Gossip protocol interval.
    pub gossip_interval: Duration,
    /// Leader election timeout.
    pub election_timeout: Duration,
    /// Node configuration.
    pub node: NodeConfiguration,
    /// Discovery configuration.
    pub discovery: DiscoveryConfiguration,
    /// Scheduler configuration.
    pub scheduler: SchedulerConfiguration,
    /// Execution configuration.
    pub execution: ExecutionConfiguration,
    /// Networking configuration.
    pub networking: NetworkingConfiguration,
    /// Security configuration.
    pub security: SecurityConfiguration,
    /// Memory configuration.
    pub memory: MemoryConfiguration,
    /// Event bus configuration.
    pub event_bus: EventBusConfiguration,
    /// Storage configuration.
    pub storage: StorageConfiguration,
    /// Monitoring configuration.
    pub monitoring: MonitoringConfiguration,
}

impl Default for ClusterConfiguration {
    fn default() -> Self {
        Self {
            name: "neo-cluster".to_string(),
            min_nodes: 1,
            max_nodes: 128,
            heartbeat_interval: Duration::from_secs(1),
            heartbeat_timeout: Duration::from_secs(5),
            gossip_interval: Duration::from_secs(2),
            election_timeout: Duration::from_secs(5),
            node: NodeConfiguration::default(),
            discovery: DiscoveryConfiguration::default(),
            scheduler: SchedulerConfiguration::default(),
            execution: ExecutionConfiguration::default(),
            networking: NetworkingConfiguration::default(),
            security: SecurityConfiguration::default(),
            memory: MemoryConfiguration::default(),
            event_bus: EventBusConfiguration::default(),
            storage: StorageConfiguration::default(),
            monitoring: MonitoringConfiguration::default(),
        }
    }
}

impl ClusterConfiguration {
    /// Create a builder for fluent configuration.
    pub fn builder() -> ClusterConfigurationBuilder {
        ClusterConfigurationBuilder::default()
    }

    /// Production-grade defaults.
    pub fn production() -> Self {
        Self {
            min_nodes: 3,
            max_nodes: 256,
            heartbeat_interval: Duration::from_secs(1),
            heartbeat_timeout: Duration::from_secs(3),
            gossip_interval: Duration::from_secs(1),
            election_timeout: Duration::from_secs(3),
            security: SecurityConfiguration {
                enabled: true,
                mtls_enabled: true,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Testing / single-node defaults.
    pub fn testing() -> Self {
        Self {
            min_nodes: 1,
            max_nodes: 1,
            heartbeat_interval: Duration::from_millis(100),
            heartbeat_timeout: Duration::from_millis(500),
            gossip_interval: Duration::from_millis(200),
            election_timeout: Duration::from_millis(500),
            security: SecurityConfiguration {
                enabled: false,
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// ClusterConfigurationBuilder
// ---------------------------------------------------------------------------

/// Fluent builder for `ClusterConfiguration`.
#[derive(Debug, Default)]
pub struct ClusterConfigurationBuilder {
    name: Option<String>,
    min_nodes: Option<usize>,
    max_nodes: Option<usize>,
    heartbeat_interval: Option<Duration>,
    enable_discovery: Option<bool>,
    enable_replication: Option<bool>,
    enable_failover: Option<bool>,
    enable_security: Option<bool>,
}

impl ClusterConfigurationBuilder {
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn min_nodes(mut self, n: usize) -> Self {
        self.min_nodes = Some(n);
        self
    }

    pub fn max_nodes(mut self, n: usize) -> Self {
        self.max_nodes = Some(n);
        self
    }

    pub fn heartbeat_interval(mut self, interval: Duration) -> Self {
        self.heartbeat_interval = Some(interval);
        self
    }

    pub fn enable_discovery(mut self, enabled: bool) -> Self {
        self.enable_discovery = Some(enabled);
        self
    }

    pub fn enable_replication(mut self, enabled: bool) -> Self {
        self.enable_replication = Some(enabled);
        self
    }

    pub fn enable_failover(mut self, enabled: bool) -> Self {
        self.enable_failover = Some(enabled);
        self
    }

    pub fn enable_security(mut self, enabled: bool) -> Self {
        self.enable_security = Some(enabled);
        self
    }

    pub fn build(self) -> ClusterConfiguration {
        let mut config = ClusterConfiguration::default();
        if let Some(name) = self.name {
            config.name = name;
        }
        if let Some(min) = self.min_nodes {
            config.min_nodes = min;
        }
        if let Some(max) = self.max_nodes {
            config.max_nodes = max;
        }
        if let Some(interval) = self.heartbeat_interval {
            config.heartbeat_interval = interval;
        }
        if let Some(enabled) = self.enable_security {
            config.security.enabled = enabled;
            config.security.mtls_enabled = enabled;
        }
        config
    }
}

// ---------------------------------------------------------------------------
// NodeConfiguration
// ---------------------------------------------------------------------------

/// Configuration for individual node behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfiguration {
    /// Node type.
    pub node_type: NodeType,
    /// Hostname override (auto-detected if empty).
    pub hostname: Option<String>,
    /// Bind address.
    pub bind_address: String,
    /// Port for inter-node communication.
    pub port: u16,
    /// Maximum concurrent tasks.
    pub max_concurrent_tasks: u32,
    /// Node zone for locality-aware scheduling.
    pub zone: String,
    /// Custom labels.
    pub labels: HashMap<String, String>,
}

impl Default for NodeConfiguration {
    fn default() -> Self {
        Self {
            node_type: NodeType::CpuWorker,
            hostname: None,
            bind_address: "0.0.0.0".to_string(),
            port: 7400,
            max_concurrent_tasks: 64,
            zone: "default".to_string(),
            labels: HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// DiscoveryConfiguration
// ---------------------------------------------------------------------------

/// Configuration for service discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfiguration {
    /// Discovery method.
    pub method: DiscoveryMethod,
    /// Static peer addresses (for static discovery).
    pub static_peers: Vec<String>,
    /// DNS domain (for DNS discovery).
    pub dns_domain: Option<String>,
    /// Multicast address.
    pub multicast_address: Option<String>,
    /// Multicast port.
    pub multicast_port: Option<u16>,
    /// Kubernetes namespace (for k8s discovery).
    pub k8s_namespace: Option<String>,
    /// Kubernetes service name.
    pub k8s_service: Option<String>,
    /// Bootstrap node addresses.
    pub bootstrap_nodes: Vec<String>,
}

impl Default for DiscoveryConfiguration {
    fn default() -> Self {
        Self {
            method: DiscoveryMethod::Static,
            static_peers: Vec::new(),
            dns_domain: None,
            multicast_address: None,
            multicast_port: None,
            k8s_namespace: None,
            k8s_service: None,
            bootstrap_nodes: Vec::new(),
        }
    }
}

/// Available discovery methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiscoveryMethod {
    /// Static peer list.
    Static,
    /// UDP multicast.
    Multicast,
    /// DNS SRV records.
    Dns,
    /// Kubernetes endpoint API.
    Kubernetes,
    /// Manual registration only.
    Manual,
}

impl std::fmt::Display for DiscoveryMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Static => write!(f, "static"),
            Self::Multicast => write!(f, "multicast"),
            Self::Dns => write!(f, "dns"),
            Self::Kubernetes => write!(f, "kubernetes"),
            Self::Manual => write!(f, "manual"),
        }
    }
}

// ---------------------------------------------------------------------------
// SchedulerConfiguration
// ---------------------------------------------------------------------------

/// Configuration for the distributed scheduler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfiguration {
    /// Default scheduling policy.
    pub default_policy: SchedulingPolicy,
    /// Maximum queue depth.
    pub max_queue_depth: usize,
    /// Default task timeout.
    pub default_timeout: Duration,
    /// Maximum retries per task.
    pub max_retries: u32,
    /// Base retry delay.
    pub retry_base_delay: Duration,
    /// Maximum retry delay.
    pub retry_max_delay: Duration,
    /// Enable load balancing.
    pub load_balancing_enabled: bool,
    /// Rebalance interval.
    pub rebalance_interval: Duration,
}

impl Default for SchedulerConfiguration {
    fn default() -> Self {
        Self {
            default_policy: SchedulingPolicy::LeastLoaded,
            max_queue_depth: 10_000,
            default_timeout: Duration::from_secs(60),
            max_retries: 3,
            retry_base_delay: Duration::from_millis(100),
            retry_max_delay: Duration::from_secs(10),
            load_balancing_enabled: true,
            rebalance_interval: Duration::from_secs(30),
        }
    }
}

/// Scheduling policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SchedulingPolicy {
    /// Assign to the least loaded node.
    LeastLoaded,
    /// Prefer nodes in the same zone / locality.
    LocalityAware,
    /// Match required capabilities.
    CapabilityAware,
    /// Prefer GPU-capable nodes.
    GpuPreferred,
    /// Prefer CPU-only nodes.
    CpuPreferred,
    /// Optimize for low latency.
    LatencyOptimized,
    /// Optimize for memory availability.
    MemoryOptimized,
    /// Round-robin across nodes.
    RoundRobin,
    /// Random assignment.
    Random,
}

impl std::fmt::Display for SchedulingPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LeastLoaded => write!(f, "least_loaded"),
            Self::LocalityAware => write!(f, "locality_aware"),
            Self::CapabilityAware => write!(f, "capability_aware"),
            Self::GpuPreferred => write!(f, "gpu_preferred"),
            Self::CpuPreferred => write!(f, "cpu_preferred"),
            Self::LatencyOptimized => write!(f, "latency_optimized"),
            Self::MemoryOptimized => write!(f, "memory_optimized"),
            Self::RoundRobin => write!(f, "round_robin"),
            Self::Random => write!(f, "random"),
        }
    }
}

// ---------------------------------------------------------------------------
// ExecutionConfiguration
// ---------------------------------------------------------------------------

/// Configuration for distributed execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfiguration {
    /// Maximum execution lease duration.
    pub lease_duration: Duration,
    /// Execution queue capacity.
    pub queue_capacity: usize,
    /// Maximum concurrent remote executions.
    pub max_concurrent: u32,
    /// Enable execution checkpointing.
    pub checkpointing_enabled: bool,
    /// Checkpoint interval.
    pub checkpoint_interval: Duration,
}

impl Default for ExecutionConfiguration {
    fn default() -> Self {
        Self {
            lease_duration: Duration::from_secs(300),
            queue_capacity: 1024,
            max_concurrent: 256,
            checkpointing_enabled: true,
            checkpoint_interval: Duration::from_secs(60),
        }
    }
}

// ---------------------------------------------------------------------------
// NetworkingConfiguration
// ---------------------------------------------------------------------------

/// Configuration for the networking layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkingConfiguration {
    /// Transport protocol.
    pub transport: TransportProtocol,
    /// Enable compression.
    pub compression_enabled: bool,
    /// Maximum message size in bytes.
    pub max_message_size: usize,
    /// Connection timeout.
    pub connection_timeout: Duration,
    /// Read timeout.
    pub read_timeout: Duration,
    /// Write timeout.
    pub write_timeout: Duration,
    /// Number of worker threads for networking.
    pub worker_threads: usize,
    /// Enable TCP keepalive.
    pub tcp_keepalive: bool,
    /// TCP keepalive interval.
    pub keepalive_interval: Duration,
}

impl Default for NetworkingConfiguration {
    fn default() -> Self {
        Self {
            transport: TransportProtocol::Tcp,
            compression_enabled: false,
            max_message_size: 16 * 1024 * 1024, // 16 MB
            connection_timeout: Duration::from_secs(5),
            read_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(30),
            worker_threads: 4,
            tcp_keepalive: true,
            keepalive_interval: Duration::from_secs(30),
        }
    }
}

/// Available transport protocols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransportProtocol {
    Tcp,
    Tls,
    Quic,
    WebSocket,
    Http2,
    Grpc,
}

impl std::fmt::Display for TransportProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tcp => write!(f, "tcp"),
            Self::Tls => write!(f, "tls"),
            Self::Quic => write!(f, "quic"),
            Self::WebSocket => write!(f, "websocket"),
            Self::Http2 => write!(f, "http2"),
            Self::Grpc => write!(f, "grpc"),
        }
    }
}

// ---------------------------------------------------------------------------
// SecurityConfiguration
// ---------------------------------------------------------------------------

/// Configuration for cluster security.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfiguration {
    /// Enable security features.
    pub enabled: bool,
    /// Enable mutual TLS.
    pub mtls_enabled: bool,
    /// Path to TLS certificate.
    pub cert_path: Option<String>,
    /// Path to TLS private key.
    pub key_path: Option<String>,
    /// Path to CA certificate.
    pub ca_path: Option<String>,
    /// Enable automatic key rotation.
    pub key_rotation_enabled: bool,
    /// Key rotation interval.
    pub key_rotation_interval: Duration,
    /// Enable signed messages.
    pub signed_messages: bool,
    /// Authorization mode.
    pub auth_mode: AuthMode,
}

impl Default for SecurityConfiguration {
    fn default() -> Self {
        Self {
            enabled: false,
            mtls_enabled: false,
            cert_path: None,
            key_path: None,
            ca_path: None,
            key_rotation_enabled: false,
            key_rotation_interval: Duration::from_secs(3600),
            signed_messages: false,
            auth_mode: AuthMode::None,
        }
    }
}

/// Authorization mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuthMode {
    None,
    Token,
    Certificate,
    Mtls,
}

// ---------------------------------------------------------------------------
// MemoryConfiguration
// ---------------------------------------------------------------------------

/// Configuration for distributed memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfiguration {
    /// Replication factor.
    pub replication_factor: usize,
    /// Consistency mode.
    pub consistency: ConsistencyMode,
    /// Cache size in bytes.
    pub cache_size_bytes: usize,
    /// Enable memory snapshots.
    pub snapshot_enabled: bool,
    /// Snapshot interval.
    pub snapshot_interval: Duration,
    /// Maximum partition count.
    pub max_partitions: usize,
}

impl Default for MemoryConfiguration {
    fn default() -> Self {
        Self {
            replication_factor: 3,
            consistency: ConsistencyMode::Quorum,
            cache_size_bytes: 256 * 1024 * 1024,
            snapshot_enabled: true,
            snapshot_interval: Duration::from_secs(300),
            max_partitions: 16,
        }
    }
}

/// Memory consistency mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConsistencyMode {
    /// Strong consistency (linearizable reads).
    Strong,
    /// Eventual consistency.
    Eventual,
    /// Quorum-based (majority must agree).
    Quorum,
}

// ---------------------------------------------------------------------------
// EventBusConfiguration
// ---------------------------------------------------------------------------

/// Configuration for the distributed event bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBusConfiguration {
    /// Event queue capacity.
    pub queue_capacity: usize,
    /// Maximum subscribers per topic.
    pub max_subscribers_per_topic: usize,
    /// Enable event replay.
    pub replay_enabled: bool,
    /// Event retention duration.
    pub retention_duration: Duration,
    /// Delivery guarantee.
    pub delivery_guarantee: DeliveryGuarantee,
    /// Enable event filtering.
    pub filtering_enabled: bool,
}

impl Default for EventBusConfiguration {
    fn default() -> Self {
        Self {
            queue_capacity: 10_000,
            max_subscribers_per_topic: 64,
            replay_enabled: true,
            retention_duration: Duration::from_secs(3600),
            delivery_guarantee: DeliveryGuarantee::AtLeastOnce,
            filtering_enabled: true,
        }
    }
}

/// Delivery guarantee for events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeliveryGuarantee {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}

// ---------------------------------------------------------------------------
// StorageConfiguration
// ---------------------------------------------------------------------------

/// Configuration for cluster storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfiguration {
    /// Storage backend.
    pub backend: StorageBackend,
    /// Data directory.
    pub data_dir: String,
    /// Enable compression.
    pub compression_enabled: bool,
    /// Checkpoint interval.
    pub checkpoint_interval: Duration,
    /// Maximum storage size in bytes.
    pub max_size_bytes: u64,
}

impl Default for StorageConfiguration {
    fn default() -> Self {
        Self {
            backend: StorageBackend::Embedded,
            data_dir: "./data/distributed".to_string(),
            compression_enabled: true,
            checkpoint_interval: Duration::from_secs(60),
            max_size_bytes: 1024 * 1024 * 1024 * 10, // 10 GB
        }
    }
}

/// Storage backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StorageBackend {
    Embedded,
    Sled,
    Sqlite,
    Redis,
    S3,
}

// ---------------------------------------------------------------------------
// MonitoringConfiguration
// ---------------------------------------------------------------------------

/// Configuration for cluster monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfiguration {
    /// Enable monitoring.
    pub enabled: bool,
    /// Metrics collection interval.
    pub metrics_interval: Duration,
    /// Enable network analytics.
    pub network_analytics: bool,
    /// Enable performance analytics.
    pub performance_analytics: bool,
    /// Enable scheduling analytics.
    pub scheduling_analytics: bool,
    /// Metrics retention duration.
    pub retention_duration: Duration,
}

impl Default for MonitoringConfiguration {
    fn default() -> Self {
        Self {
            enabled: true,
            metrics_interval: Duration::from_secs(10),
            network_analytics: true,
            performance_analytics: true,
            scheduling_analytics: true,
            retention_duration: Duration::from_secs(86400),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_cluster_config() {
        let config = ClusterConfiguration::default();
        assert_eq!(config.name, "neo-cluster");
        assert_eq!(config.min_nodes, 1);
        assert_eq!(config.max_nodes, 128);
    }

    #[test]
    fn production_config() {
        let config = ClusterConfiguration::production();
        assert_eq!(config.min_nodes, 3);
        assert!(config.security.enabled);
    }

    #[test]
    fn testing_config() {
        let config = ClusterConfiguration::testing();
        assert_eq!(config.min_nodes, 1);
        assert!(!config.security.enabled);
    }

    #[test]
    fn builder_pattern() {
        let config = ClusterConfiguration::builder()
            .name("my-cluster")
            .min_nodes(5)
            .max_nodes(50)
            .enable_security(true)
            .build();
        assert_eq!(config.name, "my-cluster");
        assert_eq!(config.min_nodes, 5);
        assert_eq!(config.max_nodes, 50);
        assert!(config.security.enabled);
    }

    #[test]
    fn scheduling_policy_display() {
        assert_eq!(
            SchedulingPolicy::LeastLoaded.to_string(),
            "least_loaded"
        );
        assert_eq!(
            SchedulingPolicy::GpuPreferred.to_string(),
            "gpu_preferred"
        );
    }

    #[test]
    fn transport_protocol_display() {
        assert_eq!(TransportProtocol::Tcp.to_string(), "tcp");
        assert_eq!(TransportProtocol::Quic.to_string(), "quic");
    }
}
