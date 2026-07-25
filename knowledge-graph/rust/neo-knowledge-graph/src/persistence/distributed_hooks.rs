use serde::{Deserialize, Serialize};

/// Configuration for distributed graph operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedConfig {
    /// Whether distributed mode is enabled.
    pub enabled: bool,
    /// Node identifier.
    pub node_id: String,
    /// Peer node addresses.
    pub peers: Vec<String>,
    /// Sync interval in seconds.
    pub sync_interval_secs: u64,
    /// Maximum replication lag allowed.
    pub max_replication_lag: u64,
}

impl Default for DistributedConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            node_id: "node-1".to_string(),
            peers: Vec::new(),
            sync_interval_secs: 30,
            max_replication_lag: 60,
        }
    }
}

/// Hook points for distributed graph replication and consistency.
pub struct DistributedGraphHooks {
    config: DistributedConfig,
}

impl DistributedGraphHooks {
    /// Create new distributed hooks.
    #[must_use]
    pub fn new(config: DistributedConfig) -> Self {
        Self { config }
    }

    /// Check if distributed mode is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get the node id.
    #[must_use]
    pub fn node_id(&self) -> &str {
        &self.config.node_id
    }

    /// Get peer addresses.
    #[must_use]
    pub fn peers(&self) -> &[String] {
        &self.config.peers
    }

    /// Hook called after an entity is created (for replication).
    pub fn after_entity_created(&self, entity_id: &str) {
        if self.config.enabled {
            tracing::debug!(
                node = %self.config.node_id,
                entity_id,
                "Distributed hook: entity created"
            );
        }
    }

    /// Hook called after a relation is created.
    pub fn after_relation_created(&self, relation_id: &str) {
        if self.config.enabled {
            tracing::debug!(
                node = %self.config.node_id,
                relation_id,
                "Distributed hook: relation created"
            );
        }
    }

    /// Hook called after an entity is updated.
    pub fn after_entity_updated(&self, entity_id: &str) {
        if self.config.enabled {
            tracing::debug!(
                node = %self.config.node_id,
                entity_id,
                "Distributed hook: entity updated"
            );
        }
    }

    /// Hook called after an entity is deleted.
    pub fn after_entity_deleted(&self, entity_id: &str) {
        if self.config.enabled {
            tracing::debug!(
                node = %self.config.node_id,
                entity_id,
                "Distributed hook: entity deleted"
            );
        }
    }
}

impl Default for DistributedGraphHooks {
    fn default() -> Self {
        Self::new(DistributedConfig::default())
    }
}
