use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use tokio::sync::RwLock;

use super::types::{ProviderMetrics, TokenUsage};

/// Aggregated metrics for the language engine.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct LanguageEngineMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_tokens_generated: u64,
    pub average_tokens_per_request: f64,
    pub average_request_latency_ms: f64,
    pub average_first_token_latency_ms: f64,
    pub average_tokens_per_second: f64,
    pub active_requests: u64,
    pub uptime_secs: u64,
    pub provider_metrics: HashMap<String, ProviderMetrics>,
}

/// Collects and aggregates metrics from language engine operations.
pub struct MetricsCollector {
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,
    total_tokens_generated: AtomicU64,
    total_request_latency_ms: AtomicU64,
    total_first_token_latency_ms: AtomicU64,
    total_tokens_per_second: AtomicU64,
    active_requests: AtomicUsize,
    started_at: std::time::Instant,
    provider_metrics: RwLock<HashMap<String, ProviderMetrics>>,
    token_usage: RwLock<HashMap<String, TokenUsage>>,
}

impl MetricsCollector {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            total_tokens_generated: AtomicU64::new(0),
            total_request_latency_ms: AtomicU64::new(0),
            total_first_token_latency_ms: AtomicU64::new(0),
            total_tokens_per_second: AtomicU64::new(0),
            active_requests: AtomicUsize::new(0),
            started_at: std::time::Instant::now(),
            provider_metrics: RwLock::new(HashMap::new()),
            token_usage: RwLock::new(HashMap::new()),
        })
    }

    /// Record a successful request.
    pub async fn record_success(
        &self,
        provider: &str,
        latency_ms: u64,
        first_token_latency_ms: u64,
        tokens_per_second: f64,
        usage: &TokenUsage,
    ) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.successful_requests.fetch_add(1, Ordering::Relaxed);
        self.total_tokens_generated
            .fetch_add(usage.total_tokens as u64, Ordering::Relaxed);
        self.total_request_latency_ms
            .fetch_add(latency_ms, Ordering::Relaxed);
        self.total_first_token_latency_ms
            .fetch_add(first_token_latency_ms, Ordering::Relaxed);
        self.total_tokens_per_second
            .fetch_add(tokens_per_second as u64, Ordering::Relaxed);

        let mut token_usage = self.token_usage.write().await;
        let entry = token_usage
            .entry(provider.to_string())
            .or_insert_with(TokenUsage::default);
        entry.prompt_tokens += usage.prompt_tokens;
        entry.completion_tokens += usage.completion_tokens;
        entry.total_tokens += usage.total_tokens;
    }

    /// Record a failed request.
    pub async fn record_failure(&self, _provider: &str) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.failed_requests.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment active requests.
    pub fn request_started(&self) {
        self.active_requests.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement active requests.
    pub fn request_completed(&self) {
        self.active_requests.fetch_sub(1, Ordering::Relaxed);
    }

    /// Update provider-specific metrics.
    pub async fn update_provider_metrics(&self, provider: &str, metrics: ProviderMetrics) {
        let mut provider_metrics = self.provider_metrics.write().await;
        provider_metrics.insert(provider.to_string(), metrics);
    }

    /// Get aggregated metrics.
    pub async fn snapshot(&self) -> LanguageEngineMetrics {
        let total = self.total_requests.load(Ordering::Relaxed);
        let total_latency = self.total_request_latency_ms.load(Ordering::Relaxed);
        let total_first_token = self.total_first_token_latency_ms.load(Ordering::Relaxed);
        let total_tps = self.total_tokens_per_second.load(Ordering::Relaxed);

        let avg_latency = if total > 0 {
            total_latency as f64 / total as f64
        } else {
            0.0
        };

        let avg_first_token = if total > 0 {
            total_first_token as f64 / total as f64
        } else {
            0.0
        };

        let avg_tps = if total > 0 {
            total_tps as f64 / total as f64
        } else {
            0.0
        };

        let total_tokens = self.total_tokens_generated.load(Ordering::Relaxed);
        let avg_tokens = if total > 0 {
            total_tokens as f64 / total as f64
        } else {
            0.0
        };

        LanguageEngineMetrics {
            total_requests: total,
            successful_requests: self.successful_requests.load(Ordering::Relaxed),
            failed_requests: self.failed_requests.load(Ordering::Relaxed),
            total_tokens_generated: total_tokens,
            average_tokens_per_request: avg_tokens,
            average_request_latency_ms: avg_latency,
            average_first_token_latency_ms: avg_first_token,
            average_tokens_per_second: avg_tps,
            active_requests: self.active_requests.load(Ordering::Relaxed) as u64,
            uptime_secs: self.started_at.elapsed().as_secs(),
            provider_metrics: self.provider_metrics.read().await.clone(),
        }
    }

    /// Get token usage for a specific provider.
    pub async fn provider_token_usage(&self, provider: &str) -> TokenUsage {
        let usage = self.token_usage.read().await;
        usage.get(provider).cloned().unwrap_or_default()
    }

    /// Reset all metrics.
    pub fn reset(&self) {
        self.total_requests.store(0, Ordering::Relaxed);
        self.successful_requests.store(0, Ordering::Relaxed);
        self.failed_requests.store(0, Ordering::Relaxed);
        self.total_tokens_generated.store(0, Ordering::Relaxed);
        self.total_request_latency_ms.store(0, Ordering::Relaxed);
        self.total_first_token_latency_ms.store(0, Ordering::Relaxed);
        self.total_tokens_per_second.store(0, Ordering::Relaxed);
        self.active_requests.store(0, Ordering::Relaxed);
    }

    /// Get failure rate as a percentage.
    pub fn failure_rate(&self) -> f64 {
        let total = self.total_requests.load(Ordering::Relaxed);
        if total == 0 {
            return 0.0;
        }
        let failed = self.failed_requests.load(Ordering::Relaxed);
        (failed as f64 / total as f64) * 100.0
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            total_tokens_generated: AtomicU64::new(0),
            total_request_latency_ms: AtomicU64::new(0),
            total_first_token_latency_ms: AtomicU64::new(0),
            total_tokens_per_second: AtomicU64::new(0),
            active_requests: AtomicUsize::new(0),
            started_at: std::time::Instant::now(),
            provider_metrics: RwLock::new(HashMap::new()),
            token_usage: RwLock::new(HashMap::new()),
        }
    }
}
