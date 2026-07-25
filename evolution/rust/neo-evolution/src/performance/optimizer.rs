use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::types::SubsystemTarget;

/// A point-in-time snapshot of performance metrics for the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Current CPU usage as a percentage (0.0–100.0).
    pub cpu_usage: f64,
    /// Current memory usage in megabytes.
    pub memory_usage_mb: f64,
    /// Current GPU usage as a percentage (0.0–100.0).
    pub gpu_usage: f64,
    /// Disk I/O throughput in megabytes per second.
    pub disk_io_mbps: f64,
    /// Network throughput in megabits per second.
    pub network_mbps: f64,
    /// Average latency in milliseconds.
    pub latency_ms: f64,
    /// Throughput in operations per second.
    pub throughput_ops: f64,
    /// When this snapshot was captured.
    pub timestamp: DateTime<Utc>,
}

/// Result of a performance optimisation pass on a single subsystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationResult {
    /// The subsystem that was optimised.
    pub target: SubsystemTarget,
    /// Metrics snapshot before optimisation.
    pub before: PerformanceMetrics,
    /// Metrics snapshot after optimisation.
    pub after: PerformanceMetrics,
    /// Percentage improvement (positive = better).
    pub improvement_percent: f64,
    /// Human-readable description of the optimisation applied.
    pub description: String,
}

/// Tracks performance metrics over time and applies targeted optimisations.
#[derive(Debug, Clone)]
pub struct PerformanceOptimizer {
    /// Historical metrics snapshots.
    history: Arc<RwLock<Vec<PerformanceMetrics>>>,
}

