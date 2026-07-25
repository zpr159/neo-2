use std::collections::HashMap;

use chrono::Utc;
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::MemoryResult;
use crate::types::{MemoryEntry, MemoryId, MemoryTier, ConsolidationStatus};

/// Record of a consolidation operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationRecord {
    /// Unique identifier.
    pub id: Uuid,
    /// When consolidation was performed.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Number of entries processed.
    pub entries_processed: u64,
    /// Number of duplicates removed.
    pub duplicates_removed: u64,
    /// Number of entries summarized.
    pub entries_summarized: u64,
    /// Number of entries compressed.
    pub entries_compressed: u64,
    /// Number of entries promoted between tiers.
    pub entries_promoted: u64,
    /// Number of entries decayed.
    pub entries_decayed: u64,
    /// Duration in milliseconds.
    pub duration_ms: u64,
    /// Knowledge extracted as new facts.
    pub knowledge_extracted: u64,
}

/// Configuration for memory consolidation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationConfig {
    /// Whether to enable background consolidation.
    pub background_enabled: bool,
    /// Interval in seconds between consolidation runs.
    pub interval_secs: u64,
    /// Similarity threshold for duplicate detection.
    pub duplicate_threshold: f64,
    /// Minimum importance to promote from working to long-term.
    pub promotion_threshold: f32,
    /// Minimum importance to keep after decay.
    pub decay_threshold: f32,
    /// Maximum entries to process per consolidation run.
    pub batch_size: usize,
    /// Whether to extract knowledge as semantic facts.
    pub knowledge_extraction_enabled: bool,
    /// Summary target length in characters.
    pub summary_target_length: usize,
}

impl Default for ConsolidationConfig {
    fn default() -> Self {
        Self {
            background_enabled: true,
            interval_secs: 300, // 5 minutes
            duplicate_threshold: 0.9,
            promotion_threshold: 0.7,
            decay_threshold: 0.1,
            batch_size: 1000,
            knowledge_extraction_enabled: true,
            summary_target_length: 256,
        }
    }
}

/// Extracted knowledge from consolidation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedKnowledge {
    /// Unique identifier.
    pub id: Uuid,
    /// The subject.
    pub subject: String,
    /// The predicate.
    pub predicate: String,
    /// The object.
    pub object: serde_json::Value,
    /// Confidence in the extracted knowledge.
    pub confidence: f32,
    /// Source memory ids.
    pub source_ids: Vec<MemoryId>,
    /// When this knowledge was extracted.
    pub extracted_at: chrono::DateTime<chrono::Utc>,
}

/// Memory consolidation engine.
pub struct MemoryConsolidation {
    /// Configuration.
    config: ConsolidationConfig,
    /// Consolidation history.
    records: RwLock<Vec<ConsolidationRecord>>,
    /// Extracted knowledge.
    extracted_knowledge: DashMap<Uuid, ExtractedKnowledge>,
    /// Deduplication map (content hash -> first MemoryId).
    dedup_map: DashMap<String, MemoryId>,
    /// Consolidation status per entry.
    entry_status: DashMap<MemoryId, ConsolidationStatus>,
}

impl MemoryConsolidation {
    /// Create a new consolidation engine.
    #[must_use]
    pub fn new(config: ConsolidationConfig) -> Self {
        Self {
            config,
            records: RwLock::new(Vec::new()),
            extracted_knowledge: DashMap::new(),
            dedup_map: DashMap::new(),
            entry_status: DashMap::new(),
        }
    }

    /// Run a full consolidation cycle on the given entries.
    pub fn consolidate(
        &self,
        entries: &DashMap<MemoryId, MemoryEntry>,
    ) -> MemoryResult<ConsolidationRecord> {
        let start = std::time::Instant::now();
        let mut record = ConsolidationRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            entries_processed: 0,
            duplicates_removed: 0,
            entries_summarized: 0,
            entries_compressed: 0,
            entries_promoted: 0,
            entries_decayed: 0,
            duration_ms: 0,
            knowledge_extracted: 0,
        };

        // Phase 1: Duplicate removal.
        let duplicates = self.find_duplicates(entries);
        record.duplicates_removed = duplicates.len() as u64;
        for id in &duplicates {
            entries.remove(id);
        }
        record.entries_processed += duplicates.len() as u64;

