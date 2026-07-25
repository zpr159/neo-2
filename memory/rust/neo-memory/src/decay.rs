use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::{MemoryEntry, MemoryId, MemoryStatus, RetentionConfig, RetentionPolicy};

/// Record of decay operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayRecord {
    /// When decay was applied.
    pub timestamp: DateTime<Utc>,
    /// Number of entries decayed.
    pub entries_decayed: u64,
    /// Number of entries archived.
    pub entries_archived: u64,
    /// Number of entries compressed.
    pub entries_compressed: u64,
    /// Number of entries garbage collected.
    pub entries_gc: u64,
}

/// Memory decay engine implementing forgetting curves and retention policies.
pub struct MemoryDecay {
    config: RetentionConfig,
    records: Vec<DecayRecord>,
}

impl MemoryDecay {
    /// Create a new decay engine.
    #[must_use]
    pub fn new(config: RetentionConfig) -> Self {
        Self {
            config,
            records: Vec::new(),
        }
    }

    /// Compute the retention probability using an Ebbinghaus-like forgetting curve.
    ///
    /// R = e^(-t/S) where t is time since last access in hours and S is stability.
    /// Stability is influenced by importance, access count, and confidence.
    #[must_use]
    pub fn retention_probability(&self, entry: &MemoryEntry) -> f64 {
        let last_accessed = entry
            .last_accessed
            .lock()
            .map_or(entry.created_at, |l| *l);

        let elapsed_hours = {
            let elapsed = Utc::now().signed_duration_since(last_accessed);
            elapsed.num_hours().max(0) as f64
        };

        // Stability is computed from entry properties.
        let access_bonus = entry.access_count.load(std::sync::atomic::Ordering::SeqCst) as f64;
        let stability = (entry.importance as f64 * 100.0)
            + (entry.confidence as f64 * 50.0)
            + (access_bonus * 5.0)
            + 10.0; // Minimum stability

        (-elapsed_hours / stability).exp()
    }

    /// Determine if an entry should be garbage collected.
    #[must_use]
    pub fn should_gc(&self, entry: &MemoryEntry) -> bool {
        if entry.status == MemoryStatus::Pinned || entry.status == MemoryStatus::Deleted {
            return false;
        }

        match self.config.policy {
            RetentionPolicy::Permanent => false,
            RetentionPolicy::TimeBased => {
                let elapsed = Utc::now().signed_duration_since(entry.created_at);
                let max_duration =
                    chrono::Duration::seconds(self.config.max_age_secs as i64);
                elapsed > max_duration
            }
            RetentionPolicy::AccessBased => {
                entry.access_count.load(std::sync::atomic::Ordering::SeqCst)
                    < self.config.min_access_count
            }
            RetentionPolicy::ImportanceBased => {
                entry.importance < self.config.min_importance
            }
            RetentionPolicy::Composite => {
                let retention = self.retention_probability(entry);
                retention < 0.1 && entry.importance < self.config.min_importance
            }
        }
    }

    /// Determine if an entry should be archived.
    #[must_use]
    pub fn should_archive(&self, entry: &MemoryEntry) -> bool {
        if entry.status != MemoryStatus::Active {
            return false;
        }

        let retention = self.retention_probability(entry);
        retention < 0.3 && entry.importance >= self.config.min_importance
    }

    /// Determine if an entry should be compressed.
    #[must_use]
    pub fn should_compress(&self, entry: &MemoryEntry) -> bool {
        if entry.status != MemoryStatus::Active {
            return false;
        }

        entry.estimated_tokens > 500
            || entry.content.to_string().len() > 4096
    }

