use serde::{Deserialize, Serialize};

use crate::types::EnvironmentType;

/// Configuration for the world model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldConfig {
    /// Maximum number of entities to track.
    pub max_entities: usize,
    /// Maximum number of relationships.
    pub max_relationships: usize,
    /// Maximum number of events to retain.
    pub max_events: usize,
    /// Maximum number of locations to track.
    pub max_locations: usize,
    /// Maximum number of causal links.
    pub max_causal_links: usize,
    /// Maximum number of environments.
    pub max_environments: usize,
    /// Auto-snapshot interval in events (0 = disabled).
    pub auto_snapshot_interval: usize,
    /// Maximum number of snapshots to retain.
    pub max_snapshots: usize,
    /// Entity decay: deactivate entities not observed for N seconds.
    pub entity_decay_secs: u64,
    /// Confidence threshold for auto-archival.
    pub archival_confidence_threshold: f32,
    /// Default environment type for new locations.
    pub default_environment_type: EnvironmentType,
    /// Enable spatial tracking.
    pub enable_spatial: bool,
    /// Enable temporal tracking.
    pub enable_temporal: bool,
    /// Enable causal tracking.
    pub enable_causal: bool,
    /// Enable perception processing.
    pub enable_perception: bool,
    /// Enable prediction engine.
    pub enable_prediction: bool,
    /// Enable simulation engine.
    pub enable_simulation: bool,
    /// Enable history tracking.
    pub enable_history: bool,
    /// Enable distributed synchronization.
    pub enable_distributed: bool,
    /// Enable persistence.
    pub enable_persistence: bool,
    /// Maximum perception queue size.
    pub max_perception_queue: usize,
    /// Maximum observation queue size.
    pub max_observation_queue: usize,
    /// Maximum number of concurrent simulations.
    pub max_concurrent_simulations: usize,
    /// Maximum prediction horizon in seconds.
    pub prediction_horizon_secs: u64,
    /// State validation interval in seconds.
    pub validation_interval_secs: u64,
    /// Metrics collection interval in seconds.
    pub metrics_interval_secs: u64,
    /// Persistence path (if enabled).
    pub persistence_path: Option<String>,
    /// Synchronization interval for distributed mode in milliseconds.
    pub sync_interval_ms: u64,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            max_entities: 100_000,
            max_relationships: 500_000,
            max_events: 1_000_000,
            max_locations: 50_000,
            max_causal_links: 500_000,
            max_environments: 10_000,
            auto_snapshot_interval: 100,
            max_snapshots: 1_000,
            entity_decay_secs: 86_400,
            archival_confidence_threshold: 0.1,
            default_environment_type: EnvironmentType::Digital,
            enable_spatial: true,
            enable_temporal: true,
            enable_causal: true,
            enable_perception: true,
            enable_prediction: true,
            enable_simulation: true,
            enable_history: true,
            enable_distributed: false,
            enable_persistence: false,
            max_perception_queue: 10_000,
            max_observation_queue: 10_000,
            max_concurrent_simulations: 10,
            prediction_horizon_secs: 3600,
            validation_interval_secs: 300,
            metrics_interval_secs: 60,
            persistence_path: None,
            sync_interval_ms: 1000,
        }
    }
}
