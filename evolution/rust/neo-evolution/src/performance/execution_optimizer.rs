use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Execution profile for a single named operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionProfile {
    /// Name of the operation being profiled.
    pub operation: String,
    /// Average duration in milliseconds.
    pub avg_duration_ms: f64,
    /// 99th-percentile duration in milliseconds.
    pub p99_duration_ms: f64,
    /// Total number of times the operation was recorded.
    pub call_count: u64,
    /// Cumulative duration in milliseconds across all recorded calls.
    pub total_duration_ms: f64,
}

/// Profiles execution times for named operations and surfaces slow paths.
#[derive(Debug, Clone)]
pub struct ExecutionOptimizer {
    /// Recorded profiles keyed by operation name.
    profiles: Arc<RwLock<HashMap<String, ExecutionProfile>>>,
}

impl ExecutionOptimizer {
    /// Create a new `ExecutionOptimizer` with no recorded profiles.
    pub fn new() -> Self {
        Self {
            profiles: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record a single execution of the named operation.
    ///
    /// The duration is incorporated into the rolling profile for `operation`.
    pub fn profile_operation(&self, operation: &str, duration_ms: f64) {
        let mut profiles = self.profiles.write();
        let entry = profiles
            .entry(operation.to_string())
            .or_insert_with(|| ExecutionProfile {
                operation: operation.to_string(),
                avg_duration_ms: 0.0,
                p99_duration_ms: 0.0,
                call_count: 0,
                total_duration_ms: 0.0,
            });

        entry.call_count += 1;
        entry.total_duration_ms += duration_ms;
        entry.avg_duration_ms = entry.total_duration_ms / entry.call_count as f64;
        // Approximate p99 as max observed so far (conservative upper bound).
        if duration_ms > entry.p99_duration_ms {
            entry.p99_duration_ms = duration_ms;
        }
    }

    /// Return the `n` slowest operations by average duration.
    pub fn get_slowest(&self, n: usize) -> Vec<ExecutionProfile> {
        let profiles = self.profiles.read();
        let mut sorted: Vec<ExecutionProfile> = profiles.values().cloned().collect();
        sorted.sort_by(|a, b| {
            b.avg_duration_ms
                .partial_cmp(&a.avg_duration_ms)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.into_iter().take(n).collect()
    }

    /// Analyse recorded profiles and return optimisation suggestions.
    pub fn optimize(&self) -> Vec<String> {
        let profiles = self.profiles.read();
        let mut suggestions: Vec<String> = Vec::new();

        for profile in profiles.values() {
            if profile.avg_duration_ms > 100.0 {
                suggestions.push(format!(
                    "Operation '{}' averages {:.1}ms — consider caching or batching (p99: {:.1}ms)",
                    profile.operation, profile.avg_duration_ms, profile.p99_duration_ms
                ));
            } else if profile.p99_duration_ms > profile.avg_duration_ms * 5.0 {
                suggestions.push(format!(
                    "Operation '{}' has high tail latency: p99 {:.1}ms vs avg {:.1}ms — investigate outlier cases",
                    profile.operation, profile.p99_duration_ms, profile.avg_duration_ms
                ));
            }
            if profile.call_count > 10_000 && profile.avg_duration_ms > 10.0 {
                suggestions.push(format!(
                    "Operation '{}' is called {} times with {:.1}ms avg — high-frequency hot path, consider optimising",
                    profile.operation, profile.call_count, profile.avg_duration_ms
                ));
            }
        }

        if suggestions.is_empty() {
            suggestions.push(
                "All profiled operations are within acceptable performance bounds.".to_string(),
            );
        }

        suggestions
    }

    /// Return a clone of all recorded execution profiles.
    pub fn get_profiles(&self) -> Vec<ExecutionProfile> {
        self.profiles.read().values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_operation_updates_stats() {
        let eo = ExecutionOptimizer::new();
        eo.profile_operation("foo", 10.0);
        eo.profile_operation("foo", 20.0);
        eo.profile_operation("foo", 5.0);

        let profiles = eo.get_profiles();
        assert_eq!(profiles.len(), 1);
        let p = &profiles[0];
        assert_eq!(p.call_count, 3);
        assert!((p.avg_duration_ms - 11.666_666_666_666_666).abs() < 0.01);
        assert!((p.p99_duration_ms - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn get_slowest_respects_n() {
        let eo = ExecutionOptimizer::new();
        eo.profile_operation("fast", 1.0);
        eo.profile_operation("slow", 100.0);
        eo.profile_operation("medium", 50.0);

        let slowest = eo.get_slowest(2);
        assert_eq!(slowest.len(), 2);
        assert_eq!(slowest[0].operation, "slow");
        assert_eq!(slowest[1].operation, "medium");
    }

    #[test]
    fn optimize_suggests_for_slow_ops() {
        let eo = ExecutionOptimizer::new();
        eo.profile_operation("slow_op", 200.0);
        let suggestions = eo.optimize();
        assert!(suggestions.iter().any(|s| s.contains("slow_op")));
    }
}
