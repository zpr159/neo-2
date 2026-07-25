/// Observability module for the Neo AGI Operating System.
///
/// Provides metrics collection, health checking, and tracing capabilities
/// for monitoring system state and performance.

pub mod health;
pub mod metrics;
pub mod tracing_setup;

use crate::component::{Component, ComponentState};
use crate::error::NeoResult;
use health::{HealthChecker, SubsystemHealthCheck};
use metrics::{AggregatedMetrics, MetricsCollector};
use tracing_setup::TracingSetup;

/// The central observability manager that ties together metrics, health, and tracing.
pub struct ObservabilityManager {
    /// The health checker instance.
    pub health_checker: HealthChecker,
    /// The metrics collector instance.
    pub metrics_collector: MetricsCollector,
    /// The tracing setup instance.
    pub tracing_setup: TracingSetup,
    /// Unique identifier for this node.
    pub node_id: String,
}

impl ObservabilityManager {
    /// Creates a new `ObservabilityManager` with default configurations.
    ///
    /// # Arguments
    ///
    /// * `node_id` - A unique identifier for this node in the system.
    pub fn new(node_id: impl Into<String>) -> Self {
        Self {
            health_checker: HealthChecker::new(),
            metrics_collector: MetricsCollector::new(),
            tracing_setup: TracingSetup::new(),
            node_id: node_id.into(),
        }
    }

    /// Initializes the tracing subsystem with the default configuration.
    pub fn init_tracing(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.tracing_setup.init()
    }

    /// Initializes the tracing subsystem with a custom configuration.
    pub fn init_tracing_with_config(
        &self,
        config: tracing_setup::TracingConfig,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.tracing_setup.init_with_config(config)
    }

    /// Retrieves the current health status across all registered subsystems.
    pub async fn get_health(&self) -> Vec<SubsystemHealthCheck> {
        self.health_checker.check_all().await
    }

    /// Collects and returns the current aggregated metrics snapshot.
    pub fn collect_metrics(&self) -> AggregatedMetrics {
        self.metrics_collector.collect(self.node_id.clone())
    }

    /// Returns a reference to the metrics collector for direct metric updates.
    pub fn metrics(&self) -> &MetricsCollector {
        &self.metrics_collector
    }

    /// Returns a reference to the health checker for subsystem registration.
    pub fn health(&self) -> &HealthChecker {
        &self.health_checker
    }
}

impl Component for ObservabilityManager {
    fn name(&self) -> &str {
        "ObservabilityManager"
    }

    fn state(&self) -> ComponentState {
        ComponentState::Running
    }

    async fn initialize(&mut self) -> NeoResult<()> {
        self.init_tracing()
            .map_err(|e| crate::error::NeoError::Config(e.to_string()))?;
        tracing::info!("ObservabilityManager initialized for node: {}", self.node_id);
        Ok(())
    }

    async fn start(&mut self) -> NeoResult<()> {
        tracing::info!("ObservabilityManager started for node: {}", self.node_id);
        Ok(())
    }

    async fn stop(&mut self) -> NeoResult<()> {
        tracing::info!("ObservabilityManager stopping for node: {}", self.node_id);
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
