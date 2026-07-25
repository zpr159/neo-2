//! Node management — registration, lifecycle, identity, registry, and
//! capabilities for cluster nodes.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{DistributedError, NeoResult};
use crate::types::{NodeCapabilities, NodeHealth, NodeId, NodeInfo, NodeResources, NodeState};

// ---------------------------------------------------------------------------
// NodeEntry
// ---------------------------------------------------------------------------

/// Full node record stored in the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeEntry {
    /// Unique node ID.
    pub id: NodeId,
    /// Static node info.
    pub info: NodeInfo,
    /// Current lifecycle state.
    pub state: NodeState,
    /// When the node joined.
    pub joined_at: DateTime<Utc>,
    /// When the node last sent a heartbeat.
    pub last_heartbeat: DateTime<Utc>,
    /// Current resource utilization.
    pub resources: NodeResources,
    /// Current health report.
    pub health: NodeHealth,
    /// Software version running on this node.
    pub version: String,
}

impl NodeEntry {
    /// Whether the node is considered reachable (heartbeat within 30s).
    pub fn is_reachable(&self) -> bool {
        let elapsed = Utc::now()
            .signed_duration_since(self.last_heartbeat)
            .num_seconds();
        elapsed < 30
    }

    /// Composite health score.
    pub fn health_score(&self) -> f32 {
        self.health.score
    }
}

// ---------------------------------------------------------------------------
// NodeManager
// ---------------------------------------------------------------------------

/// Manages the full lifecycle of cluster nodes.
///
/// Thread-safe via `DashMap` for concurrent registration/deregistration
/// and `RwLock` for state transitions.
pub struct NodeManager {
    /// All registered nodes keyed by ID.
    nodes: DashMap<NodeId, NodeEntry>,
    /// Reverse index: hostname → NodeId.
    by_hostname: DashMap<String, NodeId>,
    /// Total registrations (monotonic).
    total_registrations: AtomicUsize,
    /// Total deregistrations (monotonic).
    total_deregistrations: AtomicUsize,
}

impl NodeManager {
    /// Create an empty node manager.
    pub fn new() -> Self {
        Self {
            nodes: DashMap::new(),
            by_hostname: DashMap::new(),
            total_registrations: AtomicUsize::new(0),
            total_deregistrations: AtomicUsize::new(0),
        }
    }

    // -- Registration --

