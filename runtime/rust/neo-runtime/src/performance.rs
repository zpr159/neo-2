//! Performance monitor tracking latency, CPU, memory, GPU, thread utilization,
//! task statistics, and event statistics.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::config::PerformanceConfig;

/// A histogram for tracking latency distributions.
#[derive(Debug, Clone)]
pub struct LatencyHistogram {
    buckets: Vec<u64>,
    bucket_boundaries: Vec<u64>,
    total_count: u64,
    total_sum: u64,
    min: u64,
    max: u64,
}

impl LatencyHistogram {
    /// Create a new histogram with exponential bucket boundaries.
    pub fn new(num_buckets: usize) -> Self {
        let mut boundaries = Vec::with_capacity(num_buckets);
        let mut bound = 1u64;
        for _ in 0..num_buckets {
            boundaries.push(bound);
            bound = bound.saturating_mul(2).max(1);
        }
        Self {
            buckets: vec![0; num_buckets],
            bucket_boundaries: boundaries,
            total_count: 0,
            total_sum: 0,
            min: u64::MAX,
            max: 0,
        }
    }

    /// Record a latency value in microseconds.
    pub fn record(&mut self, value_us: u64) {
        self.total_count += 1;
        self.total_sum += value_us;
        self.min = self.min.min(value_us);
        self.max = self.max.max(value_us);

        let idx = self
            .bucket_boundaries
            .binary_search(&value_us)
            .unwrap_or_else(|i| i)
            .min(self.buckets.len() - 1);
        self.buckets[idx] += 1;
    }

    /// Get the p50 latency.
    pub fn p50(&self) -> u64 {
        self.percentile(0.5)
    }

    /// Get the p95 latency.
    pub fn p95(&self) -> u64 {
        self.percentile(0.95)
    }

    /// Get the p99 latency.
    pub fn p99(&self) -> u64 {
        self.percentile(0.99)
    }

    fn percentile(&self, p: f64) -> u64 {
        if self.total_count == 0 {
            return 0;
        }
        let target = (self.total_count as f64 * p) as u64;
        let mut cumulative = 0u64;
        for (i, &count) in self.buckets.iter().enumerate() {
            cumulative += count;
            if cumulative >= target {
                return self.bucket_boundaries[i];
            }
        }
        self.bucket_boundaries
            .last()
            .copied()
            .unwrap_or(0)
    }

    /// Get the average latency.
    pub fn avg(&self) -> u64 {
        if self.total_count == 0 {
            return 0;
        }
        self.total_sum / self.total_count
    }

    /// Get the total number of recorded values.
    pub fn count(&self) -> u64 {
        self.total_count
    }

    /// Reset the histogram.
    pub fn reset(&mut self) {
        self.buckets.iter_mut().for_each(|b| *b = 0);
        self.total_count = 0;
        self.total_sum = 0;
        self.min = u64::MAX;
        self.max = 0;
    }
}

/// Snapshot of latency statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyStats {
    pub min_us: u64,
    pub max_us: u64,
    pub avg_us: u64,
    pub p50_us: u64,
    pub p95_us: u64,
    pub p99_us: u64,
    pub count: u64,
}

/// Sliding window for tracking time-series values.
#[derive(Debug, Clone)]
pub struct SlidingWindow {
    values: VecDeque<(u64, f64)>,
    max_size: usize,
    window_ms: u64,
}

impl SlidingWindow {
    pub fn new(max_size: usize, window_ms: u64) -> Self {
        Self {
            values: VecDeque::with_capacity(max_size),
            max_size,
            window_ms,
        }
    }

    pub fn push(&mut self, timestamp_ms: u64, value: f64) {
        self.values.push_back((timestamp_ms, value));
        while self.values.len() > self.max_size {
            self.values.pop_front();
        }
        let cutoff = timestamp_ms.saturating_sub(self.window_ms);
        while self.values.front().map_or(false, |&(ts, _)| ts < cutoff) {
            self.values.pop_front();
        }
    }

    pub fn average(&self) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.values.iter().map(|(_, v)| v).sum();
        sum / self.values.len() as f64
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }
}

/// CPU usage monitor.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CpuMonitor {
    pub usage_percent: f64,
    pub core_count: usize,
    pub process_cpu_percent: f64,
}

/// Memory usage monitor.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MemoryMonitor {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub process_bytes: u64,
    pub utilization: f64,
}

/// GPU usage monitor.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GpuMonitor {
    pub gpu_count: usize,
    pub utilization_percent: f64,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub temperature_celsius: f64,
}

/// Thread utilization monitor.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ThreadMonitor {
    pub total_threads: usize,
    pub active_threads: usize,
    pub idle_threads: usize,
    pub utilization: f64,
}

