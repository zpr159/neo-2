//! Runtime configuration with profiles and hot-reload support.

use serde::{Deserialize, Serialize};
use tokio::sync::watch;

use neo_core::error::NeoResult;
use neo_core::types::Environment;

/// Runtime configuration profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuntimeProfile {
    Development,
    Testing,
    Staging,
    Production,
}

impl RuntimeProfile {
    /// Return the environment this profile targets.
    pub fn environment(&self) -> Environment {
        match self {
            Self::Development => Environment::Development,
            Self::Testing => Environment::Testing,
            Self::Staging => Environment::Staging,
            Self::Production => Environment::Production,
        }
    }
}

impl std::fmt::Display for RuntimeProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Development => write!(f, "development"),
            Self::Testing => write!(f, "testing"),
            Self::Staging => write!(f, "staging"),
            Self::Production => write!(f, "production"),
        }
    }
}

/// Thread pool configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadPoolConfig {
    pub min_workers: usize,
    pub max_workers: usize,
    pub auto_scale: bool,
    pub scale_up_threshold: f64,
    pub scale_down_threshold: f64,
    pub worker_stack_size: usize,
}

impl Default for ThreadPoolConfig {
    fn default() -> Self {
        Self {
            min_workers: 2,
            max_workers: num_cpus().max(4),
            auto_scale: true,
            scale_up_threshold: 0.8,
            scale_down_threshold: 0.2,
            worker_stack_size: 8 * 1024 * 1024,
        }
    }
}

/// Scheduler configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub max_concurrent_tasks: usize,
    pub queue_capacity: usize,
    pub default_task_timeout_ms: u64,
    pub max_retries: u32,
    pub retry_base_delay_ms: u64,
    pub retry_max_delay_ms: u64,
    pub work_stealing_enabled: bool,
    pub deadlock_detection_timeout_ms: u64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 256,
            queue_capacity: 8192,
            default_task_timeout_ms: 30_000,
            max_retries: 3,
            retry_base_delay_ms: 100,
            retry_max_delay_ms: 10_000,
            work_stealing_enabled: true,
            deadlock_detection_timeout_ms: 60_000,
        }
    }
}

/// Resource manager configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceManagerConfig {
    pub total_cpu_units: u64,
    pub total_gpu_units: u64,
    pub total_ram_bytes: u64,
    pub total_disk_bytes: u64,
    pub network_bandwidth_bps: u64,
    pub memory_pool_size_bytes: usize,
    pub enable_quotas: bool,
    pub monitoring_interval_ms: u64,
}

impl Default for ResourceManagerConfig {
    fn default() -> Self {
        Self {
            total_cpu_units: num_cpus() as u64,
            total_gpu_units: 1,
            total_ram_bytes: 256 * 1024 * 1024 * 1024,
            total_disk_bytes: 1024 * 1024 * 1024 * 1024,
            network_bandwidth_bps: 1_000_000_000,
            memory_pool_size_bytes: 128 * 1024 * 1024,
            enable_quotas: true,
            monitoring_interval_ms: 5_000,
        }
    }
}

/// Event bus configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBusConfig {
    pub broadcast_capacity: usize,
    pub max_subscribers: usize,
    pub persistent_event_limit: usize,
    pub priority_levels: usize,
    pub enable_replay: bool,
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self {
            broadcast_capacity: 1024,
            max_subscribers: 256,
            persistent_event_limit: 10_000,
            priority_levels: 5,
            enable_replay: true,
        }
    }
}

/// Message bus configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageBusConfig {
    pub max_topics: usize,
    pub message_buffer_size: usize,
    pub enable_compression: bool,
    pub max_message_size_bytes: usize,
    pub request_reply_timeout_ms: u64,
    pub streaming_buffer_size: usize,
}

