use std::collections::HashMap;

use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::error::{MemoryError, MemoryResult};
use crate::types::{MemoryEntry, MemoryId};

/// Configuration for the embedding integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Default embedding dimensions.
    pub dimensions: usize,
    /// Model name for embedding generation.
    pub model_name: String,
    /// Whether to enable automatic embedding on store.
    pub auto_embed: bool,
    /// Maximum batch size for batch embedding.
    pub max_batch_size: usize,
    /// Embedding cache size (number of entries).
    pub cache_size: usize,
    /// Whether to enable embedding cache.
    pub cache_enabled: bool,
    /// Similarity threshold for cache hits.
    pub cache_similarity_threshold: f64,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            dimensions: 768,
            model_name: "neo-default".to_string(),
            auto_embed: true,
            max_batch_size: 100,
            cache_size: 10_000,
            cache_enabled: true,
            cache_similarity_threshold: 0.99,
        }
    }
}

/// Cache entry for embeddings.
#[derive(Debug, Clone)]
struct CacheEntry {
    embedding: Vec<f32>,
    access_count: u64,
}

/// Embedding integration for automatic generation, caching, and incremental updates.
pub struct EmbeddingIntegration {
    config: EmbeddingConfig,
    /// Cache: text hash -> embedding.
    cache: DashMap<u64, CacheEntry>,
    /// Embedding dimensions used.
    dimensions: RwLock<usize>,
}

impl EmbeddingIntegration {
    /// Create a new embedding integration.
    #[must_use]
    pub fn new(config: EmbeddingConfig) -> Self {
        let dimensions = config.dimensions;
        Self {
            config,
            cache: DashMap::new(),
            dimensions: RwLock::new(dimensions),
        }
    }

    /// Generate an embedding for text content using a deterministic hash-based approach.
    ///
    /// In production, this would call an actual embedding model. Here we generate
    /// deterministic pseudo-embeddings based on text content for testing.
    pub fn embed(&self, text: &str) -> Vec<f32> {
        let hash = text_hash(text);

        // Check cache first.
        if self.config.cache_enabled {
            if let Some(mut cached) = self.cache.get_mut(&hash) {
                cached.value_mut().access_count += 1;
                return cached.value().embedding.clone();
            }
        }

        let dims = *self.dimensions.read();
        let embedding = generate_deterministic_embedding(text, dims);

        // Cache the result.
        if self.config.cache_enabled {
            self.cache.insert(
                hash,
                CacheEntry {
                    embedding: embedding.clone(),
                    access_count: 1,
                },
            );
            self.evict_cache_if_needed();
        }

        embedding
    }

    /// Generate embeddings for a batch of texts.
    pub fn embed_batch(&self, texts: &[String]) -> Vec<Vec<f32>> {
        texts
            .iter()
            .take(self.config.max_batch_size)
            .map(|text| self.embed(text))
            .collect()
    }

    /// Automatically generate and set embedding for a memory entry.
    pub fn auto_embed_entry(&self, entry: &mut MemoryEntry) -> bool {
        if !self.config.auto_embed {
            return false;
        }

        if entry.embedding.is_some() {
            return false;
        }

        let text = entry.content.to_string();
        if text.is_empty() {
            return false;
        }

        entry.embedding = Some(self.embed(&text));
        true
    }

    /// Batch generate embeddings for multiple entries.
    pub fn auto_embed_entries(&self, entries: &mut [MemoryEntry]) -> u64 {
        let mut count = 0;
        for entry in entries.iter_mut() {
            if self.auto_embed_entry(entry) {
                count += 1;
            }
        }
        count
    }

    /// Incrementally update an embedding when entry content changes.
    pub fn update_embedding(&self, entry: &mut MemoryEntry, new_content: &str) {
        let new_embedding = self.embed(new_content);
        entry.embedding = Some(new_embedding);
    }

    /// Get cache statistics.
    #[must_use]
    pub fn cache_stats(&self) -> EmbeddingCacheStats {
        let total = self.cache.len();
        let total_accesses: u64 = self
            .cache
            .iter()
            .map(|e| e.value().access_count)
            .sum();

        EmbeddingCacheStats {
            cached_entries: total,
            total_accesses,
            dimensions: *self.dimensions.read(),
        }
    }

