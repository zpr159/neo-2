use serde::{Deserialize, Serialize};

use crate::types::MemoryEntry;

/// Configuration for the context builder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBuilderConfig {
    /// Maximum token budget for the context window.
    pub max_tokens: usize,
    /// Whether to enable sliding window context.
    pub sliding_window: bool,
    /// Sliding window size in tokens.
    pub sliding_window_tokens: usize,
    /// Compression ratio when exceeding budget.
    pub compression_ratio: f64,
    /// Whether to enable deduplication.
    pub deduplication: bool,
    /// Deduplication similarity threshold.
    pub dedup_threshold: f64,
    /// Minimum relevance score to include.
    pub min_relevance: f64,
    /// Token estimate per character (rough approximation).
    pub tokens_per_char: f64,
}

impl Default for ContextBuilderConfig {
    fn default() -> Self {
        Self {
            max_tokens: 8192,
            sliding_window: true,
            sliding_window_tokens: 4096,
            compression_ratio: 0.5,
            deduplication: true,
            dedup_threshold: 0.9,
            min_relevance: 0.1,
            tokens_per_char: 0.25,
        }
    }
}

/// A ranked context item ready for inclusion in the inference context.
#[derive(Debug, Clone)]
pub struct ContextItem {
    /// The memory entry.
    pub entry: MemoryEntry,
    /// Rank score (higher is more relevant).
    pub score: f64,
    /// Estimated tokens.
    pub estimated_tokens: usize,
    /// Source tier description.
    pub source: String,
}

/// Built context ready for inference.
#[derive(Debug, Clone)]
pub struct BuiltContext {
    /// Items in the context, ordered by relevance.
    pub items: Vec<ContextItem>,
    /// Total tokens used.
    pub total_tokens: usize,
    /// Maximum tokens available.
    pub max_tokens: usize,
    /// Number of items dropped due to budget.
    pub dropped_count: usize,
    /// Whether compression was applied.
    pub compressed: bool,
}

/// Context builder for constructing inference contexts from memory.
pub struct ContextBuilder {
    config: ContextBuilderConfig,
}

impl ContextBuilder {
    /// Create a new context builder.
    #[must_use]
    pub fn new(config: ContextBuilderConfig) -> Self {
        Self { config }
    }

    /// Build an inference context from a set of memory entries.
    #[must_use]
    pub fn build(&self, entries: &[MemoryEntry], query_embedding: Option<&[f32]>) -> BuiltContext {
        // Score and rank entries.
        let mut items: Vec<ContextItem> = entries
            .iter()
            .map(|e| {
                let score = self.score_entry(e, query_embedding);
                let tokens = self.estimate_tokens(e);
                ContextItem {
                    entry: e.clone(),
                    score,
                    estimated_tokens: tokens,
                    source: e.tier.to_string(),
                }
            })
            .filter(|item| item.score >= self.config.min_relevance)
            .collect();

        // Sort by score (descending).
        items.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Deduplication.
        if self.config.deduplication {
            items = self.deduplicate(items);
        }

        // Fit within token budget.
        let mut total_tokens = 0;
        let mut included = Vec::new();
        let mut dropped_count = 0;

        for item in items {
            if total_tokens + item.estimated_tokens <= self.config.max_tokens {
                total_tokens += item.estimated_tokens;
                included.push(item);
            } else {
                dropped_count += 1;
            }
        }

        let compressed = dropped_count > 0;

        BuiltContext {
            items: included,
            total_tokens,
            max_tokens: self.config.max_tokens,
            dropped_count,
            compressed,
        }
    }

    /// Build context with a sliding window approach.
    #[must_use]
    pub fn build_sliding(&self, entries: &[MemoryEntry]) -> BuiltContext {
        if !self.config.sliding_window {
            return self.build(entries, None);
        }

        // Take the most recent entries that fit in the window.
        let window_tokens = self.config.sliding_window_tokens;
        let mut total_tokens = 0;
        let mut items = Vec::new();

        // Iterate from newest to oldest.
        for entry in entries.iter().rev() {
            let tokens = self.estimate_tokens(entry);
            if total_tokens + tokens <= window_tokens {
                total_tokens += tokens;
                items.push(ContextItem {
                    entry: entry.clone(),
                    score: entry.score(),
                    estimated_tokens: tokens,
                    source: entry.tier.to_string(),
                });
            }
        }

        // Re-sort by score.
        items.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        BuiltContext {
            items,
            total_tokens,
            max_tokens: window_tokens,
            dropped_count: 0,
            compressed: false,
        }
    }

    /// Build context with token budgeting: rank, then fit.
    #[must_use]
    pub fn build_ranked(&self, entries: &[MemoryEntry], budget: usize) -> BuiltContext {
        let mut items: Vec<ContextItem> = entries
            .iter()
            .map(|e| {
                let tokens = self.estimate_tokens(e);
                ContextItem {
                    entry: e.clone(),
                    score: e.score(),
                    estimated_tokens: tokens,
                    source: e.tier.to_string(),
                }
            })
            .collect();

        items.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut total_tokens = 0;
        let mut included = Vec::new();
        let mut dropped = 0;

        for item in items {
            if total_tokens + item.estimated_tokens <= budget {
                total_tokens += item.estimated_tokens;
                included.push(item);
            } else {
                dropped += 1;
            }
        }

        BuiltContext {
            items: included,
            total_tokens,
            max_tokens: budget,
            dropped_count: dropped,
            compressed: dropped > 0,
        }
    }

