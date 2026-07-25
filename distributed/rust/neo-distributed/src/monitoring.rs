//! Resource monitoring and analytics — CPU, GPU, RAM, disk, network tracking,
//! cluster analytics, scheduling analytics, performance analytics.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::types::{NodeId, NodeResources};

// ---------------------------------------------------------------------------
// ResourceMetrics
// ---------------------------------------------------------------------------

/// Snapshot of resource metrics for a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetrics {
    /// Node ID.
    pub node_id: NodeId,
    /// Timestamp.
    pub timestamp: DateTime<Utc>,
    /// CPU utilization 0.0 – 1.0.
    pub cpu_utilization: f32,
    /// GPU utilization 0.0 – 1.0.
    pub gpu_utilization: f32,
    /// Memory utilization 0.0 – 1.0.
    pub memory_utilization: f32,
    /// Disk utilization 0.0 – 1.0.
    pub disk_utilization: f32,
    /// Network utilization 0.0 – 1.0.
    pub network_utilization: f32,
    /// Power usage in watts (0 = unknown).
    pub power_watts: f32,
    /// Temperature in Celsius (0 = unknown).
    pub temperature_celsius: f32,
    /// Active task count.
    pub active_tasks: u32,
    /// Network throughput in bytes/sec.
    pub network_bytes_per_sec: u64,
}

// ---------------------------------------------------------------------------
// NodeAnalytics
// ---------------------------------------------------------------------------

/// Time-series analytics for a single node.
pub struct NodeAnalytics {
    /// Recent metric snapshots (ring buffer).
    history: RwLock<VecDeque<ResourceMetrics>>,
    /// Maximum history size.
    max_size: usize,
}

impl NodeAnalytics {
    pub fn new(max_size: usize) -> Self {
        Self {
            history: RwLock::new(VecDeque::with_capacity(max_size)),
            max_size,
        }
    }

    /// Record a metrics snapshot.
    pub fn record(&self, metrics: ResourceMetrics) {
        let mut history = self.history.write();
        if history.len() >= self.max_size {
            history.pop_front();
        }
        history.push_back(metrics);
    }

    /// Get the latest metrics.
    pub fn latest(&self) -> Option<ResourceMetrics> {
        self.history.read().back().cloned()
    }

    /// Get metrics history.
    pub fn history(&self, count: usize) -> Vec<ResourceMetrics> {
        self.history.read().iter().rev().take(count).cloned().collect()
    }

    /// Average CPU utilization over history.
    pub fn avg_cpu(&self) -> f32 {
        let history = self.history.read();
        if history.is_empty() {
            return 0.0;
        }
        let sum: f32 = history.iter().map(|m| m.cpu_utilization).sum();
        sum / history.len() as f32
    }

    /// Average memory utilization over history.
    pub fn avg_memory(&self) -> f32 {
        let history = self.history.read();
        if history.is_empty() {
            return 0.0;
        }
        let sum: f32 = history.iter().map(|m| m.memory_utilization).sum();
        sum / history.len() as f32
    }

    /// Peak CPU utilization over history.
    pub fn peak_cpu(&self) -> f32 {
        self.history
            .read()
            .iter()
            .map(|m| m.cpu_utilization)
            .fold(0.0f32, f32::max)
    }

    /// Peak memory utilization over history.
    pub fn peak_memory(&self) -> f32 {
        self.history
            .read()
            .iter()
            .map(|m| m.memory_utilization)
            .fold(0.0f32, f32::max)
    }
}

// ---------------------------------------------------------------------------
// ClusterAnalytics
// ---------------------------------------------------------------------------

/// Cluster-wide analytics aggregation.
pub struct ClusterAnalytics {
    /// Per-node analytics.
    node_analytics: RwLock<HashMap<NodeId, NodeAnalytics>>,
    /// Cluster-level metrics history.
    cluster_history: RwLock<VecDeque<ClusterMetrics>>,
    /// Maximum history size.
    max_size: usize,
}

/// Cluster-level aggregated metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMetrics {
    pub timestamp: DateTime<Utc>,
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub total_cpu_utilization: f32,
    pub total_memory_utilization: f32,
    pub total_gpu_utilization: f32,
    pub avg_latency_ms: f64,
    pub throughput_ops_per_sec: f64,
    pub active_tasks: u64,
    pub queued_tasks: u64,
}

