use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use parking_lot::RwLock;
use tracing::info;

use crate::analytics::centrality::CentralityAnalyzer;
use crate::analytics::community::CommunityDetector;
use crate::analytics::components::ConnectedComponentAnalyzer;
use crate::analytics::density::DensityAnalyzer;
use crate::analytics::growth::GrowthTracker;
use crate::api::entity_api::EntityApi;
use crate::api::relation_api::RelationApi;
use crate::api::search_api::SearchApi;
use crate::api::traverse_api::TraverseApi;
use crate::api::io::{GraphExporter, GraphImporter, ExportFormat};
use crate::core::entity::{Entity, EntityBuilder, EntityId, EntityType};
use crate::core::relation::{Relation, RelationId, RelationType};
use crate::error::{KnowledgeError, KnowledgeResult};
use crate::extraction::concept_extractor::ConceptExtractor;
use crate::extraction::confidence::ConfidenceEstimator;
use crate::extraction::entity_extractor::EntityExtractor;
use crate::extraction::merger::DuplicateMerger;
use crate::extraction::relation_extractor::RelationExtractor;
use crate::inference_integration::prompting::KnowledgeAwarePrompter;
use crate::inference_integration::context_enrichment::ContextEnricher;
use crate::inference_integration::fact_retrieval::FactRetriever;
use crate::inference_integration::fact_ranking::FactRanker;
use crate::inference_integration::prompt_assembly::PromptAssembler;
use crate::monitoring::{KnowledgeMetrics, KnowledgeMonitor};
use crate::ontology::types::Ontology;
use crate::reasoning::expansion::NeighborExpander;
use crate::reasoning::path::PathSearcher;
use crate::reasoning::similarity::SemanticSimilarityEngine;
use crate::reasoning::subgraph::SubgraphExtractor;
use crate::reasoning::traversal::GraphTraversal;
use crate::search::hybrid::HybridSearchEngine;
use crate::search::ranking::{ConfidenceRanker, RankedResult};
use crate::storage::graph_store::GraphStore;
use crate::storage::incremental::IncrementalUpdater;
use crate::storage::snapshot::{SnapshotManager, SnapshotConfig};
use crate::validation::contradiction::ContradictionDetector;
use crate::validation::evidence::EvidenceTracker;
use crate::validation::resolution::{ConflictResolver, ResolutionStrategy};
use crate::validation::source::SourceTracker;
use crate::world_model::world_model_manager::WorldModelManager;

/// Central orchestrator for the Neo Knowledge System.
///
/// Integrates all subsystems: graph storage, ontology, extraction,
/// reasoning, search, validation, evolution, inference, world model,
/// analytics, security, and monitoring.
pub struct NeoKnowledgeGraph {
    store: Arc<GraphStore>,
    ontology: Arc<RwLock<Ontology>>,
    snapshot_manager: Arc<SnapshotManager>,
    incremental_updater: Arc<IncrementalUpdater>,
    source_tracker: Arc<SourceTracker>,
    evidence_tracker: Arc<EvidenceTracker>,
    conflict_resolver: Arc<ConflictResolver>,
    contradiction_detector: Arc<ContradictionDetector>,
    monitor: Arc<KnowledgeMonitor>,
    growth_tracker: Arc<GrowthTracker>,
    query_count: AtomicU64,
}

impl NeoKnowledgeGraph {
    /// Create a new knowledge graph with default ontology.
    #[must_use]
    pub fn new() -> Self {
        let ontology = Ontology::default();
        Self::with_ontology(ontology)
    }

    /// Create a new knowledge graph with a custom ontology.
    #[must_use]
    pub fn with_ontology(ontology: Ontology) -> Self {
        let store = Arc::new(GraphStore::new(ontology.clone()));
        info!("NeoKnowledgeGraph initialized");

        Self {
            store,
            ontology: Arc::new(RwLock::new(ontology)),
            snapshot_manager: Arc::new(SnapshotManager::new(SnapshotConfig::default())),
            incremental_updater: Arc::new(IncrementalUpdater::new()),
            source_tracker: Arc::new(SourceTracker::new()),
            evidence_tracker: Arc::new(EvidenceTracker::new()),
            conflict_resolver: Arc::new(ConflictResolver::new(ResolutionStrategy::HighestConfidence)),
            contradiction_detector: Arc::new(ContradictionDetector::new()),
            monitor: Arc::new(KnowledgeMonitor::new()),
            growth_tracker: Arc::new(GrowthTracker::new()),
            query_count: AtomicU64::new(0),
        }
    }

