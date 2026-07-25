use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::WorldVersion;

/// A distributed node in the world model cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedNode {
    pub node_id: String,
    pub address: String,
    pub region: String,
    pub current_version: WorldVersion,
    pub last_heartbeat: DateTime<Utc>,
    pub is_healthy: bool,
    pub capabilities: Vec<String>,
    pub metadata: HashMap<String, String>,
}

/// Manages distributed world model operations.
pub struct DistributedManager {
    nodes: dashmap::DashMap<String, DistributedNode>,
    local_node_id: String,
}

impl DistributedManager {
    pub fn new(local_node_id: impl Into<String>) -> Self {
        Self {
            nodes: dashmap::DashMap::new(),
            local_node_id: local_node_id.into(),
        }
    }

    pub fn register_node(&self, node: DistributedNode) {
        self.nodes.insert(node.node_id.clone(), node);
    }

    pub fn heartbeat(&self, node_id: &str, version: WorldVersion) {
        if let Some(mut node) = self.nodes.get_mut(node_id) {
            node.last_heartbeat = Utc::now();
            node.current_version = version;
            node.is_healthy = true;
        }
    }

    pub fn healthy_nodes(&self) -> Vec<DistributedNode> {
        self.nodes
            .iter()
            .filter(|n| n.value().is_healthy)
            .map(|n| n.value().clone())
            .collect()
    }

    pub fn all_nodes(&self) -> Vec<DistributedNode> {
        self.nodes.iter().map(|n| n.value().clone()).collect()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn local_node_id(&self) -> &str {
        &self.local_node_id
    }

    pub fn min_version(&self) -> WorldVersion {
        self.nodes
            .iter()
            .map(|n| n.value().current_version)
            .min()
            .unwrap_or(WorldVersion::initial())
    }

    pub fn max_version(&self) -> WorldVersion {
        self.nodes
            .iter()
            .map(|n| n.value().current_version)
            .max()
            .unwrap_or(WorldVersion::initial())
    }
}

impl Default for DistributedManager {
    fn default() -> Self {
        Self::new("local")
    }
}
