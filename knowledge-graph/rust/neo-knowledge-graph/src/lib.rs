//! # Neo Knowledge Graph
//!
//! Dynamic knowledge representation, ontology management, extraction,
//! reasoning, search, validation, evolution, and world model for the
//! Neo AGI Operating System.
//!
//! ## Architecture
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────────┐
//! │                    NeoKnowledgeGraph (Orchestrator)               │
//! ├───────────┬───────────┬───────────┬───────────┬─────────────────┤
//! │ Extraction│ Reasoning │ Search    │ Validation│ Evolution       │
//! │ (concepts,│ (traverse,│ (hybrid,  │ (source,  │ (merge, split,  │
//! │  entities,│  paths,   │  keyword, │  evidence,│  discover,      │
//! │  relations│  similar, │  temporal,│  contra-  │  prune, refine) │
//! │  merge)   │  subgraph)│  metadata)│  dictions)│                 │
//! ├───────────┴───────────┴───────────┴───────────┴─────────────────┤
//! │                    Inference Integration                          │
//! │  (prompting, context enrichment, fact retrieval, prompt assembly)│
//! ├──────────────────────────────────────────────────────────────────┤
//! │                    World Model                                    │
//! │  (person, place, org, object, event, task, goal, skill, project)│
//! ├──────────────────────────────────────────────────────────────────┤
//! │  Ontology System  │  Knowledge Storage  │  Graph Analytics      │
//! │  (types, taxonomy,│  (graph, indexes,   │  (centrality,         │
//! │   schema)         │   snapshots, comp.) │   communities, growth)│
//! ├────────────────────┴─────────────────────┴──────────────────────┤
//! │  Security         │  Persistence         │  Monitoring          │
//! │  (namespaces,     │  (SQLite, KV store,  │  (metrics, health,   │
//! │   permissions,    │   distributed hooks) │   latency)           │
//! │   encryption,     │                      │                      │
//! │   audit)          │                      │                      │
//! └────────────────────┴─────────────────────┴──────────────────────┘
//! ```

pub mod error;
pub mod core;
pub mod ontology;
pub mod extraction;
pub mod storage;
pub mod reasoning;
pub mod search;
pub mod validation;
pub mod evolution;
pub mod inference_integration;
pub mod world_model;
pub mod analytics;
pub mod api;
pub mod persistence;
pub mod security;
pub mod monitoring;
pub mod knowledge_graph;

// Re-exports for convenience
pub use error::{KnowledgeError, KnowledgeResult, KnowledgeErrorCode};
pub use core::{
    Entity, EntityId, EntityType, EntityBuilder,
    Relation, RelationId, RelationType, RelationBuilder, Directedness,
    Attribute, AttributeId, AttributeType, AttributeValue,
    KnowledgeNamespace, NamespaceRegistry, NamespaceConfig,
    VersionVector, VersionTracker, VersionedChange, ChangeType,
    KnowledgeId, IdType,
};
pub use ontology::{
    Ontology, EntityTypeDefinition, RelationTypeDefinition, PropertyDefinition,
    TaxonomyNode, TaxonomyTree, TaxonomyPath,
    OntologyValidator, ValidationResult, ValidationViolation,
};
pub use extraction::{
    ConceptExtractor, ExtractedConcept,
    EntityExtractor, ExtractedEntity,
    RelationExtractor, ExtractedRelation,
    DuplicateMerger, MergeResult,
    ConfidenceEstimator, ConfidenceReport, ConflictDetection,
};
pub use storage::{
    GraphStore, GraphIndexes, IndexType, IndexStats,
    SnapshotManager, GraphSnapshot, SnapshotConfig,
    GraphCompressor, CompressionConfig, CompressionResult,
    IncrementalUpdater, DeltaChange, DeltaRecord,
    RecoveryManager, RecoveryPlan, RecoveryStatus,
};
pub use reasoning::{
    NeighborExpander,
    PathSearcher,
    SemanticSimilarityEngine,
    GraphTraversal, TraversalResult, TraversalConfig,
    SubgraphExtractor,
};
pub use search::{
    HybridSearchEngine,
    KeywordSearch,
    MetadataSearch,
    TemporalSearch,
    ConfidenceRanker, RankedResult,
};
pub use validation::{
    SourceTracker,
    EvidenceTracker,
    ContradictionDetector,
    ConflictResolver, ResolutionStrategy, ResolutionResult,
};
pub use evolution::{
    ConceptMerger,
    ConceptSplitter,
    TaxonomyRefiner,
    RelationshipDiscovery,
    KnowledgePruner,
};
pub use inference_integration::{
    KnowledgeAwarePrompter,
    ContextEnricher,
    FactRetriever,
    FactRanker,
    PromptAssembler,
};
pub use world_model::{
    PersonEntity, PlaceEntity, OrganizationEntity, ObjectEntity,
    EventEntity, TaskEntity, GoalEntity, SkillEntity, ProjectEntity,
    WorldModelManager,
};
pub use analytics::{
    CentralityAnalyzer, CommunityDetector, ClusterAnalyzer,
    ConnectedComponentAnalyzer, DensityAnalyzer, GrowthTracker,
};
pub use api::{
    EntityApi, RelationApi, SearchApi, TraverseApi,
    GraphExporter, GraphImporter, ExportFormat,
};
pub use persistence::{
    SqliteStore, RocksDbStore, DistributedGraphHooks,
};
pub use security::{
    NamespacePermissions, AccessController, GraphEncryption, AuditTrail,
    PermissionLevel, AuditAction,
};
pub use monitoring::{KnowledgeMetrics, KnowledgeMonitor};
pub use knowledge_graph::NeoKnowledgeGraph;
