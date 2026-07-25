use std::collections::HashMap;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::error::MemoryResult;
use crate::retrieval::cosine_similarity;
use crate::types::{MemoryEntry, MemoryId, MemoryTier};

/// Index statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexStats {
    /// Number of entries in the vector index.
    pub vector_count: usize,
    /// Number of entries in the keyword index.
    pub keyword_count: usize,
    /// Number of entries in the temporal index.
    pub temporal_count: usize,
    /// Number of graph edges.
    pub graph_edges: usize,
    /// Vector dimensions.
    pub vector_dimensions: usize,
}

/// Configuration for memory indexes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    /// Default vector dimensions.
    pub vector_dimensions: usize,
    /// Whether to enable vector index.
    pub enable_vector: bool,
    /// Whether to enable keyword index.
    pub enable_keyword: bool,
    /// Whether to enable temporal index.
    pub enable_temporal: bool,
    /// Whether to enable graph index hooks.
    pub enable_graph: bool,
    /// Block size for temporal index partitioning (in hours).
    pub temporal_block_hours: u64,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            vector_dimensions: 768,
            enable_vector: true,
            enable_keyword: true,
            enable_temporal: true,
            enable_graph: true,
            temporal_block_hours: 24,
        }
    }
}

/// Graph edge for relationship tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Source memory id.
    pub from: MemoryId,
    /// Target memory id.
    pub to: MemoryId,
    /// Relationship type.
    pub relationship: String,
    /// Edge weight.
    pub weight: f32,
    /// When the edge was created.
    pub created_at: DateTime<Utc>,
}

/// Memory indexes for fast retrieval across multiple dimensions.
pub struct MemoryIndexes {
    /// Vector index: memory_id -> embedding.
    vector_index: DashMap<MemoryId, Vec<f32>>,
    /// Keyword index: lowercase word -> set of memory ids.
    keyword_index: DashMap<String, Vec<MemoryId>>,
    /// Temporal index: hour-block -> sorted memory ids.
    temporal_index: DashMap<i64, Vec<MemoryId>>,
    /// Graph index: memory_id -> outgoing edges.
    graph_index: DashMap<MemoryId, Vec<GraphEdge>>,
    /// Reverse graph index: memory_id -> incoming edges.
    graph_reverse: DashMap<MemoryId, Vec<GraphEdge>>,
    /// All indexed ids for fast enumeration.
    all_ids: DashMap<MemoryId, MemoryTier>,
    /// Configuration.
    config: IndexConfig,
    /// Statistics.
    stats: RwLock<IndexStats>,
}

impl MemoryIndexes {
    /// Create a new index system.
    #[must_use]
    pub fn new(config: IndexConfig) -> Self {
        Self {
            vector_index: DashMap::new(),
            keyword_index: DashMap::new(),
            temporal_index: DashMap::new(),
            graph_index: DashMap::new(),
            graph_reverse: DashMap::new(),
            all_ids: DashMap::new(),
            config,
            stats: RwLock::new(IndexStats::default()),
        }
    }

    /// Index a memory entry across all enabled indexes.
    pub fn index_entry(&self, entry: &MemoryEntry) {
        let id = entry.id;

        // Vector index.
        if self.config.enable_vector {
            if let Some(ref embedding) = entry.embedding {
                self.vector_index.insert(id, embedding.clone());
            }
        }

        // Keyword index.
        if self.config.enable_keyword {
            self.index_keywords(entry);
        }

        // Temporal index.
        if self.config.enable_temporal {
            let hour_block = entry
                .created_at
                .timestamp()
                / (self.config.temporal_block_hours as i64 * 3600);
            self.temporal_index
                .entry(hour_block)
                .or_default()
                .push(id);
        }

        self.all_ids.insert(id, entry.tier);
        self.update_stats();
    }

