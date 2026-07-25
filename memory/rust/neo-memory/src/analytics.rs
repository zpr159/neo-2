use std::collections::HashMap;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::types::{AnalyticsSnapshot, MemoryEntry, MemoryTier, MemoryStatus};

/// Configuration for analytics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsConfig {
    /// Whether to enable analytics tracking.
    pub enabled: bool,
    /// Interval in seconds between analytics snapshots.
    pub snapshot_interval_secs: u64,
    /// Maximum number of historical snapshots.
    pub max_history: usize,
    /// Whether to track per-entry access patterns.
    pub track_access_patterns: bool,
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            snapshot_interval_secs: 60,
            max_history: 1440, // 24 hours of minute snapshots
            track_access_patterns: true,
        }
    }
}

/// Memory analytics engine tracking usage, growth, recall rates, and health.
pub struct MemoryAnalytics {
    config: AnalyticsConfig,
    /// Historical snapshots.
    history: RwLock<Vec<AnalyticsSnapshot>>,
    /// Recall attempt counter.
    recall_attempts: RwLock<u64>,
    /// Recall hit counter.
    recall_hits: RwLock<u64>,
    /// Created entries counter.
    created_count: RwLock<u64>,
    /// Accessed entries counter.
    accessed_count: RwLock<u64>,
    /// Per-tier creation counts.
    tier_creations: RwLock<HashMap<String, u64>>,
}

impl MemoryAnalytics {
    /// Create a new analytics engine.
    #[must_use]
    pub fn new(config: AnalyticsConfig) -> Self {
        Self {
            config,
            history: RwLock::new(Vec::new()),
            recall_attempts: RwLock::new(0),
            recall_hits: RwLock::new(0),
            created_count: RwLock::new(0),
            accessed_count: RwLock::new(0),
            tier_creations: RwLock::new(HashMap::new()),
        }
    }

    /// Record a memory creation.
    pub fn record_creation(&self, tier: MemoryTier) {
        if !self.config.enabled {
            return;
        }
        *self.created_count.write() += 1;
        *self
            .tier_creations
            .write()
            .entry(tier.to_string())
            .or_insert(0) += 1;
    }

    /// Record a memory access.
    pub fn record_access(&self) {
        if !self.config.enabled {
            return;
        }
        *self.accessed_count.write() += 1;
    }

    /// Record a recall attempt.
    pub fn record_recall_attempt(&self) {
        if !self.config.enabled {
            return;
        }
        *self.recall_attempts.write() += 1;
    }

    /// Record a recall hit.
    pub fn record_recall_hit(&self) {
        if !self.config.enabled {
            return;
        }
        *self.recall_hits.write() += 1;
    }

    /// Take a snapshot of current analytics state.
    pub fn take_snapshot(
        &self,
        entries: &[MemoryEntry],
    ) -> AnalyticsSnapshot {
        let total_memories = entries.len() as u64;
        let mut per_tier = HashMap::new();
        let mut total_bytes = 0u64;
        let mut total_importance = 0.0f64;
        let mut archived_count = 0u64;
        let mut deleted_pending_gc = 0u64;
        let mut per_namespace = HashMap::new();

        let one_hour_ago =
            chrono::Utc::now() - chrono::Duration::hours(1);
        let mut created_last_hour = 0u64;
        let mut accessed_last_hour = 0u64;

        for entry in entries {
            *per_tier
                .entry(entry.tier.to_string())
                .or_insert(0u64) += 1;

            let content_size = entry.content.to_string().len() as u64;
            total_bytes += content_size;

            total_importance += entry.importance as f64;

            match entry.status {
                MemoryStatus::Archived => archived_count += 1,
                MemoryStatus::Deleted => deleted_pending_gc += 1,
                _ => {}
            }

            *per_namespace
                .entry(entry.namespace.0.clone())
                .or_insert(0u64) += 1;

            if entry.created_at > one_hour_ago {
                created_last_hour += 1;
            }

            let last_accessed = entry
                .last_accessed
                .lock()
                .map_or(entry.created_at, |l| *l);
            if last_accessed > one_hour_ago {
                accessed_last_hour += 1;
            }
        }

        let recall_attempts = *self.recall_attempts.read();
        let recall_hits = *self.recall_hits.read();
        let recall_rate = if recall_attempts > 0 {
            recall_hits as f64 / recall_attempts as f64
        } else {
            0.0
        };

        let avg_importance = if total_memories > 0 {
            total_importance / total_memories as f64
        } else {
            0.0
        };

        let health_score = self.compute_health_score(
            total_memories,
            recall_rate,
            avg_importance,
            archived_count,
            deleted_pending_gc,
        );

        let snapshot = AnalyticsSnapshot {
            total_memories,
            per_tier,
            total_bytes,
            recall_attempts,
            recall_hits,
            recall_rate,
            avg_importance,
            created_last_hour,
            accessed_last_hour,
            compression_ratio: 1.0, // Default uncompressed
            health_score,
            archived_count,
            deleted_pending_gc,
            per_namespace,
        };

        // Store in history.
        let mut history = self.history.write();
        history.push(snapshot.clone());
        if history.len() > self.config.max_history {
            history.remove(0);
        }

        snapshot
    }

