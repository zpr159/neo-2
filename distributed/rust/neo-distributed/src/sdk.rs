//! Fluent builder SDK for constructing distributed runtime components.

use std::time::Duration;

use crate::cluster::Cluster;
use crate::config::{
    ClusterConfiguration, DiscoveryConfiguration, DiscoveryMethod, ExecutionConfiguration,
    MemoryConfiguration, NetworkingConfiguration, SchedulerConfiguration, SecurityConfiguration,
    SchedulingPolicy,
};
use crate::discovery::DiscoveryService;
use crate::error::NeoResult;
use crate::execution::RemoteExecutionEngine;
use crate::failure::{FailureDetector, RecoveryCoordinator, RecoveryStrategy};
use crate::heartbeat::HeartbeatService;
use crate::memory::DistributedMemory;
use crate::node::NodeManager;
use crate::scheduler::DistributedScheduler;
use crate::types::{NodeCapabilities, NodeId};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// DistributedRuntime
// ---------------------------------------------------------------------------

/// The top-level entry point for the distributed runtime.
///
/// Construct via `DistributedRuntime::builder()` and call `.build()`.
pub struct DistributedRuntime {
    /// Cluster instance.
    pub cluster: Arc<Cluster>,
    /// Discovery service.
    pub discovery: Arc<DiscoveryService>,
    /// Heartbeat service.
    pub heartbeat: Arc<HeartbeatService>,
    /// Scheduler.
    pub scheduler: Arc<DistributedScheduler>,
    /// Execution engine.
    pub execution: Arc<RemoteExecutionEngine>,
    /// Failure detector.
    pub failure_detector: Arc<FailureDetector>,
    /// Recovery coordinator.
    pub recovery: Arc<RecoveryCoordinator>,
    /// Distributed memory.
    pub memory: Arc<DistributedMemory>,
}

impl DistributedRuntime {
    /// Create a builder for fluent construction.
    pub fn builder() -> DistributedRuntimeBuilder {
        DistributedRuntimeBuilder::default()
    }
}

impl std::fmt::Debug for DistributedRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DistributedRuntime")
            .field("cluster", &self.cluster)
            .field("nodes", &self.cluster.node_count())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// DistributedRuntimeBuilder
// ---------------------------------------------------------------------------

/// Fluent builder for `DistributedRuntime`.
#[derive(Default)]
pub struct DistributedRuntimeBuilder {
    config: Option<ClusterConfiguration>,
    enable_discovery: bool,
    enable_replication: bool,
    enable_failover: bool,
    enable_memory: bool,
    node_id: Option<NodeId>,
    heartbeat_interval: Option<Duration>,
    scheduling_policy: Option<SchedulingPolicy>,
}

impl DistributedRuntimeBuilder {
    /// Set the cluster configuration.
    pub fn config(mut self, config: ClusterConfiguration) -> Self {
        self.config = Some(config);
        self
    }

    /// Enable or disable service discovery.
    pub fn enable_discovery(mut self, enabled: bool) -> Self {
        self.enable_discovery = enabled;
        self
    }

    /// Enable or disable memory replication.
    pub fn enable_replication(mut self, enabled: bool) -> Self {
        self.enable_replication = enabled;
        self
    }

    /// Enable or disable failover.
    pub fn enable_failover(mut self, enabled: bool) -> Self {
        self.enable_failover = enabled;
        self
    }

    /// Enable or disable distributed memory.
    pub fn enable_memory(mut self, enabled: bool) -> Self {
        self.enable_memory = enabled;
        self
    }

    /// Set the node ID.
    pub fn node_id(mut self, id: NodeId) -> Self {
        self.node_id = Some(id);
        self
    }

    /// Set heartbeat interval.
    pub fn heartbeat_interval(mut self, interval: Duration) -> Self {
        self.heartbeat_interval = Some(interval);
        self
    }

    /// Set scheduling policy.
    pub fn scheduling_policy(mut self, policy: SchedulingPolicy) -> Self {
        self.scheduling_policy = Some(policy);
        self
    }

