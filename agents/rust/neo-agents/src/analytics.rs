use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::{AgentId, AgentMetrics, AgentStatistics};

// ---------------------------------------------------------------------------
// AgentAnalytics
// ---------------------------------------------------------------------------

/// Analytics data for a single agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAnalytics {
    /// Agent identifier.
    pub agent_id: AgentId,
    /// Agent name.
    pub agent_name: String,
    /// Current metrics.
    pub metrics: AgentMetrics,
    /// Task completion rate (0.0 to 1.0).
    pub task_completion_rate: f64,
    /// Average task duration in milliseconds.
    pub avg_task_duration_ms: f64,
    /// Message throughput (messages per second).
    pub message_throughput: f64,
    /// Error rate (errors per task).
    pub error_rate: f64,
    /// Recovery count.
    pub recovery_count: u32,
    /// Uptime in seconds.
    pub uptime_secs: u64,
    /// When this analytics snapshot was taken.
    pub snapshot_at: DateTime<Utc>,
}

impl AgentAnalytics {
    /// Calculate analytics from agent metrics.
    #[must_use]
    pub fn from_metrics(agent_id: AgentId, agent_name: String, metrics: &AgentMetrics) -> Self {
        let total_tasks = metrics.tasks_completed + metrics.tasks_failed;
        let task_completion_rate = if total_tasks > 0 {
            metrics.tasks_completed as f64 / total_tasks as f64
        } else {
            0.0
        };

        let error_rate = if total_tasks > 0 {
            metrics.error_count as f64 / total_tasks as f64
        } else {
            0.0
        };

        let message_throughput = if metrics.uptime_secs > 0 {
            (metrics.messages_sent + metrics.messages_received) as f64 / metrics.uptime_secs as f64
        } else {
            0.0
        };

        Self {
            agent_id,
            agent_name,
            metrics: metrics.clone(),
            task_completion_rate,
            avg_task_duration_ms: metrics.avg_response_latency_ms,
            message_throughput,
            error_rate,
            recovery_count: metrics.recovery_count as u32,
            uptime_secs: metrics.uptime_secs,
            snapshot_at: Utc::now(),
        }
    }
}

// ---------------------------------------------------------------------------
// TaskAnalytics
// ---------------------------------------------------------------------------

/// Aggregate analytics for tasks across the system.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskAnalytics {
    /// Total tasks created.
    pub total_created: u64,
    /// Total tasks completed successfully.
    pub total_completed: u64,
    /// Total tasks failed.
    pub total_failed: u64,
    /// Total tasks cancelled.
    pub total_cancelled: u64,
    /// Average task duration in milliseconds.
    pub avg_duration_ms: f64,
    /// Median task duration in milliseconds.
    pub median_duration_ms: f64,
    /// P95 task duration in milliseconds.
    pub p95_duration_ms: f64,
    /// P99 task duration in milliseconds.
    pub p99_duration_ms: f64,
    /// Task completion rate.
    pub completion_rate: f64,
    /// Tasks per second throughput.
    pub throughput_per_sec: f64,
    /// Average retry count.
    pub avg_retries: f64,
    /// Duration distribution buckets: duration_range -> count.
    pub duration_distribution: HashMap<String, u64>,
}

