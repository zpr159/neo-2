use super::api::ResearchTaskMetrics;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Accumulates metrics across the research subsystem.
#[derive(Debug, Default)]
pub struct ResearchMetrics {
    pub tasks_submitted: AtomicU64,
    pub tasks_completed: AtomicU64,
    pub tasks_failed: AtomicU64,
    pub tasks_cancelled: AtomicU64,
    pub total_searches: AtomicU64,
    pub total_fetches: AtomicU64,
    pub total_facts_extracted: AtomicU64,
    pub total_facts_validated: AtomicU64,
    pub total_facts_rejected: AtomicU64,
    pub total_duplicates_removed: AtomicU64,
    pub total_citations_generated: AtomicU64,
    pub total_knowledge_updates: AtomicU64,
    pub total_world_updates: AtomicU64,
    pub total_memory_updates: AtomicU64,
    pub total_duration_ms: AtomicU64,
}

impl ResearchMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_task_completion(&self, task_metrics: &ResearchTaskMetrics) {
        self.tasks_completed.fetch_add(1, Ordering::Relaxed);
        self.total_searches
            .fetch_add(task_metrics.sources_searched as u64, Ordering::Relaxed);
        self.total_fetches
            .fetch_add(task_metrics.sources_fetched as u64, Ordering::Relaxed);
        self.total_facts_extracted
            .fetch_add(task_metrics.facts_extracted as u64, Ordering::Relaxed);
        self.total_facts_validated
            .fetch_add(task_metrics.facts_validated as u64, Ordering::Relaxed);
        self.total_facts_rejected
            .fetch_add(task_metrics.facts_rejected as u64, Ordering::Relaxed);
        self.total_duplicates_removed
            .fetch_add(task_metrics.duplicates_removed as u64, Ordering::Relaxed);
        self.total_citations_generated
            .fetch_add(task_metrics.citations_generated as u64, Ordering::Relaxed);
        self.total_knowledge_updates
            .fetch_add(task_metrics.knowledge_updates_approved as u64, Ordering::Relaxed);
        self.total_world_updates
            .fetch_add(task_metrics.world_updates_proposed as u64, Ordering::Relaxed);
        self.total_memory_updates
            .fetch_add(task_metrics.memory_updates_proposed as u64, Ordering::Relaxed);
        self.total_duration_ms
            .fetch_add(task_metrics.total_duration_ms, Ordering::Relaxed);
    }

    pub fn record_task_failure(&self) {
        self.tasks_failed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_task_cancellation(&self) {
        self.tasks_cancelled.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            tasks_submitted: self.tasks_submitted.load(Ordering::Relaxed),
            tasks_completed: self.tasks_completed.load(Ordering::Relaxed),
            tasks_failed: self.tasks_failed.load(Ordering::Relaxed),
            tasks_cancelled: self.tasks_cancelled.load(Ordering::Relaxed),
            total_searches: self.total_searches.load(Ordering::Relaxed),
            total_fetches: self.total_fetches.load(Ordering::Relaxed),
            total_facts_extracted: self.total_facts_extracted.load(Ordering::Relaxed),
            total_facts_validated: self.total_facts_validated.load(Ordering::Relaxed),
            total_facts_rejected: self.total_facts_rejected.load(Ordering::Relaxed),
            total_duplicates_removed: self.total_duplicates_removed.load(Ordering::Relaxed),
            total_citations_generated: self.total_citations_generated.load(Ordering::Relaxed),
            total_knowledge_updates: self.total_knowledge_updates.load(Ordering::Relaxed),
            total_world_updates: self.total_world_updates.load(Ordering::Relaxed),
            total_memory_updates: self.total_memory_updates.load(Ordering::Relaxed),
            total_duration_ms: self.total_duration_ms.load(Ordering::Relaxed),
        }
    }

    pub fn average_duration_ms(&self) -> f64 {
        let completed = self.tasks_completed.load(Ordering::Relaxed);
        if completed == 0 {
            return 0.0;
        }
        let total = self.total_duration_ms.load(Ordering::Relaxed);
        total as f64 / completed as f64
    }

    pub fn success_rate(&self) -> f64 {
        let completed = self.tasks_completed.load(Ordering::Relaxed);
        let failed = self.tasks_failed.load(Ordering::Relaxed);
        let total = completed + failed;
        if total == 0 {
            return 1.0;
        }
        completed as f64 / total as f64
    }

    pub fn validation_rate(&self) -> f64 {
        let extracted = self.total_facts_extracted.load(Ordering::Relaxed);
        if extracted == 0 {
            return 0.0;
        }
        let validated = self.total_facts_validated.load(Ordering::Relaxed);
        validated as f64 / extracted as f64
    }
}

/// A point-in-time snapshot of research metrics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MetricsSnapshot {
    pub tasks_submitted: u64,
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub tasks_cancelled: u64,
    pub total_searches: u64,
    pub total_fetches: u64,
    pub total_facts_extracted: u64,
    pub total_facts_validated: u64,
    pub total_facts_rejected: u64,
    pub total_duplicates_removed: u64,
    pub total_citations_generated: u64,
    pub total_knowledge_updates: u64,
    pub total_world_updates: u64,
    pub total_memory_updates: u64,
    pub total_duration_ms: u64,
}

impl MetricsSnapshot {
    pub fn success_rate(&self) -> f64 {
        let total = self.tasks_completed + self.tasks_failed;
        if total == 0 {
            return 1.0;
        }
        self.tasks_completed as f64 / total as f64
    }

    pub fn average_duration_ms(&self) -> f64 {
        if self.tasks_completed == 0 {
            return 0.0;
        }
        self.total_duration_ms as f64 / self.tasks_completed as f64
    }

    pub fn validation_rate(&self) -> f64 {
        if self.total_facts_extracted == 0 {
            return 0.0;
        }
        self.total_facts_validated as f64 / self.total_facts_extracted as f64
    }
}

/// Shared metrics instance for the research subsystem.
pub type SharedResearchMetrics = Arc<ResearchMetrics>;
