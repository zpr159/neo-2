use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::shape::Shape;

/// A single profiling event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileEvent {
    pub name: String,
    pub duration_us: u64,
    pub timestamp_secs: f64,
    pub input_shapes: Vec<Vec<usize>>,
    pub device: String,
    pub flops: Option<u64>,
    pub memory_bytes: Option<u64>,
    pub metadata: HashMap<String, String>,
}

/// Aggregated statistics for a named operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpStats {
    pub name: String,
    pub call_count: u64,
    pub total_duration_us: u64,
    pub avg_duration_us: u64,
    pub max_duration_us: u64,
    pub min_duration_us: u64,
    pub total_flops: u64,
    pub avg_flops: u64,
    pub total_memory_bytes: u64,
    pub avg_memory_bytes: u64,
}

impl OpStats {
    fn new(name: String) -> Self {
        Self {
            name,
            call_count: 0,
            total_duration_us: 0,
            avg_duration_us: 0,
            max_duration_us: 0,
            min_duration_us: u64::MAX,
            total_flops: 0,
            avg_flops: 0,
            total_memory_bytes: 0,
            avg_memory_bytes: 0,
        }
    }

    fn update(&mut self, duration_us: u64, flops: u64, memory_bytes: u64) {
        self.call_count += 1;
        self.total_duration_us += duration_us;
        self.avg_duration_us = self.total_duration_us / self.call_count;
        if duration_us > self.max_duration_us {
            self.max_duration_us = duration_us;
        }
        if duration_us < self.min_duration_us {
            self.min_duration_us = duration_us;
        }
        self.total_flops += flops;
        if self.call_count > 0 {
            self.avg_flops = self.total_flops / self.call_count;
        }
        self.total_memory_bytes += memory_bytes;
        if self.call_count > 0 {
            self.avg_memory_bytes = self.total_memory_bytes / self.call_count;
        }
    }
}

/// Overall profiling summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSummary {
    pub total_events: usize,
    pub total_duration_us: u64,
    pub total_flops: u64,
    pub total_memory_bytes: u64,
    pub ops: Vec<OpStats>,
}

/// Profiling scope guard for automatic timing.
#[derive(Debug)]
pub struct ProfileScope {
    name: String,
    start: Instant,
    input_shapes: Vec<Shape>,
    device: String,
    flops: Option<u64>,
    memory_bytes: Option<u64>,
    profiler: Arc<Profiler>,
}

impl ProfileScope {
    fn new(
        profiler: Arc<Profiler>,
        name: String,
        input_shapes: Vec<Shape>,
        device: String,
        flops: Option<u64>,
        memory_bytes: Option<u64>,
    ) -> Self {
        Self {
            name,
            start: Instant::now(),
            input_shapes,
            device,
            flops,
            memory_bytes,
            profiler,
        }
    }
}

impl Drop for ProfileScope {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        let event = ProfileEvent {
            name: self.name.clone(),
            duration_us: duration.as_micros() as u64,
            timestamp_secs: duration.as_secs_f64(),
            input_shapes: self.input_shapes.iter().map(|s| s.to_vec()).collect(),
            device: self.device.clone(),
            flops: self.flops,
            memory_bytes: self.memory_bytes,
            metadata: HashMap::new(),
        };
        self.profiler.record_event(event);
    }
}

/// Comprehensive profiler for the neural engine.
pub struct Profiler {
    events: Mutex<Vec<ProfileEvent>>,
    op_stats: Mutex<HashMap<String, OpStats>>,
    enabled: AtomicBool,
    total_flops: AtomicU64,
    total_memory: AtomicU64,
}