impl TaskAnalytics {
    /// Calculate task analytics from a list of task durations and outcomes.
    #[must_use]
    pub fn calculate(
        completed: u64,
        failed: u64,
        cancelled: u64,
        durations_ms: &mut [f64],
        total_duration_secs: f64,
    ) -> Self {
        let total = completed + failed + cancelled;
        let completion_rate = if total > 0 {
            completed as f64 / total as f64
        } else {
            0.0
        };

        durations_ms.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let avg_duration_ms = if durations_ms.is_empty() {
            0.0
        } else {
            durations_ms.iter().sum::<f64>() / durations_ms.len() as f64
        };

        let percentile = |p: f64| -> f64 {
            if durations_ms.is_empty() {
                0.0
            } else {
                let idx = ((durations_ms.len() as f64) * p).floor() as usize;
                durations_ms[idx.min(durations_ms.len() - 1)]
            }
        };

        let throughput = if total_duration_secs > 0.0 {
            completed as f64 / total_duration_secs
        } else {
            0.0
        };

        Self {
            total_created: total,
            total_completed: completed,
            total_failed: failed,
            total_cancelled: cancelled,
            avg_duration_ms,
            median_duration_ms: percentile(0.5),
            p95_duration_ms: percentile(0.95),
            p99_duration_ms: percentile(0.99),
            completion_rate,
            throughput_per_sec: throughput,
            avg_retries: 0.0,
            duration_distribution: HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// CommunicationAnalytics
// ---------------------------------------------------------------------------

/// Analytics for inter-agent communication.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommunicationAnalytics {
    /// Total messages sent.
    pub total_messages_sent: u64,
    /// Total messages received.
    pub total_messages_received: u64,
    /// Messages by type: type -> count.
    pub messages_by_type: HashMap<String, u64>,
    /// Average message latency in milliseconds.
    pub avg_message_latency_ms: f64,
    /// Delivery success rate.
    pub delivery_success_rate: f64,
    /// Total broadcasts.
    pub total_broadcasts: u64,
    /// Total conversations.
    pub total_conversations: u64,
    /// Average messages per conversation.
    pub avg_messages_per_conversation: f64,
    /// Active conversations.
    pub active_conversations: u64,
}

// ---------------------------------------------------------------------------
// PerformanceAnalytics
// ---------------------------------------------------------------------------

/// System-wide performance analytics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceAnalytics {
    /// CPU utilization (0.0 to 1.0).
    pub cpu_utilization: f64,
    /// Memory utilization (0.0 to 1.0).
    pub memory_utilization: f64,
    /// Task queue depth.
    pub task_queue_depth: usize,
    /// Agent count.
    pub agent_count: usize,
    /// Active agent count.
    pub active_agent_count: usize,
    /// Average response time across all agents in milliseconds.
    pub avg_response_time_ms: f64,
    /// System throughput in tasks per second.
    pub system_throughput: f64,
    /// System uptime in seconds.
    pub uptime_secs: u64,
}

// ---------------------------------------------------------------------------
// ResourceAnalytics
// ---------------------------------------------------------------------------

/// Analytics for resource utilization.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceAnalytics {
    /// CPU usage per agent: agent_id -> usage.
    pub cpu_usage_per_agent: HashMap<String, f64>,
    /// Memory usage per agent: agent_id -> bytes.
    pub memory_usage_per_agent: HashMap<String, u64>,
    /// Total CPU usage.
    pub total_cpu_usage: f64,
    /// Total memory usage in bytes.
    pub total_memory_usage: u64,
    /// CPU limit.
    pub cpu_limit: f64,
    /// Memory limit in bytes.
    pub memory_limit: u64,
    /// CPU utilization ratio.
    pub cpu_utilization: f64,
    /// Memory utilization ratio.
    pub memory_utilization: f64,
}

impl ResourceAnalytics {
    /// Calculate utilization ratios.
    pub fn calculate_utilization(&mut self) {
        self.cpu_utilization = if self.cpu_limit > 0.0 {
            self.total_cpu_usage / self.cpu_limit
        } else {
            0.0
        };
        self.memory_utilization = if self.memory_limit > 0 {
            self.total_memory_usage as f64 / self.memory_limit as f64
        } else {
            0.0
        };
    }
}

// ---------------------------------------------------------------------------
// SystemAnalytics
// ---------------------------------------------------------------------------

/// Complete system analytics combining all subsystem analytics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemAnalytics {
    /// Overall agent statistics.
    pub agent_statistics: AgentStatistics,
    /// Task analytics.
    pub task_analytics: TaskAnalytics,
    /// Communication analytics.
    pub communication_analytics: CommunicationAnalytics,
    /// Performance analytics.
    pub performance: PerformanceAnalytics,
    /// Resource analytics.
    pub resource_analytics: ResourceAnalytics,
    /// Per-agent analytics.
    pub per_agent: Vec<AgentAnalytics>,
    /// When this snapshot was taken.
    pub snapshot_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_analytics() {
        let metrics = AgentMetrics {
            uptime_secs: 3600,
            messages_sent: 100,
            messages_received: 95,
            tasks_completed: 50,
            tasks_failed: 5,
            tasks_active: 2,
            memory_used_bytes: 1024 * 1024,
            cpu_time_ms: 5000,
            avg_response_latency_ms: 150.0,
            queue_depth: 3,
            error_count: 5,
            recovery_count: 1,
            throughput_tasks_per_sec: 0.014,
        };

        let analytics = AgentAnalytics::from_metrics(AgentId::new(), "test".to_string(), &metrics);

        assert!((analytics.task_completion_rate - 0.909).abs() < 0.01);
        assert!(analytics.message_throughput > 0.0);
        assert!((analytics.error_rate - 0.09).abs() < 0.01);
    }

    #[test]
    fn test_task_analytics() {
        let mut durations = vec![100.0, 200.0, 150.0, 300.0, 120.0];
        let analytics = TaskAnalytics::calculate(4, 1, 0, &mut durations, 100.0);

        assert_eq!(analytics.total_completed, 4);
        assert_eq!(analytics.total_failed, 1);
        assert!((analytics.completion_rate - 0.8).abs() < f64::EPSILON);
        assert_eq!(analytics.median_duration_ms, 150.0);
    }

    #[test]
    fn test_resource_analytics() {
        let mut ra = ResourceAnalytics {
            total_cpu_usage: 8.0,
            total_memory_usage: 4 * 1024 * 1024 * 1024,
            cpu_limit: 16.0,
            memory_limit: 8 * 1024 * 1024 * 1024,
            ..ResourceAnalytics::default()
        };
        ra.calculate_utilization();
        assert!((ra.cpu_utilization - 0.5).abs() < f64::EPSILON);
        assert!((ra.memory_utilization - 0.5).abs() < f64::EPSILON);
    }
}
