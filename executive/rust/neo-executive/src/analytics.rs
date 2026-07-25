use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::context::GlobalState;
use crate::scheduler::SchedulerStats;

/// Executive analytics provides comprehensive monitoring and metrics.
#[derive(Clone)]
pub struct ExecutiveAnalytics {
    inner: Arc<AnalyticsInner>,
}

struct AnalyticsInner {
    task_latencies: RwLock<Vec<(String, f64)>>,
    goal_completions: RwLock<Vec<(String, chrono::DateTime<chrono::Utc>)>>,
    resource_utilization_history: RwLock<Vec<(String, HashMap<String, f64>, chrono::DateTime<chrono::Utc>)>>,
    decision_quality_scores: RwLock<Vec<f64>>,
    scheduler_snapshots: RwLock<Vec<SchedulerStats>>,
    system_snapshots: RwLock<Vec<GlobalState>>,
    start_time: std::time::Instant,
}

/// Comprehensive executive analytics snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsSnapshot {
    pub uptime_ms: u64,
    pub task_latency_avg_ms: f64,
    pub task_latency_p95_ms: f64,
    pub task_latency_p99_ms: f64,
    pub total_goals_completed: u64,
    pub goal_completion_rate: f64,
    pub total_tasks_completed: u64,
    pub task_success_rate: f64,
    pub decision_quality_avg: f64,
    pub scheduler_efficiency: f64,
    pub resource_utilization: HashMap<String, f64>,
    pub active_goals: usize,
    pub active_tasks: usize,
}

/// Task latency statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyStats {
    pub avg_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub count: usize,
}

impl ExecutiveAnalytics {
    /// Create a new analytics instance.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AnalyticsInner {
                task_latencies: RwLock::new(Vec::new()),
                goal_completions: RwLock::new(Vec::new()),
                resource_utilization_history: RwLock::new(Vec::new()),
                decision_quality_scores: RwLock::new(Vec::new()),
                scheduler_snapshots: RwLock::new(Vec::new()),
                system_snapshots: RwLock::new(Vec::new()),
                start_time: std::time::Instant::now(),
            }),
        }
    }

    /// Record task latency.
    pub fn record_task_latency(&self, task_name: &str, latency_ms: f64) {
        self.inner
            .task_latencies
            .write()
            .push((task_name.to_string(), latency_ms));
    }

    /// Record goal completion.
    pub fn record_goal_completion(&self, goal_description: &str) {
        self.inner
            .goal_completions
            .write()
            .push((goal_description.to_string(), Utc::now()));
    }

    /// Record resource utilization.
    pub fn record_resource_utilization(&self, utilization: HashMap<String, f64>) {
        self.inner
            .resource_utilization_history
            .write()
            .push((String::new(), utilization, Utc::now()));
    }

    /// Record decision quality score.
    pub fn record_decision_quality(&self, score: f64) {
        self.inner
            .decision_quality_scores
            .write()
            .push(score.clamp(0.0, 1.0));
    }

    /// Record scheduler snapshot.
    pub fn record_scheduler_snapshot(&self, stats: SchedulerStats) {
        self.inner.scheduler_snapshots.write().push(stats);
    }

    /// Record system state snapshot.
    pub fn record_system_snapshot(&self, state: GlobalState) {
        self.inner.system_snapshots.write().push(state);
    }

    /// Get task latency statistics.
    pub fn task_latency_stats(&self) -> LatencyStats {
        let latencies: Vec<f64> = self
            .inner
            .task_latencies
            .read()
            .iter()
            .map(|(_, l)| *l)
            .collect();

        if latencies.is_empty() {
            return LatencyStats {
                avg_ms: 0.0,
                min_ms: 0.0,
                max_ms: 0.0,
                p50_ms: 0.0,
                p95_ms: 0.0,
                p99_ms: 0.0,
                count: 0,
            };
        }

        let mut sorted = latencies.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let avg = latencies.iter().sum::<f64>() / latencies.len() as f64;
        let min = sorted.first().copied().unwrap_or(0.0);
        let max = sorted.last().copied().unwrap_or(0.0);
        let p50 = percentile(&sorted, 0.50);
        let p95 = percentile(&sorted, 0.95);
        let p99 = percentile(&sorted, 0.99);

        LatencyStats {
            avg_ms: avg,
            min_ms: min,
            max_ms: max,
            p50_ms: p50,
            p95_ms: p95,
            p99_ms: p99,
            count: latencies.len(),
        }
    }

    /// Get goal completion count.
    pub fn goal_completion_count(&self) -> usize {
        self.inner.goal_completions.read().len()
    }

    /// Get decision quality average.
    pub fn decision_quality_average(&self) -> f64 {
        let scores = self.inner.decision_quality_scores.read();
        if scores.is_empty() {
            return 0.0;
        }
        scores.iter().sum::<f64>() / scores.len() as f64
    }

    /// Get the latest scheduler statistics.
    pub fn latest_scheduler_stats(&self) -> Option<SchedulerStats> {
        self.inner.scheduler_snapshots.read().last().cloned()
    }

    /// Get the latest resource utilization.
    pub fn latest_resource_utilization(&self) -> HashMap<String, f64> {
        self.inner
            .resource_utilization_history
            .read()
            .last()
            .map(|(_, u, _)| u.clone())
            .unwrap_or_default()
    }

    /// Generate a comprehensive analytics snapshot.
    pub fn snapshot(
        &self,
        global_state: &GlobalState,
        scheduler_stats: &SchedulerStats,
    ) -> AnalyticsSnapshot {
        let latency_stats = self.task_latency_stats();
        let total_goals = global_state.completed_goals + global_state.failed_goals + global_state.cancelled_goals;
        let goal_rate = if total_goals > 0 {
            global_state.completed_goals as f64 / total_goals as f64
        } else {
            0.0
        };

        let total_tasks = global_state.completed_tasks + global_state.failed_tasks + global_state.cancelled_tasks;
        let task_rate = if total_tasks > 0 {
            global_state.completed_tasks as f64 / total_tasks as f64
        } else {
            0.0
        };

        AnalyticsSnapshot {
            uptime_ms: global_state.uptime_ms,
            task_latency_avg_ms: latency_stats.avg_ms,
            task_latency_p95_ms: latency_stats.p95_ms,
            task_latency_p99_ms: latency_stats.p99_ms,
            total_goals_completed: global_state.completed_goals,
            goal_completion_rate: goal_rate,
            total_tasks_completed: global_state.completed_tasks,
            task_success_rate: task_rate,
            decision_quality_avg: self.decision_quality_average(),
            scheduler_efficiency: if scheduler_stats.total_scheduled > 0 {
                scheduler_stats.total_completed as f64 / scheduler_stats.total_scheduled as f64
            } else {
                0.0
            },
            resource_utilization: global_state.resource_utilization.clone(),
            active_goals: global_state.active_goal_count,
            active_tasks: global_state.active_task_count,
        }
    }

    /// Get uptime in milliseconds.
    pub fn uptime_ms(&self) -> u64 {
        self.inner.start_time.elapsed().as_millis() as u64
    }

    /// Get total recorded task latencies.
    pub fn task_latency_count(&self) -> usize {
        self.inner.task_latencies.read().len()
    }

    /// Get total decision quality samples.
    pub fn decision_quality_count(&self) -> usize {
        self.inner.decision_quality_scores.read().len()
    }

    /// Clear all analytics data.
    pub fn clear(&self) {
        self.inner.task_latencies.write().clear();
        self.inner.goal_completions.write().clear();
        self.inner.resource_utilization_history.write().clear();
        self.inner.decision_quality_scores.write().clear();
        self.inner.scheduler_snapshots.write().clear();
        self.inner.system_snapshots.write().clear();
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = (p * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx]
}