    /// Apply decay to a collection of entries.
    pub fn apply_decay(
        &mut self,
        entries: &mut Vec<MemoryEntry>,
    ) -> DecayRecord {
        let mut record = DecayRecord {
            timestamp: Utc::now(),
            entries_decayed: 0,
            entries_archived: 0,
            entries_compressed: 0,
            entries_gc: 0,
        };

        for entry in entries.iter_mut() {
            if entry.status == MemoryStatus::Pinned {
                continue;
            }

            // Apply importance decay based on retention.
            let retention = self.retention_probability(entry);
            if retention < 0.5 && entry.importance > self.config.min_importance {
                entry.importance *= retention as f32;
                entry.importance = entry.importance.max(0.0);
                record.entries_decayed += 1;
            }

            // Archive low-retention entries.
            if self.should_archive(entry) {
                entry.mark_archived();
                record.entries_archived += 1;
            }

            // Compress large entries.
            if self.should_compress(entry) {
                entry.mark_compressed();
                record.entries_compressed += 1;
            }
        }

        // Garbage collect.
        let before = entries.len();
        entries.retain(|e| !self.should_gc(e));
        record.entries_gc = (before - entries.len()) as u64;

        self.records.push(record.clone());
        record
    }

    /// Get retention probability for an entry.
    #[must_use]
    pub fn get_retention(&self, entry: &MemoryEntry) -> f64 {
        self.retention_probability(entry)
    }

    /// Estimate remaining useful life of a memory entry in hours.
    #[must_use]
    pub fn estimated_remaining_life_hours(&self, entry: &MemoryEntry) -> f64 {
        let retention = self.retention_probability(entry);
        if retention <= 0.01 {
            return 0.0;
        }

        let access_bonus = entry.access_count.load(std::sync::atomic::Ordering::SeqCst) as f64;
        let stability = (entry.importance as f64 * 100.0)
            + (entry.confidence as f64 * 50.0)
            + (access_bonus * 5.0)
            + 10.0;

        // Time until retention drops to 0.01.
        stability * (0.01_f64).ln().abs()
    }

    /// Get decay records.
    #[must_use]
    pub fn records(&self) -> &[DecayRecord] {
        &self.records
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MemoryTier, MemoryPriority};
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
    fn retention_high_for_new_important() {
        let decay = MemoryDecay::new(RetentionConfig::default());
        let entry = make_entry(0.9);
        let retention = decay.retention_probability(&entry);
        assert!(retention > 0.9);
    }

    #[test]
    fn gc_for_unimportant_old() {
        let config = RetentionConfig {
            policy: RetentionPolicy::ImportanceBased,
            min_importance: 0.3,
            ..RetentionConfig::default()
        };
        let decay = MemoryDecay::new(config);

        let entry = make_entry(0.1);
        assert!(decay.should_gc(&entry));
    }

    #[test]
    fn no_gc_for_pinned() {
        let decay = MemoryDecay::new(RetentionConfig::default());
        let mut entry = make_entry(0.0);
        entry.status = MemoryStatus::Pinned;
        assert!(!decay.should_gc(&entry));
    }

    #[test]
    fn archive_low_retention() {
        let decay = MemoryDecay::new(RetentionConfig::default());
        let mut entry = make_entry(0.5);
        // Set old last_accessed to reduce retention.
        entry.last_accessed = std::sync::Mutex::new(
            Utc::now() - chrono::Duration::hours(1000),
        );
        assert!(decay.should_archive(&entry));
    }

    #[test]
    fn apply_decay() {
        let mut decay = MemoryDecay::new(RetentionConfig::default());
        let mut entries = vec![make_entry(0.9), make_entry(0.1)];
        entries[1].last_accessed = std::sync::Mutex::new(
            Utc::now() - chrono::Duration::hours(10000),
        );

        let record = decay.apply_decay(&mut entries);
        assert!(record.entries_decayed > 0 || record.entries_archived > 0 || record.entries_gc > 0);
    }

    #[test]
    fn remaining_life() {
        let decay = MemoryDecay::new(RetentionConfig::default());
        let mut entry = make_entry(0.8);
        entry.access_count = std::sync::atomic::AtomicU64::new(10);

        let life = decay.estimated_remaining_life_hours(&entry);
        assert!(life > 0.0);
    }
}
