use crate::core::entity::{Entity, EntityId};
use crate::storage::graph_store::GraphStore;

/// Computes semantic similarity between entities based on graph structure and properties.
pub struct SemanticSimilarityEngine;

impl SemanticSimilarityEngine {
    /// Create a new similarity engine.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Compute Jaccard similarity between two entities based on their neighbors.
    #[must_use]
    pub fn neighbor_similarity(&self, store: &GraphStore, a: EntityId, b: EntityId) -> f32 {
        let neighbors_a: std::collections::HashSet<EntityId> =
            store.neighbors(a).into_iter().collect();
        let neighbors_b: std::collections::HashSet<EntityId> =
            store.neighbors(b).into_iter().collect();

        if neighbors_a.is_empty() && neighbors_b.is_empty() {
            return 0.0;
        }

        let intersection = neighbors_a.intersection(&neighbors_b).count() as f32;
        let union = neighbors_a.union(&neighbors_b).count() as f32;

        if union > 0.0 {
            intersection / union
        } else {
            0.0
        }
    }

    /// Compute label similarity (Jaccard on words).
    #[must_use]
    pub fn label_similarity(&self, a: &Entity, b: &Entity) -> f32 {
        let words_a: std::collections::HashSet<String> = a
            .label
            .to_lowercase()
            .split_whitespace()
            .map(String::from)
            .collect();
        let words_b: std::collections::HashSet<String> = b
            .label
            .to_lowercase()
            .split_whitespace()
            .map(String::from)
            .collect();

        if words_a.is_empty() && words_b.is_empty() {
            return 1.0;
        }

        let intersection = words_a.intersection(&words_b).count() as f32;
        let union = words_a.union(&words_b).count() as f32;

        if union > 0.0 {
            intersection / union
        } else {
            0.0
        }
    }

    /// Compute combined similarity score.
    #[must_use]
    pub fn combined_similarity(
        &self,
        store: &GraphStore,
        a: &Entity,
        b: &Entity,
    ) -> f32 {
        let neighbor_sim = self.neighbor_similarity(store, a.id, b.id);
        let label_sim = self.label_similarity(a, b);
        let type_sim = if a.entity_type == b.entity_type { 1.0 } else { 0.0 };

        neighbor_sim * 0.4 + label_sim * 0.35 + type_sim * 0.25
    }

    /// Find the most similar entities to a query entity.
    #[must_use]
    pub fn find_similar(
        &self,
        store: &GraphStore,
        query: EntityId,
        top_k: usize,
    ) -> Vec<(Entity, f32)> {
        let query_entity = match store.get_entity(query) {
            Some(e) => e,
            None => return Vec::new(),
        };

        let mut scores: Vec<(Entity, f32)> = store
            .all_entities()
            .into_iter()
            .filter(|e| e.id != query && e.active)
            .map(|e| {
                let score = self.combined_similarity(store, &query_entity, &e);
                (e, score)
            })
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);
        scores
    }
}

impl Default for SemanticSimilarityEngine {
    fn default() -> Self {
        Self::new()
    }
}