/// Task execution statistics.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TaskPerfStatistics {
    pub total_submitted: u64,
    pub total_completed: u64,
    pub total_failed: u64,
    pub total_cancelled: u64,
    pub avg_execution_time_ms: f64,
    pub max_execution_time_ms: f64,
}

/// Event throughput statistics.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct EventPerfStatistics {
    pub total_published: u64,
    pub total_delivered: u64,
    pub avg_throughput_per_sec: f64,
    pub peak_throughput_per_sec: f64,
}

/// Comprehensive performance snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSnapshot {
    pub timestamp_ms: u64,
    pub latency: LatencyStats,
    pub cpu: CpuMonitor,
    pub memory: MemoryMonitor,
    pub gpu: GpuMonitor,
    pub threads: ThreadMonitor,
    pub tasks: TaskPerfStatistics,
    pub events: EventPerfStatistics,
}

/// Performance monitor aggregating all metrics.
pub struct PerformanceMonitor {
    latency_histogram: RwLock<LatencyHistogram>,
    cpu: RwLock<CpuMonitor>,
    memory: RwLock<MemoryMonitor>,
    gpu: RwLock<GpuMonitor>,
    threads: RwLock<ThreadMonitor>,
    tasks: RwLock<TaskPerfStatistics>,
    events: RwLock<EventPerfStatistics>,
    history: RwLock<VecDeque<PerformanceSnapshot>>,
    config: PerformanceConfig,
}

impl PerformanceMonitor {
    /// Create a new performance monitor.
    pub fn new(config: PerformanceConfig) -> Self {
        Self {
            latency_histogram: RwLock::new(LatencyHistogram::new(
                config.latency_histogram_buckets,
            )),
            cpu: RwLock::new(CpuMonitor::default()),
            memory: RwLock::new(MemoryMonitor::default()),
            gpu: RwLock::new(GpuMonitor::default()),
            threads: RwLock::new(ThreadMonitor::default()),
            tasks: RwLock::new(TaskPerfStatistics::default()),
            events: RwLock::new(EventPerfStatistics::default()),
            history: RwLock::new(VecDeque::with_capacity(config.statistics_window_size)),
            config,
        }
    }

    /// Record a latency measurement in microseconds.
    pub fn record_latency(&self, value_us: u64) {
        self.latency_histogram.write().record(value_us);
    }

    /// Update CPU metrics.
    pub fn update_cpu(&self, usage: f64, process_usage: f64) {
        let mut cpu = self.cpu.write();
        cpu.usage_percent = usage;
        cpu.process_cpu_percent = process_usage;
    }

    /// Update memory metrics.
    pub fn update_memory(&self, total: u64, used: u64, process: u64) {
        let mut mem = self.memory.write();
        mem.total_bytes = total;
        mem.used_bytes = used;
        mem.available_bytes = total.saturating_sub(used);
        mem.process_bytes = process;
        mem.utilization = if total > 0 {
            used as f64 / total as f64
        } else {
            0.0
        };
    }

    /// Update GPU metrics.
    pub fn update_gpu(&self, utilization: f64, mem_used: u64, mem_total: u64, temp: f64) {
        let mut gpu = self.gpu.write();
        gpu.gpu_count = 1;
        gpu.utilization_percent = utilization;
        gpu.memory_used_bytes = mem_used;
        gpu.memory_total_bytes = mem_total;
        gpu.temperature_celsius = temp;
    }

    /// Update thread metrics.
    pub fn update_threads(&self, total: usize, active: usize) {
        let mut threads = self.threads.write();
        threads.total_threads = total;
        threads.active_threads = active;
        threads.idle_threads = total.saturating_sub(active);
        threads.utilization = if total > 0 {
            active as f64 / total as f64
        } else {
            0.0
        };
    }

    /// Record task execution.
    pub fn record_task(&self, execution_time_ms: f64, success: bool) {
        let mut tasks = self.tasks.write();
        tasks.total_submitted += 1;
        if success {
            tasks.total_completed += 1;
        } else {
            tasks.total_failed += 1;
        }
        let total = tasks.total_completed + tasks.total_failed;
        if total > 0 {
            tasks.avg_execution_time_ms =
                (tasks.avg_execution_time_ms * (total - 1) as f64 + execution_time_ms) / total as f64;
        }
        tasks.max_execution_time_ms = tasks.max_execution_time_ms.max(execution_time_ms);
    }

    /// Record event publication.
    pub fn record_event(&self, count: u64) {
        let mut events = self.events.write();
        events.total_published += count;
    }

