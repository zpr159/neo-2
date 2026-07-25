use crate::core::entity::{Entity, EntityId};
use crate::core::relation::Relation;
use crate::reasoning::similarity::SemanticSimilarityEngine;
use crate::search::keyword::KeywordSearch;
use crate::search::ranking::{ConfidenceRanker, RankedResult};
use crate::storage::graph_store::GraphStore;

/// Hybrid search combining graph structure, keyword matching, and similarity.
pub struct HybridSearchEngine {
    keyword_search: KeywordSearch,
    similarity_engine: SemanticSimilarityEngine,
    ranker: ConfidenceRanker,
}

impl HybridSearchEngine {
    /// Create a new hybrid search engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            keyword_search: KeywordSearch::new(),
            similarity_engine: SemanticSimilarityEngine::new(),
            ranker: ConfidenceRanker::new(),
        }
    }

    /// Search combining keyword and graph similarity.
    #[must_use]
    pub fn search(
        &self,
        store: &GraphStore,
        query: &str,
        top_k: usize,
    ) -> Vec<RankedResult> {
        // Keyword matches
        let keyword_matches: Vec<Entity> = store
            .all_entities()
            .into_iter()
            .filter(|e| e.active && self.keyword_search.matches(e, query))
            .collect();

        // Graph neighbor scoring
        let mut scored: Vec<RankedResult> = keyword_matches
            .into_iter()
            .map(|entity| {
                let keyword_score = self.keyword_search.score(&entity, query);
                let neighbor_score = self.neighbor_relevance(store, &entity, query);
                let confidence_score = entity.confidence;

                let combined = keyword_score * 0.4 + neighbor_score * 0.35 + confidence_score * 0.25;

                RankedResult {
                    entity_id: entity.id,
                    label: entity.label,
                    score: combined,
                    explanation: format!(
                        "keyword={:.2} neighbor={:.2} confidence={:.2}",
                        keyword_score, neighbor_score, confidence_score
                    ),
                }
            })
            .collect();

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        scored
    }

    fn neighbor_relevance(&self, store: &GraphStore, entity: &Entity, query: &str) -> f32 {
        let neighbors = store.neighbors(entity.id);
        if neighbors.is_empty() {
            return 0.0;
        }

        let relevant = neighbors.iter().filter_map(|nid| store.get_entity(*nid)).filter(|e| {
            e.label.to_lowercase().contains(&query.to_lowercase())
                || e.description.to_lowercase().contains(&query.to_lowercase())
        }).count();

        (relevant as f32 / neighbors.len() as f32).min(1.0)
    }
}

impl Default for HybridSearchEngine {
    fn default() -> Self {
        Self::new()
    }
}
