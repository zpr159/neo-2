use std::collections::HashMap;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{MemoryError, MemoryResult};
use crate::types::{EpisodeOutcome, MemoryEntry, MemoryId, MemoryTier};

/// A single episodic experience with rich temporal and emotional context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    /// Unique identifier for this episode.
    pub id: Uuid,
    /// Associated memory entry id.
    pub memory_id: MemoryId,
    /// Human-readable description of the event.
    pub event_description: String,
    /// Arbitrary contextual key-value pairs.
    pub context: HashMap<String, serde_json::Value>,
    /// Emotional valence ranging from -1.0 (negative) to 1.0 (positive).
    pub emotion_valence: Option<f32>,
    /// Emotional arousal (intensity) ranging from 0.0 to 1.0.
    pub emotion_arousal: Option<f32>,
    /// When this episode occurred.
    pub timestamp: DateTime<Utc>,
    /// Duration of the event in milliseconds.
    pub duration_ms: Option<u64>,
    /// Outcome of the episode.
    pub outcome: EpisodeOutcome,
    /// Participants or entities involved.
    pub participants: Vec<String>,
    /// Location or context of the event.
    pub location: Option<String>,
    /// Summary of the episode (generated or manual).
    pub summary: Option<String>,
    /// Compressed representation (for long-term storage).
    pub compressed_content: Option<Vec<u8>>,
    /// Tags for categorization.
    pub tags: Vec<String>,
}

impl Episode {
    /// Create a new episode.
    pub fn new(
        memory_id: MemoryId,
        event_description: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            memory_id,
            event_description: event_description.into(),
            context: HashMap::new(),
            emotion_valence: None,
            emotion_arousal: None,
            timestamp: Utc::now(),
            duration_ms: None,
            outcome: EpisodeOutcome::Unknown,
            participants: Vec::new(),
            location: None,
            summary: None,
            compressed_content: None,
            tags: Vec::new(),
        }
    }

    /// Set emotional metadata.
    #[must_use]
    pub fn with_emotion(mut self, valence: f32, arousal: f32) -> Self {
        self.emotion_valence = Some(valence.clamp(-1.0, 1.0));
        self.emotion_arousal = Some(arousal.clamp(0.0, 1.0));
        self
    }

    /// Set outcome.
    #[must_use]
    pub fn with_outcome(mut self, outcome: EpisodeOutcome) -> Self {
        self.outcome = outcome;
        self
    }

    /// Set context.
    #[must_use]
    pub fn with_context(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.context.insert(key.into(), value);
        self
    }

    /// Set duration.
    #[must_use]
    pub fn with_duration_ms(mut self, ms: u64) -> Self {
        self.duration_ms = Some(ms);
        self
    }

    /// Compute emotional intensity (combination of valence magnitude and arousal).
    #[must_use]
    pub fn emotional_intensity(&self) -> f32 {
        let valence_mag = self.emotion_valence.map_or(0.0, |v| v.abs());
        let arousal = self.emotion_arousal.unwrap_or(0.0);
        (valence_mag + arousal) / 2.0
    }

    /// Compute a temporal relevance score based on recency and importance.
    #[must_use]
    pub fn temporal_relevance(&self, importance: f32) -> f64 {
        let elapsed = Utc::now().signed_duration_since(self.timestamp);
        let hours = elapsed.num_hours().max(0) as f64;
        let recency = 1.0 / (1.0 + hours / 24.0); // Decay over days
        let emotional = self.emotional_intensity() as f64;
        (importance as f64 * 0.4) + (recency * 0.35) + (emotional * 0.25)
    }
}

/// Configuration for episodic memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicMemoryConfig {
    /// Maximum number of episodes in memory (in-memory store).
    pub max_in_memory: usize,
    /// Whether to persist to sled DB.
    pub persistence_enabled: bool,
    /// Path for sled DB persistence.
    pub persistence_path: Option<String>,
    /// Maximum summary length for auto-summarization.
    pub max_summary_length: usize,
    /// Compression threshold in bytes (compress episodes larger than this).
    pub compression_threshold: usize,
}

