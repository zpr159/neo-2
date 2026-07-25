//! Cluster management — orchestrating multi-node clusters with full lifecycle,
//! state management, topology tracking, and metadata.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use uuid::Uuid;

use crate::config::ClusterConfiguration;
use crate::error::{DistributedError, NeoResult};
use crate::node::NodeManager;
use crate::types::{ClusterMetadata, ClusterState, NodeId, NodeInfo, NodeState};

// ---------------------------------------------------------------------------
// Cluster
// ---------------------------------------------------------------------------

/// The core cluster abstraction.
///
/// Manages cluster membership, state transitions, and topology. Thread-safe
/// for concurrent access from multiple async tasks.
pub struct Cluster {
    /// Unique cluster identifier.
    pub id: Uuid,
    /// Cluster configuration.
    pub config: RwLock<ClusterConfiguration>,
    /// Current cluster state.
    pub state: RwLock<ClusterState>,
    /// Node manager.
    pub node_manager: Arc<NodeManager>,
    /// When the cluster was created.
    pub created_at: DateTime<Utc>,
    /// When the cluster became active.
    pub activated_at: RwLock<Option<DateTime<Utc>>>,
    /// Current leader node.
    pub leader: RwLock<Option<NodeId>>,
    /// Pending node joins.
    pub pending_joins: RwLock<Vec<NodeId>>,
    /// Cluster-wide version counter for optimistic concurrency.
    pub version: RwLock<u64>,
}

impl Cluster {
    /// Create a new cluster from configuration.
    pub fn new(config: ClusterConfiguration) -> Self {
        let node_manager = Arc::new(NodeManager::new());
        tracing::info!(
            cluster_name = %config.name,
            min_nodes = config.min_nodes,
            max_nodes = config.max_nodes,
            "cluster created"
        );
        Self {
            id: Uuid::new_v4(),
            config: RwLock::new(config),
            state: RwLock::new(ClusterState::Forming),
            node_manager,
            created_at: Utc::now(),
            activated_at: RwLock::new(None),
            leader: RwLock::new(None),
            pending_joins: RwLock::new(Vec::new()),
            version: RwLock::new(0),
        }
    }

    // -- State queries --

    /// Current cluster state.
    pub fn state(&self) -> ClusterState {
        *self.state.read()
    }

    /// Whether the cluster is operational.
    pub fn is_active(&self) -> bool {
        self.state() == ClusterState::Active
    }

    /// Whether the cluster accepts operations.
    pub fn accepts_operations(&self) -> bool {
        self.state().accepts_operations()
    }

    /// Total node count.
    pub fn node_count(&self) -> usize {
        self.node_manager.count()
    }

    /// Number of nodes in Ready or Busy state.
    pub fn healthy_node_count(&self) -> usize {
        self.node_manager
            .nodes()
            .iter()
            .filter(|n| matches!(n.state, NodeState::Ready | NodeState::Busy))
            .count()
    }

    /// Current leader.
    pub fn leader(&self) -> Option<NodeId> {
        *self.leader.read()
    }