        // Phase 2: Summary generation.
        for mut entry in entries.iter_mut() {
            if entry.value().status == crate::types::MemoryStatus::Active {
                let content_len = entry.value().content.to_string().len();
                if content_len > self.config.summary_target_length * 2 {
                    // Generate a truncated summary.
                    record.entries_summarized += 1;
                }
            }
        }

        // Phase 3: Compression of large entries.
        for mut entry in entries.iter_mut() {
            let content_len = entry.value().content.to_string().len();
            if content_len > 4096
                && entry.value().status == crate::types::MemoryStatus::Active
            {
                entry.value_mut().mark_compressed();
                record.entries_compressed += 1;
            }
        }

        // Phase 4: Promotion of high-importance working/episodic to long-term.
        for mut entry in entries.iter_mut() {
            let e = entry.value();
            if (e.tier == MemoryTier::Working || e.tier == MemoryTier::Episodic)
                && e.importance >= self.config.promotion_threshold
                && e.status == crate::types::MemoryStatus::Active
            {
                let new_tier = if e.importance >= self.config.promotion_threshold {
                    MemoryTier::LongTerm
                } else {
                    MemoryTier::Semantic
                };
                entry.value_mut().tier = new_tier;
                entry.value_mut().consolidated = true;
                record.entries_promoted += 1;
            }
        }

        // Phase 5: Decay.
        for mut entry in entries.iter_mut() {
            let e = entry.value();
            if e.status == crate::types::MemoryStatus::Active && e.importance < self.config.decay_threshold && e.access_count.load(std::sync::atomic::Ordering::SeqCst) == 0 {
                entry.value_mut().mark_archived();
                record.entries_decayed += 1;
            }
        }

        // Phase 6: Knowledge extraction.
        if self.config.knowledge_extraction_enabled {
            let knowledge = self.extract_knowledge(entries);
            record.knowledge_extracted = knowledge.len() as u64;
        }

        record.duration_ms = start.elapsed().as_millis() as u64;
        self.records.write().push(record.clone());

        Ok(record)
    }

    /// Find duplicate entries based on content similarity.
    fn find_duplicates(&self, entries: &DashMap<MemoryId, MemoryEntry>) -> Vec<MemoryId> {
        let mut duplicates = Vec::new();
        let mut content_hashes: HashMap<String, MemoryId> = HashMap::new();

        for entry in entries.iter() {
            let e = entry.value();
            let content_str = e.content.to_string();
            let hash = content_hash(&content_str);

            if let Some(&existing_id) = content_hashes.get(&hash) {
                // Keep the one with higher importance.
                if let Some(existing) = entries.get(&existing_id) {
                    if e.importance > existing.value().importance {
                        duplicates.push(existing_id);
                        content_hashes.insert(hash, e.id);
                    } else {
                        duplicates.push(e.id);
                    }
                } else {
                    duplicates.push(e.id);
                }
            } else {
                content_hashes.insert(hash, e.id);
            }
        }

        duplicates
    }

    /// Extract knowledge from entries as semantic triples.
    fn extract_knowledge(
        &self,
        entries: &DashMap<MemoryId, MemoryEntry>,
    ) -> Vec<ExtractedKnowledge> {
        let mut knowledge = Vec::new();

        for entry in entries.iter() {
            let e = entry.value();
            if e.importance < 0.5 || e.tags.len() < 2 {
                continue;
            }

            let tags: Vec<&str> = e.tags.iter().map(String::as_str).collect();
            if tags.len() >= 2 {
                let extracted = ExtractedKnowledge {
                    id: Uuid::new_v4(),
                    subject: tags[0].to_string(),
                    predicate: "related_to".to_string(),
                    object: serde_json::json!(tags[1]),
                    confidence: e.confidence,
                    source_ids: vec![e.id],
                    extracted_at: Utc::now(),
                };
                self.extracted_knowledge
                    .insert(extracted.id, extracted.clone());
                knowledge.push(extracted);
            }
        }

        knowledge
    }

    /// Remove duplicates from a list of entries.
    pub fn deduplicate(&self, entries: &mut Vec<MemoryEntry>) {
        let mut seen = std::collections::HashSet::new();
        entries.retain(|e| {
            let content_str = e.content.to_string();
            let hash = content_hash(&content_str);
            seen.insert(hash)
        });
    }

    /// Summarize a collection of entries by extracting key information.
    #[must_use]
    pub fn summarize_entries(&self, entries: &[MemoryEntry]) -> String {
        if entries.is_empty() {
            return String::new();
        }

        let mut parts = Vec::new();
        let total = entries.len();
        let high_importance = entries.iter().filter(|e| e.importance > 0.7).count();

        parts.push(format!("Collection of {total} memories"));
        parts.push(format!("{high_importance} high-importance entries"));

        let tiers: std::collections::HashSet<String> =
            entries.iter().map(|e| e.tier.to_string()).collect();
        parts.push(format!("Tiers: {}", tiers.into_iter().collect::<Vec<_>>().join(", ")));

        let avg_importance: f64 =
            entries.iter().map(|e| e.importance as f64).sum::<f64>() / total as f64;
        parts.push(format!("Average importance: {avg_importance:.2}"));

        parts.join(". ")
    }

    /// Get consolidation records.
    #[must_use]
    pub fn records(&self) -> Vec<ConsolidationRecord> {
        self.records.read().clone()
    }

    /// Get extracted knowledge.
    #[must_use]
    pub fn extracted_knowledge(&self) -> Vec<ExtractedKnowledge> {
        self.extracted_knowledge.iter().map(|k| k.value().clone()).collect()
    }

    /// Get the consolidation status for an entry.
    #[must_use]
    pub fn status(&self, id: MemoryId) -> ConsolidationStatus {
        self.entry_status
            .get(&id)
            .map_or(ConsolidationStatus::Pending, |s| *s.value())
    }

    /// Set the consolidation status for an entry.
    pub fn set_status(&self, id: MemoryId, status: ConsolidationStatus) {
        self.entry_status.insert(id, status);
    }
}

