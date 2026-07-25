use crate::core::entity::Entity;
use crate::search::hybrid::HybridSearchEngine;
use crate::search::keyword::KeywordSearch;
use crate::search::metadata::MetadataSearch;
use crate::search::temporal::TemporalSearch;
use crate::search::ranking::{ConfidenceRanker, RankedResult};
use crate::storage::graph_store::GraphStore;

/// Unified search API for the knowledge graph.
pub struct SearchApi<'a> {
    store: &'a GraphStore,
    hybrid: HybridSearchEngine,
    keyword: KeywordSearch,
    metadata: MetadataSearch,
    temporal: TemporalSearch,
    ranker: ConfidenceRanker,
}

impl<'a> SearchApi<'a> {
    /// Create a new search API.
    #[must_use]
    pub fn new(store: &'a GraphStore) -> Self {
        Self {
            store,
            hybrid: HybridSearchEngine::new(),
            keyword: KeywordSearch::new(),
            metadata: MetadataSearch::new(),
            temporal: TemporalSearch::new(),
            ranker: ConfidenceRanker::new(),
        }
    }

    /// Full-text hybrid search.
    #[must_use]
    pub fn search(&self, query: &str, top_k: usize) -> Vec<RankedResult> {
        self.hybrid.search(self.store, query, top_k)
    }

    /// Keyword-only search.
    #[must_use]
    pub fn keyword_search(&self, query: &str) -> Vec<(Entity, f32)> {
        let entities = self.store.all_entities();
        self.keyword.search_all(&entities, query)
    }

    /// Search by property.
    #[must_use]
    pub fn by_property(&self, key: &str, value: &serde_json::Value) -> Vec<Entity> {
        let entities = self.store.all_entities();
        self.metadata.search_by_property(&entities, key, value)
    }

    /// Search by namespace.
    #[must_use]
    pub fn by_namespace(&self, namespace: &str) -> Vec<Entity> {
        let entities = self.store.all_entities();
        self.metadata.search_by_namespace(&entities, namespace)
    }

    /// Search by minimum confidence.
    #[must_use]
    pub fn by_confidence(&self, min: f32) -> Vec<Entity> {
        let entities = self.store.all_entities();
        self.metadata.search_by_min_confidence(&entities, min)
    }

    /// Search recently updated entities.
    #[must_use]
    pub fn recently_updated(&self, seconds: i64) -> Vec<Entity> {
        let entities = self.store.all_entities();
        self.temporal.updated_within(&entities, seconds)
    }

    /// Search entities created today.
    #[must_use]
    pub fn created_today(&self) -> Vec<Entity> {
        let entities = self.store.all_entities();
        self.temporal.created_today(&entities)
    }
}