impl Default for EpisodicMemoryConfig {
    fn default() -> Self {
        Self {
            max_in_memory: 10_000,
            persistence_enabled: false,
            persistence_path: None,
            max_summary_length: 256,
            compression_threshold: 4096,
        }
    }
}

/// Episodic memory store backed by an in-memory map and optional persistent sled DB.
#[derive(Debug)]
pub struct EpisodicMemory {
    episodes: DashMap<Uuid, Episode>,
    entries: DashMap<MemoryId, MemoryEntry>,
    index_by_memory: DashMap<MemoryId, Uuid>,
    index_by_time: RwLock<Vec<(DateTime<Utc>, Uuid)>>,
    db: Option<sled::Db>,
    config: EpisodicMemoryConfig,
}

impl EpisodicMemory {
    /// Create a new episodic memory store.
    pub fn new(config: EpisodicMemoryConfig) -> MemoryResult<Self> {
        let db = if config.persistence_enabled {
            let path = config
                .persistence_path
                .as_deref()
                .unwrap_or("/tmp/neo-episodic");
            Some(
                sled::open(path)
                    .map_err(|e| MemoryError::PersistenceError(e.to_string()))?,
            )
        } else {
            None
        };
        Ok(Self {
            episodes: DashMap::new(),
            entries: DashMap::new(),
            index_by_memory: DashMap::new(),
            index_by_time: RwLock::new(Vec::new()),
            db,
            config,
        })
    }

    /// Store an episode alongside its memory entry.
    pub fn store_episode(
        &self,
        mut entry: MemoryEntry,
        mut episode: Episode,
    ) -> MemoryResult<MemoryId> {
        if self.episodes.len() >= self.config.max_in_memory {
            self.evict_oldest(1)?;
        }

        let memory_id = entry.id;
        episode.memory_id = memory_id;

        // Persist to sled DB if enabled.
        if let Some(ref db) = self.db {
            let key = episode.id.as_bytes().to_vec();
            let value = serde_json::to_vec(&episode)
                .map_err(|e| MemoryError::SerializationError(e.to_string()))?;
            db.insert(key, value)
                .map_err(|e| MemoryError::PersistenceError(e.to_string()))?;

            let entry_key = format!("entry:{}", memory_id);
            let entry_val = serde_json::to_vec(&entry)
                .map_err(|e| MemoryError::SerializationError(e.to_string()))?;
            db.insert(entry_key.as_bytes(), entry_val)
                .map_err(|e| MemoryError::PersistenceError(e.to_string()))?;
        }

        // Update indexes.
        self.index_by_time
            .write()
            .push((episode.timestamp, episode.id));
        self.index_by_memory.insert(memory_id, episode.id);
        self.entries.insert(memory_id, entry);
        self.episodes.insert(episode.id, episode);

        Ok(memory_id)
    }

    /// Recall a memory entry and its associated episode by memory id.
    pub fn recall(&self, id: MemoryId) -> Option<(MemoryEntry, Episode)> {
        let episode_id = self.index_by_memory.get(&id)?;
        let episode = self.episodes.get(&episode_id)?;
        let entry = self.entries.get(&id)?;
        let mut entry_clone = entry.value().clone();
        entry_clone.access();
        Some((entry_clone, episode.value().clone()))
    }

    /// Recall by episode id.
    pub fn recall_by_episode(&self, episode_id: Uuid) -> Option<(MemoryEntry, Episode)> {
        let episode = self.episodes.get(&episode_id)?;
        let memory_id = episode.value().memory_id;
        let entry = self.entries.get(&memory_id)?;
        let mut entry_clone = entry.value().clone();
        entry_clone.access();
        Some((entry_clone, episode.value().clone()))
    }