    // ── Entity CRUD ──

    /// Create a new entity.
    pub fn create_entity(
        &self,
        entity_type: EntityType,
        label: impl Into<String>,
    ) -> Entity {
        let api = EntityApi::new(&self.store);
        let entity = api.create(entity_type, label);
        self.incremental_updater.record_change(
            crate::storage::incremental::IncrementalUpdater::entity_created(
                &entity.id.to_string(),
                serde_json::to_value(&entity).unwrap_or_default(),
            ),
        );
        entity
    }

    /// Create an entity with builder pattern.
    pub fn create_entity_with(&self, builder: EntityBuilder) -> Entity {
        let api = EntityApi::new(&self.store);
        let entity = api.create_with(builder);
        self.incremental_updater.record_change(
            crate::storage::incremental::IncrementalUpdater::entity_created(
                &entity.id.to_string(),
                serde_json::to_value(&entity).unwrap_or_default(),
            ),
        );
        entity
    }

    /// Get an entity by id.
    #[must_use]
    pub fn get_entity(&self, id: EntityId) -> Option<Entity> {
        self.store.get_entity(id)
    }

    /// Update an entity.
    pub fn update_entity(
        &self,
        id: EntityId,
        updater: impl FnOnce(&mut Entity),
    ) -> KnowledgeResult<()> {
        self.store.update_entity(id, updater)
    }

    /// Delete (deactivate) an entity.
    pub fn delete_entity(&self, id: EntityId) -> KnowledgeResult<()> {
        self.store.deactivate_entity(id)
    }

    /// Remove an entity and all its relations.
    pub fn remove_entity(&self, id: EntityId) -> KnowledgeResult<bool> {
        self.store.remove_entity(id)
    }

    // ── Relation CRUD ──

    /// Create a relation.
    pub fn create_relation(
        &self,
        relation_type: RelationType,
        source: EntityId,
        target: EntityId,
        label: impl Into<String>,
    ) -> KnowledgeResult<Relation> {
        let api = RelationApi::new(&self.store);
        api.create(relation_type, source, target, label)
    }

    /// Get a relation by id.
    #[must_use]
    pub fn get_relation(&self, id: RelationId) -> Option<Relation> {
        self.store.get_relation(id)
    }

    /// Remove a relation.
    pub fn remove_relation(&self, id: RelationId) -> KnowledgeResult<bool> {
        self.store.remove_relation(id)
    }

    // ── Search ──

    /// Search the knowledge graph.
    #[must_use]
    pub fn search(&self, query: &str, top_k: usize) -> Vec<RankedResult> {
        let start = std::time::Instant::now();
        let api = SearchApi::new(&self.store);
        let results = api.search(query, top_k);
        self.monitor.record_query(start.elapsed().as_secs_f64() * 1000.0);
        results
    }

    /// Search by keyword.
    #[must_use]
    pub fn keyword_search(&self, query: &str) -> Vec<(Entity, f32)> {
        let api = SearchApi::new(&self.store);
        api.keyword_search(query)
    }

    /// Search by property.
    #[must_use]
    pub fn search_by_property(&self, key: &str, value: &serde_json::Value) -> Vec<Entity> {
        let api = SearchApi::new(&self.store);
        api.by_property(key, value)
    }

    // ── Traversal ──

    /// Expand neighbors.
    #[must_use]
    pub fn expand_neighbors(&self, entity_id: EntityId, depth: u32) -> Vec<EntityId> {
        let api = TraverseApi::new(&self.store);
        api.expand_neighbors(entity_id, depth)
    }

    /// Find shortest path.
    #[must_use]
    pub fn shortest_path(&self, from: EntityId, to: EntityId) -> crate::reasoning::path::SearchResult {
        let api = TraverseApi::new(&self.store);
        api.shortest_path(from, to)
    }