    /// Compute memory health score based on multiple factors.
    fn compute_health_score(
        &self,
        total_memories: u64,
        recall_rate: f64,
        avg_importance: f64,
        archived_count: u64,
        deleted_pending_gc: u64,
    ) -> f64 {
        let mut score = 0.0;

        // Factor 1: Recall rate (0.0-0.3).
        score += recall_rate * 0.3;

        // Factor 2: Average importance (0.0-0.25).
        score += avg_importance * 0.25;

        // Factor 3: Active ratio (0.0-0.25).
        let active = total_memories.saturating_sub(archived_count + deleted_pending_gc);
        let active_ratio = if total_memories > 0 {
            active as f64 / total_memories as f64
        } else {
            1.0
        };
        score += active_ratio * 0.25;

        // Factor 4: Growth health (0.0-0.2).
        let created = *self.created_count.read();
        let accessed = *self.accessed_count.read();
        let activity_ratio = if created > 0 {
            (accessed as f64 / created as f64).min(1.0)
        } else {
            0.5
        };
        score += activity_ratio * 0.2;

        score.clamp(0.0, 1.0)
    }

    /// Get historical snapshots.
    #[must_use]
    pub fn history(&self) -> Vec<AnalyticsSnapshot> {
        self.history.read().clone()
    }

    /// Get the latest snapshot.
    #[must_use]
    pub fn latest_snapshot(&self) -> Option<AnalyticsSnapshot> {
        self.history.read().last().cloned()
    }

    /// Get growth rate (memories created per hour).
    #[must_use]
    pub fn growth_rate(&self) -> f64 {
        let history = self.history.read();
        if history.len() < 2 {
            return 0.0;
        }

        let first = &history[0];
        let last = &history[1]; // Use second as first may be current
        let time_diff_hours = 1.0; // Assuming 1-hour interval

        (last.total_memories as f64 - first.total_memories as f64) / time_diff_hours
    }

    /// Get recall trend (recall rate over time).
    #[must_use]
    pub fn recall_trend(&self) -> Vec<f64> {
        self.history
            .read()
            .iter()
            .map(|s| s.recall_rate)
            .collect()
    }

    /// Get total recall statistics.
    #[must_use]
    pub fn recall_stats(&self) -> (u64, u64, f64) {
        let attempts = *self.recall_attempts.read();
        let hits = *self.recall_hits.read();
        let rate = if attempts > 0 {
            hits as f64 / attempts as f64
        } else {
            0.0
        };
        (attempts, hits, rate)
    }

    /// Reset analytics counters.
    pub fn reset(&self) {
        *self.recall_attempts.write() = 0;
        *self.recall_hits.write() = 0;
        *self.created_count.write() = 0;
        *self.accessed_count.write() = 0;
        self.tier_creations.write().clear();
        self.history.write().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn make_entries(count: usize) -> Vec<MemoryEntry> {
        (0..count)
            .map(|i| {
                let mut entry = MemoryEntry::new(
                    MemoryTier::LongTerm,
                    serde_json::json!({"data": i}),
                    HashSet::new(),
                );
                entry.importance = 0.5;
                entry
            })
            .collect()
    }

    #[test]
    fn snapshot() {
        let analytics = MemoryAnalytics::new(AnalyticsConfig::default());
        let entries = make_entries(10);
        let snapshot = analytics.take_snapshot(&entries);
        assert_eq!(snapshot.total_memories, 10);
        assert!(snapshot.health_score >= 0.0);
    }

    #[test]
    fn recall_tracking() {
        let analytics = MemoryAnalytics::new(AnalyticsConfig::default());
        analytics.record_recall_attempt();
        analytics.record_recall_attempt();
        analytics.record_recall_hit();

        let (attempts, hits, rate) = analytics.recall_stats();
        assert_eq!(attempts, 2);
        assert_eq!(hits, 1);
        assert!((rate - 0.5).abs() < 0.01);
    }

    #[test]
    fn creation_tracking() {
        let analytics = MemoryAnalytics::new(AnalyticsConfig::default());
        analytics.record_creation(MemoryTier::Working);
        analytics.record_creation(MemoryTier::Episodic);
        analytics.record_creation(MemoryTier::Working);

        let entries = make_entries(0);
        let snapshot = analytics.take_snapshot(&entries);
        assert_eq!(snapshot.total_memories, 0);
    }

    #[test]
    fn health_score() {
        let analytics = MemoryAnalytics::new(AnalyticsConfig::default());
        analytics.record_recall_attempt();
        analytics.record_recall_hit();

        let mut entry = MemoryEntry::new(
            MemoryTier::LongTerm,
            serde_json::json!("test"),
            HashSet::new(),
        );
        entry.importance = 0.8;

        let snapshot = analytics.take_snapshot(&[entry]);
        assert!(snapshot.health_score > 0.5);
    }

    #[test]
    fn growth_rate() {
        let analytics = MemoryAnalytics::new(AnalyticsConfig::default());
        let entries = make_entries(5);
        analytics.take_snapshot(&entries);
        analytics.take_snapshot(&entries);

        let rate = analytics.growth_rate();
        assert!(rate >= 0.0);
    }
}
