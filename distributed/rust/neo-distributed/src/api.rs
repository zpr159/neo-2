//! REST API handlers for cluster management, node operations, metrics,
//! and health endpoints.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::cluster::Cluster;
use crate::error::{DistributedError, NeoResult};
use crate::monitoring::ClusterMetrics;
use crate::node::NodeManager;
use crate::types::{
    ClusterMetadata, ClusterState, NodeCapabilities, NodeId, NodeInfo, NodeResources,
    NodeState, NodeType,
};

// ---------------------------------------------------------------------------
// API Request / Response types
// ---------------------------------------------------------------------------

/// Request to register a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterNodeRequest {
    pub hostname: String,
    pub ip_address: String,
    pub port: u16,
    pub node_type: NodeType,
    pub capabilities: NodeCapabilities,
    pub zone: Option<String>,
}

/// Response to node registration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterNodeResponse {
    pub node_id: NodeId,
    pub cluster_state: ClusterState,
}

/// Request to drain a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrainNodeRequest {
    pub node_id: NodeId,
    pub timeout_secs: Option<u64>,
}

/// Cluster status response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStatusResponse {
    pub metadata: ClusterMetadata,
    pub nodes: Vec<NodeStatus>,
    pub is_healthy: bool,
}

/// Status of a single node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    pub id: NodeId,
    pub hostname: String,
    pub state: NodeState,
    pub node_type: NodeType,
    pub zone: String,
    pub resources: NodeResources,
    pub health_score: f32,
    pub latency_ms: f64,
}

/// Metrics response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub cluster: ClusterMetrics,
    pub node_count: usize,
    pub healthy_nodes: usize,
    pub total_registrations: u64,
    pub total_deregistrations: u64,
}

/// Health check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub cluster_state: ClusterState,
    pub node_count: usize,
    pub healthy_nodes: usize,
    pub uptime_secs: u64,
}

// ---------------------------------------------------------------------------
// DistributedApi
// ---------------------------------------------------------------------------

/// REST API handler for the distributed runtime.
pub struct DistributedApi {
    cluster: Arc<Cluster>,
    start_time: std::time::Instant,
}

impl DistributedApi {
    /// Create a new API handler.
    pub fn new(cluster: Arc<Cluster>) -> Self {
        Self {
            cluster,
            start_time: std::time::Instant::now(),
        }
    }

    // -- Cluster endpoints --

    /// GET /cluster — Get cluster status.
    pub fn cluster_status(&self) -> ClusterStatusResponse {
        let metadata = self.cluster.metadata();
        let nodes: Vec<NodeStatus> = self
            .cluster
            .all_nodes()
            .into_iter()
            .map(|entry| NodeStatus {
                id: entry.id,
                hostname: entry.info.hostname.clone(),
                state: entry.state,
                node_type: entry.info.node_type,
                zone: entry.info.zone.clone(),
                resources: entry.resources.clone(),
                health_score: entry.health.score,
                latency_ms: entry.health.latency_ms,
            })
            .collect();

        ClusterStatusResponse {
            metadata,
            nodes,
            is_healthy: self.cluster.is_healthy(),
        }
    }

    // -- Node endpoints --

    /// POST /nodes/register — Register a new node.
    pub fn register_node(&self, req: RegisterNodeRequest) -> NeoResult<RegisterNodeResponse> {
        let info = NodeInfo {
            hostname: req.hostname,
            ip_address: req.ip_address,
            port: req.port,
            node_type: req.node_type,
            capabilities: req.capabilities,
            version: env!("CARGO_PKG_VERSION").to_string(),
            zone: req.zone.unwrap_or_else(|| "default".to_string()),
            rack: None,
        };

        let node_id = self.cluster.add_node(info)?;

        Ok(RegisterNodeResponse {
            node_id,
            cluster_state: self.cluster.state(),
        })
    }

    /// GET /nodes — List all nodes.
    pub fn list_nodes(&self) -> Vec<NodeStatus> {
        self.cluster
            .all_nodes()
            .into_iter()
            .map(|entry| NodeStatus {
                id: entry.id,
                hostname: entry.info.hostname.clone(),
                state: entry.state,
                node_type: entry.info.node_type,
                zone: entry.info.zone.clone(),
                resources: entry.resources.clone(),
                health_score: entry.health.score,
                latency_ms: entry.health.latency_ms,
            })
            .collect()
    }