    /// Cluster metadata snapshot.
    pub fn metadata(&self) -> ClusterMetadata {
        ClusterMetadata {
            name: self.config.read().name.clone(),
            id: self.id,
            created_at: self.created_at,
            leader: *self.leader.read(),
            state: self.state(),
            node_count: self.node_count(),
            healthy_node_count: self.healthy_node_count(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    // -- State transitions --

    /// Transition the cluster to a new state.
    pub fn transition(&self, target: ClusterState) -> NeoResult<()> {
        let current = self.state();
        if !self.valid_transition(current, target) {
            return Err(DistributedError::cluster(format!(
                "cannot transition from {current} to {target}"
            )));
        }
        *self.state.write() = target;
        *self.version.write() += 1;

        tracing::info!(
            from = ?current,
            to = ?target,
            version = *self.version.read(),
            "cluster state transition"
        );

        if target == ClusterState::Active {
            *self.activated_at.write() = Some(Utc::now());
        }

        Ok(())
    }

    fn valid_transition(&self, from: ClusterState, to: ClusterState) -> bool {
        matches!(
            (from, to),
            (ClusterState::Forming, ClusterState::Active)
                | (ClusterState::Forming, ClusterState::Dissolving)
                | (ClusterState::Active, ClusterState::Degraded)
                | (ClusterState::Active, ClusterState::Partitioned)
                | (ClusterState::Active, ClusterState::Rebalancing)
                | (ClusterState::Active, ClusterState::Dissolving)
                | (ClusterState::Degraded, ClusterState::Active)
                | (ClusterState::Degraded, ClusterState::Partitioned)
                | (ClusterState::Degraded, ClusterState::Dissolving)
                | (ClusterState::Partitioned, ClusterState::Active)
                | (ClusterState::Partitioned, ClusterState::Degraded)
                | (ClusterState::Rebalancing, ClusterState::Active)
                | (ClusterState::Rebalancing, ClusterState::Degraded)
                | (ClusterState::Dissolving, ClusterState::Dissolved)
        )
    }

    // -- Node management --

    /// Register a new node with the cluster.
    pub fn add_node(&self, info: NodeInfo) -> NeoResult<NodeId> {
        let config = self.config.read();
        let current_count = self.node_count();
        if current_count >= config.max_nodes {
            return Err(DistributedError::cluster(format!(
                "cluster is full: {current_count}/{}",
                config.max_nodes
            )));
        }
        if self.state() == ClusterState::Dissolved {
            return Err(DistributedError::cluster(
                "cluster is dissolved and cannot accept nodes",
            ));
        }

        let node = self.node_manager.register(info)?;
        let node_id = node.id;

        tracing::info!(
            node_id = %node_id,
            total_nodes = self.node_count(),
            "node added to cluster"
        );

        // Check if we can activate the cluster.
        if self.state() == ClusterState::Forming
            && self.healthy_node_count() >= config.min_nodes
        {
            drop(config); // Release read lock before transition.
            self.transition(ClusterState::Active)?;
        }

        *self.version.write() += 1;
        Ok(node_id)
    }

    /// Remove a node from the cluster.
    pub fn remove_node(&self, node_id: NodeId) -> NeoResult<()> {
        self.node_manager.deregister(node_id)?;

        tracing::info!(
            node_id = %node_id,
            remaining_nodes = self.node_count(),
            "node removed from cluster"
        );

        // Check if we need to degrade.
        let config = self.config.read();
        if self.state() == ClusterState::Active
            && self.healthy_node_count() < config.min_nodes
        {
            drop(config);
            tracing::warn!("cluster below minimum node count, degrading");
            let _ = self.transition(ClusterState::Degraded);
        }

        *self.version.write() += 1;
        Ok(())
    }

    /// Get a node by ID.
    pub fn get_node(&self, node_id: NodeId) -> Option<crate::types::NodeInfo> {
        self.node_manager.get(node_id).map(|n| n.info.clone())
    }

    /// Get all node IDs.
    pub fn node_ids(&self) -> Vec<NodeId> {
        self.node_manager.ids()
    }

    /// Get healthy node IDs (Ready or Busy).
    pub fn healthy_node_ids(&self) -> Vec<NodeId> {
        self.node_manager
            .nodes()
            .iter()
            .filter(|n| matches!(n.state, NodeState::Ready | NodeState::Busy))
            .map(|n| n.id)
            .collect()
    }

    /// Transition a node to a new state.
    pub fn transition_node(&self, node_id: NodeId, target: NodeState) -> NeoResult<()> {
        self.node_manager.transition(node_id, target)?;
        *self.version.write() += 1;
        Ok(())
    }

    /// Set the cluster leader.
    pub fn set_leader(&self, leader: NodeId) {
        tracing::info!(leader = %leader, "cluster leader set");
        *self.leader.write() = Some(leader);
        *self.version.write() += 1;
    }

    /// Clear the cluster leader.
    pub fn clear_leader(&self) {
        *self.leader.write() = None;
        *self.version.write() += 1;
    }

    /// Check cluster health — returns true if cluster is in a good state.
    pub fn is_healthy(&self) -> bool {
        let state = self.state();
        let healthy = self.healthy_node_count();
        let config = self.config.read();
        (state == ClusterState::Active || state == ClusterState::Degraded)
            && healthy >= config.min_nodes
    }

    /// Get a list of all node infos.
    pub fn all_nodes(&self) -> Vec<crate::node::NodeEntry> {
        self.node_manager.nodes()
    }

    /// Dissolve the cluster.
    pub fn dissolve(&self) -> NeoResult<()> {
        tracing::warn!("cluster dissolving");
        self.transition(ClusterState::Dissolving)?;
        self.transition(ClusterState::Dissolved)?;
        Ok(())
    }
}

impl std::fmt::Debug for Cluster {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cluster")
            .field("id", &self.id)
            .field("state", &self.state())
            .field("nodes", &self.node_count())
            .field("leader", &self.leader())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::NodeCapabilities;

    fn test_info(hostname: &str) -> NodeInfo {
        NodeInfo {
            hostname: hostname.to_string(),
            ip_address: "127.0.0.1".to_string(),
            port: 7000,
            node_type: crate::types::NodeType::CpuWorker,
            capabilities: NodeCapabilities {
                cpu_cores: 8,
                memory_bytes: 16 * 1024 * 1024 * 1024,
                ..Default::default()
            },
            version: "0.1.0".to_string(),
            zone: "default".to_string(),
            rack: None,
        }
    }

    #[test]
    fn cluster_creation() {
        let config = ClusterConfiguration::testing();
        let cluster = Cluster::new(config);
        assert_eq!(cluster.state(), ClusterState::Forming);
        assert_eq!(cluster.node_count(), 0);
    }

    #[test]
    fn cluster_add_remove_node() {
        let config = ClusterConfiguration::testing();
        let cluster = Cluster::new(config);

        let id = cluster.add_node(test_info("node-1")).unwrap();
        assert_eq!(cluster.node_count(), 1);

        cluster.remove_node(id).unwrap();
        assert_eq!(cluster.node_count(), 0);
    }

    #[test]
    fn cluster_metadata() {
        let config = ClusterConfiguration::testing();
        let cluster = Cluster::new(config);
        let meta = cluster.metadata();
        assert_eq!(meta.name, "neo-cluster");
        assert_eq!(meta.node_count, 0);
    }

    #[test]
    fn cluster_state_transitions() {
        let config = ClusterConfiguration::testing();
        let cluster = Cluster::new(config);
        cluster.add_node(test_info("node-1")).unwrap();
        // testing config has min_nodes=1, so cluster should be active
        assert_eq!(cluster.state(), ClusterState::Active);
    }

    #[test]
    fn cluster_healthy_check() {
        let config = ClusterConfiguration::testing();
        let cluster = Cluster::new(config);
        cluster.add_node(test_info("node-1")).unwrap();
        assert!(cluster.is_healthy());
    }

    #[test]
    fn cluster_dissolve() {
        let config = ClusterConfiguration::testing();
        let cluster = Cluster::new(config);
        cluster.dissolve().unwrap();
        assert_eq!(cluster.state(), ClusterState::Dissolved);
    }
}
