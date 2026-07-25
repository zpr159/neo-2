use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    pub enabled: bool,
    pub export_interval_secs: u64,
    pub endpoint: Option<String>,
    pub sample_rate: f64,
    pub enable_gpu_metrics: bool,
    pub enable_memory_metrics: bool,
    pub enable_latency_histograms: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            export_interval_secs: 30,
            endpoint: None,
            sample_rate: 1.0,
            enable_gpu_metrics: true,
            enable_memory_metrics: true,
            enable_latency_histograms: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyMetrics {
    pub p50_ms: f64,
    pub p90_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub max_ms: f64,
    pub mean_ms: f64,
}

impl Default for LatencyMetrics {
    fn default() -> Self {
        Self {
            p50_ms: 0.0,
            p90_ms: 0.0,
            p95_ms: 0.0,
            p99_ms: 0.0,
            max_ms: 0.0,
            mean_ms: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputMetrics {
    pub requests_per_second: f64,
    pub tokens_per_second: f64,
    pub input_tokens_per_second: f64,
    pub output_tokens_per_second: f64,
    pub batches_per_second: f64,
}

impl Default for ThroughputMetrics {
    fn default() -> Self {
        Self {
            requests_per_second: 0.0,
            tokens_per_second: 0.0,
            input_tokens_per_second: 0.0,
            output_tokens_per_second: 0.0,
            batches_per_second: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuMetrics {
    pub device_id: u32,
    pub utilization: f64,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub memory_utilization: f64,
    pub temperature_celsius: Option<f64>,
    pub power_watts: Option<f64>,
    pub clock_mhz: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendStatistics {
    pub backend_name: String,
    pub total_inferences: u64,
    pub successful_inferences: u64,
    pub failed_inferences: u64,
    pub avg_latency_ms: f64,
    pub active_models: usize,
    pub memory_used_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySnapshot {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub uptime_seconds: u64,
    pub latency: LatencyMetrics,
    pub throughput: ThroughputMetrics,
    pub gpu_metrics: Vec<GpuMetrics>,
    pub backend_stats: Vec<BackendStatistics>,
    pub total_requests: u64,
    pub active_requests: usize,
    pub models_loaded: usize,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub queue_depth: usize,
}

pub struct InferenceTelemetry {
    config: TelemetryConfig,
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,
    total_tokens: AtomicU64,
    total_input_tokens: AtomicU64,
    total_output_tokens: AtomicU64,
    active_requests: AtomicUsize,
    latency_sum_ms: AtomicU64,
    latency_count: AtomicU64,
    latency_max_ms: AtomicU64,
    start_time: Instant,
    recent_latencies: parking_lot::Mutex<Vec<f64>>,
}

impl std::fmt::Debug for InferenceTelemetry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InferenceTelemetry")
            .field("total_requests", &self.total_requests.load(Ordering::Relaxed))
            .field("active_requests", &self.active_requests.load(Ordering::Relaxed))
            .finish()
    }
}

impl InferenceTelemetry {
    pub fn new(config: TelemetryConfig) -> Self {
        Self {
            config,
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            total_tokens: AtomicU64::new(0),
            total_input_tokens: AtomicU64::new(0),
            total_output_tokens: AtomicU64::new(0),
            active_requests: AtomicUsize::new(0),
            latency_sum_ms: AtomicU64::new(0),
            latency_count: AtomicU64::new(0),
            latency_max_ms: AtomicU64::new(0),
            start_time: Instant::now(),
            recent_latencies: parking_lot::Mutex::new(Vec::with_capacity(10000)),
        }
    }

    pub fn record_request_start(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.active_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_request_complete(&self, latency_ms: f64, success: bool, tokens: u64, input_tokens: u64, output_tokens: u64) {
        self.active_requests.fetch_sub(1, Ordering::Relaxed);
        if success {
            self.successful_requests.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_requests.fetch_add(1, Ordering::Relaxed);
        }
        self.total_tokens.fetch_add(tokens, Ordering::Relaxed);
        self.total_input_tokens.fetch_add(input_tokens, Ordering::Relaxed);
        self.total_output_tokens.fetch_add(output_tokens, Ordering::Relaxed);
        self.latency_sum_ms.fetch_add((latency_ms * 1000.0) as u64, Ordering::Relaxed);
        self.latency_count.fetch_add(1, Ordering::Relaxed);
        let max = self.latency_max_ms.load(Ordering::Relaxed);
        let new_max = (latency_ms * 1000.0) as u64;
        if new_max > max {
            self.latency_max_ms.store(new_max, Ordering::Relaxed);
        }
        let mut latencies = self.recent_latencies.lock();
        latencies.push(latency_ms);
        if latencies.len() > 10000 {
            latencies.drain(0..5000);
        }
    }

    #[must_use]
    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    #[must_use]
    pub fn active_requests(&self) -> usize {
        self.active_requests.load(Ordering::Relaxed)
    }

    pub fn snapshot(&self) -> TelemetrySnapshot {
        let latencies = self.recent_latencies.lock().clone();
        let mut sorted_latencies = latencies.clone();
        sorted_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let percentile = |p: f64| -> f64 {
            if sorted_latencies.is_empty() {
                return 0.0;
            }
            let idx = ((sorted_latencies.len() as f64) * p) as usize;
            sorted_latencies[idx.min(sorted_latencies.len() - 1)]
        };
        let count = self.latency_count.load(Ordering::Relaxed);
        let sum = self.latency_sum_ms.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed().as_secs_f64();
        TelemetrySnapshot {
            timestamp: chrono::Utc::now(),
            uptime_seconds: self.uptime_seconds(),
            latency: LatencyMetrics {
                p50_ms: percentile(0.50),
                p90_ms: percentile(0.90),
                p95_ms: percentile(0.95),
                p99_ms: percentile(0.99),
                max_ms: self.latency_max_ms.load(Ordering::Relaxed) as f64 / 1000.0,
                mean_ms: if count > 0 { sum as f64 / count as f64 / 1000.0 } else { 0.0 },
            },
            throughput: ThroughputMetrics {
                requests_per_second: if elapsed > 0.0 { self.total_requests.load(Ordering::Relaxed) as f64 / elapsed } else { 0.0 },
                tokens_per_second: if elapsed > 0.0 { self.total_tokens.load(Ordering::Relaxed) as f64 / elapsed } else { 0.0 },
                input_tokens_per_second: if elapsed > 0.0 { self.total_input_tokens.load(Ordering::Relaxed) as f64 / elapsed } else { 0.0 },
                output_tokens_per_second: if elapsed > 0.0 { self.total_output_tokens.load(Ordering::Relaxed) as f64 / elapsed } else { 0.0 },
                batches_per_second: 0.0,
            },
            gpu_metrics: Vec::new(),
            backend_stats: Vec::new(),
            total_requests: self.total_requests.load(Ordering::Relaxed),
            active_requests: self.active_requests(),
            models_loaded: 0,
            memory_used_bytes: 0,
            memory_total_bytes: 0,
            queue_depth: 0,
        }
    }
}
