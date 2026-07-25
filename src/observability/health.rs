/// Health checking subsystem for the Neo AGI Operating System.
///
/// Provides a registry-based health checker that can monitor the health
/// status of individual subsystems and derive an overall system health status.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Represents the health status of a subsystem.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealthStatus {
    /// The subsystem is operating normally.
    Healthy,
    /// The subsystem is experiencing degraded performance.
    Degraded,
    /// The subsystem is non-functional.
    Unhealthy,
}

impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "Healthy"),
            HealthStatus::Degraded => write!(f, "Degraded"),
            HealthStatus::Unhealthy => write!(f, "Unhealthy"),
        }
    }
}

/// Health check result for an individual subsystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsystemHealthCheck {
    /// Name of the subsystem.
    pub name: String,
    /// Current health status.
    pub status: HealthStatus,
    /// Latency of the health check in milliseconds.
    pub latency_ms: f64,
    /// Optional diagnostic message.
    pub message: String,
    /// Timestamp when this check was performed (Unix epoch seconds).
    pub last_checked: u64,
}

/// A health check function that returns the subsystem's status.
pub type HealthCheckFn = Box<dyn Fn() -> SubsystemHealthCheck + Send + Sync>;

/// Registry of subsystem health check functions.
struct SubsystemEntry {
    check_fn: HealthCheckFn,
}

/// Configuration for how overall health is derived from subsystem statuses.
#[derive(Debug, Clone)]
pub struct HealthConfig {
    /// If any subsystem is unhealthy, the overall status is Unhealthy.
    pub unhealthy_threshold: bool,
    /// If any subsystem is degraded (and none are unhealthy), overall is Degraded.
    pub degraded_threshold: bool,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            unhealthy_threshold: true,
            degraded_threshold: true,
        }
    }
}

/// Manages health checks for all registered subsystems.
///
/// Uses a `tokio::sync::RwLock` internally for thread-safe concurrent access.
pub struct HealthChecker {
    subsystems: RwLock<HashMap<String, SubsystemEntry>>,
    config: HealthConfig,
}

impl HealthChecker {
    /// Creates a new `HealthChecker` with default configuration.
    pub fn new() -> Self {
        Self {
            subsystems: RwLock::new(HashMap::new()),
            config: HealthConfig::default(),
        }
    }

    /// Creates a new `HealthChecker` with custom configuration.
    pub fn with_config(config: HealthConfig) -> Self {
        Self {
            subsystems: RwLock::new(HashMap::new()),
            config,
        }
    }

    /// Registers a subsystem with a synchronous health check function.
    ///
    /// The provided closure will be called each time a health check is
    /// requested for this subsystem.
    pub async fn register_subsystem(
        &self,
        name: impl Into<String>,
        check_fn: impl Fn() -> SubsystemHealthCheck + Send + Sync + 'static,
    ) {
        let mut subsystems = self.subsystems.write().await;
        subsystems.insert(
            name.into(),
            SubsystemEntry {
                check_fn: Box::new(check_fn),
            },
        );
    }

    /// Registers a subsystem with a predefined static status.
    ///
    /// Useful for subsystems where the health status is tracked externally.
    pub async fn register_static(
        &self,
        name: impl Into<String>,
        status: HealthStatus,
        message: impl Into<String>,
    ) {
        let name = name.into();
        let message = message.into();
        self.register_subsystem(name.clone(), move || SubsystemHealthCheck {
            name: name.clone(),
            status: status.clone(),
            latency_ms: 0.0,
            message: message.clone(),
            last_checked: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        })
        .await;
    }

    /// Performs a health check on a single named subsystem.
    ///
    /// Returns `None` if the subsystem is not registered.
    pub async fn check_subsystem(&self, name: &str) -> Option<SubsystemHealthCheck> {
        let subsystems = self.subsystems.read().await;
        subsystems.get(name).map(|entry| (entry.check_fn)())
    }

    /// Performs health checks on all registered subsystems.
    ///
    /// Returns a vector of `SubsystemHealthCheck` results for every registered subsystem.
    pub async fn check_all(&self) -> Vec<SubsystemHealthCheck> {
        let subsystems = self.subsystems.read().await;
        subsystems.values().map(|entry| (entry.check_fn)()).collect()
    }

    /// Derives the overall system health status from the latest subsystem checks.
    ///
    /// - If any subsystem is `Unhealthy`, returns `Unhealthy`.
    /// - If any subsystem is `Degraded`, returns `Degraded`.
    /// - Otherwise returns `Healthy`.
    pub async fn get_overall_status(&self) -> HealthStatus {
        let results = self.check_all().await;

        if results.is_empty() {
            return HealthStatus::Healthy;
        }

        if self.config.unhealthy_threshold
            && results.iter().any(|r| r.status == HealthStatus::Unhealthy)
        {
            return HealthStatus::Unhealthy;
        }

        if self.config.degraded_threshold
            && results.iter().any(|r| r.status == HealthStatus::Degraded)
        {
            return HealthStatus::Degraded;
        }

        HealthStatus::Healthy
    }

    /// Returns the number of registered subsystems.
    pub async fn subsystem_count(&self) -> usize {
        let subsystems = self.subsystems.read().await;
        subsystems.len()
    }

    /// Returns a list of all registered subsystem names.
    pub async fn registered_subsystems(&self) -> Vec<String> {
        let subsystems = self.subsystems.read().await;
        subsystems.keys().cloned().collect()
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}
