use serde::{Deserialize, Serialize};

use crate::core::entity::EntityId;

/// A single ranked search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedResult {
    /// Entity id.
    pub entity_id: EntityId,
    /// Entity label.
    pub label: String,
    /// Combined relevance score (0.0 - 1.0).
    pub score: f32,
    /// Human-readable explanation of the score.
    pub explanation: String,
}

/// Ranks search results by confidence and relevance.
pub struct ConfidenceRanker;

impl ConfidenceRanker {
    /// Create a new ranker.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Re-rank results incorporating entity confidence scores.
    #[must_use]
    pub fn rerank(
        &self,
        results: &mut Vec<RankedResult>,
        entities: &std::collections::HashMap<EntityId, f32>,
    ) {
        for result in results.iter_mut() {
            if let Some(&conf) = entities.get(&result.entity_id) {
                result.score = result.score * 0.7 + conf * 0.3;
            }
        }
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    }

    /// Sort results by score descending.
    pub fn sort_by_score(results: &mut Vec<RankedResult>) {
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    }

    /// Filter results by minimum score.
    #[must_use]
    pub fn filter_min_score(results: Vec<RankedResult>, min_score: f32) -> Vec<RankedResult> {
        results.into_iter().filter(|r| r.score >= min_score).collect()
    }
}

impl Default for ConfidenceRanker {
    fn default() -> Self {
        Self::new()
    }
}
