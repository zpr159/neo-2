use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::{CapabilityId, ResourceRequirements};

/// A single execution metric entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetric {
    pub execution_id: Uuid,
    pub capability_id: CapabilityId,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub success: bool,
    pub duration_ms: u64,
    pub resources_used: ResourceRequirements,
    pub error: Option<String>,
}

impl ExecutionMetric {
    /// Create a new execution metric.
    pub fn new(capability_id: CapabilityId) -> Self {
        Self {
            execution_id: Uuid::new_v4(),
            capability_id,
            started_at: Utc::now(),
            completed_at: None,
            success: false,
            duration_ms: 0,
            resources_used: ResourceRequirements::default(),
            error: None,
        }
    }

    /// Mark execution as completed.
    pub fn complete(mut self, success: bool, duration_ms: u64) -> Self {
        self.completed_at = Some(Utc::now());
        self.success = success;
        self.duration_ms = duration_ms;
        self
    }

    /// Set an error.
    pub fn with_error(mut self, error: String) -> Self {
        self.error = Some(error);
        self.success = false;
        self
    }

    /// Get duration.
    pub fn duration(&self) -> Duration {
        Duration::milliseconds(self.duration_ms as i64)
    }
}

/// Latency statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LatencyStats {
    pub min_ms: u64,
    pub max_ms: u64,
    pub avg_ms: f64,
    pub p50_ms: u64,
    pub p95_ms: u64,
    pub p99_ms: u64,
    pub total_count: u64,
}

impl LatencyStats {
    /// Create empty latency stats.
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate from a list of durations.
    pub fn from_durations(durations: &[u64]) -> Self {
        if durations.is_empty() {
            return Self::default();
        }

        let mut sorted = durations.to_vec();
        sorted.sort_unstable();

        let sum: u64 = sorted.iter().sum();
        let count = sorted.len() as u64;

        Self {
            min_ms: sorted[0],
            max_ms: sorted[sorted.len() - 1],
            avg_ms: sum as f64 / count as f64,
            p50_ms: percentile(&sorted, 50.0),
            p95_ms: percentile(&sorted, 95.0),
            p99_ms: percentile(&sorted, 99.0),
            total_count: count,
        }
    }
}

fn percentile(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((p / 100.0) * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

/// Analytics for a single capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityAnalytics {
    pub capability_id: CapabilityId,
    pub total_executions: u64,
    pub successful: u64,
    pub failed: u64,
    pub cancelled: u64,
    pub avg_duration_ms: f64,
    pub latency: LatencyStats,
    pub total_cpu_time_ms: u64,
    pub total_gpu_time_ms: u64,
    pub total_memory_bytes: u64,
    pub total_inference_tokens: u32,
    pub first_executed: Option<DateTime<Utc>>,
    pub last_executed: Option<DateTime<Utc>>,
    pub errors: HashMap<String, u64>,
    pub hourly_execution_count: HashMap<String, u64>,
}

impl CapabilityAnalytics {
    /// Create new analytics for a capability.
    pub fn new(capability_id: CapabilityId) -> Self {
        Self {
            capability_id,
            total_executions: 0,
            successful: 0,
            failed: 0,
            cancelled: 0,
            avg_duration_ms: 0.0,
            latency: LatencyStats::new(),
            total_cpu_time_ms: 0,
            total_gpu_time_ms: 0,
            total_memory_bytes: 0,
            total_inference_tokens: 0,
            first_executed: None,
            last_executed: None,
            errors: HashMap::new(),
            hourly_execution_count: HashMap::new(),
        }
    }

    /// Record an execution.
    pub fn record_execution(&mut self, metric: &ExecutionMetric) {
        self.total_executions += 1;
        if metric.success {
            self.successful += 1;
        } else {
            self.failed += 1;
        }

        self.avg_duration_ms = ((self.avg_duration_ms * (self.total_executions - 1) as f64)
            + metric.duration_ms as f64)
            / self.total_executions as f64;

        if let Some(err) = &metric.error {
            *self.errors.entry(err.clone()).or_insert(0) += 1;
        }

        self.total_cpu_time_ms += metric.resources_used.cpu_units as u64;
        self.total_memory_bytes += metric.resources_used.memory_bytes;
        self.total_inference_tokens += metric.resources_used.inference_tokens;

        if self.first_executed.is_none() {
            self.first_executed = Some(metric.started_at);
        }
        self.last_executed = Some(metric.started_at);

        let hour_key = metric.started_at.format("%Y-%m-%d-%H").to_string();
        *self.hourly_execution_count.entry(hour_key).or_insert(0) += 1;
    }

    /// Get success rate (0.0 - 1.0).
    pub fn success_rate(&self) -> f64 {
        if self.total_executions == 0 {
            return 0.0;
        }
        self.successful as f64 / self.total_executions as f64
    }

    /// Get failure rate (0.0 - 1.0).
    pub fn failure_rate(&self) -> f64 {
        if self.total_executions == 0 {
            return 0.0;
        }
        self.failed as f64 / self.total_executions as f64
    }

    /// Get top errors.
    pub fn top_errors(&self, limit: usize) -> Vec<(String, u64)> {
        let mut errors: Vec<_> = self.errors.iter().map(|(k, v)| (k.clone(), *v)).collect();
        errors.sort_by(|a, b| b.1.cmp(&a.1));
        errors.into_iter().take(limit).collect()
    }

    /// Get peak hours.
    pub fn peak_hours(&self, limit: usize) -> Vec<(String, u64)> {
        let mut hours: Vec<_> = self
            .hourly_execution_count
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        hours.sort_by(|a, b| b.1.cmp(&a.1));
        hours.into_iter().take(limit).collect()
    }

    /// Calculate popularity score (0.0 - 100.0).
    pub fn popularity_score(&self) -> f64 {
        if self.total_executions == 0 {
            return 0.0;
        }
        let frequency_score = (self.total_executions as f64).min(1000.0) / 10.0;
        let success_bonus = self.success_rate() * 50.0;
        (frequency_score + success_bonus).min(100.0)
    }

    /// Reset all analytics.
    pub fn reset(&mut self) {
        self.total_executions = 0;
        self.successful = 0;
        self.failed = 0;
        self.cancelled = 0;
        self.avg_duration_ms = 0.0;
        self.latency = LatencyStats::new();
        self.total_cpu_time_ms = 0;
        self.total_gpu_time_ms = 0;
        self.total_memory_bytes = 0;
        self.total_inference_tokens = 0;
        self.first_executed = None;
        self.last_executed = None;
        self.errors.clear();
        self.hourly_execution_count.clear();
    }
}

/// Sort metric for ranking capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMetric {
    Executions,
    SuccessRate,
    Popularity,
    AvgLatency,
}