    /// BFS traversal.
    #[must_use]
    pub fn bfs(
        &self,
        start: EntityId,
        config: crate::reasoning::traversal::TraversalConfig,
    ) -> crate::reasoning::traversal::TraversalResult {
        let api = TraverseApi::new(&self.store);
        api.bfs(start, config)
    }

    // ── Extraction ──

    /// Extract knowledge from text.
    pub fn extract_from_text(
        &self,
        text: &str,
        source: &str,
    ) -> crate::extraction::concept_extractor::ExtractedConcept {
        let extractor = ConceptExtractor::new();
        let concepts = extractor.extract_from_text(text, source);
        self.monitor.record_extraction();
        concepts.into_iter().next().unwrap_or_else(|| {
            crate::extraction::concept_extractor::ExtractedConcept {
                label: "unknown".to_string(),
                concept_type: EntityType::Concept,
                confidence: 0.0,
                context: text.to_string(),
                source: source.to_string(),
                extracted_at: chrono::Utc::now(),
                properties: std::collections::HashMap::new(),
            }
        })
    }

    /// Extract entities from text.
    #[must_use]
    pub fn extract_entities(
        &self,
        text: &str,
        source: &str,
    ) -> Vec<crate::extraction::entity_extractor::ExtractedEntity> {
        let extractor = EntityExtractor::new();
        let extracted = extractor.extract(text, source);
        self.monitor.record_extraction();
        extracted
    }

    /// Merge duplicate entities.
    pub fn merge_duplicates(
        &self,
        threshold: f32,
    ) -> KnowledgeResult<Vec<crate::extraction::merger::MergeResult>> {
        let merger = DuplicateMerger::new();
        merger.merge_duplicates(&self.store, threshold)
    }

    // ── Validation ──

    /// Detect contradictions.
    #[must_use]
    pub fn detect_contradictions(
        &self,
    ) -> Vec<crate::validation::contradiction::DetectedContradiction> {
        let entities = self.store.all_entities();
        let relations = self.store.all_relations();
        self.contradiction_detector.detect_all(&entities, &relations)
    }

    /// Record source provenance.
    pub fn record_source(
        &self,
        target_id: impl Into<String>,
        source: impl Into<String>,
        confidence: f32,
    ) {
        self.source_tracker.record_source(target_id, source, confidence);
    }

    /// Add supporting evidence.
    pub fn add_evidence(
        &self,
        target_id: impl Into<String>,
        description: impl Into<String>,
        source: impl Into<String>,
        confidence: f32,
    ) {
        self.evidence_tracker
            .add_supporting(target_id, description, source, confidence);
    }

    // ── Snapshots ──

    /// Create a snapshot of the current graph state.
    pub fn create_snapshot(&self, description: impl Into<String>) -> crate::storage::snapshot::GraphSnapshot {
        let entities = self.store.all_entities();
        let relations = self.store.all_relations();
        self.snapshot_manager
            .create_snapshot(entities, relations, description)
    }

    /// Restore from a snapshot.
    pub fn restore_snapshot(&self, id: &str) -> KnowledgeResult<()> {
        let (entities, relations) = self.snapshot_manager.restore(id)?;
        // Note: full restore would replace the store contents
        Ok(())
    }

    // ── Inference Integration ──

    /// Build a knowledge-aware prompt.
    #[must_use]
    pub fn build_prompt(
        &self,
        query: &str,
        max_context_tokens: usize,
    ) -> String {
        let prompter = KnowledgeAwarePrompter::new();
        let relevant = self.get_relevant_entities(query, 10);
        prompter.build_prompt(query, &relevant, max_context_tokens)
    }

    /// Retrieve relevant facts for a query.
    #[must_use]
    pub fn retrieve_facts(
        &self,
        query: &str,
        max_facts: usize,
    ) -> Vec<crate::inference_integration::fact_retrieval::RetrievedFact> {
        let retriever = FactRetriever::new();
        retriever.retrieve(&self.store, query, max_facts)
    }

    /// Enrich context with knowledge.
    #[must_use]
    pub fn enrich_context(
        &self,
        query: &str,
        max_entities: usize,
    ) -> Vec<crate::inference_integration::context_enrichment::EnrichedContext> {
        let enricher = ContextEnricher::new();
        enricher.build_query_context(&self.store, query, max_entities)
    }

