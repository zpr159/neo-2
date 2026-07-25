use std::collections::HashMap;
use std::sync::atomic::Ordering;

use serde::{Deserialize, Serialize};

use crate::error::MemoryResult;
use crate::types::{MemoryEntry, MemoryId, MemoryTier};

/// Search query for memory retrieval.
#[derive(Debug, Clone)]
pub struct MemoryQuery {
    /// The text query for keyword search.
    pub text: Option<String>,
    /// Optional embedding vector for vector search.
    pub embedding: Option<Vec<f32>>,
    /// Tiers to search.
    pub tiers: Vec<MemoryTier>,
    /// Maximum results to return.
    pub limit: usize,
    /// Minimum similarity score for vector search.
    pub min_similarity: f64,
    /// Minimum importance threshold.
    pub min_importance: f32,
    /// Minimum confidence threshold.
    pub min_confidence: f32,
    /// Filter by tags.
    pub tags: Vec<String>,
    /// Filter by namespace.
    pub namespace: Option<String>,
    /// Time range filter (start).
    pub time_start: Option<chrono::DateTime<chrono::Utc>>,
    /// Time range filter (end).
    pub time_end: Option<chrono::DateTime<chrono::Utc>>,
    /// Whether to include archived entries.
    pub include_archived: bool,
    /// Whether to include deleted entries.
    pub include_deleted: bool,
    /// Sort order.
    pub sort_by: SortOrder,
}

impl Default for MemoryQuery {
    fn default() -> Self {
        Self {
            text: None,
            embedding: None,
            tiers: vec![
                MemoryTier::Working,
                MemoryTier::Episodic,
                MemoryTier::Semantic,
                MemoryTier::Procedural,
                MemoryTier::LongTerm,
            ],
            limit: 10,
            min_similarity: 0.3,
            min_importance: 0.0,
            min_confidence: 0.0,
            tags: Vec::new(),
            namespace: None,
            time_start: None,
            time_end: None,
            include_archived: false,
            include_deleted: false,
            sort_by: SortOrder::Relevance,
        }
    }
}

/// Sort order for search results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SortOrder {
    /// Sort by relevance score (descending).
    Relevance,
    /// Sort by recency (newest first).
    Recency,
    /// Sort by importance (descending).
    Importance,
    /// Sort by access count (descending).
    Frequency,
}

/// A single search result with scoring information.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// The memory entry.
    pub entry: MemoryEntry,
    /// Overall hybrid score.
    pub score: f64,
    /// Vector similarity score (if applicable).
    pub vector_score: Option<f64>,
    /// Keyword match score (if applicable).
    pub keyword_score: Option<f64>,
    /// Metadata match score.
    pub metadata_score: f64,
    /// Recency score.
    pub recency_score: f64,
    /// Importance score.
    pub importance_score: f64,
}

/// Scoring weights for hybrid retrieval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringWeights {
    /// Weight for vector similarity.
    pub vector: f64,
    /// Weight for keyword matching.
    pub keyword: f64,
    /// Weight for metadata matching.
    pub metadata: f64,
    /// Weight for recency.
    pub recency: f64,
    /// Weight for importance.
    pub importance: f64,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            vector: 0.35,
            keyword: 0.25,
            metadata: 0.15,
            recency: 0.15,
            importance: 0.10,
        }
    }
}

/// Configuration for the retrieval engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalConfig {
    /// Scoring weights for hybrid retrieval.
    pub scoring_weights: ScoringWeights,
    /// Maximum number of results.
    pub max_results: usize,
    /// Whether to enable deduplication.
    pub deduplication: bool,
    /// Deduplication similarity threshold.
    pub dedup_threshold: f64,
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            scoring_weights: ScoringWeights::default(),
            max_results: 100,
            deduplication: true,
            dedup_threshold: 0.95,
        }
    }
}

/// Memory retrieval engine providing hybrid search capabilities.
pub struct RetrievalEngine {
    /// Configuration.
    config: RetrievalConfig,
}

