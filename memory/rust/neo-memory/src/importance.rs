use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::types::MemoryEntry;

/// Configuration for memory importance scoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportanceConfig {
    /// Weight for novelty score.
    pub novelty_weight: f64,
    /// Weight for usage frequency score.
    pub frequency_weight: f64,
    /// Weight for recency score.
    pub recency_weight: f64,
    /// Weight for confidence score.
    pub confidence_weight: f64,
    /// Weight for inherent importance.
    pub importance_weight: f64,
    /// Decay factor for usage frequency (per access, how much novelty decays).
    pub frequency_decay: f64,
    /// Time constant for recency decay in hours.
    pub recency_time_constant_hours: f64,
}

impl Default for ImportanceConfig {
    fn default() -> Self {
        Self {
            novelty_weight: 0.25,
            frequency_weight: 0.20,
            recency_weight: 0.25,
            confidence_weight: 0.15,
            importance_weight: 0.15,
            frequency_decay: 0.05,
            recency_time_constant_hours: 24.0,
        }
    }
}

/// Comprehensive importance scoring for memory entries.
pub struct MemoryImportance {
    config: ImportanceConfig,
}

impl MemoryImportance {
    /// Create a new importance scorer.
    #[must_use]
    pub fn new(config: ImportanceConfig) -> Self {
        Self { config }
    }

    /// Compute the comprehensive importance score for a memory entry.
    #[must_use]
    pub fn compute_score(&self, entry: &MemoryEntry) -> f64 {
        let novelty = self.novelty_score(entry);
        let frequency = self.frequency_score(entry);
        let recency = self.recency_score(entry);
        let confidence = entry.confidence as f64;
        let importance = entry.importance as f64;

        let w = &self.config;
        (novelty * w.novelty_weight)
            + (frequency * w.frequency_weight)
            + (recency * w.recency_weight)
            + (confidence * w.confidence_weight)
            + (importance * w.importance_weight)
    }

    /// Novelty score: how unique or rare this memory is.
    ///
    /// High novelty for recently created memories with few accesses.
    /// Low novelty for frequently accessed or old memories.
    #[must_use]
    pub fn novelty_score(&self, entry: &MemoryEntry) -> f64 {
        let access_count = entry.access_count.load(std::sync::atomic::Ordering::SeqCst) as f64;
        let access_novelty = 1.0 / (1.0 + access_count * self.config.frequency_decay);

        let age_hours = {
            let elapsed = Utc::now().signed_duration_since(entry.created_at);
            elapsed.num_hours().max(0) as f64
        };
        let age_novelty = 1.0 / (1.0 + age_hours / 168.0); // Decay over weeks

        (access_novelty * 0.6) + (age_novelty * 0.4)
    }

    /// Usage frequency score: how often this memory is accessed.
    ///
    /// Normalized log scale so that very frequent access doesn't dominate.
    #[must_use]
    pub fn frequency_score(&self, entry: &MemoryEntry) -> f64 {
        let access_count = entry.access_count.load(std::sync::atomic::Ordering::SeqCst) as f64;
        (access_count.ln() / (access_count.ln() + 10.0)).max(0.0)
    }

    /// Recency score: how recently this memory was accessed.
    #[must_use]
    pub fn recency_score(&self, entry: &MemoryEntry) -> f64 {
        let last_accessed = entry
            .last_accessed
            .lock()
            .map_or(entry.created_at, |l| *l);

        let elapsed_hours = {
            let elapsed = Utc::now().signed_duration_since(last_accessed);
            elapsed.num_hours().max(0) as f64
        };

        (-elapsed_hours / self.config.recency_time_constant_hours).exp()
    }

    /// Rank a collection of entries by their composite score.
    #[must_use]
    pub fn rank_entries(&self, entries: &[MemoryEntry]) -> Vec<(MemoryEntry, f64)> {
        let mut scored: Vec<(MemoryEntry, f64)> = entries
            .iter()
            .map(|e| (e.clone(), self.compute_score(e)))
            .collect();

        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        scored
    }

    /// Update an entry's importance based on its composite score.
    pub fn update_importance(&self, entry: &mut MemoryEntry) {
        let score = self.compute_score(entry);
        entry.importance = score.clamp(0.0, 1.0) as f32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MemoryId, MemoryTier};
    use std::collections::HashSet;

    fn make_entry(importance: f32) -> MemoryEntry {
        MemoryEntry::new(
            MemoryTier::Working,
            serde_json::json!("test"),
            HashSet::new(),
        )
        .with_importance(importance)
    }

    #[test]
    fn score_computation() {
        let scorer = MemoryImportance::new(ImportanceConfig::default());
        let entry = make_entry(0.7);
        let score = scorer.compute_score(&entry);
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn novelty_high_for_new() {
        let scorer = MemoryImportance::new(ImportanceConfig::default());
        let entry = make_entry(0.5);
        let novelty = scorer.novelty_score(&entry);
        assert!(novelty > 0.5);
    }

    #[test]
    fn recency_score() {
        let scorer = MemoryImportance::new(ImportanceConfig::default());
        let entry = make_entry(0.5);
        let recency = scorer.recency_score(&entry);
        assert!(recency > 0.9); // Just created, should be very recent
    }

    #[test]
    fn ranking() {
        let scorer = MemoryImportance::new(ImportanceConfig::default());
        let mut entries = vec![make_entry(0.3), make_entry(0.9), make_entry(0.5)];
        entries[1].access_count = std::sync::atomic::AtomicU64::new(50);

        let ranked = scorer.rank_entries(&entries);
        assert_eq!(ranked.len(), 3);
        assert!(ranked[0].1 >= ranked[1].1);
    }

    #[test]
    fn frequency_score_log() {
        let scorer = MemoryImportance::new(ImportanceConfig::default());

        let mut e1 = make_entry(0.5);
        e1.access_count = std::sync::atomic::AtomicU64::new(1);
        let f1 = scorer.frequency_score(&e1);

        let mut e2 = make_entry(0.5);
        e2.access_count = std::sync::atomic::AtomicU64::new(100);
        let f2 = scorer.frequency_score(&e2);

        assert!(f2 > f1);
        assert!(f2 < 1.0); // Log scale shouldn't reach 1.0 easily
    }
}