/// Global statistics across all capabilities.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlobalStats {
    pub total_executions: u64,
    pub total_successful: u64,
    pub total_failed: u64,
    pub total_cancelled: u64,
    pub avg_duration_ms: f64,
    pub total_unique_capabilities: usize,
    pub overall_success_rate: f64,
}

/// Analytics store for all capabilities.
pub struct CapabilityAnalyticsStore {
    capabilities: RwLock<HashMap<CapabilityId, CapabilityAnalytics>>,
    global_metrics: RwLock<Vec<ExecutionMetric>>,
}

impl CapabilityAnalyticsStore {
    /// Create a new analytics store.
    pub fn new() -> Self {
        Self {
            capabilities: RwLock::new(HashMap::new()),
            global_metrics: RwLock::new(Vec::new()),
        }
    }

    /// Record an execution metric.
    pub fn record_execution(&self, metric: ExecutionMetric) {
        let cap_id = metric.capability_id;
        {
            let mut caps = self.capabilities.write();
            let analytics = caps.entry(cap_id).or_insert_with(|| CapabilityAnalytics::new(cap_id));
            analytics.record_execution(&metric);
        }
        self.global_metrics.write().push(metric);
    }

    /// Get analytics for a capability.
    pub fn get_analytics(&self, capability_id: CapabilityId) -> CapabilityAnalytics {
        self.capabilities
            .read()
            .get(&capability_id)
            .cloned()
            .unwrap_or_else(|| CapabilityAnalytics::new(capability_id))
    }

    /// Get all analytics.
    pub fn get_all_analytics(&self) -> Vec<CapabilityAnalytics> {
        self.capabilities.read().values().cloned().collect()
    }

    /// Get global stats.
    pub fn global_stats(&self) -> GlobalStats {
        let metrics = self.global_metrics.read();
        let total = metrics.len() as u64;
        let successful = metrics.iter().filter(|m| m.success).count() as u64;
        let failed = metrics.iter().filter(|m| !m.success).count() as u64;
        let total_duration: u64 = metrics.iter().map(|m| m.duration_ms).sum();

        GlobalStats {
            total_executions: total,
            total_successful: successful,
            total_failed: failed,
            total_cancelled: 0,
            avg_duration_ms: if total > 0 {
                total_duration as f64 / total as f64
            } else {
                0.0
            },
            total_unique_capabilities: self.capabilities.read().len(),
            overall_success_rate: if total > 0 {
                successful as f64 / total as f64
            } else {
                0.0
            },
        }
    }

