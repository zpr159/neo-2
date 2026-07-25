use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

/// World model operational metrics.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WorldMetrics {
    pub entities_created: AtomicU64,
    pub entities_updated: AtomicU64,
    pub entities_deleted: AtomicU64,
    pub relationships_created: AtomicU64,
    pub relationships_deleted: AtomicU64,
    pub events_recorded: AtomicU64,
    pub observations_processed: AtomicU64,
    pub perceptions_fused: AtomicU64,
    pub predictions_made: AtomicU64,
    pub predictions_correct: AtomicU64,
    pub simulations_run: AtomicU64,
    pub snapshots_taken: AtomicU64,
    pub queries_processed: AtomicU64,
    pub total_query_time_ms: AtomicU64,
}

/// Snapshot of metrics values.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub entities_created: u64,
    pub entities_updated: u64,
    pub entities_deleted: u64,
    pub relationships_created: u64,
    pub relationships_deleted: u64,
    pub events_recorded: u64,
    pub observations_processed: u64,
    pub perceptions_fused: u64,
    pub predictions_made: u64,
    pub predictions_correct: u64,
    pub simulations_run: u64,
    pub snapshots_taken: u64,
    pub queries_processed: u64,
    pub total_query_time_ms: u64,
}

impl WorldMetrics {
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            entities_created: self.entities_created.load(Ordering::Relaxed),
            entities_updated: self.entities_updated.load(Ordering::Relaxed),
            entities_deleted: self.entities_deleted.load(Ordering::Relaxed),
            relationships_created: self.relationships_created.load(Ordering::Relaxed),
            relationships_deleted: self.relationships_deleted.load(Ordering::Relaxed),
            events_recorded: self.events_recorded.load(Ordering::Relaxed),
            observations_processed: self.observations_processed.load(Ordering::Relaxed),
            perceptions_fused: self.perceptions_fused.load(Ordering::Relaxed),
            predictions_made: self.predictions_made.load(Ordering::Relaxed),
            predictions_correct: self.predictions_correct.load(Ordering::Relaxed),
            simulations_run: self.simulations_run.load(Ordering::Relaxed),
            snapshots_taken: self.snapshots_taken.load(Ordering::Relaxed),
            queries_processed: self.queries_processed.load(Ordering::Relaxed),
            total_query_time_ms: self.total_query_time_ms.load(Ordering::Relaxed),
        }
    }

    pub fn record_query(&self, time_ms: u64) {
        self.queries_processed.fetch_add(1, Ordering::Relaxed);
        self.total_query_time_ms.fetch_add(time_ms, Ordering::Relaxed);
    }

    pub fn average_query_time_ms(&self) -> u64 {
        let queries = self.queries_processed.load(Ordering::Relaxed);
        if queries == 0 {
            return 0;
        }
        self.total_query_time_ms.load(Ordering::Relaxed) / queries
    }
}