    /// Clear the embedding cache.
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Evict least-accessed entries if cache exceeds size limit.
    fn evict_cache_if_needed(&self) {
        if self.cache.len() <= self.config.cache_size {
            return;
        }

        // Find the entry with lowest access count and remove it.
        let mut min_access = u64::MAX;
        let mut min_hash = 0u64;

        for entry in self.cache.iter() {
            if entry.value().access_count < min_access {
                min_access = entry.value().access_count;
                min_hash = *entry.key();
            }
        }

        if min_access < u64::MAX {
            self.cache.remove(&min_hash);
        }
    }

    /// Get the configured dimensions.
    #[must_use]
    pub fn dimensions(&self) -> usize {
        *self.dimensions.read()
    }
}

/// Statistics about the embedding cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingCacheStats {
    /// Number of cached embeddings.
    pub cached_entries: usize,
    /// Total cache accesses.
    pub total_accesses: u64,
    /// Embedding dimensions.
    pub dimensions: usize,
}

/// Compute a hash of text content for caching.
fn text_hash(text: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

/// Generate a deterministic embedding from text using hash-based initialization.
fn generate_deterministic_embedding(text: &str, dimensions: usize) -> Vec<f32> {
    let bytes = text.as_bytes();
    let mut hash_state: u64 = 0xcbf29ce484222325;
    for &byte in bytes {
        hash_state ^= byte as u64;
        hash_state = hash_state.wrapping_mul(0x100000001b3);
    }

    let mut embedding = Vec::with_capacity(dimensions);
    let mut rng_state = hash_state;

    for i in 0..dimensions {
        rng_state = rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(i as u64 + 1);
        let raw = ((rng_state >> 11) as f64 / (1u64 << 53) as f64) * 2.0 - 1.0;
        let positional =
            ((i as f64 / dimensions as f64) * std::f64::consts::PI * 2.0).sin() * 0.1;
        embedding.push((raw * 0.9 + positional * 0.1) as f32);
    }

    // Normalize.
    let norm: f32 = embedding.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in &mut embedding {
            *v /= norm;
        }
    }

    embedding
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embed_deterministic() {
        let integration = EmbeddingIntegration::new(EmbeddingConfig::default());
        let e1 = integration.embed("hello world");
        let e2 = integration.embed("hello world");
        assert_eq!(e1, e2);
    }

    #[test]
    fn embed_different_text() {
        let integration = EmbeddingIntegration::new(EmbeddingConfig::default());
        let e1 = integration.embed("hello");
        let e2 = integration.embed("goodbye");
        assert_ne!(e1, e2);
    }

    #[test]
    fn cache_hit() {
        let integration = EmbeddingIntegration::new(EmbeddingConfig::default());
        let _ = integration.embed("cached text");
        let stats = integration.cache_stats();
        assert_eq!(stats.cached_entries, 1);
    }

    #[test]
    fn batch_embed() {
        let integration = EmbeddingIntegration::new(EmbeddingConfig::default());
        let texts = vec![
            "text one".to_string(),
            "text two".to_string(),
            "text three".to_string(),
        ];
        let embeddings = integration.embed_batch(&texts);
        assert_eq!(embeddings.len(), 3);
    }

    #[test]
    fn auto_embed_entry() {
        let integration = EmbeddingIntegration::new(EmbeddingConfig::default());
        let mut entry = MemoryEntry::new(
            crate::types::MemoryTier::LongTerm,
            serde_json::json!("Some content to embed"),
            std::collections::HashSet::new(),
        );

        assert!(integration.auto_embed_entry(&mut entry));
        assert!(entry.embedding.is_some());
        assert_eq!(entry.embedding.as_ref().unwrap().len(), 768);
    }

    #[test]
    fn no_re_embed() {
        let integration = EmbeddingIntegration::new(EmbeddingConfig::default());
        let mut entry = MemoryEntry::new(
            crate::types::MemoryTier::LongTerm,
            serde_json::json!("content"),
            std::collections::HashSet::new(),
        );
        entry.embedding = Some(vec![0.0; 768]);

        assert!(!integration.auto_embed_entry(&mut entry));
    }

    #[test]
    fn custom_dimensions() {
        let config = EmbeddingConfig {
            dimensions: 256,
            ..EmbeddingConfig::default()
        };
        let integration = EmbeddingIntegration::new(config);
        let embedding = integration.embed("test");
        assert_eq!(embedding.len(), 256);
    }

    #[test]
    fn normalize() {
        let integration = EmbeddingIntegration::new(EmbeddingConfig::default());
        let embedding = integration.embed("normalize test");
        let norm: f32 = embedding.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01);
    }
}