impl RetrievalEngine {
    /// Create a new retrieval engine.
    #[must_use]
    pub fn new(config: RetrievalConfig) -> Self {
        Self { config }
    }

    /// Search through a collection of memory entries using hybrid scoring.
    pub fn search(
        &self,
        query: &MemoryQuery,
        entries: &[MemoryEntry],
    ) -> Vec<SearchResult> {
        let mut results: Vec<SearchResult> = entries
            .iter()
            .filter(|e| self.matches_filters(e, query))
            .map(|e| self.score_entry(e, query))
            .collect();

        // Sort by score.
        match query.sort_by {
            SortOrder::Relevance => {
                results.sort_by(|a, b| {
                    b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SortOrder::Recency => {
                results.sort_by(|a, b| {
                    b.entry
                        .created_at
                        .partial_cmp(&a.entry.created_at)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SortOrder::Importance => {
                results.sort_by(|a, b| {
                    b.entry
                        .importance
                        .partial_cmp(&a.entry.importance)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SortOrder::Frequency => {
                results.sort_by(|a, b| {
                    b.entry
                        .access_count
                        .load(Ordering::SeqCst)
                        .cmp(&a.entry.access_count.load(Ordering::SeqCst))
                });
            }
        }

        results.truncate(query.limit);

        // Deduplication.
        if self.config.deduplication {
            results = self.deduplicate(results);
        }

        results
    }

    /// Check if an entry matches the query filters.
    fn matches_filters(&self, entry: &MemoryEntry, query: &MemoryQuery) -> bool {
        // Tier filter.
        if !query.tiers.contains(&entry.tier) {
            return false;
        }

        // Active status filter.
        if entry.status == crate::types::MemoryStatus::Deleted && !query.include_deleted {
            return false;
        }
        if entry.status == crate::types::MemoryStatus::Archived && !query.include_archived {
            return false;
        }

        // Namespace filter.
        if let Some(ref ns) = query.namespace {
            if entry.namespace.0 != *ns {
                return false;
            }
        }

        // Importance filter.
        if entry.importance < query.min_importance {
            return false;
        }

        // Confidence filter.
        if entry.confidence < query.min_confidence {
            return false;
        }

        // Tag filter.
        if !query.tags.is_empty() {
            let has_tag = query
                .tags
                .iter()
                .any(|t| entry.tags.contains(t));
            if !has_tag {
                return false;
            }
        }

        // Time range filter.
        if let Some(start) = query.time_start {
            if entry.created_at < start {
                return false;
            }
        }
        if let Some(end) = query.time_end {
            if entry.created_at > end {
                return false;
            }
        }

        true
    }

    /// Score a single entry against the query.
    fn score_entry(&self, entry: &MemoryEntry, query: &MemoryQuery) -> SearchResult {
        let w = &self.config.scoring_weights;

        // Vector similarity score.
        let vector_score = if let (Some(ref q_emb), Some(ref e_emb)) =
            (&query.embedding, &entry.embedding)
        {
            Some(cosine_similarity(q_emb, e_emb))
        } else {
            None
        };

        // Keyword score.
        let keyword_score = if let Some(ref text) = query.text {
            Some(self.keyword_match_score(text, entry))
        } else {
            None
        };

        // Metadata score (tags, source).
        let metadata_score = self.metadata_score(entry, query);

        // Recency score.
        let recency_score = self.recency_score(entry);

        // Importance score.
        let importance_score = entry.importance as f64;

        // Weighted hybrid score.
        let mut score = 0.0;
        let mut total_weight = 0.0;

        if let Some(vs) = vector_score {
            score += vs * w.vector;
            total_weight += w.vector;
        }
        if let Some(ks) = keyword_score {
            score += ks * w.keyword;
            total_weight += w.keyword;
        }
        score += metadata_score * w.metadata;
        total_weight += w.metadata;
        score += recency_score * w.recency;
        total_weight += w.recency;
        score += importance_score * w.importance;
        total_weight += w.importance;

        let final_score = if total_weight > 0.0 {
            score / total_weight
        } else {
            0.0
        };

        SearchResult {
            entry: entry.clone(),
            score: final_score,
            vector_score,
            keyword_score,
            metadata_score,
            recency_score,
            importance_score,
        }
    }

    /// Compute keyword matching score.
    fn keyword_match_score(&self, query: &str, entry: &MemoryEntry) -> f64 {
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        let content = entry.content.to_string().to_lowercase();
        let content_words: Vec<&str> = content.split_whitespace().collect();

        if query_words.is_empty() || content_words.is_empty() {
            return 0.0;
        }

        let matched = query_words
            .iter()
            .filter(|qw| content_words.iter().any(|cw| cw.contains(*qw)))
            .count();

        let tag_match = entry
            .tags
            .iter()
            .filter(|t| query_lower.contains(&t.to_lowercase()))
            .count();

        let word_score = matched as f64 / query_words.len() as f64;
        let tag_bonus = (tag_match as f64 * 0.2).min(0.3);

        (word_score + tag_bonus).min(1.0)
    }

    /// Compute metadata matching score.
    fn metadata_score(&self, entry: &MemoryEntry, query: &MemoryQuery) -> f64 {
        let mut score = 0.0;

        if !query.tags.is_empty() {
            let tag_matches = query
                .tags
                .iter()
                .filter(|t| entry.tags.contains(*t))
                .count();
            score += tag_matches as f64 / query.tags.len() as f64 * 0.5;
        }

        if entry.source.is_some() {
            score += 0.1;
        }

        score.min(1.0)
    }

    /// Compute recency score.
    fn recency_score(&self, entry: &MemoryEntry) -> f64 {
        let now = chrono::Utc::now();
        let elapsed = now.signed_duration_since(entry.created_at);
        let hours = elapsed.num_hours().max(0) as f64;
        1.0 / (1.0 + hours / 24.0)
    }

    /// Deduplicate results by similarity.
    fn deduplicate(&self, mut results: Vec<SearchResult>) -> Vec<SearchResult> {
        if results.len() <= 1 {
            return results;
        }

        let mut unique = Vec::new();
        for result in results {
            let is_dup = unique.iter().any(|existing: &SearchResult| {
                if let (Some(ref a_emb), Some(ref b_emb)) =
                    (&result.entry.embedding, &existing.entry.embedding)
                {
                    cosine_similarity(a_emb, b_emb) > self.config.dedup_threshold
                } else {
                    result.entry.id == existing.entry.id
                }
            });

            if !is_dup {
                unique.push(result);
            }
        }

        unique
    }
}

/// Compute cosine similarity between two embedding vectors.
#[must_use]
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| (*x as f64) * (*y as f64)).sum();
    let norm_a: f64 = a.iter().map(|x| (*x as f64) * (*x as f64)).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| (*x as f64) * (*x as f64)).sum::<f64>().sqrt();

    if norm_a > 0.0 && norm_b > 0.0 {
        dot / (norm_a * norm_b)
    } else {
        0.0
    }
}

/// Find top-k most similar entries based on embedding vectors.
#[must_use]
pub fn vector_search(
    query_embedding: &[f32],
    entries: &[MemoryEntry],
    top_k: usize,
) -> Vec<(MemoryId, f64)> {
    let mut scores: Vec<(MemoryId, f64)> = entries
        .iter()
        .filter_map(|e| {
            e.embedding.as_ref().map(|emb| {
                (e.id, cosine_similarity(query_embedding, emb))
            })
        })
        .collect();

    scores.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scores.truncate(top_k);
    scores
}

/// Find entries matching keywords.
#[must_use]
pub fn keyword_search(
    keywords: &[String],
    entries: &[MemoryEntry],
    top_k: usize,
) -> Vec<(MemoryId, f64)> {
    let mut scores: Vec<(MemoryId, f64)> = entries
        .iter()
        .map(|e| {
            let content = e.content.to_string().to_lowercase();
            let matched = keywords
                .iter()
                .filter(|kw| content.contains(&kw.to_lowercase()))
                .count();
            let score = if keywords.is_empty() {
                0.0
            } else {
                matched as f64 / keywords.len() as f64
            };
            (e.id, score)
        })
        .filter(|(_, score)| *score > 0.0)
        .collect();

    scores.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scores.truncate(top_k);
    scores
}

/// Find entries within a time range.
#[must_use]
pub fn temporal_search(
    entries: &[MemoryEntry],
    start: Option<chrono::DateTime<chrono::Utc>>,
    end: Option<chrono::DateTime<chrono::Utc>>,
    top_k: usize,
) -> Vec<MemoryEntry> {
    let mut filtered: Vec<MemoryEntry> = entries
        .iter()
        .filter(|e| {
            if let Some(s) = start {
                if e.created_at < s {
                    return false;
                }
            }
            if let Some(e_end) = end {
                if e.created_at > e_end {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect();

    filtered.sort_by(|a, b| {
        b.created_at
            .partial_cmp(&a.created_at)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    filtered.truncate(top_k);
    filtered
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn make_search_entry(importance: f32, tags: Vec<&str>) -> MemoryEntry {
        let content_str = format!(
            "test content for {} search",
            tags.first().map_or("general", |t| *t)
        );
        let mut entry = MemoryEntry::new(
            MemoryTier::LongTerm,
            serde_json::json!(content_str),
            tags.into_iter().map(String::from).collect(),
        );
        entry.importance = importance;
        entry
    }

    #[test]
    fn keyword_search_basic() {
        let entries = vec![
            make_search_entry(0.5, vec!["alpha"]),
            make_search_entry(0.8, vec!["beta"]),
        ];

        let keywords = vec!["alpha".to_string()];
        let results = keyword_search(&keywords, &entries, 10);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn vector_search_basic() {
        let mut e1 = make_search_entry(0.5, vec![]);
        e1.embedding = Some(vec![1.0, 0.0, 0.0]);
        let mut e2 = make_search_entry(0.5, vec![]);
        e2.embedding = Some(vec![0.0, 1.0, 0.0]);

        let results = vector_search(&[1.0, 0.0, 0.0], &[e1, e2], 10);
        assert_eq!(results.len(), 2);
        assert!(results[0].1 > results[1].1);
    }

    #[test]
    fn cosine_similarity_basic() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &c).abs() < 0.001);
    }

    #[test]
    fn hybrid_search() {
        let engine = RetrievalEngine::new(RetrievalConfig::default());
        let mut entries = Vec::new();

        let mut e1 = make_search_entry(0.8, vec!["important"]);
        e1.embedding = Some(vec![1.0, 0.0]);
        entries.push(e1);

        let mut e2 = make_search_entry(0.3, vec!["casual"]);
        e2.embedding = Some(vec![0.0, 1.0]);
        entries.push(e2);

        let query = MemoryQuery {
            text: Some("important".to_string()),
            embedding: Some(vec![1.0, 0.0]),
            limit: 10,
            ..MemoryQuery::default()
        };

        let results = engine.search(&query, &entries);
        assert_eq!(results.len(), 2);
        assert!(results[0].score >= results[1].score);
    }

    #[test]
    fn temporal_search_basic() {
        let entries = vec![make_search_entry(0.5, vec![])];
        let results = temporal_search(&entries, None, None, 10);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn tag_filter() {
        let engine = RetrievalEngine::new(RetrievalConfig::default());
        let entries = vec![
            make_search_entry(0.5, vec!["rust"]),
            make_search_entry(0.5, vec!["python"]),
        ];

        let query = MemoryQuery {
            tags: vec!["rust".to_string()],
            limit: 10,
            ..MemoryQuery::default()
        };

        let results = engine.search(&query, &entries);
        assert_eq!(results.len(), 1);
    }
}