    /// GET /nodes/{id} — Get a specific node.
    pub fn get_node(&self, node_id: NodeId) -> NeoResult<NodeStatus> {
        let entry = self.cluster.node_manager.get(node_id)
            .ok_or_else(|| DistributedError::node(format!("node not found: {node_id}")))?;

        Ok(NodeStatus {
            id: entry.id,
            hostname: entry.info.hostname.clone(),
            state: entry.state,
            node_type: entry.info.node_type,
            zone: entry.info.zone.clone(),
            resources: entry.resources.clone(),
            health_score: entry.health.score,
            latency_ms: entry.health.latency_ms,
        })
    }

    /// POST /nodes/{id}/drain — Drain a node.
    pub fn drain_node(&self, node_id: NodeId) -> NeoResult<()> {
        self.cluster
            .transition_node(node_id, NodeState::Draining)
    }

    /// POST /nodes/{id}/leave — Make a node leave.
    pub fn node_leave(&self, node_id: NodeId) -> NeoResult<()> {
        self.cluster.remove_node(node_id)
    }

    // -- Cluster operations --

    /// POST /cluster/rebalance — Rebalance the cluster.
    pub fn rebalance(&self) -> NeoResult<()> {
        self.cluster.transition(ClusterState::Rebalancing)?;
        // In a real implementation, this would trigger task migration.
        self.cluster.transition(ClusterState::Active)?;
        Ok(())
    }

    /// POST /cluster/failover — Trigger failover.
    pub fn failover(&self) -> NeoResult<()> {
        // Step down current leader.
        self.cluster.clear_leader();
        tracing::warn!("cluster failover triggered");
        Ok(())
    }

    // -- Metrics endpoints --

    /// GET /metrics — Get cluster metrics.
    pub fn metrics(&self) -> MetricsResponse {
        let nodes = self.cluster.node_manager.nodes();
        let healthy = nodes
            .iter()
            .filter(|n| matches!(n.state, NodeState::Ready | NodeState::Busy))
            .count();

        MetricsResponse {
            cluster: ClusterMetrics {
                timestamp: chrono::Utc::now(),
                total_nodes: nodes.len(),
                healthy_nodes: healthy,
                total_cpu_utilization: 0.0,
                total_memory_utilization: 0.0,
                total_gpu_utilization: 0.0,
                avg_latency_ms: 0.0,
                throughput_ops_per_sec: 0.0,
                active_tasks: 0,
                queued_tasks: 0,
            },
            node_count: nodes.len(),
            healthy_nodes: healthy,
            total_registrations: self.cluster.node_manager.total_registrations() as u64,
            total_deregistrations: self.cluster.node_manager.total_deregistrations() as u64,
        }
    }

    /// GET /health — Health check.
    pub fn health(&self) -> HealthResponse {
        HealthResponse {
            status: if self.cluster.is_healthy() {
                "healthy".to_string()
            } else {
                "unhealthy".to_string()
            },
            cluster_state: self.cluster.state(),
            node_count: self.cluster.node_count(),
            healthy_nodes: self.cluster.healthy_node_count(),
            uptime_secs: self.start_time.elapsed().as_secs(),
        }
    }

    /// GET /topology — Get cluster topology.
    pub fn topology(&self) -> HashMap<String, serde_json::Value> {
        let mut topology = HashMap::new();
        topology.insert(
            "cluster".to_string(),
            serde_json::to_value(self.cluster.metadata()).unwrap_or_default(),
        );
        topology.insert(
            "nodes".to_string(),
            serde_json::to_value(self.list_nodes()).unwrap_or_default(),
        );
        topology
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ClusterConfiguration;
    use crate::types::NodeCapabilities;

    fn make_api() -> DistributedApi {
        let config = ClusterConfiguration::testing();
        let cluster = Arc::new(Cluster::new(config));
        DistributedApi::new(cluster)
    }

    #[test]
    fn cluster_status() {
        let api = make_api();
        let status = api.cluster_status();
        assert_eq!(status.nodes.len(), 0);
    }

    #[test]
    fn register_node() {
        let api = make_api();
        let req = RegisterNodeRequest {
            hostname: "node-1".to_string(),
            ip_address: "127.0.0.1".to_string(),
            port: 7000,
            node_type: NodeType::CpuWorker,
            capabilities: NodeCapabilities::default(),
            zone: None,
        };
        let resp = api.register_node(req).unwrap();
        assert_eq!(api.list_nodes().len(), 1);
    }

    #[test]
    fn health_check() {
        let api = make_api();
        let health = api.health();
        assert_eq!(health.node_count, 0);
    }

    #[test]
    fn metrics() {
        let api = make_api();
        let metrics = api.metrics();
        assert_eq!(metrics.node_count, 0);
    }

    #[test]
    fn topology() {
        let api = make_api();
        let topo = api.topology();
        assert!(topo.contains_key("cluster"));
        assert!(topo.contains_key("nodes"));
    }
}