    /// Search for episodes within a time range.
    #[must_use]
    pub fn search_by_time(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<(MemoryEntry, Episode)> {
        let time_index = self.index_by_time.read();
        let matching_ids: Vec<Uuid> = time_index
            .iter()
            .filter(|(ts, _)| *ts >= start && *ts <= end)
            .map(|(_, id)| *id)
            .collect();

        let mut results = Vec::new();
        for episode_id in matching_ids {
            if let Some(episode) = self.episodes.get(&episode_id) {
                let memory_id = episode.value().memory_id;
                if let Some(entry) = self.entries.get(&memory_id) {
                    results.push((entry.value().clone(), episode.value().clone()));
                }
            }
        }
        results
    }

    /// Search for episodes by keyword in event description or context.
    #[must_use]
    pub fn search_by_keyword(&self, keyword: &str) -> Vec<(MemoryEntry, Episode)> {
        let lower = keyword.to_lowercase();
        let mut results = Vec::new();

        for episode_ref in self.episodes.iter() {
            let episode = episode_ref.value();
            if episode.event_description.to_lowercase().contains(&lower)
                || episode
                    .tags
                    .iter()
                    .any(|t| t.to_lowercase().contains(&lower))
                || episode.participants.iter().any(|p| p.to_lowercase().contains(&lower))
                || episode
                    .context
                    .values()
                    .any(|v| v.to_string().to_lowercase().contains(&lower))
            {
                if let Some(entry) = self.entries.get(&episode.memory_id) {
                    results.push((entry.value().clone(), episode.clone()));
                }
            }
        }
        results
    }

    /// Search for episodes by outcome.
    #[must_use]
    pub fn search_by_outcome(&self, outcome: EpisodeOutcome) -> Vec<(MemoryEntry, Episode)> {
        let mut results = Vec::new();
        for episode_ref in self.episodes.iter() {
            let episode = episode_ref.value();
            if episode.outcome == outcome {
                if let Some(entry) = self.entries.get(&episode.memory_id) {
                    results.push((entry.value().clone(), episode.clone()));
                }
            }
        }
        results
    }

    /// Search for episodes by participant.
    #[must_use]
    pub fn search_by_participant(&self, participant: &str) -> Vec<(MemoryEntry, Episode)> {
        let lower = participant.to_lowercase();
        let mut results = Vec::new();
        for episode_ref in self.episodes.iter() {
            let episode = episode_ref.value();
            if episode.participants.iter().any(|p| p.to_lowercase() == lower) {
                if let Some(entry) = self.entries.get(&episode.memory_id) {
                    results.push((entry.value().clone(), episode.clone()));
                }
            }
        }
        results
    }

    /// Retrieve the most recent episodes with their memory entries.
    #[must_use]
    pub fn recent(&self, count: usize) -> Vec<(MemoryEntry, Episode)> {
        let time_index = self.index_by_time.read();
        let recent_ids: Vec<Uuid> = time_index
            .iter()
            .rev()
            .take(count)
            .map(|(_, id)| *id)
            .collect();

        let mut results = Vec::new();
        for episode_id in recent_ids {
            if let Some(episode) = self.episodes.get(&episode_id) {
                let memory_id = episode.value().memory_id;
                if let Some(entry) = self.entries.get(&memory_id) {
                    results.push((entry.value().clone(), episode.value().clone()));
                }
            }
        }
        results
    }

    /// Retrieve episodes sorted by emotional intensity.
    #[must_use]
    pub fn most_emotional(&self, count: usize) -> Vec<(MemoryEntry, Episode)> {
        let mut all: Vec<(MemoryEntry, Episode)> = self
            .episodes
            .iter()
            .filter_map(|ep| {
                let memory_id = ep.value().memory_id;
                self.entries.get(&memory_id).map(|entry| {
                    (entry.value().clone(), ep.value().clone())
                })
            })
            .collect();

        all.sort_by(|a, b| {
            b.1.emotional_intensity()
                .partial_cmp(&a.1.emotional_intensity())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        all.into_iter().take(count).collect()
    }

    /// Generate a summary for an episode by extracting key information.
    pub fn summarize_episode(&self, episode_id: Uuid) -> MemoryResult<String> {
        let episode = self
            .episodes
            .get(&episode_id)
            .ok_or_else(|| MemoryError::NotFound(format!("Episode {episode_id} not found")))?;

        let mut parts = Vec::new();
        parts.push(episode.event_description.clone());

        if let Some(ref location) = episode.location {
            parts.push(format!("Location: {location}"));
        }

        if !episode.participants.is_empty() {
            parts.push(format!("Participants: {}", episode.participants.join(", ")));
        }

        parts.push(format!("Outcome: {}", episode.outcome));

        if let Some(valence) = episode.emotion_valence {
            let sentiment = if valence > 0.3 {
                "positive"
            } else if valence < -0.3 {
                "negative"
            } else {
                "neutral"
            };
            parts.push(format!("Sentiment: {sentiment}"));
        }

        let summary = parts.join(". ");

        // Truncate if necessary.
        let truncated = if summary.len() > self.config.max_summary_length {
            format!(
                "{}...",
                &summary[..self.config.max_summary_length.saturating_sub(3)]
            )
        } else {
            summary
        };

        // Store the summary back.
        drop(episode);
        if let Some(mut episode) = self.episodes.get_mut(&episode_id) {
            episode.summary = Some(truncated.clone());
        }

        Ok(truncated)
    }

    /// Compress an episode's content for long-term storage.
    pub fn compress_episode(&self, episode_id: Uuid) -> MemoryResult<()> {
        let episode = self
            .episodes
            .get(&episode_id)
            .ok_or_else(|| MemoryError::NotFound(format!("Episode {episode_id} not found")))?;

        let content_bytes = episode.event_description.as_bytes();
        if content_bytes.len() < self.config.compression_threshold {
            return Ok(());
        }

        // Simple run-length encoding for demonstration.
        // In production, this would use a real compression library.
        let compressed = simple_compress(content_bytes);
        drop(episode);

        if let Some(mut episode) = self.episodes.get_mut(&episode_id) {
            episode.compressed_content = Some(compressed);

            // Also update the memory entry.
            if let Some(mut entry) = self.entries.get_mut(&episode.value().memory_id) {
                entry.mark_compressed();
            }
        }

        Ok(())
    }

    /// Return the total number of stored episodes.
    #[must_use]
    pub fn count(&self) -> usize {
        self.episodes.len()
    }

    /// Remove an episode by id.
    pub fn remove_episode(&self, episode_id: Uuid) -> MemoryResult<bool> {
        if let Some((_, episode)) = self.episodes.remove(&episode_id) {
            self.entries.remove(&episode.memory_id);
            self.index_by_memory.remove(&episode.memory_id);

            // Remove from time index.
            {
                let mut time_index = self.index_by_time.write();
                time_index.retain(|(_, id)| *id != episode_id);
            }

            // Remove from sled DB.
            if let Some(ref db) = self.db {
                let _ = db.remove(episode_id.as_bytes());
                let entry_key = format!("entry:{}", episode.memory_id);
                let _ = db.remove(entry_key.as_bytes());
            }

            return Ok(true);
        }
        Ok(false)
    }

    /// Evict the oldest N episodes.
    fn evict_oldest(&self, count: usize) -> MemoryResult<()> {
        let to_evict: Vec<Uuid> = {
            let time_index = self.index_by_time.read();
            time_index.iter().take(count).map(|(_, id)| *id).collect()
        };

        for id in to_evict {
            self.remove_episode(id)?;
        }
        Ok(())
    }
}

/// Simple compression using run-length encoding.
fn simple_compress(data: &[u8]) -> Vec<u8> {
    let mut compressed = Vec::new();
    if data.is_empty() {
        return compressed;
    }

    let mut i = 0;
    while i < data.len() {
        let byte = data[i];
        let mut run = 1u8;
        while i + (run as usize) < data.len()
            && data[i + (run as usize)] == byte
            && run < 255
        {
            run += 1;
        }
        compressed.push(run);
        compressed.push(byte);
        i += run as usize;
    }
    compressed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MemoryPriority;

    fn make_episode_entry() -> MemoryEntry {
        let mut entry = MemoryEntry::new(
            MemoryTier::Episodic,
            serde_json::json!("test event"),
            std::collections::HashSet::from(["test".to_string()]),
        );
        entry.importance = 0.7;
        entry.priority = MemoryPriority::Normal;
        entry
    }

    #[test]
    fn store_and_recall() {
        let mem = EpisodicMemory::new(EpisodicMemoryConfig::default()).unwrap();
        let entry = make_episode_entry();
        let memory_id = entry.id;
        let episode = Episode::new(memory_id, "Test event happened");

        let id = mem.store_episode(entry, episode).unwrap();
        assert_eq!(id, memory_id);

        let result = mem.recall(memory_id);
        assert!(result.is_some());
        let (entry, episode) = result.unwrap();
        assert_eq!(entry.id, memory_id);
        assert_eq!(episode.event_description, "Test event happened");
    }

    #[test]
    fn search_by_time() {
        let mem = EpisodicMemory::new(EpisodicMemoryConfig::default()).unwrap();
        let entry = make_episode_entry();
        let episode = Episode::new(entry.id, "Event at specific time");
        mem.store_episode(entry, episode).unwrap();

        let now = Utc::now();
        let results = mem.search_by_time(
            now - chrono::Duration::hours(1),
            now + chrono::Duration::hours(1),
        );
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_by_keyword() {
        let mem = EpisodicMemory::new(EpisodicMemoryConfig::default()).unwrap();
        let entry = make_episode_entry();
        let episode = Episode::new(entry.id, "Learned about neural networks");
        mem.store_episode(entry, episode).unwrap();

        let results = mem.search_by_keyword("neural");
        assert_eq!(results.len(), 1);
        assert!(results[0]
            .1
            .event_description
            .contains("neural"));
    }

    #[test]
    fn search_by_outcome() {
        let mem = EpisodicMemory::new(EpisodicMemoryConfig::default()).unwrap();
        let entry = make_episode_entry();
        let episode = Episode::new(entry.id, "Task completed").with_outcome(EpisodeOutcome::Success);
        mem.store_episode(entry, episode).unwrap();

        let results = mem.search_by_outcome(EpisodeOutcome::Success);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn emotional_intensity() {
        let mut episode = Episode::new(MemoryId::new(), "test");
        episode.emotion_valence = Some(0.8);
        episode.emotion_arousal = Some(0.9);

        let intensity = episode.emotional_intensity();
        assert!((intensity - 0.85).abs() < 0.01);
    }

    #[test]
    fn recent_episodes() {
        let mem = EpisodicMemory::new(EpisodicMemoryConfig::default()).unwrap();

        for i in 0..5 {
            let entry = make_episode_entry();
            let episode =
                Episode::new(entry.id, format!("Event {i}"));
            mem.store_episode(entry, episode).unwrap();
        }

        let recent = mem.recent(3);
        assert_eq!(recent.len(), 3);
    }

    #[test]
    fn remove_episode() {
        let mem = EpisodicMemory::new(EpisodicMemoryConfig::default()).unwrap();
        let entry = make_episode_entry();
        let eid = entry.id;
        let episode = Episode::new(entry.id, "Disposable");
        mem.store_episode(entry, episode).unwrap();

        let episode_id = *mem.index_by_memory.get(&eid).unwrap().value();
        let removed = mem.remove_episode(episode_id).unwrap();
        assert!(removed);
        assert_eq!(mem.count(), 0);
    }

    #[test]
    fn participant_search() {
        let mem = EpisodicMemory::new(EpisodicMemoryConfig::default()).unwrap();
        let entry = make_episode_entry();
        let mut episode = Episode::new(entry.id, "Meeting with Alice");
        episode.participants = vec!["Alice".to_string(), "Bob".to_string()];
        mem.store_episode(entry, episode).unwrap();

        let results = mem.search_by_participant("Alice");
        assert_eq!(results.len(), 1);
    }
}