impl Profiler {
    /// Creates a new profiler.
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            op_stats: Mutex::new(HashMap::new()),
            enabled: AtomicBool::new(true),
            total_flops: AtomicU64::new(0),
            total_memory: AtomicU64::new(0),
        }
    }

    /// Returns an Arc-wrapped reference to self for use in scoped profiling.
    #[must_use]
    pub fn arc(self) -> Arc<Self> {
        Arc::new(self)
    }

    /// Records a profiling event.
    pub fn record_event(&self, event: ProfileEvent) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }

        if let Some(flops) = event.flops {
            self.total_flops.fetch_add(flops, Ordering::Relaxed);
        }
        if let Some(mem) = event.memory_bytes {
            self.total_memory.fetch_add(mem, Ordering::Relaxed);
        }

        {
            let mut stats = self.op_stats.lock();
            let entry = stats
                .entry(event.name.clone())
                .or_insert_with(|| OpStats::new(event.name.clone()));
            entry.update(
                event.duration_us,
                event.flops.unwrap_or(0),
                event.memory_bytes.unwrap_or(0),
            );
        }

        {
            let mut events = self.events.lock();
            events.push(event);
        }
    }

    /// Creates a scoped profiling timer.
    #[must_use]
    pub fn scope(self: &Arc<Self>, name: &str, input_shapes: Vec<Shape>) -> ProfileScope {
        ProfileScope::new(
            Arc::clone(self),
            name.to_string(),
            input_shapes,
            "cpu".to_string(),
            None,
            None,
        )
    }

    /// Creates a scoped profiling timer with FLOPS estimate.
    #[must_use]
    pub fn scope_with_flops(
        self: &Arc<Self>,
        name: &str,
        input_shapes: Vec<Shape>,
        flops: u64,
    ) -> ProfileScope {
        ProfileScope::new(
            Arc::clone(self),
            name.to_string(),
            input_shapes,
            "cpu".to_string(),
            Some(flops),
            None,
        )
    }

    /// Returns per-operation statistics.
    #[must_use]
    pub fn op_stats(&self) -> HashMap<String, OpStats> {
        self.op_stats.lock().clone()
    }

    /// Returns the summary of all profiling data.
    #[must_use]
    pub fn summary(&self) -> ProfileSummary {
        let events = self.events.lock().len();
        let total_duration: u64 = self
            .events
            .lock()
            .iter()
            .map(|ev| ev.duration_us)
            .sum();
        let ops: Vec<OpStats> = {
            let s = self.op_stats.lock();
            let mut v: Vec<OpStats> = s.values().cloned().collect();
            v.sort_by(|a, b| b.total_duration_us.cmp(&a.total_duration_us));
            v
        };

        ProfileSummary {
            total_events: events,
            total_duration_us: total_duration,
            total_flops: self.total_flops.load(Ordering::Relaxed),
            total_memory_bytes: self.total_memory.load(Ordering::Relaxed),
            ops,
        }
    }

    /// Returns total FLOPS counted.
    #[must_use]
    pub fn total_flops(&self) -> u64 {
        self.total_flops.load(Ordering::Relaxed)
    }

    /// Returns total memory usage counted.
    #[must_use]
    pub fn total_memory(&self) -> u64 {
        self.total_memory.load(Ordering::Relaxed)
    }

    /// Clears all profiling data.
    pub fn clear(&self) {
        self.events.lock().clear();
        self.op_stats.lock().clear();
        self.total_flops.store(0, Ordering::Relaxed);
        self.total_memory.store(0, Ordering::Relaxed);
    }

    /// Enables or disables profiling.
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Returns whether profiling is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Returns a formatted summary string.
    #[must_use]
    pub fn format_summary(&self) -> String {
        let summary = self.summary();
        let mut lines = vec![
            "=== Neural Engine Profiler Summary ===".to_string(),
            format!("Total events:    {}", summary.total_events),
            format!(
                "Total duration:  {:.3} ms",
                summary.total_duration_us as f64 / 1000.0
            ),
            format!("Total FLOPS:     {}", summary.total_flops),
            format!(
                "Total memory:    {:.2} MB",
                summary.total_memory_bytes as f64 / (1024.0 * 1024.0)
            ),
            "--- Per Operation ---".to_string(),
        ];

        for stat in &summary.ops {
            lines.push(format!(
                "  {:<20} {:>8} calls  avg {:.3} ms  total {:.3} ms",
                stat.name,
                stat.call_count,
                stat.avg_duration_us as f64 / 1000.0,
                stat.total_duration_us as f64 / 1000.0,
            ));
        }

        lines.join("\n")
    }
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Profiler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Profiler")
            .field("enabled", &self.is_enabled())
            .field("total_flops", &self.total_flops())
            .field("total_memory", &self.total_memory())
            .finish()
    }
}

/// Computes FLOPS estimate for a matrix multiplication.
#[must_use]
pub fn matmul_flops(m: usize, n: usize, k: usize) -> u64 {
    (2 * m * n * k) as u64
}

/// Computes FLOPS estimate for element-wise binary operation.
#[must_use]
pub fn elementwise_flops(numel: usize) -> u64 {
    numel as u64
}

/// Computes FLOPS estimate for a reduction.
#[must_use]
pub fn reduce_flops(numel: usize, axis_size: usize) -> u64 {
    (numel * axis_size) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiler_basic() {
        let profiler = Profiler::new();
        assert!(profiler.is_enabled());
        assert_eq!(profiler.total_flops(), 0);
    }

    #[test]
    fn profiler_event() {
        let profiler = Profiler::new();
        let event = ProfileEvent {
            name: "test_op".to_string(),
            duration_us: 100,
            timestamp_secs: 0.0,
            input_shapes: vec![vec![2, 3]],
            device: "cpu".to_string(),
            flops: Some(1000),
            memory_bytes: Some(256),
            metadata: HashMap::new(),
        };
        profiler.record_event(event);
        assert_eq!(profiler.total_flops(), 1000);
        assert_eq!(profiler.total_memory(), 256);
    }

    #[test]
    fn profiler_stats() {
        let profiler = Profiler::new();
        for _ in 0..5 {
            profiler.record_event(ProfileEvent {
                name: "matmul".to_string(),
                duration_us: 100,
                timestamp_secs: 0.0,
                input_shapes: vec![],
                device: "cpu".to_string(),
                flops: Some(2000),
                memory_bytes: None,
                metadata: HashMap::new(),
            });
        }
        let stats = profiler.op_stats();
        let matmul_stats = stats.get("matmul").unwrap();
        assert_eq!(matmul_stats.call_count, 5);
        assert_eq!(matmul_stats.total_flops, 10000);
    }

    #[test]
    fn flops_estimates() {
        assert_eq!(matmul_flops(128, 128, 128), 4194304);
        assert_eq!(elementwise_flops(1024), 1024);
    }

    #[test]
    fn profiler_clear() {
        let profiler = Profiler::new();
        profiler.record_event(ProfileEvent {
            name: "test".to_string(),
            duration_us: 50,
            timestamp_secs: 0.0,
            input_shapes: vec![],
            device: "cpu".to_string(),
            flops: Some(100),
            memory_bytes: None,
            metadata: HashMap::new(),
        });
        profiler.clear();
        assert_eq!(profiler.total_flops(), 0);
    }

    #[test]
    fn profiler_format() {
        let profiler = Profiler::new();
        let output = profiler.format_summary();
        assert!(output.contains("Neural Engine Profiler Summary"));
    }
}
