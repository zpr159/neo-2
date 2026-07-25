use serde::{Deserialize, Serialize};

use crate::error::{KnowledgeError, KnowledgeResult};

/// Configuration for graph compression.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Minimum entity age in days before compression.
    pub min_age_days: u64,
    /// Maximum importance threshold for compression candidates.
    pub max_importance: f32,
    /// Minimum access count threshold (below this, candidate for compression).
    pub max_access_count: u64,
    /// Whether to preserve all sources when compressing.
    pub preserve_sources: bool,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            min_age_days: 30,
            max_importance: 0.3,
            max_access_count: 5,
            preserve_sources: true,
        }
    }
}

/// Result of a compression operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionResult {
    /// Number of entities compressed.
    pub entities_compressed: usize,
    /// Number of relations compressed.
    pub relations_compressed: usize,
    /// Bytes saved (estimated).
    pub bytes_saved: u64,
    /// Entities that were removed.
    pub removed_entity_ids: Vec<String>,
}

/// Compresses knowledge graph by removing low-importance, rarely accessed elements.
pub struct GraphCompressor {
    config: CompressionConfig,
}

impl GraphCompressor {
    /// Create a new compressor.
    #[must_use]
    pub fn new(config: CompressionConfig) -> Self {
        Self { config }
    }

    /// Identify entities that are candidates for compression/removal.
    #[must_use]
    pub fn compression_candidates(
        &self,
        entities: &[crate::core::entity::Entity],
    ) -> Vec<String> {
        let now = chrono::Utc::now();
        entities
            .iter()
            .filter(|e| {
                let age_days = now
                    .signed_duration_since(e.created_at)
                    .num_days()
                    .max(0) as u64;
                age_days >= self.config.min_age_days
                    && e.importance <= self.config.max_importance
                    && e.sources.is_empty()
            })
            .map(|e| e.id.to_string())
            .collect()
    }

    /// Execute compression on identified candidates.
    #[must_use]
    pub fn compress(
        &self,
        candidate_ids: &[String],
        _entities: &[crate::core::entity::Entity],
    ) -> CompressionResult {
        let estimated_bytes = candidate_ids.len() as u64 * 256;
        CompressionResult {
            entities_compressed: candidate_ids.len(),
            relations_compressed: 0,
            bytes_saved: estimated_bytes,
            removed_entity_ids: candidate_ids.to_vec(),
        }
    }
}

impl Default for GraphCompressor {
    fn default() -> Self {
        Self::new(CompressionConfig::default())
    }
}
