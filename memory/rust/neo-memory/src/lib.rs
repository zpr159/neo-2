//! # Neo Cognitive Memory System
//!
//! A cognitive architecture-style memory system for the Neo AGI Operating System.
//!
//! This is NOT a simple vector database. It is a multi-tiered cognitive memory
//! system that serves as the foundation for reasoning, planning, learning,
//! and long-term autonomy.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │              Cognitive Memory Manager                    │
//! │  (unified API, lifecycle, cross-tier operations)        │
//! ├───────────┬───────────┬───────────┬───────────┬────────┤
//! │ Working   │ Episodic  │ Semantic  │ Procedural│ Long-  │
//! │ Memory    │ Memory    │ Memory    │ Memory    │ Term   │
//! │ (context, │ (events,  │ (facts,   │ (skills,  │ Memory │
//! │  scratch) │ timeline) │ concepts) │ workflows)│ (persist)│
//! ├───────────┴───────────┴───────────┴───────────┴────────┤
//! │           Retrieval Engine (hybrid search)               │
//! ├─────────────────────────────────────────────────────────┤
//! │           Consolidation Engine                          │
//! │  (dedup, summarize, compress, promote, decay)           │
//! ├─────────────────────────────────────────────────────────┤
//! │  Indexes │ Embeddings │ Context Builder │ Persistence   │
//! ├──────────┴────────────┴────────────────┴───────────────┤
//! │  Security │ Analytics │ Importance │ Decay Engine       │
//! └─────────────────────────────────────────────────────────┘
//! ```

pub mod error;
pub mod types;
pub mod working;
pub mod episodic;
pub mod semantic;
pub mod procedural;
pub mod long_term;
pub mod retrieval;
pub mod consolidation;
pub mod importance;
pub mod decay;
pub mod indexes;
pub mod embedding;
pub mod context_builder;
pub mod persistence;
pub mod api;
pub mod security;
pub mod analytics;
pub mod manager;

// Re-exports for convenience.
pub use error::{MemoryError, MemoryResult, MemoryErrorCode};
pub use types::{
    MemoryId, MemoryTier, MemoryEntry, MemoryPriority, MemoryStatus,
    MemoryNamespace, MemoryPermission, EpisodeOutcome, RetentionPolicy,
    RetentionConfig, SecurityConfig, AnalyticsSnapshot, ConsolidationStatus, AuditEntry,
};
pub use working::{WorkingMemory, WorkingMemoryConfig, WorkingMemoryStats};
pub use episodic::{Episode, EpisodicMemory, EpisodicMemoryConfig};
pub use semantic::{SemanticFact, SemanticConcept, SemanticMemory, SemanticMemoryConfig};
pub use procedural::{
    Procedure, ProcedureStep, ProceduralMemory, ProceduralMemoryConfig,
    ExecutionRecord, StepResult, OptimizationRecord, OptimizationType,
};
pub use long_term::{LongTermMemory, LongTermMemoryConfig, MemorySnapshot, CompressionRecord};
pub use retrieval::{
    MemoryQuery, RetrievalEngine, RetrievalConfig, SearchResult,
    SortOrder, ScoringWeights, cosine_similarity, vector_search,
    keyword_search, temporal_search,
};
pub use consolidation::{
    MemoryConsolidation, ConsolidationConfig, ConsolidationRecord,
    ExtractedKnowledge,
};
pub use importance::{MemoryImportance, ImportanceConfig};
pub use decay::{MemoryDecay, DecayRecord};
pub use indexes::{MemoryIndexes, IndexConfig, IndexStats, GraphEdge};
pub use embedding::{
    EmbeddingIntegration, EmbeddingConfig, EmbeddingCacheStats,
};
pub use context_builder::{
    ContextBuilder, ContextBuilderConfig, ContextItem, BuiltContext,
};
pub use persistence::{
    MemoryPersistence, PersistenceConfig, StorageBackend, BackupRecord,
};
pub use api::{
    StoreRequest, StoreResponse, UpdateRequest, SearchRequest, SearchResponse,
    MemoryEntrySummary, MergeRequest, MergeStrategy, MergeResponse,
    ExportRequest, ExportFormat, ExportResponse, ImportRequest, ImportResponse,
    SplitRequest, SplitResponse, SummarizeRequest, SummarizeResponse, HealthResponse,
};
pub use security::{MemorySecurity, NamespacePermission};
pub use analytics::{MemoryAnalytics, AnalyticsConfig};
pub use manager::{CognitiveMemoryManager, UnifiedMemoryConfig};
