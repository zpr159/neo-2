use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Comprehensive monitoring metrics for the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeMetrics {
    /// Total entities.
    pub entity_count: usize,
    /// Active entities.
    pub active_entity_count: usize,
    /// Total relations.
    pub relation_count: usize,
    /// Active relations.
    pub active_relation_count: usize,
    /// Total namespaces.
    pub namespace_count: usize,
    /// Average entity confidence.
    pub avg_entity_confidence: f32,
    /// Average relation confidence.
    pub avg_relation_confidence: f32,
    /// Average entity importance.
    pub avg_entity_importance: f32,
    /// Entities created in the last hour.
    pub entities_last_hour: usize,
    /// Relations created in the last hour.
    pub relations_last_hour: usize,
    /// Total queries executed.
    pub total_queries: u64,
    /// Average query latency in milliseconds.
    pub avg_query_latency_ms: f64,
    /// Total extractions performed.
    pub total_extractions: u64,
    /// Extraction accuracy (estimated).
    pub extraction_accuracy: f32,
    /// Knowledge freshness (ratio updated in last 24h).
    pub knowledge_freshness: f32,
    /// Consistency score (no contradictions / total).
    pub consistency_score: f32,
    /// When this snapshot was taken.
    pub timestamp: DateTime<Utc>,
}

impl Default for KnowledgeMetrics {
    fn default() -> Self {
        Self {
            entity_count: 0,
            active_entity_count: 0,
            relation_count: 0,
            active_relation_count: 0,
            namespace_count: 0,
            avg_entity_confidence: 0.0,
            avg_relation_confidence: 0.0,
            avg_entity_importance: 0.0,
            entities_last_hour: 0,
            relations_last_hour: 0,
            total_queries: 0,
            avg_query_latency_ms: 0.0,
            total_extractions: 0,
            extraction_accuracy: 0.0,
            knowledge_freshness: 0.0,
            consistency_score: 1.0,
            timestamp: Utc::now(),
        }
    }
}

/// Monitors the health and performance of the knowledge graph.
pub struct KnowledgeMonitor {
    query_count: std::sync::atomic::AtomicU64,
    total_query_time_bits: std::sync::atomic::AtomicU64,
    extraction_count: std::sync::atomic::AtomicU64,
}

impl KnowledgeMonitor {
    /// Create a new monitor.
    #[must_use]
    pub fn new() -> Self {
        Self {
            query_count: std::sync::atomic::AtomicU64::new(0),
            total_query_time_bits: std::sync::atomic::AtomicU64::new(0),
            extraction_count: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Record a query execution.
    pub fn record_query(&self, latency_ms: f64) {
        self.query_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        // Atomically add to total using compare_exchange loop on f64 bits
        loop {
            let old_bits = self.total_query_time_bits.load(std::sync::atomic::Ordering::Relaxed);
            let old_val = f64::from_bits(old_bits);
            let new_val = old_val + latency_ms;
            let new_bits = new_val.to_bits();
            match self.total_query_time_bits.compare_exchange_weak(
                old_bits,
                new_bits,
                std::sync::atomic::Ordering::Relaxed,
                std::sync::atomic::Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(_) => continue,
            }
        }
    }

    /// Record an extraction.
    pub fn record_extraction(&self) {
        self.extraction_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get the total query count.
    #[must_use]
    pub fn query_count(&self) -> u64 {
        self.query_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Get the average query latency.
    #[must_use]
    pub fn avg_query_latency_ms(&self) -> f64 {
        let count = self.query_count
            .load(std::sync::atomic::Ordering::Relaxed);
        if count == 0 {
            return 0.0;
        }
        let total_bits = self.total_query_time_bits
            .load(std::sync::atomic::Ordering::Relaxed);
        f64::from_bits(total_bits) / count as f64
    }

    /// Get the total extraction count.
    #[must_use]
    pub fn extraction_count(&self) -> u64 {
        self.extraction_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl Default for KnowledgeMonitor {
    fn default() -> Self {
        Self::new()
    }
}