    /// Score a memory entry for context relevance.
    fn score_entry(&self, entry: &MemoryEntry, query_embedding: Option<&[f32]>) -> f64 {
        let mut score = entry.score();

        // Boost if we have a query embedding and the entry has one.
        if let (Some(q_emb), Some(e_emb)) = (query_embedding, &entry.embedding) {
            let similarity =
                crate::retrieval::cosine_similarity(q_emb, e_emb);
            score = score * 0.5 + similarity * 0.5;
        }

        score
    }

    /// Estimate token count for an entry.
    fn estimate_tokens(&self, entry: &MemoryEntry) -> usize {
        if entry.estimated_tokens > 0 {
            return entry.estimated_tokens;
        }

        let content_str = entry.content.to_string();
        let chars = content_str.len();
        (chars as f64 * self.config.tokens_per_char) as usize
    }

    /// Deduplicate items by content similarity.
    fn deduplicate(&self, items: Vec<ContextItem>) -> Vec<ContextItem> {
        let mut unique = Vec::new();
        for item in items {
            let is_dup = unique.iter().any(|existing: &ContextItem| {
                if let (Some(a_emb), Some(b_emb)) = (
                    &item.entry.embedding,
                    &existing.entry.embedding,
                ) {
                    crate::retrieval::cosine_similarity(a_emb, b_emb)
                        > self.config.dedup_threshold
                } else {
                    item.entry.id == existing.entry.id
                }
            });

            if !is_dup {
                unique.push(item);
            }
        }
        unique
    }

    /// Compress context by summarizing groups of related items.
    #[must_use]
    pub fn compress_context(&self, context: &BuiltContext) -> BuiltContext {
        if context.total_tokens <= self.config.max_tokens {
            return context.clone();
        }

        let target_tokens =
            (self.config.max_tokens as f64 * self.config.compression_ratio) as usize;

        let mut compressed_items = Vec::new();
        let mut total_tokens = 0;

        // Keep the highest-scored items.
        for item in &context.items {
            if total_tokens + item.estimated_tokens <= target_tokens {
                total_tokens += item.estimated_tokens;
                compressed_items.push(item.clone());
            }
        }

        BuiltContext {
            items: compressed_items,
            total_tokens,
            max_tokens: target_tokens,
            dropped_count: context.items.len(),
            compressed: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn make_ctx_entry(importance: f32, tokens: usize) -> MemoryEntry {
        let mut entry = MemoryEntry::new(
            crate::types::MemoryTier::LongTerm,
            serde_json::json!("context test content"),
            HashSet::new(),
        );
        entry.importance = importance;
        entry.estimated_tokens = tokens;
        entry
    }

    #[test]
    fn build_basic() {
        let builder = ContextBuilder::new(ContextBuilderConfig {
            max_tokens: 1000,
            ..ContextBuilderConfig::default()
        });

        let entries = vec![
            make_ctx_entry(0.9, 100),
            make_ctx_entry(0.5, 200),
            make_ctx_entry(0.1, 50),
        ];

        let ctx = builder.build(&entries, None);
        assert!(ctx.total_tokens <= 1000);
        assert!(!ctx.items.is_empty());
    }

    #[test]
    fn build_with_budget() {
        let builder = ContextBuilder::new(ContextBuilderConfig::default());
        let entries = vec![
            make_ctx_entry(0.9, 500),
            make_ctx_entry(0.5, 600),
        ];

        let ctx = builder.build_ranked(&entries, 800);
        assert!(ctx.total_tokens <= 800);
    }

    #[test]
    fn sliding_window() {
        let builder = ContextBuilder::new(ContextBuilderConfig {
            sliding_window: true,
            sliding_window_tokens: 300,
            ..ContextBuilderConfig::default()
        });

        let entries = vec![
            make_ctx_entry(0.5, 100),
            make_ctx_entry(0.5, 100),
            make_ctx_entry(0.5, 100),
            make_ctx_entry(0.5, 100),
        ];

        let ctx = builder.build_sliding(&entries);
        assert!(ctx.total_tokens <= 300);
    }

    #[test]
    fn compression() {
        let builder = ContextBuilder::new(ContextBuilderConfig {
            max_tokens: 200,
            compression_ratio: 0.5,
            ..ContextBuilderConfig::default()
        });

        let context = BuiltContext {
            items: vec![
                ContextItem {
                    entry: make_ctx_entry(0.5, 100),
                    score: 0.5,
                    estimated_tokens: 100,
                    source: "test".to_string(),
                },
                ContextItem {
                    entry: make_ctx_entry(0.3, 100),
                    score: 0.3,
                    estimated_tokens: 100,
                    source: "test".to_string(),
                },
            ],
            total_tokens: 201,
            max_tokens: 200,
            dropped_count: 0,
            compressed: false,
        };

        let compressed = builder.compress_context(&context);
        assert!(compressed.total_tokens <= 100);
    }
}