impl Default for MessageBusConfig {
    fn default() -> Self {
        Self {
            max_topics: 256,
            message_buffer_size: 4096,
            enable_compression: false,
            max_message_size_bytes: 16 * 1024 * 1024,
            request_reply_timeout_ms: 10_000,
            streaming_buffer_size: 1024,
        }
    }
}

/// Plugin loader configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub enabled: bool,
    pub plugin_directory: String,
    pub enable_hot_reload: bool,
    pub hot_reload_interval_ms: u64,
    pub enable_sandbox: bool,
    pub enable_verification: bool,
    pub max_plugins: usize,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            plugin_directory: "./plugins".to_string(),
            enable_hot_reload: true,
            hot_reload_interval_ms: 5_000,
            enable_sandbox: true,
            enable_verification: true,
            max_plugins: 64,
        }
    }
}

/// Health monitor configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    pub heartbeat_interval_ms: u64,
    pub heartbeat_timeout_ms: u64,
    pub max_alerts: usize,
    pub enable_self_diagnostics: bool,
    pub diagnostic_interval_ms: u64,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval_ms: 1_000,
            heartbeat_timeout_ms: 5_000,
            max_alerts: 256,
            enable_self_diagnostics: true,
            diagnostic_interval_ms: 30_000,
        }
    }
}

/// Performance monitor configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub latency_histogram_buckets: usize,
    pub enable_cpu_monitoring: bool,
    pub enable_memory_monitoring: bool,
    pub enable_gpu_monitoring: bool,
    pub sampling_interval_ms: u64,
    pub statistics_window_size: usize,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            latency_histogram_buckets: 64,
            enable_cpu_monitoring: true,
            enable_memory_monitoring: true,
            enable_gpu_monitoring: false,
            sampling_interval_ms: 1_000,
            statistics_window_size: 300,
        }
    }
}

/// Top-level runtime configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfiguration {
    pub profile: RuntimeProfile,
    pub thread_pool: ThreadPoolConfig,
    pub scheduler: SchedulerConfig,
    pub resources: ResourceManagerConfig,
    pub event_bus: EventBusConfig,
    pub message_bus: MessageBusConfig,
    pub plugin: PluginConfig,
    pub health: HealthConfig,
    pub performance: PerformanceConfig,
    pub shutdown_timeout_ms: u64,
}

impl Default for RuntimeConfiguration {
    fn default() -> Self {
        Self {
            profile: RuntimeProfile::Development,
            thread_pool: ThreadPoolConfig::default(),
            scheduler: SchedulerConfig::default(),
            resources: ResourceManagerConfig::default(),
            event_bus: EventBusConfig::default(),
            message_bus: MessageBusConfig::default(),
            plugin: PluginConfig::default(),
            health: HealthConfig::default(),
            performance: PerformanceConfig::default(),
            shutdown_timeout_ms: 30_000,
        }
    }
}

impl RuntimeConfiguration {
    /// Create a configuration for the development profile.
    pub fn development() -> Self {
        Self {
            profile: RuntimeProfile::Development,
            ..Self::default()
        }
    }

    /// Create a configuration for the testing profile.
    pub fn testing() -> Self {
        Self {
            profile: RuntimeProfile::Testing,
            thread_pool: ThreadPoolConfig {
                min_workers: 1,
                max_workers: 4,
                auto_scale: false,
                ..ThreadPoolConfig::default()
            },
            scheduler: SchedulerConfig {
                max_concurrent_tasks: 32,
                queue_capacity: 256,
                ..SchedulerConfig::default()
            },
            ..Self::default()
        }
    }

    /// Create a configuration for the production profile.
    pub fn production() -> Self {
        Self {
            profile: RuntimeProfile::Production,
            thread_pool: ThreadPoolConfig {
                min_workers: 4,
                max_workers: num_cpus() * 2,
                auto_scale: true,
                ..ThreadPoolConfig::default()
            },
            scheduler: SchedulerConfig {
                max_concurrent_tasks: 1024,
                queue_capacity: 65536,
                ..SchedulerConfig::default()
            },
            ..Self::default()
        }
    }