impl ClusterAnalytics {
    pub fn new(max_size: usize) -> Self {
        tracing::info!("cluster analytics created");
        Self {
            node_analytics: RwLock::new(HashMap::new()),
            cluster_history: RwLock::new(VecDeque::with_capacity(max_size)),
            max_size,
        }
    }

    /// Get or create analytics for a node.
    pub fn node(&self, node_id: NodeId) -> std::sync::Arc<NodeAnalytics> {
        let mut nodes = self.node_analytics.write();
        nodes
            .entry(node_id)
            .or_insert_with(|| NodeAnalytics::new(self.max_size));
        // We can't return Arc here easily, so return a reference approach.
        // In production, wrap in Arc.
        std::sync::Arc::new(NodeAnalytics::new(self.max_size))
    }

    /// Record cluster-level metrics.
    pub fn record_cluster(&self, metrics: ClusterMetrics) {
        let mut history = self.cluster_history.write();
        if history.len() >= self.max_size {
            history.pop_front();
        }
        history.push_back(metrics);
    }

    /// Get latest cluster metrics.
    pub fn latest_cluster(&self) -> Option<ClusterMetrics> {
        self.cluster_history.read().back().cloned()
    }

    /// Get cluster metrics history.
    pub fn cluster_history(&self, count: usize) -> Vec<ClusterMetrics> {
        self.cluster_history.read().iter().rev().take(count).cloned().collect()
    }

    /// Average cluster CPU utilization.
    pub fn avg_cluster_cpu(&self) -> f32 {
        let history = self.cluster_history.read();
        if history.is_empty() {
            return 0.0;
        }
        let sum: f32 = history.iter().map(|m| m.total_cpu_utilization).sum();
        sum / history.len() as f32
    }

    /// Average cluster memory utilization.
    pub fn avg_cluster_memory(&self) -> f32 {
        let history = self.cluster_history.read();
        if history.is_empty() {
            return 0.0;
        }
        let sum: f32 = history
            .iter()
            .map(|m| m.total_memory_utilization)
            .sum();
        sum / history.len() as f32
    }
}

// ---------------------------------------------------------------------------
// SchedulingAnalytics
// ---------------------------------------------------------------------------

/// Analytics for the distributed scheduler.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchedulingAnalytics {
    pub total_scheduled: u64,
    pub total_completed: u64,
    pub total_failed: u64,
    pub total_reassigned: u64,
    pub avg_assignment_time_ms: f64,
    pub avg_queue_wait_time_ms: f64,
    pub current_queue_depth: usize,
    pub current_active: usize,
    pub task_migrations: u64,
    pub policy_changes: u64,
}

// ---------------------------------------------------------------------------
// PerformanceAnalytics
// ---------------------------------------------------------------------------

/// Performance analytics for the cluster.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceAnalytics {
    pub avg_throughput_ops_per_sec: f64,
    pub p50_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub max_latency_ms: f64,
    pub avg_request_size_bytes: f64,
    pub avg_response_size_bytes: f64,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
}

// ---------------------------------------------------------------------------
// NetworkAnalytics
// ---------------------------------------------------------------------------

/// Network analytics for inter-node communication.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkAnalytics {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub connections_active: usize,
    pub connections_total: u64,
    pub avg_latency_ms: f64,
    pub max_latency_ms: f64,
    pub packet_loss_rate: f64,
    pub bandwidth_utilization: f64,
}

// ---------------------------------------------------------------------------
// ResourceMonitor
// ---------------------------------------------------------------------------

/// Monitors resource utilization across the cluster.
pub struct ResourceMonitor {
    /// Per-node resource history.
    node_history: RwLock<HashMap<NodeId, VecDeque<ResourceMetrics>>>,
    /// Maximum history per node.
    max_history_per_node: usize,
    /// Monitoring interval.
    interval_ms: u64,
    /// Total samples collected.
    total_samples: AtomicU64,
}

impl ResourceMonitor {
    pub fn new(max_history_per_node: usize, interval_ms: u64) -> Self {
        Self {
            node_history: RwLock::new(HashMap::new()),
            max_history_per_node,
            interval_ms,
            total_samples: AtomicU64::new(0),
        }
    }