    /// Get top capabilities by metric.
    pub fn top_capabilities(&self, limit: usize, sort_by: SortMetric) -> Vec<CapabilityId> {
        let caps = self.capabilities.read();
        let mut entries: Vec<_> = caps.iter().collect();

        match sort_by {
            SortMetric::Executions => {
                entries.sort_by(|a, b| b.1.total_executions.cmp(&a.1.total_executions));
            }
            SortMetric::SuccessRate => {
                entries.sort_by(|a, b| {
                    b.1.success_rate()
                        .partial_cmp(&a.1.success_rate())
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SortMetric::Popularity => {
                entries.sort_by(|a, b| {
                    b.1.popularity_score()
                        .partial_cmp(&a.1.popularity_score())
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SortMetric::AvgLatency => {
                entries.sort_by(|a, b| {
                    a.1.avg_duration_ms
                        .partial_cmp(&b.1.avg_duration_ms)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }

        entries.into_iter().take(limit).map(|(k, _)| *k).collect()
    }

    /// Get failed executions.
    pub fn failed_executions(&self, limit: usize) -> Vec<ExecutionMetric> {
        self.global_metrics
            .read()
            .iter()
            .filter(|m| !m.success)
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get executions in time range.
    pub fn time_range_executions(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<ExecutionMetric> {
        self.global_metrics
            .read()
            .iter()
            .filter(|m| m.started_at >= start && m.started_at <= end)
            .cloned()
            .collect()
    }

    /// Cleanup old entries.
    pub fn cleanup_old_entries(&self, older_than_days: u64) {
        let cutoff = Utc::now() - Duration::days(older_than_days as i64);
        self.global_metrics
            .write()
            .retain(|m| m.started_at > cutoff);
    }

    /// Total metrics count.
    pub fn total_metrics(&self) -> usize {
        self.global_metrics.read().len()
    }
}

impl Default for CapabilityAnalyticsStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execution_metric_creation() {
        let metric = ExecutionMetric::new(CapabilityId::new())
            .complete(true, 100);
        assert!(metric.success);
        assert_eq!(metric.duration_ms, 100);
    }

    #[test]
    fn latency_stats_empty() {
        let stats = LatencyStats::from_durations(&[]);
        assert_eq!(stats.total_count, 0);
    }

    #[test]
    fn latency_stats_percentiles() {
        let mut data = Vec::new();
        for i in 1..=100 {
            data.push(i);
        }
        let stats = LatencyStats::from_durations(&data);
        assert_eq!(stats.min_ms, 1);
        assert_eq!(stats.max_ms, 100);
        assert!((stats.avg_ms - 50.5).abs() < 0.01);
    }

    #[test]
    fn capability_analytics() {
        let cap_id = CapabilityId::new();
        let mut analytics = CapabilityAnalytics::new(cap_id);

        let metric = ExecutionMetric::new(cap_id).complete(true, 50);
        analytics.record_execution(&metric);

        assert_eq!(analytics.total_executions, 1);
        assert_eq!(analytics.successful, 1);
        assert!((analytics.success_rate() - 1.0).abs() < 0.01);
    }

    #[test]
    fn analytics_store() {
        let store = CapabilityAnalyticsStore::new();
        let cap_id = CapabilityId::new();
        let metric = ExecutionMetric::new(cap_id).complete(true, 100);
        store.record_execution(metric);

        assert_eq!(store.total_metrics(), 1);
        let analytics = store.get_analytics(cap_id);
        assert_eq!(analytics.total_executions, 1);
    }

    #[test]
    fn global_stats() {
        let store = CapabilityAnalyticsStore::new();
        let cap_id = CapabilityId::new();
        store.record_execution(ExecutionMetric::new(cap_id).complete(true, 100));
        store.record_execution(ExecutionMetric::new(cap_id).complete(false, 200));

        let stats = store.global_stats();
        assert_eq!(stats.total_executions, 2);
        assert!((stats.overall_success_rate - 0.5).abs() < 0.01);
    }

    #[test]
    fn top_capabilities() {
        let store = CapabilityAnalyticsStore::new();
        let cap1 = CapabilityId::new();
        let cap2 = CapabilityId::new();

        for _ in 0..10 {
            store.record_execution(ExecutionMetric::new(cap1).complete(true, 50));
        }
        for _ in 0..5 {
            store.record_execution(ExecutionMetric::new(cap2).complete(true, 100));
        }

        let top = store.top_capabilities(10, SortMetric::Executions);
        assert_eq!(top[0], cap1);
    }
}