    /// Load configuration from a TOML string.
    pub fn from_toml(toml_str: &str) -> NeoResult<Self> {
        let config: RuntimeConfiguration =
            toml::from_str(toml_str).map_err(|e| neo_core::error::NeoError::Config(e.to_string()))?;
        Ok(config)
    }

    /// Serialize configuration to TOML.
    pub fn to_toml(&self) -> NeoResult<String> {
        toml::to_string_pretty(self)
            .map_err(|e| neo_core::error::NeoError::Config(e.to_string()))
    }
}

/// Handle for hot-reloading configuration.
///
/// Wraps a `watch` channel so that producers can push new configurations
/// and consumers can asynchronously receive updates.
#[derive(Debug)]
pub struct HotReloadConfig {
    sender: watch::Sender<RuntimeConfiguration>,
    receiver: watch::Receiver<RuntimeConfiguration>,
}

impl HotReloadConfig {
    /// Create a new hot-reload handle with the given initial configuration.
    pub fn new(initial: RuntimeConfiguration) -> Self {
        let (sender, receiver) = watch::channel(initial);
        Self { sender, receiver }
    }

    /// Push a new configuration, notifying all receivers.
    pub fn update(&self, config: RuntimeConfiguration) {
        let _ = self.sender.send(config);
    }

    /// Get a clone of the current configuration.
    pub fn current(&self) -> RuntimeConfiguration {
        self.receiver.borrow().clone()
    }

    /// Watch for configuration changes. Returns a receiver that will receive
    /// the new value each time the configuration is updated.
    pub fn watch(&self) -> watch::Receiver<RuntimeConfiguration> {
        self.receiver.clone()
    }

    /// Block until a new configuration is received.
    pub async fn changed(&mut self) -> NeoResult<()> {
        self.receiver
            .changed()
            .await
            .map_err(|e| neo_core::error::NeoError::Config(e.to_string()))
    }
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_configuration() {
        let config = RuntimeConfiguration::default();
        assert_eq!(config.profile, RuntimeProfile::Development);
        assert!(config.thread_pool.auto_scale);
    }

    #[test]
    fn development_profile() {
        let config = RuntimeConfiguration::development();
        assert_eq!(config.profile, RuntimeProfile::Development);
    }

    #[test]
    fn testing_profile() {
        let config = RuntimeConfiguration::testing();
        assert_eq!(config.profile, RuntimeProfile::Testing);
        assert!(!config.thread_pool.auto_scale);
        assert_eq!(config.scheduler.max_concurrent_tasks, 32);
    }

    #[test]
    fn production_profile() {
        let config = RuntimeConfiguration::production();
        assert_eq!(config.profile, RuntimeProfile::Production);
        assert!(config.thread_pool.auto_scale);
        assert_eq!(config.scheduler.max_concurrent_tasks, 1024);
    }

    #[test]
    fn toml_roundtrip() {
        let config = RuntimeConfiguration::testing();
        let toml_str = config.to_toml().unwrap();
        let restored = RuntimeConfiguration::from_toml(&toml_str).unwrap();
        assert_eq!(restored.profile, config.profile);
        assert_eq!(
            restored.scheduler.max_concurrent_tasks,
            config.scheduler.max_concurrent_tasks
        );
    }

    #[test]
    fn hot_reload_update() {
        let initial = RuntimeConfiguration::development();
        let hot = HotReloadConfig::new(initial);
        assert_eq!(hot.current().profile, RuntimeProfile::Development);

        let updated = RuntimeConfiguration::production();
        hot.update(updated);
        assert_eq!(hot.current().profile, RuntimeProfile::Production);
    }

    #[test]
    fn profile_environment_mapping() {
        assert_eq!(
            RuntimeProfile::Development.environment(),
            Environment::Development
        );
        assert_eq!(
            RuntimeProfile::Production.environment(),
            Environment::Production
        );
    }
}