    /// Register a new node and return its entry.
    pub fn register(&self, info: NodeInfo) -> NeoResult<NodeEntry> {
        let node_id = NodeId::new();
        let now = Utc::now();

        let entry = NodeEntry {
            id: node_id,
            info: info.clone(),
            state: NodeState::Joining,
            joined_at: now,
            last_heartbeat: now,
            resources: NodeResources::default(),
            health: NodeHealth {
                score: 1.0,
                state: NodeState::Joining,
                last_heartbeat: now,
                latency_ms: 0.0,
                clock_drift_us: 0,
                responsive: true,
                warnings: vec![],
            },
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        self.by_hostname.insert(info.hostname.clone(), node_id);
        self.nodes.insert(node_id, entry.clone());
        self.total_registrations.fetch_add(1, Ordering::Relaxed);

        tracing::info!(
            node_id = %node_id,
            hostname = %info.hostname,
            "node registered"
        );

        Ok(entry)
    }

    /// Deregister a node.
    pub fn deregister(&self, node_id: NodeId) -> NeoResult<()> {
        let entry = self
            .nodes
            .remove(&node_id)
            .ok_or_else(|| DistributedError::node(format!("node not found: {node_id}")))?;

        self.by_hostname.remove(&entry.1.info.hostname);
        self.total_deregistrations
            .fetch_add(1, Ordering::Relaxed);

        tracing::info!(node_id = %node_id, "node deregistered");
        Ok(())
    }

    // -- Queries --

    /// Get a node entry by ID.
    pub fn get(&self, node_id: NodeId) -> Option<NodeEntry> {
        self.nodes.get(&node_id).map(|r| r.value().clone())
    }

    /// Get a node ID by hostname.
    pub fn get_by_hostname(&self, hostname: &str) -> Option<NodeId> {
        self.by_hostname.get(hostname).map(|r| *r.value())
    }

    /// Get all node entries.
    pub fn nodes(&self) -> Vec<NodeEntry> {
        self.nodes.iter().map(|r| r.value().clone()).collect()
    }

    /// Get all node IDs.
    pub fn ids(&self) -> Vec<NodeId> {
        self.nodes.iter().map(|r| *r.key()).collect()
    }

    /// Number of registered nodes.
    pub fn count(&self) -> usize {
        self.nodes.len()
    }

    /// Get nodes matching a predicate.
    pub fn filter<F>(&self, pred: F) -> Vec<NodeEntry>
    where
        F: Fn(&NodeEntry) -> bool,
    {
        self.nodes
            .iter()
            .filter(|r| pred(r.value()))
            .map(|r| r.value().clone())
            .collect()
    }

    /// Get nodes in a specific state.
    pub fn in_state(&self, state: NodeState) -> Vec<NodeEntry> {
        self.filter(|e| e.state == state)
    }

    /// Get healthy nodes (Ready or Busy).
    pub fn healthy(&self) -> Vec<NodeEntry> {
        self.filter(|e| matches!(e.state, NodeState::Ready | NodeState::Busy))
    }

    // -- State transitions --

    /// Transition a node to a new state.
    pub fn transition(&self, node_id: NodeId, target: NodeState) -> NeoResult<()> {
        let mut entry = self
            .nodes
            .get_mut(&node_id)
            .ok_or_else(|| DistributedError::node(format!("node not found: {node_id}")))?;
        let current = entry.state;
        if !current.can_transition_to(target) {
            tracing::warn!(
                node_id = %node_id,
                from = ?current,
                to = ?target,
                "invalid node state transition"
            );
            return Ok(());
        }
        entry.state = target;
        entry.health.state = target;
        tracing::info!(
            node_id = %node_id,
            from = ?current,
            to = ?target,
            "node state transition"
        );
        Ok(())
    }

    /// Update heartbeat timestamp for a node.
    pub fn heartbeat(&self, node_id: NodeId, latency_ms: f64) -> NeoResult<()> {
        let mut entry = self
            .nodes
            .get_mut(&node_id)
            .ok_or_else(|| DistributedError::node(format!("node not found: {node_id}")))?;
        entry.last_heartbeat = Utc::now();
        entry.health.last_heartbeat = Utc::now();
        entry.health.latency_ms = latency_ms;
        entry.health.responsive = true;
        entry.health.score = (entry.health.score * 0.9 + 0.1).min(1.0);
        Ok(())
    }

    /// Update resource utilization for a node.
    pub fn update_resources(&self, node_id: NodeId, resources: NodeResources) -> NeoResult<()> {
        let mut entry = self
            .nodes
            .get_mut(&node_id)
            .ok_or_else(|| DistributedError::node(format!("node not found: {node_id}")))?;
        entry.resources = resources;
        Ok(())
    }

    /// Mark a node as suspect (missed heartbeats).
    pub fn mark_suspect(&self, node_id: NodeId) -> NeoResult<()> {
        let mut entry = self
            .nodes
            .get_mut(&node_id)
            .ok_or_else(|| DistributedError::node(format!("node not found: {node_id}")))?;
        entry.health.score *= 0.5;
        entry.health.responsive = false;
        entry.health
            .warnings
            .push("suspect: missed heartbeat".to_string());
        Ok(())
    }

    /// Mark a node as failed.
    pub fn mark_failed(&self, node_id: NodeId) -> NeoResult<()> {
        self.transition(node_id, NodeState::Failed)?;
        let mut entry = self
            .nodes
            .get_mut(&node_id)
            .ok_or_else(|| DistributedError::node(format!("node not found: {node_id}")))?;
        entry.health.score = 0.0;
        entry.health.responsive = false;
        Ok(())
    }

    // -- Analytics --

    /// Total registrations (monotonic).
    pub fn total_registrations(&self) -> usize {
        self.total_registrations.load(Ordering::Relaxed)
    }

    /// Total deregistrations (monotonic).
    pub fn total_deregistrations(&self) -> usize {
        self.total_deregistrations.load(Ordering::Relaxed)
    }

    /// Find the least loaded node.
    pub fn least_loaded(&self) -> Option<NodeEntry> {
        self.healthy()
            .into_iter()
            .min_by(|a, b| {
                a.resources
                    .load_score()
                    .partial_cmp(&b.resources.load_score())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Find nodes with GPU capabilities.
    pub fn gpu_nodes(&self) -> Vec<NodeEntry> {
        self.filter(|e| e.info.capabilities.has_gpu())
    }

    /// Find nodes supporting a specific capability.
    pub fn with_capability(&self, cap: &str) -> Vec<NodeEntry> {
        self.filter(|e| e.info.capabilities.supports_capability(cap))
    }
}

impl Default for NodeManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// NodeIdentity
// ---------------------------------------------------------------------------

/// Cryptographic identity for a node (used by the security layer).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeIdentity {
    /// Node ID.
    pub node_id: NodeId,
    /// Public key (hex-encoded).
    pub public_key: String,
    /// Certificate PEM (if mTLS is enabled).
    pub certificate: Option<String>,
    /// When the identity was created.
    pub created_at: DateTime<Utc>,
    /// When the certificate expires.
    pub expires_at: Option<DateTime<Utc>>,
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
    fn register_and_deregister() {
        let mgr = NodeManager::new();
        let entry = mgr.register(test_info("host-a")).unwrap();
        assert_eq!(mgr.count(), 1);

        mgr.deregister(entry.id).unwrap();
        assert_eq!(mgr.count(), 0);
    }

    #[test]
    fn get_by_hostname() {
        let mgr = NodeManager::new();
        let entry = mgr.register(test_info("host-b")).unwrap();
        let found = mgr.get_by_hostname("host-b");
        assert_eq!(found, Some(entry.id));
    }

    #[test]
    fn node_state_transition() {
        let mgr = NodeManager::new();
        let entry = mgr.register(test_info("host-c")).unwrap();
        mgr.transition(entry.id, NodeState::Initializing).unwrap();
        let node = mgr.get(entry.id).unwrap();
        assert_eq!(node.state, NodeState::Initializing);
    }

    #[test]
    fn heartbeat_update() {
        let mgr = NodeManager::new();
        let entry = mgr.register(test_info("host-d")).unwrap();
        mgr.heartbeat(entry.id, 5.0).unwrap();
        let node = mgr.get(entry.id).unwrap();
        assert!(node.health.responsive);
    }

    #[test]
    fn healthy_nodes() {
        let mgr = NodeManager::new();
        let e1 = mgr.register(test_info("h1")).unwrap();
        let e2 = mgr.register(test_info("h2")).unwrap();
        mgr.transition(e1.id, NodeState::Ready).unwrap();
        mgr.transition(e2.id, NodeState::Failed).unwrap();
        let healthy = mgr.healthy();
        assert_eq!(healthy.len(), 1);
        assert_eq!(healthy[0].id, e1.id);
    }

    #[test]
    fn least_loaded_node() {
        let mgr = NodeManager::new();
        let _e1 = mgr.register(test_info("h1")).unwrap();
        let _e2 = mgr.register(test_info("h2")).unwrap();
        // Both are Joining, so none are healthy.
        assert!(mgr.least_loaded().is_none());
    }

    #[test]
    fn node_filter() {
        let mgr = NodeManager::new();
        mgr.register(test_info("h1")).unwrap();
        mgr.register(test_info("h2")).unwrap();
        let joining = mgr.in_state(NodeState::Joining);
        assert_eq!(joining.len(), 2);
    }
}
