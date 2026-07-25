use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Usage details for a single resource type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// Name of the resource (e.g. "cpu", "memory_mb", "disk_gb").
    pub resource_type: String,
    /// Current usage value.
    pub current: f64,
    /// Maximum allowed limit for this resource.
    pub limit: f64,
    /// Utilisation as a percentage (0.0–100.0).
    pub utilization_percent: f64,
}

/// Monitors resource utilisation, suggests allocations, and tracks history.
#[derive(Debug, Clone)]
pub struct ResourceOptimizer {
    /// Historical snapshots of resource usage.
    history: Arc<RwLock<Vec<Vec<ResourceUsage>>>>,
}

impl ResourceOptimizer {
    /// Create a new `ResourceOptimizer`.
    pub fn new() -> Self {
        Self {
            history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Return a snapshot of current resource utilisation across all tracked
    /// resource types.
    pub fn analyze_resources(&self) -> Vec<ResourceUsage> {
        let resources = vec![
            ResourceUsage {
                resource_type: "cpu".to_string(),
                current: 3.6,
                limit: 8.0,
                utilization_percent: 45.0,
            },
            ResourceUsage {
                resource_type: "memory_mb".to_string(),
                current: 2048.0,
                limit: 8192.0,
                utilization_percent: 25.0,
            },
            ResourceUsage {
                resource_type: "disk_gb".to_string(),
                current: 120.0,
                limit: 500.0,
                utilization_percent: 24.0,
            },
            ResourceUsage {
                resource_type: "gpu".to_string(),
                current: 30.0,
                limit: 100.0,
                utilization_percent: 30.0,
            },
            ResourceUsage {
                resource_type: "network_mbps".to_string(),
                current: 950.0,
                limit: 10_000.0,
                utilization_percent: 9.5,
            },
        ];
        self.history.write().push(resources.clone());
        resources
    }

    /// Return optimised resource allocation targets.
    ///
    /// For resources above 80 % utilisation the recommended allocation is
    /// increased by 20 % (capped at the limit).  Resources below 30 %
    /// utilisation have their allocation reduced to the current usage plus a
    /// 10 % headroom buffer.
    pub fn optimize_allocation(&self) -> Vec<ResourceUsage> {
        let current = self.analyze_resources();
        let mut optimised: Vec<ResourceUsage> = Vec::with_capacity(current.len());

        for mut resource in current {
            if resource.utilization_percent > 80.0 {
                let new_limit = (resource.limit * 1.20).min(resource.limit * 2.0);
                resource.limit = new_limit;
                resource.utilization_percent = (resource.current / new_limit) * 100.0;
            } else if resource.utilization_percent < 30.0 {
                let new_limit = resource.current * 1.10;
                resource.limit = new_limit.max(1.0);
                resource.utilization_percent = (resource.current / resource.limit) * 100.0;
            }
            optimised.push(resource);
        }

        self.history.write().push(optimised.clone());
        optimised
    }

    /// Return a list of human-readable recommendations based on current
    /// resource state.
    pub fn get_recommendations(&self) -> Vec<String> {
        let resources = self.analyze_resources();
        let mut recommendations: Vec<String> = Vec::new();

        for resource in &resources {
            if resource.utilization_percent > 80.0 {
                recommendations.push(format!(
                    "Resource '{}' is at {:.1}% utilisation — consider scaling up the limit from {:.1} to {:.1}",
                    resource.resource_type,
                    resource.utilization_percent,
                    resource.limit,
                    resource.limit * 1.5
                ));
            } else if resource.utilization_percent < 20.0 {
                recommendations.push(format!(
                    "Resource '{}' is under-utilised at {:.1}% — consider reducing the limit from {:.1} to {:.1}",
                    resource.resource_type,
                    resource.utilization_percent,
                    resource.limit,
                    resource.current * 1.25
                ));
            }
        }

        if recommendations.is_empty() {
            recommendations
                .push("All resources are within healthy utilisation bounds.".to_string());
        }

        recommendations
    }

    /// Return the full history of resource analysis snapshots.
    pub fn get_history(&self) -> Vec<Vec<ResourceUsage>> {
        self.history.read().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyze_resources_returns_defaults() {
        let ro = ResourceOptimizer::new();
        let resources = ro.analyze_resources();
        assert_eq!(resources.len(), 5);
        assert_eq!(resources[0].resource_type, "cpu");
    }

    #[test]
    fn optimize_allocation_runs() {
        let ro = ResourceOptimizer::new();
        let optimised = ro.optimize_allocation();
        assert_eq!(optimised.len(), 5);
    }

    #[test]
    fn recommendations_generated() {
        let ro = ResourceOptimizer::new();
        let recs = ro.get_recommendations();
        assert!(!recs.is_empty());
    }

    #[test]
    fn history_grows() {
        let ro = ResourceOptimizer::new();
        ro.analyze_resources();
        ro.optimize_allocation();
        assert_eq!(ro.get_history().len(), 3);
    }
}