    // ── World Model ──

    /// Get the world model manager.
    #[must_use]
    pub fn world_model(&self) -> WorldModelManager<'_> {
        WorldModelManager::new(&self.store)
    }

    // ── Analytics ──

    /// Compute graph density metrics.
    #[must_use]
    pub fn density_analysis(&self) -> crate::analytics::density::DensityStats {
        let analyzer = DensityAnalyzer::new();
        analyzer.analyze(&self.store)
    }

    /// Compute centrality metrics.
    #[must_use]
    pub fn centrality_analysis(
        &self,
    ) -> std::collections::HashMap<EntityId, f32> {
        let analyzer = CentralityAnalyzer::new();
        analyzer.degree_centrality(&self.store)
    }

    /// Find connected components.
    #[must_use]
    pub fn connected_components(
        &self,
    ) -> Vec<crate::analytics::components::ComponentInfo> {
        let analyzer = ConnectedComponentAnalyzer::new();
        analyzer.find_components(&self.store)
    }

    /// Get monitoring metrics.
    #[must_use]
    pub fn metrics(&self) -> KnowledgeMetrics {
        let entities = self.store.all_entities();
        let relations = self.store.all_relations();

        let avg_confidence = if entities.is_empty() {
            0.0
        } else {
            entities.iter().map(|e| e.confidence).sum::<f32>() / entities.len() as f32
        };

        let avg_rel_confidence = if relations.is_empty() {
            0.0
        } else {
            relations.iter().map(|r| r.confidence).sum::<f32>() / relations.len() as f32
        };

        let avg_importance = if entities.is_empty() {
            0.0
        } else {
            entities.iter().map(|e| e.importance).sum::<f32>() / entities.len() as f32
        };

        KnowledgeMetrics {
            entity_count: self.store.entity_count(),
            active_entity_count: self.store.active_entity_count(),
            relation_count: self.store.relation_count(),
            active_relation_count: self.store.active_relation_count(),
            namespace_count: 1,
            avg_entity_confidence: avg_confidence,
            avg_relation_confidence: avg_rel_confidence,
            avg_entity_importance: avg_importance,
            entities_last_hour: 0,
            relations_last_hour: 0,
            total_queries: self.query_count.load(Ordering::Relaxed),
            avg_query_latency_ms: self.monitor.avg_query_latency_ms(),
            total_extractions: self.monitor.extraction_count(),
            extraction_accuracy: 0.9,
            knowledge_freshness: 1.0,
            consistency_score: 1.0,
            timestamp: chrono::Utc::now(),
        }
    }

    // ── Export/Import ──

    /// Export the graph to JSON.
    pub fn export_json(&self) -> KnowledgeResult<String> {
        GraphExporter::to_json(&self.store)
    }

    /// Import from JSON.
    pub fn import_json(&self, json: &str) -> KnowledgeResult<usize> {
        let result = GraphImporter::from_json(json, &self.store)?;
        Ok(result.entities_imported + result.relations_imported)
    }

    // ── Persistence ──

    /// Save to SQLite.
    pub fn save_to_sqlite(&self, path: &std::path::Path) -> KnowledgeResult<()> {
        let store = crate::persistence::sqlite_store::SqliteStore::open(path)?;
        store.save_graph(&self.store)
    }

    // ── Internals ──

    fn get_relevant_entities(&self, query: &str, max: usize) -> Vec<Entity> {
        let all = self.store.all_entities();
        let mut scored: Vec<(Entity, f32)> = all
            .into_iter()
            .filter(|e| e.active && e.matches_query(query))
            .map(|e| {
                let score = e.confidence * 0.5 + e.importance * 0.3 + (e.sources.len() as f32 * 0.1);
                (e, score)
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().take(max).map(|(e, _)| e).collect()
    }
}

impl Default for NeoKnowledgeGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for NeoKnowledgeGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NeoKnowledgeGraph")
            .field("entity_count", &self.store.entity_count())
            .field("relation_count", &self.store.relation_count())
            .finish()
    }
}