    /// Remove a memory entry from all indexes.
    pub fn remove_entry(&self, id: MemoryId) {
        self.vector_index.remove(&id);
        self.all_ids.remove(&id);

        // Remove from keyword index.
        for mut entry in self.keyword_index.iter_mut() {
            entry.value_mut().retain(|&x| x != id);
        }

        // Remove from temporal index.
        for mut entry in self.temporal_index.iter_mut() {
            entry.value_mut().retain(|&x| x != id);
        }

        // Remove from graph index.
        self.graph_index.remove(&id);
        self.graph_reverse.remove(&id);

        // Remove graph edges pointing to this id.
        for mut entry in self.graph_index.iter_mut() {
            entry.value_mut().retain(|e| e.to != id);
        }
        for mut entry in self.graph_reverse.iter_mut() {
            entry.value_mut().retain(|e| e.from != id);
        }

        self.update_stats();
    }

    /// Index keywords from a memory entry.
    fn index_keywords(&self, entry: &MemoryEntry) {
        let content = entry.content.to_string();
        let words = tokenize(&content);

        let mut seen = std::collections::HashSet::new();
        for word in words {
            if seen.insert(word.clone()) {
                self.keyword_index
                    .entry(word)
                    .or_default()
                    .push(entry.id);
            }
        }

        for tag in &entry.tags {
            let lower = tag.to_lowercase();
            self.keyword_index
                .entry(lower)
                .or_default()
                .push(entry.id);
        }
    }

    /// Add a graph edge between two memory entries.
    pub fn add_graph_edge(&self, from: MemoryId, to: MemoryId, relationship: &str, weight: f32) {
        let edge = GraphEdge {
            from,
            to,
            relationship: relationship.to_string(),
            weight,
            created_at: Utc::now(),
        };

        self.graph_index.entry(from).or_default().push(edge.clone());
        self.graph_reverse.entry(to).or_default().push(edge);
        self.update_stats();
    }

    /// Get neighbors of a memory entry in the graph.
    #[must_use]
    pub fn graph_neighbors(&self, id: MemoryId) -> Vec<GraphEdge> {
        self.graph_index
            .get(&id)
            .map(|e| e.value().clone())
            .unwrap_or_default()
    }

    /// Get reverse neighbors (entries that point to this one).
    #[must_use]
    pub fn graph_reverse_neighbors(&self, id: MemoryId) -> Vec<GraphEdge> {
        self.graph_reverse
            .get(&id)
            .map(|e| e.value().clone())
            .unwrap_or_default()
    }