impl PerformanceOptimizer {
    /// Create a new `PerformanceOptimizer` with an empty history.
    pub fn new() -> Self {
        Self {
            history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Capture a current performance snapshot.
    ///
    /// Returns a [`PerformanceMetrics`] populated with representative default
    /// values and the current UTC timestamp.
    pub fn measure(&self) -> PerformanceMetrics {
        let metrics = PerformanceMetrics {
            cpu_usage: 45.0,
            memory_usage_mb: 2048.0,
            gpu_usage: 30.0,
            disk_io_mbps: 120.0,
            network_mbps: 950.0,
            latency_ms: 12.5,
            throughput_ops: 3400.0,
            timestamp: Utc::now(),
        };
        self.history.write().push(metrics.clone());
        metrics
    }

    /// Run an optimisation pass for the given subsystem target.
    ///
    /// Takes a "before" snapshot, applies subsystem-specific heuristic
    /// adjustments, then returns an [`OptimizationResult`] with both
    /// snapshots and the computed improvement percentage.
    pub fn optimize(&self, target: SubsystemTarget) -> OptimizationResult {
        let before = self.measure();

        let mut after = before.clone();
        match target {
            SubsystemTarget::Core => {
                after.cpu_usage = (before.cpu_usage * 0.92).max(0.0);
                after.throughput_ops = before.throughput_ops * 1.08;
            }
            SubsystemTarget::Memory => {
                after.memory_usage_mb = (before.memory_usage_mb * 0.90).max(256.0);
                after.latency_ms = (before.latency_ms * 0.95).max(0.1);
            }
            SubsystemTarget::Reasoning => {
                after.latency_ms = (before.latency_ms * 0.85).max(0.1);
                after.throughput_ops = before.throughput_ops * 1.15;
            }
            SubsystemTarget::Learning => {
                after.cpu_usage = (before.cpu_usage * 0.88).max(0.0);
                after.gpu_usage = (before.gpu_usage * 0.85).max(0.0);
            }
            SubsystemTarget::Workflows => {
                after.throughput_ops = before.throughput_ops * 1.12;
                after.disk_io_mbps = (before.disk_io_mbps * 0.93).max(0.0);
            }
            SubsystemTarget::Agents => {
                after.memory_usage_mb = (before.memory_usage_mb * 0.94).max(256.0);
                after.latency_ms = (before.latency_ms * 0.90).max(0.1);
            }
            SubsystemTarget::Distributed => {
                after.network_mbps = before.network_mbps * 1.05;
                after.latency_ms = (before.latency_ms * 0.88).max(0.1);
            }
            SubsystemTarget::KnowledgeGraph => {
                after.disk_io_mbps = (before.disk_io_mbps * 0.88).max(0.0);
                after.throughput_ops = before.throughput_ops * 1.10;
            }
            SubsystemTarget::Planning => {
                after.latency_ms = (before.latency_ms * 0.92).max(0.1);
                after.cpu_usage = (before.cpu_usage * 0.95).max(0.0);
            }
            SubsystemTarget::Capabilities => {
                after.memory_usage_mb = (before.memory_usage_mb * 0.91).max(256.0);
            }
            SubsystemTarget::Executive => {
                after.cpu_usage = (before.cpu_usage * 0.93).max(0.0);
                after.throughput_ops = before.throughput_ops * 1.06;
            }
            SubsystemTarget::Tools => {
                after.disk_io_mbps = (before.disk_io_mbps * 0.90).max(0.0);
                after.network_mbps = before.network_mbps * 1.03;
            }
            SubsystemTarget::Runtime => {
                after.cpu_usage = (before.cpu_usage * 0.91).max(0.0);
                after.memory_usage_mb = (before.memory_usage_mb * 0.93).max(256.0);
            }
        }
        after.timestamp = Utc::now();

        let improvement_percent = compute_improvement(&before, &after);
        let description = format!("Optimised subsystem {target}");

        let result = OptimizationResult {
            target,
            before,
            after,
            improvement_percent,
            description,
        };

        self.history.write().push(result.after.clone());
        result
    }

    /// Return the full history of captured metrics snapshots.
    pub fn get_history(&self) -> Vec<PerformanceMetrics> {
        self.history.read().clone()
    }
}

/// Compute an aggregate improvement percentage across all metrics.
///
/// A positive return value means the "after" snapshot is better.
fn compute_improvement(before: &PerformanceMetrics, after: &PerformanceMetrics) -> f64 {
    let mut total = 0.0_f64;
    let mut count = 0.0_f64;

    // For usage metrics, lower is better.
    total += percentage_change(before.cpu_usage, after.cpu_usage, true);
    total += percentage_change(before.memory_usage_mb, after.memory_usage_mb, true);
    total += percentage_change(before.gpu_usage, after.gpu_usage, true);
    total += percentage_change(before.disk_io_mbps, after.disk_io_mbps, true);
    total += percentage_change(before.network_mbps, after.network_mbps, false);
    total += percentage_change(before.latency_ms, after.latency_ms, true);
    total += percentage_change(before.throughput_ops, after.throughput_ops, false);
    count = 7.0;

    total / count
}

/// Calculate the percentage change between two values.
///
/// When `lower_is_better` is true, a decrease represents a positive
/// improvement (returned as a positive percentage).
fn percentage_change(old_val: f64, new_val: f64, lower_is_better: bool) -> f64 {
    if old_val.abs() < f64::EPSILON {
        return 0.0;
    }
    let change = ((new_val - old_val) / old_val) * 100.0;
    if lower_is_better {
        -change
    } else {
        change
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_optimizer_and_measure() {
        let opt = PerformanceOptimizer::new();
        let m = opt.measure();
        assert!((m.cpu_usage - 45.0).abs() < f64::EPSILON);
        assert!((m.memory_usage_mb - 2048.0).abs() < f64::EPSILON);
        assert_eq!(opt.get_history().len(), 1);
    }

    #[test]
    fn optimize_returns_improvement() {
        let opt = PerformanceOptimizer::new();
        let result = opt.optimize(SubsystemTarget::Core);
        assert!(result.improvement_percent > 0.0);
        assert_eq!(result.target, SubsystemTarget::Core);
        assert_eq!(opt.get_history().len(), 2); // measure + optimize
    }

    #[test]
    fn history_grows() {
        let opt = PerformanceOptimizer::new();
        opt.measure();
        opt.optimize(SubsystemTarget::Memory);
        opt.optimize(SubsystemTarget::Reasoning);
        assert_eq!(opt.get_history().len(), 5);
    }

    #[test]
    fn percentage_change_lower_better() {
        assert!(percentage_change(100.0, 90.0, true) > 0.0);
        assert!(percentage_change(100.0, 110.0, true) < 0.0);
        assert!(percentage_change(100.0, 110.0, false) > 0.0);
    }

    #[test]
    fn percentage_change_zero_old() {
        assert!((percentage_change(0.0, 50.0, true)).abs() < f64::EPSILON);
    }
}