    /// Build the distributed runtime.
    pub fn build(self) -> NeoResult<DistributedRuntime> {
        let config = self.config.unwrap_or_default();
        let node_id = self.node_id.unwrap_or_else(NodeId::new);

        // Cluster.
        let cluster = Arc::new(Cluster::new(config.clone()));

        // Discovery.
        let discovery = Arc::new(DiscoveryService::new(config.discovery.clone()));

        // Heartbeat.
        let heartbeat = Arc::new(HeartbeatService::new(
            node_id,
            config.heartbeat_interval,
            config.heartbeat_timeout,
        ));

        // Scheduler.
        let mut sched_config = config.scheduler.clone();
        if let Some(policy) = self.scheduling_policy {
            sched_config.default_policy = policy;
        }
        let scheduler = Arc::new(DistributedScheduler::new(sched_config));

        // Execution.
        let execution = Arc::new(RemoteExecutionEngine::new(config.execution.clone()));

        // Recovery.
        let recovery_strategy = RecoveryStrategy {
            migrate_workloads: self.enable_failover,
            ..Default::default()
        };
        let recovery = Arc::new(RecoveryCoordinator::new(recovery_strategy));
        let failure_detector = Arc::new(FailureDetector::new(
            cluster.node_manager.clone(),
            recovery.clone(),
            config.heartbeat_interval,
        ));

        // Memory.
        let memory = Arc::new(DistributedMemory::new(config.memory.clone()));

        tracing::info!(
            cluster_name = %config.name,
            node_id = %node_id,
            discovery = self.enable_discovery,
            replication = self.enable_replication,
            failover = self.enable_failover,
            "distributed runtime built"
        );

        Ok(DistributedRuntime {
            cluster,
            discovery,
            heartbeat,
            scheduler,
            execution,
            failure_detector,
            recovery,
            memory,
        })
    }
}

// ---------------------------------------------------------------------------
// ClusterManagerBuilder
// ---------------------------------------------------------------------------

/// Builder for `ClusterManager` operations.
pub struct ClusterManagerBuilder {
    config: ClusterConfiguration,
}

impl ClusterManagerBuilder {
    pub fn new() -> Self {
        Self {
            config: ClusterConfiguration::default(),
        }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.config.name = name.into();
        self
    }

    pub fn min_nodes(mut self, n: usize) -> Self {
        self.config.min_nodes = n;
        self
    }

    pub fn max_nodes(mut self, n: usize) -> Self {
        self.config.max_nodes = n;
        self
    }

    pub fn build(self) -> Cluster {
        Cluster::new(self.config)
    }
}

impl Default for ClusterManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// DistributedSchedulerBuilder
// ---------------------------------------------------------------------------

/// Builder for `DistributedScheduler`.
pub struct DistributedSchedulerBuilder {
    config: SchedulerConfiguration,
}

impl DistributedSchedulerBuilder {
    pub fn new() -> Self {
        Self {
            config: SchedulerConfiguration::default(),
        }
    }

    pub fn policy(mut self, policy: SchedulingPolicy) -> Self {
        self.config.default_policy = policy;
        self
    }

    pub fn max_queue_depth(mut self, depth: usize) -> Self {
        self.config.max_queue_depth = depth;
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.default_timeout = timeout;
        self
    }

    pub fn build(self) -> DistributedScheduler {
        DistributedScheduler::new(self.config)
    }
}

impl Default for DistributedSchedulerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// NodeManagerBuilder
// ---------------------------------------------------------------------------

/// Builder for `NodeManager`.
pub struct NodeManagerBuilder {
    labels: std::collections::HashMap<String, String>,
    zone: Option<String>,
}

impl NodeManagerBuilder {
    pub fn new() -> Self {
        Self {
            labels: std::collections::HashMap::new(),
            zone: None,
        }
    }

    pub fn label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    pub fn zone(mut self, zone: impl Into<String>) -> Self {
        self.zone = Some(zone.into());
        self
    }

    pub fn build(self) -> NodeManager {
        NodeManager::new()
    }
}

impl Default for NodeManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_builder() {
        let config = ClusterConfiguration::testing();
        let runtime = DistributedRuntime::builder()
            .config(config)
            .enable_discovery(true)
            .enable_replication(true)
            .enable_failover(true)
            .build()
            .unwrap();

        assert_eq!(runtime.cluster.node_count(), 0);
        assert!(runtime.cluster.is_active() || runtime.cluster.state() == crate::types::ClusterState::Forming);
    }

    #[test]
    fn cluster_manager_builder() {
        let cluster = ClusterManagerBuilder::new()
            .name("my-cluster")
            .min_nodes(3)
            .max_nodes(50)
            .build();
        assert_eq!(cluster.config.read().name, "my-cluster");
        assert_eq!(cluster.config.read().min_nodes, 3);
    }

    #[test]
    fn scheduler_builder() {
        let scheduler = DistributedSchedulerBuilder::new()
            .policy(SchedulingPolicy::GpuPreferred)
            .max_queue_depth(5000)
            .build();
        assert_eq!(scheduler.node_count(), 0);
    }
}