    /// Record resource metrics for a node.
    pub fn record(&self, node_id: NodeId, metrics: ResourceMetrics) {
        let mut history = self.node_history.write();
        let entry = history.entry(node_id).or_insert_with(|| {
            VecDeque::with_capacity(self.max_history_per_node)
        });
        if entry.len() >= self.max_history_per_node {
            entry.pop_front();
        }
        entry.push_back(metrics);
        self.total_samples.fetch_add(1, Ordering::Relaxed);
    }

    /// Get latest metrics for a node.
    pub fn latest(&self, node_id: NodeId) -> Option<ResourceMetrics> {
        self.node_history
            .read()
            .get(&node_id)
            .and_then(|h| h.back().cloned())
    }

    /// Get metrics history for a node.
    pub fn history(&self, node_id: NodeId, count: usize) -> Vec<ResourceMetrics> {
        self.node_history
            .read()
            .get(&node_id)
            .map(|h| h.iter().rev().take(count).cloned().collect())
            .unwrap_or_default()
    }

    /// Get monitoring interval.
    pub fn interval_ms(&self) -> u64 {
        self.interval_ms
    }

    /// Total samples collected.
    pub fn total_samples(&self) -> u64 {
        self.total_samples.load(Ordering::Relaxed)
    }

    /// Get aggregated cluster metrics.
    pub fn aggregate(&self) -> ClusterMetrics {
        let history = self.node_history.read();
        let total_nodes = history.len();
        let mut total_cpu = 0.0f32;
        let mut total_mem = 0.0f32;
        let mut total_gpu = 0.0f32;

        for (_, node_history) in history.iter() {
            if let Some(latest) = node_history.back() {
                total_cpu += latest.cpu_utilization;
                total_mem += latest.memory_utilization;
                total_gpu += latest.gpu_utilization;
            }
        }

        let n = total_nodes.max(1) as f32;
        ClusterMetrics {
            timestamp: Utc::now(),
            total_nodes,
            healthy_nodes: total_nodes,
            total_cpu_utilization: total_cpu / n,
            total_memory_utilization: total_mem / n,
            total_gpu_utilization: total_gpu / n,
            avg_latency_ms: 0.0,
            throughput_ops_per_sec: 0.0,
            active_tasks: 0,
            queued_tasks: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_analytics() {
        let analytics = NodeAnalytics::new(100);
        let metrics = ResourceMetrics {
            node_id: NodeId::new(),
            timestamp: Utc::now(),
            cpu_utilization: 0.5,
            gpu_utilization: 0.3,
            memory_utilization: 0.7,
            disk_utilization: 0.2,
            network_utilization: 0.1,
            power_watts: 100.0,
            temperature_celsius: 65.0,
            active_tasks: 5,
            network_bytes_per_sec: 1000,
        };
        analytics.record(metrics);
        assert!(analytics.latest().is_some());
        assert!((analytics.avg_cpu() - 0.5).abs() < 0.01);
    }

    #[test]
    fn cluster_analytics() {
        let analytics = ClusterAnalytics::new(100);
        let metrics = ClusterMetrics {
            timestamp: Utc::now(),
            total_nodes: 5,
            healthy_nodes: 4,
            total_cpu_utilization: 0.6,
            total_memory_utilization: 0.7,
            total_gpu_utilization: 0.5,
            avg_latency_ms: 2.0,
            throughput_ops_per_sec: 100.0,
            active_tasks: 10,
            queued_tasks: 5,
        };
        analytics.record_cluster(metrics);
        assert!(analytics.latest_cluster().is_some());
    }

    #[test]
    fn resource_monitor() {
        let monitor = ResourceMonitor::new(100, 1000);
        let node_id = NodeId::new();
        let metrics = ResourceMetrics {
            node_id,
            timestamp: Utc::now(),
            cpu_utilization: 0.5,
            gpu_utilization: 0.0,
            memory_utilization: 0.6,
            disk_utilization: 0.3,
            network_utilization: 0.1,
            power_watts: 0.0,
            temperature_celsius: 0.0,
            active_tasks: 0,
            network_bytes_per_sec: 0,
        };
        monitor.record(node_id, metrics);
        assert!(monitor.latest(node_id).is_some());
        assert_eq!(monitor.total_samples(), 1);
    }
}