    /// Vector similarity search: find top-k most similar entries.
    #[must_use]
    pub fn vector_search(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Vec<(MemoryId, f64)> {
        let mut results: Vec<(MemoryId, f64)> = self
            .vector_index
            .iter()
            .map(|entry| {
                let similarity = cosine_similarity(query_embedding, entry.value());
                (*entry.key(), similarity)
            })
            .collect();

        results.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(top_k);
        results
    }

    /// Keyword search: find entries matching all keywords.
    #[must_use]
    pub fn keyword_search(&self, keywords: &[String]) -> Vec<MemoryId> {
        if keywords.is_empty() {
            return Vec::new();
        }

        let mut result_sets: Vec<Vec<MemoryId>> = keywords
            .iter()
            .filter_map(|kw| {
                let lower = kw.to_lowercase();
                self.keyword_index.get(&lower).map(|ids| ids.value().clone())
            })
            .collect();

        if result_sets.is_empty() {
            return Vec::new();
        }

        result_sets.sort_by_key(|s| s.len());
        let mut intersection = result_sets.remove(0);
        for set in &result_sets {
            intersection.retain(|id| set.contains(id));
        }
        intersection
    }

    /// Temporal search: find entries within a time block range.
    #[must_use]
    pub fn temporal_search(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<MemoryId> {
        let start_block = start.timestamp() / (self.config.temporal_block_hours as i64 * 3600);
        let end_block = end.timestamp() / (self.config.temporal_block_hours as i64 * 3600);

        let mut ids = Vec::new();
        for block in start_block..=end_block {
            if let Some(entries) = self.temporal_index.get(&block) {
                ids.extend(entries.value().iter());
            }
        }
        ids
    }

    /// Get all indexed memory ids.
    #[must_use]
    pub fn all_ids(&self) -> Vec<MemoryId> {
        self.all_ids.iter().map(|e| *e.key()).collect()
    }

    /// Get all indexed memory ids for a specific tier.
    #[must_use]
    pub fn ids_by_tier(&self, tier: MemoryTier) -> Vec<MemoryId> {
        self.all_ids
            .iter()
            .filter(|e| *e.value() == tier)
            .map(|e| *e.key())
            .collect()
    }

    /// Get index statistics.
    #[must_use]
    pub fn stats(&self) -> IndexStats {
        self.stats.read().clone()
    }

    /// Update internal statistics.
    fn update_stats(&self) {
        let mut stats = self.stats.write();
        stats.vector_count = self.vector_index.len();
        stats.keyword_count = self.keyword_index.len();
        stats.temporal_count = self.temporal_index.len();
        stats.graph_edges = self
            .graph_index
            .iter()
            .map(|e| e.value().len())
            .sum();
    }

    /// Rebuild all indexes from a collection of entries.
    pub fn rebuild(&self, entries: &[MemoryEntry]) {
        self.vector_index.clear();
        self.keyword_index.clear();
        self.temporal_index.clear();
        self.graph_index.clear();
        self.graph_reverse.clear();
        self.all_ids.clear();

        for entry in entries {
            self.index_entry(entry);
        }
    }
}

/// Tokenize text into lowercase words for keyword indexing.
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 2)
        .map(String::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn make_indexed_entry(tag: &str) -> MemoryEntry {
        let mut entry = MemoryEntry::new(
            MemoryTier::LongTerm,
            serde_json::json!("test content here"),
            HashSet::from([tag.to_string()]),
        );
        entry.embedding = Some(vec![1.0, 0.0, 0.0]);
        entry
    }

    #[test]
    fn index_and_search() {
        let indexes = MemoryIndexes::new(IndexConfig::default());
        let entry = make_indexed_entry("alpha");
        let id = entry.id;
        indexes.index_entry(&entry);

        let results = indexes.keyword_search(&["alpha".to_string()]);
        assert!(results.contains(&id));
    }

    #[test]
    fn vector_search() {
        let indexes = MemoryIndexes::new(IndexConfig::default());

        let mut e1 = make_indexed_entry("a");
        e1.embedding = Some(vec![1.0, 0.0, 0.0]);
        indexes.index_entry(&e1);

        let mut e2 = make_indexed_entry("b");
        e2.embedding = Some(vec![0.0, 1.0, 0.0]);
        indexes.index_entry(&e2);

        let results = indexes.vector_search(&[1.0, 0.0, 0.0], 10);
        assert_eq!(results.len(), 2);
        assert!(results[0].1 > results[1].1);
    }

    #[test]
    fn graph_edges() {
        let indexes = MemoryIndexes::new(IndexConfig::default());
        let id1 = MemoryId::new();
        let id2 = MemoryId::new();

        indexes.add_graph_edge(id1, id2, "related_to", 1.0);

        let neighbors = indexes.graph_neighbors(id1);
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].to, id2);

        let reverse = indexes.graph_reverse_neighbors(id2);
        assert_eq!(reverse.len(), 1);
    }

    #[test]
    fn remove_entry() {
        let indexes = MemoryIndexes::new(IndexConfig::default());
        let entry = make_indexed_entry("test");
        let id = entry.id;
        indexes.index_entry(&entry);

        indexes.remove_entry(id);
        assert!(indexes.all_ids().is_empty());
    }

    #[test]
    fn temporal_search() {
        let indexes = MemoryIndexes::new(IndexConfig {
            temporal_block_hours: 1,
            ..IndexConfig::default()
        });

        let entry = make_indexed_entry("time");
        indexes.index_entry(&entry);

        let now = Utc::now();
        let results = indexes.temporal_search(
            now - chrono::Duration::hours(1),
            now + chrono::Duration::hours(1),
        );
        assert!(results.contains(&entry.id));
    }

    #[test]
    fn tokenize_basic() {
        let words = tokenize("Hello, World! This is a test.");
        assert!(words.contains(&"hello".to_string()));
        assert!(words.contains(&"world".to_string()));
    }
}
