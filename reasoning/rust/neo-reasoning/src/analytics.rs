use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReasoningAnalyticsSnapshot {
    pub total_sessions: u64,
    pub completed_sessions: u64,
    pub failed_sessions: u64,
    pub cancelled_sessions: u64,
    pub avg_latency_ms: f64,
    pub p50_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub avg_reasoning_depth: f64,
    pub max_reasoning_depth: u32,
    pub avg_confidence: f32,
    pub min_confidence: f32,
    pub max_confidence: f32,
    pub strategy_usage: HashMap<String, u64>,
    pub cache_hit_rate: f64,
    pub knowledge_retrieval_count: u64,
    pub reflection_count: u64,
    pub hypothesis_count: u64,
    pub decision_count: u64,
    pub tool_usage_count: u64,
    pub memory_usage_entries: u64,
    pub uptime_secs: u64,
}

#[derive(Debug)]
pub struct ReasoningAnalytics {
    total_sessions: AtomicU64,
    completed_sessions: AtomicU64,
    failed_sessions: AtomicU64,
    cancelled_sessions: AtomicU64,
    latencies_ms: RwLock<Vec<f64>>,
    reasoning_depths: RwLock<Vec<u32>>,
    confidences: RwLock<Vec<f32>>,
    strategy_usage: RwLock<HashMap<String, u64>>,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    knowledge_retrievals: AtomicU64,
    reflection_count: AtomicU64,
    hypothesis_count: AtomicU64,
    decision_count: AtomicU64,
    tool_usage_count: AtomicU64,
    start_time: Instant,
}

impl ReasoningAnalytics {
    pub fn new() -> Self {
        Self {
            total_sessions: AtomicU64::new(0),
            completed_sessions: AtomicU64::new(0),
            failed_sessions: AtomicU64::new(0),
            cancelled_sessions: AtomicU64::new(0),
            latencies_ms: RwLock::new(Vec::new()),
            reasoning_depths: RwLock::new(Vec::new()),
            confidences: RwLock::new(Vec::new()),
            strategy_usage: RwLock::new(HashMap::new()),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            knowledge_retrievals: AtomicU64::new(0),
            reflection_count: AtomicU64::new(0),
            hypothesis_count: AtomicU64::new(0),
            decision_count: AtomicU64::new(0),
            tool_usage_count: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    pub fn record_session_start(&self) {
        self.total_sessions.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_session_complete(
        &self,
        latency_ms: f64,
        depth: u32,
        confidence: f32,
        strategy: &str,
    ) {
        self.completed_sessions.fetch_add(1, Ordering::Relaxed);

        self.latencies_ms.write().push(latency_ms);
        self.reasoning_depths.write().push(depth);
        self.confidences.write().push(confidence);

        self.strategy_usage
            .write()
            .entry(strategy.to_string())
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }

    pub fn record_session_failed(&self) {
        self.failed_sessions.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_session_cancelled(&self) {
        self.cancelled_sessions.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_knowledge_retrieval(&self) {
        self.knowledge_retrievals.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_reflection(&self) {
        self.reflection_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_hypothesis(&self) {
        self.hypothesis_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_decision(&self) {
        self.decision_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_tool_usage(&self) {
        self.tool_usage_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> ReasoningAnalyticsSnapshot {
        let latencies = self.latencies_ms.read();
        let depths = self.reasoning_depths.read();
        let confidences = self.confidences.read();
        let strategies = self.strategy_usage.read();

        let total_latencies = latencies.len() as f64;
        let avg_latency = if total_latencies > 0.0 {
            latencies.iter().sum::<f64>() / total_latencies
        } else {
            0.0
        };

        let sorted_latencies = {
            let mut sorted = latencies.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            sorted
        };

        let percentile = |p: f64| -> f64 {
            if sorted_latencies.is_empty() {
                return 0.0;
            }
            let idx = ((p * sorted_latencies.len() as f64) as usize)
                .min(sorted_latencies.len() - 1);
            sorted_latencies[idx]
        };

        let avg_depth = if depths.is_empty() {
            0.0
        } else {
            depths.iter().map(|d| *d as f64).sum::<f64>() / depths.len() as f64
        };

        let max_depth = depths.iter().copied().max().unwrap_or(0);

        let avg_confidence = if confidences.is_empty() {
            0.0
        } else {
            confidences.iter().sum::<f32>() / confidences.len() as f32
        };

        let min_confidence = confidences.iter().copied().fold(1.0f32, f32::min);
        let max_confidence = confidences.iter().copied().fold(0.0f32, f32::max);

        let cache_total = self.cache_hits.load(Ordering::Relaxed)
            + self.cache_misses.load(Ordering::Relaxed);
        let cache_hit_rate = if cache_total > 0 {
            self.cache_hits.load(Ordering::Relaxed) as f64 / cache_total as f64
        } else {
            0.0
        };

        ReasoningAnalyticsSnapshot {
            total_sessions: self.total_sessions.load(Ordering::Relaxed),
            completed_sessions: self.completed_sessions.load(Ordering::Relaxed),
            failed_sessions: self.failed_sessions.load(Ordering::Relaxed),
            cancelled_sessions: self.cancelled_sessions.load(Ordering::Relaxed),
            avg_latency_ms: avg_latency,
            p50_latency_ms: percentile(0.5),
            p95_latency_ms: percentile(0.95),
            p99_latency_ms: percentile(0.99),
            avg_reasoning_depth: avg_depth,
            max_reasoning_depth: max_depth,
            avg_confidence,
            min_confidence,
            max_confidence,
            strategy_usage: strategies.clone(),
            cache_hit_rate,
            knowledge_retrieval_count: self.knowledge_retrievals.load(Ordering::Relaxed),
            reflection_count: self.reflection_count.load(Ordering::Relaxed),
            hypothesis_count: self.hypothesis_count.load(Ordering::Relaxed),
            decision_count: self.decision_count.load(Ordering::Relaxed),
            tool_usage_count: self.tool_usage_count.load(Ordering::Relaxed),
            memory_usage_entries: 0,
            uptime_secs: self.start_time.elapsed().as_secs(),
        }
    }

    pub fn reset(&self) {
        self.total_sessions.store(0, Ordering::Relaxed);
        self.completed_sessions.store(0, Ordering::Relaxed);
        self.failed_sessions.store(0, Ordering::Relaxed);
        self.cancelled_sessions.store(0, Ordering::Relaxed);
        self.latencies_ms.write().clear();
        self.reasoning_depths.write().clear();
        self.confidences.write().clear();
        self.strategy_usage.write().clear();
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
        self.knowledge_retrievals.store(0, Ordering::Relaxed);
        self.reflection_count.store(0, Ordering::Relaxed);
        self.hypothesis_count.store(0, Ordering::Relaxed);
        self.decision_count.store(0, Ordering::Relaxed);
        self.tool_usage_count.store(0, Ordering::Relaxed);
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.total_sessions.load(Ordering::Relaxed);
        let completed = self.completed_sessions.load(Ordering::Relaxed);
        if total == 0 {
            return 0.0;
        }
        completed as f64 / total as f64
    }
}

impl Default for ReasoningAnalytics {
    fn default() -> Self {
        Self::new()
    }
}