impl Default for ExecutiveAnalytics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latency_stats() {
        let analytics = ExecutiveAnalytics::new();
        analytics.record_task_latency("task1", 100.0);
        analytics.record_task_latency("task2", 200.0);
        analytics.record_task_latency("task3", 150.0);

        let stats = analytics.task_latency_stats();
        assert_eq!(stats.count, 3);
        assert!((stats.avg_ms - 150.0).abs() < f64::EPSILON);
        assert!((stats.min_ms - 100.0).abs() < f64::EPSILON);
        assert!((stats.max_ms - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn goal_completions() {
        let analytics = ExecutiveAnalytics::new();
        analytics.record_goal_completion("goal1");
        analytics.record_goal_completion("goal2");
        assert_eq!(analytics.goal_completion_count(), 2);
    }

    #[test]
    fn decision_quality() {
        let analytics = ExecutiveAnalytics::new();
        analytics.record_decision_quality(0.8);
        analytics.record_decision_quality(0.9);
        let avg = analytics.decision_quality_average();
        assert!((avg - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn snapshot_generation() {
        let analytics = ExecutiveAnalytics::new();
        let state = GlobalState::new();
        let sched_stats = SchedulerStats::default();

        let snap = analytics.snapshot(&state, &sched_stats);
        assert_eq!(snap.total_goals_completed, 0);
        assert_eq!(snap.total_tasks_completed, 0);
    }

    #[test]
    fn percentile_calculation() {
        let sorted = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((percentile(&sorted, 0.5) - 3.0).abs() < f64::EPSILON);
        assert!((percentile(&sorted, 0.95) - 5.0).abs() < f64::EPSILON);
    }
}