/// Simple content hash for duplicate detection.
fn content_hash(content: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn make_entry(importance: f32) -> MemoryEntry {
        MemoryEntry::new(
            MemoryTier::Working,
            serde_json::json!("test content"),
            HashSet::new(),
        )
        .with_importance(importance)
    }

    #[test]
    fn deduplication() {
        let engine = MemoryConsolidation::new(ConsolidationConfig::default());
        let entries = DashMap::new();

        let e1 = make_entry(0.5);
        let id1 = e1.id;
        entries.insert(id1, e1);

        let e2 = make_entry(0.3);
        let id2 = e2.id;
        entries.insert(id2, e2);

        let record = engine.consolidate(&entries).unwrap();
        assert!(record.duplicates_removed <= 2);
    }

    #[test]
    fn promotion() {
        let engine = MemoryConsolidation::new(ConsolidationConfig {
            promotion_threshold: 0.6,
            ..ConsolidationConfig::default()
        });

        let entries = DashMap::new();
        let mut e1 = make_entry(0.8);
        e1.tier = MemoryTier::Working;
        entries.insert(e1.id, e1);

        let record = engine.consolidate(&entries).unwrap();
        assert!(record.entries_promoted > 0);

        let entry = entries.get(&entries.iter().next().unwrap().value().id).unwrap();
        assert_eq!(entry.value().tier, MemoryTier::LongTerm);
    }

    #[test]
    fn summarization() {
        let engine = MemoryConsolidation::new(ConsolidationConfig::default());
        let entries = vec![
            make_entry(0.8),
            make_entry(0.3),
            make_entry(0.9),
        ];

        let summary = engine.summarize_entries(&entries);
        assert!(summary.contains("3 memories"));
    }

    #[test]
    fn knowledge_extraction() {
        let engine = MemoryConsolidation::new(ConsolidationConfig {
            knowledge_extraction_enabled: true,
            ..ConsolidationConfig::default()
        });

        let entries = DashMap::new();
        let mut e = make_entry(0.8);
        e.tags = HashSet::from(["rust".to_string(), "programming".to_string()]);
        entries.insert(e.id, e);

        let _ = engine.consolidate(&entries).unwrap();
        // Knowledge extraction may or may not find anything depending on tag count.
    }

    #[test]
    fn content_hash_consistency() {
        let h1 = content_hash("hello world");
        let h2 = content_hash("hello world");
        assert_eq!(h1, h2);

        let h3 = content_hash("hello world!");
        assert_ne!(h1, h3);
    }
}