    /// Take a snapshot of all current metrics.
    pub fn snapshot(&self) -> PerformanceSnapshot {
        let hist = self.latency_histogram.read();
        let latency = LatencyStats {
            min_us: if hist.total_count > 0 { hist.min } else { 0 },
            max_us: hist.max,
            avg_us: hist.avg(),
            p50_us: hist.p50(),
            p95_us: hist.p95(),
            p99_us: hist.p99(),
            count: hist.total_count,
        };
        drop(hist);

        let snapshot = PerformanceSnapshot {
            timestamp_ms: now_ms(),
            latency,
            cpu: self.cpu.read().clone(),
            memory: self.memory.read().clone(),
            gpu: self.gpu.read().clone(),
            threads: self.threads.read().clone(),
            tasks: self.tasks.read().clone(),
            events: self.events.read().clone(),
        };

        let mut history = self.history.write();
        if history.len() >= self.config.statistics_window_size {
            history.pop_front();
        }
        history.push_back(snapshot.clone());

        snapshot
    }

    /// Get historical snapshots.
    pub fn history(&self) -> Vec<PerformanceSnapshot> {
        self.history.read().iter().cloned().collect()
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new(PerformanceConfig::default())
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latency_histogram() {
        let mut hist = LatencyHistogram::new(10);
        hist.record(100);
        hist.record(200);
        hist.record(300);

        assert_eq!(hist.count(), 3);
        assert!(hist.avg() > 0);
        assert!(hist.min <= 100);
        assert!(hist.max >= 300);
    }

    #[test]
    fn latency_histogram_empty() {
        let hist = LatencyHistogram::new(10);
        assert_eq!(hist.count(), 0);
        assert_eq!(hist.avg(), 0);
        assert_eq!(hist.p50(), 0);
    }

    #[test]
    fn latency_histogram_reset() {
        let mut hist = LatencyHistogram::new(10);
        hist.record(100);
        hist.reset();
        assert_eq!(hist.count(), 0);
    }

    #[test]
    fn sliding_window() {
        let mut window = SlidingWindow::new(100, 1000);
        window.push(1000, 1.0);
        window.push(1500, 2.0);
        window.push(2000, 3.0);

        assert_eq!(window.len(), 3);
        let avg = window.average();
        assert!((avg - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn sliding_window_eviction() {
        let mut window = SlidingWindow::new(100, 500);
        window.push(1000, 1.0);
        window.push(2000, 3.0);
        assert_eq!(window.len(), 1);
    }

    #[test]
    fn performance_monitor() {
        let monitor = PerformanceMonitor::new(PerformanceConfig::default());

        monitor.record_latency(1000);
        monitor.record_latency(2000);
        monitor.update_cpu(50.0, 10.0);
        monitor.update_memory(1024, 512, 256);
        monitor.record_task(15.0, true);
        monitor.record_event(1);

        let snap = monitor.snapshot();
        assert_eq!(snap.latency.count, 2);
        assert!((snap.cpu.usage_percent - 50.0).abs() < f64::EPSILON);
        assert_eq!(snap.memory.total_bytes, 1024);
        assert_eq!(snap.tasks.total_completed, 1);
        assert_eq!(snap.events.total_published, 1);
    }

    #[test]
    fn performance_history() {
        let monitor = PerformanceMonitor::new(PerformanceConfig {
            statistics_window_size: 3,
            ..PerformanceConfig::default()
        });

        for _ in 0..5 {
            monitor.record_latency(100);
            monitor.snapshot();
        }

        let history = monitor.history();
        assert!(history.len() <= 3);
    }

    #[test]
    fn cpu_monitor() {
        let mut cpu = CpuMonitor::default();
        cpu.usage_percent = 75.0;
        cpu.core_count = 8;
        assert!((cpu.usage_percent - 75.0).abs() < f64::EPSILON);
    }

    #[test]
    fn gpu_monitor() {
        let mut gpu = GpuMonitor::default();
        gpu.utilization_percent = 90.0;
        gpu.memory_used_bytes = 4_000_000_000;
        gpu.memory_total_bytes = 8_000_000_000;
        assert!((gpu.utilization_percent - 90.0).abs() < f64::EPSILON);
    }

    #[test]
    fn task_statistics_tracking() {
        let monitor = PerformanceMonitor::new(PerformanceConfig::default());
        monitor.record_task(10.0, true);
        monitor.record_task(20.0, true);
        monitor.record_task(30.0, false);

        let snap = monitor.snapshot();
        assert_eq!(snap.tasks.total_completed, 2);
        assert_eq!(snap.tasks.total_failed, 1);
        assert!(snap.tasks.avg_execution_time_ms > 0.0);
    }
}
