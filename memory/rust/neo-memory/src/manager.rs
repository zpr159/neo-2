use std::collections::{HashMap, HashSet};
use std::sync::atomic::Ordering;

use chrono::Utc;
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::analytics::{AnalyticsConfig, MemoryAnalytics};
use crate::api::*;
use crate::consolidation::{ConsolidationConfig, MemoryConsolidation};
use crate::context_builder::{ContextBuilder, ContextBuilderConfig, BuiltContext};
use crate::decay::{DecayRecord, MemoryDecay};
use crate::embedding::{EmbeddingConfig, EmbeddingIntegration};
use crate::episodic::{Episode, EpisodicMemory, EpisodicMemoryConfig};
use crate::error::{MemoryError, MemoryResult};
use crate::importance::{ImportanceConfig, MemoryImportance};
use crate::indexes::{IndexConfig, MemoryIndexes};
use crate::long_term::{LongTermMemory, LongTermMemoryConfig};
use crate::persistence::{MemoryPersistence, PersistenceConfig};
use crate::procedural::{
    Procedure, ProceduralMemory, ProceduralMemoryConfig, ExecutionRecord, OptimizationRecord,
};
use crate::retrieval::{MemoryQuery, RetrievalConfig, RetrievalEngine, SearchResult, SortOrder};
use crate::security::MemorySecurity;
use crate::semantic::{SemanticFact, SemanticMemory, SemanticMemoryConfig};
use crate::types::*;
use crate::working::{WorkingMemory, WorkingMemoryConfig};

/// Unified memory configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedMemoryConfig {
    /// Working memory configuration.
    pub working: WorkingMemoryConfig,
    /// Episodic memory configuration.
    pub episodic: EpisodicMemoryConfig,
    /// Semantic memory configuration.
    pub semantic: SemanticMemoryConfig,
    /// Procedural memory configuration.
    pub procedural: ProceduralMemoryConfig,
    /// Long-term memory configuration.
    pub long_term: LongTermMemoryConfig,
    /// Retrieval engine configuration.
    pub retrieval: RetrievalConfig,
    /// Consolidation configuration.
    pub consolidation: ConsolidationConfig,
    /// Embedding configuration.
    pub embedding: EmbeddingConfig,
    /// Index configuration.
    pub indexes: IndexConfig,
    /// Context builder configuration.
    pub context: ContextBuilderConfig,
    /// Persistence configuration.
    pub persistence: PersistenceConfig,
    /// Security configuration.
    pub security: SecurityConfig,
    /// Analytics configuration.
    pub analytics: AnalyticsConfig,
    /// Retention configuration.
    pub retention: crate::types::RetentionConfig,
    /// Importance configuration.
    pub importance: ImportanceConfig,
}

impl Default for UnifiedMemoryConfig {
    fn default() -> Self {
        Self {
            working: WorkingMemoryConfig::default(),
            episodic: EpisodicMemoryConfig::default(),
            semantic: SemanticMemoryConfig::default(),
            procedural: ProceduralMemoryConfig::default(),
            long_term: LongTermMemoryConfig::default(),
            retrieval: RetrievalConfig::default(),
            consolidation: ConsolidationConfig::default(),
            embedding: EmbeddingConfig::default(),
            indexes: IndexConfig::default(),
            context: ContextBuilderConfig::default(),
            persistence: PersistenceConfig::default(),
            security: SecurityConfig::default(),
            analytics: AnalyticsConfig::default(),
            retention: RetentionConfig::default(),
            importance: ImportanceConfig::default(),
        }
    }
}

/// Unified Memory Manager integrating all cognitive memory subsystems.
pub struct CognitiveMemoryManager {
    config: UnifiedMemoryConfig,
    working: WorkingMemory,
    episodic: EpisodicMemory,
    semantic: SemanticMemory,
    procedural: ProceduralMemory,
    long_term: LongTermMemory,
    retrieval: RetrievalEngine,
    consolidation: MemoryConsolidation,
    embedding: EmbeddingIntegration,
    indexes: MemoryIndexes,
    context_builder: ContextBuilder,
    persistence: MemoryPersistence,
    security: MemorySecurity,
    analytics: MemoryAnalytics,
    decay: RwLock<MemoryDecay>,
    importance: MemoryImportance,
    start_time: std::time::Instant,
}

impl CognitiveMemoryManager {
    /// Create a new cognitive memory manager.
    pub fn new(config: UnifiedMemoryConfig) -> MemoryResult<Self> {
        let working = WorkingMemory::new(config.working.clone());
        let episodic = EpisodicMemory::new(config.episodic.clone())?;
        let semantic = SemanticMemory::new(config.semantic.clone())?;
        let procedural = ProceduralMemory::new(config.procedural.clone())?;
        let long_term = LongTermMemory::new(config.long_term.clone())?;
        let retrieval = RetrievalEngine::new(config.retrieval.clone());
        let consolidation = MemoryConsolidation::new(config.consolidation.clone());
        let embedding = EmbeddingIntegration::new(config.embedding.clone());
        let indexes = MemoryIndexes::new(config.indexes.clone());
        let context_builder = ContextBuilder::new(config.context.clone());
        let persistence = MemoryPersistence::new(config.persistence.clone())?;
        let security = MemorySecurity::new(config.security.clone());
        let analytics = MemoryAnalytics::new(config.analytics.clone());
        let decay = RwLock::new(MemoryDecay::new(config.retention.clone()));
        let importance = MemoryImportance::new(config.importance.clone());

        Ok(Self {
            config,
            working,
            episodic,
            semantic,
            procedural,
            long_term,
            retrieval,
            consolidation,
            embedding,
            indexes,
            context_builder,
            persistence,
            security,
            analytics,
            decay,
            importance,
            start_time: std::time::Instant::now(),
        })
    }

    /// Store a memory entry.
    pub fn store(&self, request: StoreRequest) -> MemoryResult<StoreResponse> {
        let mut tags: HashSet<String> = request.tags.into_iter().collect();
        if tags.is_empty() {
            tags.insert("general".to_string());
        }

        let mut entry = MemoryEntry::new(request.tier, request.content, tags);

        if let Some(imp) = request.importance {
            entry.importance = imp.clamp(0.0, 1.0);
        }
        if let Some(pri) = request.priority {
            entry.priority = pri;
        }
        if let Some(ref ns) = request.namespace {
            entry.namespace = MemoryNamespace::new(ns);
        }
        if let Some(secs) = request.ttl_secs {
            entry.ttl = Some(std::time::Duration::from_secs(secs));
        }
        if let Some(ref src) = request.source {
            entry.source = Some(src.clone());
        }

        // Auto-generate embedding.
        let content_text = entry.content.to_string();
        self.embedding.auto_embed_entry(&mut entry);

        // Index the entry.
        self.indexes.index_entry(&entry);

        let entry_id = entry.id;

        // Route to the appropriate memory tier.
        match request.tier {
            MemoryTier::Working => {
                self.working.push(entry);
            }
            MemoryTier::Episodic => {
                let episode = Episode::new(
                    entry.id,
                    content_text.chars().take(256).collect::<String>(),
                );
                self.episodic.store_episode(entry, episode)?;
            }
            MemoryTier::Semantic => {
                // For semantic, store as a generic fact.
                let fact = SemanticFact::new(
                    "memory",
                    "content",
                    entry.content.clone(),
                )
                .with_confidence(entry.confidence);
                self.semantic.store_with_entry(entry, fact)?;
            }
            MemoryTier::Procedural => {
                // For procedural, store the entry alongside.
                self.long_term.store(entry)?;
            }
            MemoryTier::LongTerm => {
                self.long_term.store(entry)?;
            }
        }

        self.analytics.record_creation(request.tier);

        Ok(StoreResponse {
            id: entry_id.to_string(),
            created_at: Utc::now(),
        })
    }

    /// Retrieve a memory by id.
    pub fn recall(&self, id: MemoryId) -> Option<MemoryEntry> {
        self.analytics.record_recall_attempt();

        // Search working memory first.
        if let Some(entry) = self.working.get(id) {
            self.analytics.record_recall_hit();
            return Some(entry);
        }

        // Search episodic memory.
        if let Some((entry, _)) = self.episodic.recall(id) {
            self.analytics.record_recall_hit();
            return Some(entry);
        }

        // Search long-term memory.
        if let Some(entry) = self.long_term.get(id) {
            self.analytics.record_recall_hit();
            return Some(entry);
        }

        None
    }

    /// Search memories.
    pub fn search(&self, request: SearchRequest) -> MemoryResult<SearchResponse> {
        let tiers: Vec<MemoryTier> = request
            .tiers
            .map(|ts| {
                ts.iter()
                    .filter_map(|t| match t.as_str() {
                        "working" => Some(MemoryTier::Working),
                        "episodic" => Some(MemoryTier::Episodic),
                        "semantic" => Some(MemoryTier::Semantic),
                        "procedural" => Some(MemoryTier::Procedural),
                        "long_term" => Some(MemoryTier::LongTerm),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_else(|| {
                vec![
                    MemoryTier::Working,
                    MemoryTier::Episodic,
                    MemoryTier::Semantic,
                    MemoryTier::Procedural,
                    MemoryTier::LongTerm,
                ]
            });

        let limit = request.limit.unwrap_or(10);

        let query = MemoryQuery {
            text: request.query,
            tiers,
            limit,
            tags: request.tags.unwrap_or_default(),
            namespace: request.namespace,
            min_importance: request.min_importance.unwrap_or(0.0),
            ..MemoryQuery::default()
        };

        // Collect all searchable entries.
        let mut all_entries = Vec::new();

        // From working memory.
        for entry in self.working.entries() {
            all_entries.push(entry);
        }

        // From long-term memory.
        for entry in self.long_term.active_entries() {
            all_entries.push(entry);
        }

        let results = self.retrieval.search(&query, &all_entries);

        let total = results.len();
        let summaries: Vec<MemoryEntrySummary> = results
            .iter()
            .map(|r| MemoryEntrySummary::from(&r.entry))
            .collect();

        Ok(SearchResponse {
            results: summaries,
            total,
        })
    }

    /// Build inference context from memory.
    pub fn build_context(
        &self,
        query_embedding: Option<&[f32]>,
        max_tokens: usize,
    ) -> BuiltContext {
        let mut all_entries = Vec::new();

        for entry in self.working.entries() {
            all_entries.push(entry);
        }
        for entry in self.long_term.active_entries() {
            all_entries.push(entry);
        }

        let config = ContextBuilderConfig {
            max_tokens,
            ..self.config.context.clone()
        };
        let builder = ContextBuilder::new(config);
        builder.build(&all_entries, query_embedding)
    }

    /// Store an episodic memory with full episode data.
    pub fn store_episode(
        &self,
        content: serde_json::Value,
        episode: Episode,
        tags: HashSet<String>,
    ) -> MemoryResult<MemoryId> {
        let entry = MemoryEntry::new(MemoryTier::Episodic, content, tags);
        self.episodic.store_episode(entry, episode)
    }

    /// Add a semantic fact.
    pub fn add_fact(&self, fact: SemanticFact) -> MemoryResult<uuid::Uuid> {
        self.semantic.add_fact(fact)
    }

    /// Query semantic facts by subject.
    pub fn query_facts(&self, subject: &str) -> Vec<SemanticFact> {
        self.semantic.query_subject(subject)
    }

    /// Store a procedure.
    pub fn store_procedure(&self, procedure: Procedure) -> MemoryResult<uuid::Uuid> {
        self.procedural.store_procedure(procedure)
    }

    /// Search procedures.
    pub fn search_procedures(&self, name: &str) -> Vec<Procedure> {
        self.procedural.search_by_name(name)
    }

    /// Record procedure execution.
    pub fn record_execution(&self, record: ExecutionRecord) -> MemoryResult<()> {
        self.procedural.record_execution(record)
    }

    /// Run memory consolidation.
    pub fn consolidate(&self) -> MemoryResult<crate::consolidation::ConsolidationRecord> {
        let mut entries = DashMap::new();
        for entry in self.working.entries() {
            entries.insert(entry.id, entry);
        }
        for entry in self.long_term.active_entries() {
            entries.insert(entry.id, entry);
        }
        self.consolidation.consolidate(&entries)
    }

    /// Apply decay to all memories.
    pub fn apply_decay(&self) -> MemoryResult<DecayRecord> {
        let mut entries: Vec<MemoryEntry> = self
            .long_term
            .active_entries()
            .into_iter()
            .chain(self.working.entries())
            .collect();

        let mut decay = self.decay.write();
        Ok(decay.apply_decay(&mut entries))
    }

    /// Take a memory snapshot.
    pub fn take_snapshot(
        &self,
        description: Option<String>,
    ) -> MemoryResult<crate::long_term::MemorySnapshot> {
        self.long_term.take_snapshot(description)
    }

    /// Get analytics snapshot.
    pub fn analytics(&self) -> AnalyticsSnapshot {
        let mut all_entries = self.working.entries();
        all_entries.extend(self.long_term.active_entries());
        self.analytics.take_snapshot(&all_entries)
    }

    /// Health check.
    pub fn health(&self) -> HealthResponse {
        let analytics = self.analytics();
        HealthResponse {
            status: "healthy".to_string(),
            total_memories: analytics.total_memories,
            per_tier: analytics.per_tier,
            total_bytes: analytics.total_bytes,
            cache_hit_rate: 0.0,
            uptime_secs: self.start_time.elapsed().as_secs(),
        }
    }

    /// Update a memory entry.
    pub fn update(
        &self,
        id: MemoryId,
        request: UpdateRequest,
    ) -> MemoryResult<()> {
        // Try working memory first.
        if let Some(mut entry) = self.working.get(id) {
            if let Some(content) = request.content {
                entry.content = content;
            }
            if let Some(tags) = request.tags {
                entry.tags = tags.into_iter().collect();
            }
            if let Some(imp) = request.importance {
                entry.importance = imp;
            }
            if let Some(pri) = request.priority {
                entry.priority = pri;
            }
            return Ok(());
        }

        // Try long-term memory.
        self.long_term.update(id, |entry| {
            if let Some(content) = request.content {
                entry.content = content;
            }
            if let Some(tags) = request.tags {
                entry.tags = tags.into_iter().collect();
            }
            if let Some(imp) = request.importance {
                entry.importance = imp;
            }
            if let Some(pri) = request.priority {
                entry.priority = pri;
            }
        })
    }

    /// Delete a memory entry.
    pub fn delete(&self, id: MemoryId) -> MemoryResult<bool> {
        // Try working memory.
        if self.working.remove(id).is_some() {
            return Ok(true);
        }

        // Try long-term memory.
        self.long_term.delete(id)
    }

    /// Merge multiple memory entries.
    pub fn merge(&self, request: MergeRequest) -> MemoryResult<MergeResponse> {
        let mut entries = Vec::new();
        let mut ids = Vec::new();

        for id_str in &request.ids {
            let id = MemoryId::from(uuid::Uuid::parse_str(id_str)
                .map_err(|e| MemoryError::InvalidInput(e.to_string()))?);

            if let Some(entry) = self.recall(id) {
                entries.push(entry);
                ids.push(id);
            }
        }

        if entries.is_empty() {
            return Err(MemoryError::NotFound("No entries found to merge".to_string()));
        }

        let merged_content = match request.strategy {
            MergeStrategy::MostRecent => {
                entries
                    .iter()
                    .max_by_key(|e| e.created_at)
                    .map(|e| e.content.clone())
                    .unwrap_or(serde_json::json!(null))
            }
            MergeStrategy::Concatenate => {
                let contents: Vec<serde_json::Value> =
                    entries.iter().map(|e| e.content.clone()).collect();
                serde_json::json!(contents)
            }
            MergeStrategy::HighestImportance => {
                entries
                    .iter()
                    .max_by(|a, b| {
                        a.importance
                            .partial_cmp(&b.importance)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|e| e.content.clone())
                    .unwrap_or(serde_json::json!(null))
            }
        };

        let mut all_tags = HashSet::new();
        let mut max_importance = 0.0f32;
        for entry in &entries {
            all_tags.extend(entry.tags.iter().cloned());
            max_importance = max_importance.max(entry.importance);
        }

        let merged = MemoryEntry::new(MemoryTier::LongTerm, merged_content, all_tags)
            .with_importance(max_importance);

        let merged_id = merged.id.to_string();
        self.long_term.store(merged)?;

        // Delete originals.
        for id in ids {
            let _ = self.delete(id);
        }

        Ok(MergeResponse {
            merged_id,
            entries_merged: entries.len(),
        })
    }

    /// Summarize multiple entries.
    pub fn summarize(&self, request: SummarizeRequest) -> MemoryResult<SummarizeResponse> {
        let mut entries = Vec::new();
        for id_str in &request.ids {
            let id = MemoryId::from(uuid::Uuid::parse_str(id_str)
                .map_err(|e| MemoryError::InvalidInput(e.to_string()))?);
            if let Some(entry) = self.recall(id) {
                entries.push(entry);
            }
        }

        let summary = self.consolidation.summarize_entries(&entries);

        let max_len = request.max_length.unwrap_or(512);
        let truncated = if summary.len() > max_len {
            format!("{}...", &summary[..max_len.saturating_sub(3)])
        } else {
            summary
        };

        Ok(SummarizeResponse {
            summary: truncated,
            entries_summarized: entries.len(),
        })
    }

    /// Export memories.
    pub fn export(&self, request: ExportRequest) -> MemoryResult<ExportResponse> {
        let mut entries = Vec::new();

        let tiers: Vec<MemoryTier> = request
            .tiers
            .map(|ts| {
                ts.iter()
                    .filter_map(|t| match t.as_str() {
                        "working" => Some(MemoryTier::Working),
                        "episodic" => Some(MemoryTier::Episodic),
                        "semantic" => Some(MemoryTier::Semantic),
                        "procedural" => Some(MemoryTier::Procedural),
                        "long_term" => Some(MemoryTier::LongTerm),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_else(|| {
                vec![
                    MemoryTier::Working,
                    MemoryTier::Episodic,
                    MemoryTier::Semantic,
                    MemoryTier::Procedural,
                    MemoryTier::LongTerm,
                ]
            });

        for entry in self.working.entries() {
            if tiers.contains(&entry.tier) {
                if let Some(ref ns) = request.namespace {
                    if entry.namespace.0 == *ns {
                        entries.push(entry);
                    }
                } else {
                    entries.push(entry);
                }
            }
        }

        for entry in self.long_term.active_entries() {
            if tiers.contains(&entry.tier) {
                if let Some(ref ns) = request.namespace {
                    if entry.namespace.0 == *ns {
                        entries.push(entry);
                    }
                } else {
                    entries.push(entry);
                }
            }
        }

        let count = entries.len();
        let data = match request.format {
            ExportFormat::Json => {
                serde_json::to_string_pretty(&entries)
                    .map_err(|e| MemoryError::SerializationError(e.to_string()))?
            }
            ExportFormat::Csv => {
                let mut csv = String::from("id,tier,namespace,importance,created_at,tags\n");
                for entry in &entries {
                    let tags = entry.tags.iter().cloned().collect::<Vec<_>>().join(";");
                    csv.push_str(&format!(
                        "{},{},{},{},{},{}\n",
                        entry.id,
                        entry.tier,
                        entry.namespace.0,
                        entry.importance,
                        entry.created_at.to_rfc3339(),
                        tags,
                    ));
                }
                csv
            }
        };

        Ok(ExportResponse {
            data,
            count,
            format: format!("{:?}", request.format),
        })
    }

    /// Import memories.
    pub fn import(&self, request: ImportRequest) -> MemoryResult<ImportResponse> {
        let mut errors = Vec::new();
        let mut count = 0;

        match request.format {
            ExportFormat::Json => {
                let entries: Vec<MemoryEntry> = serde_json::from_str(&request.data)
                    .map_err(|e| MemoryError::SerializationError(e.to_string()))?;

                for entry in entries {
                    match self.long_term.store(entry) {
                        Ok(_) => count += 1,
                        Err(e) => errors.push(e.to_string()),
                    }
                }
            }
            ExportFormat::Csv => {
                errors.push("CSV import not yet supported".to_string());
            }
        }

        Ok(ImportResponse { count, errors })
    }

    /// Get access to the working memory subsystem.
    #[must_use]
    pub fn working_memory(&self) -> &WorkingMemory {
        &self.working
    }

    /// Get access to the episodic memory subsystem.
    #[must_use]
    pub fn episodic_memory(&self) -> &EpisodicMemory {
        &self.episodic
    }

    /// Get access to the semantic memory subsystem.
    #[must_use]
    pub fn semantic_memory(&self) -> &SemanticMemory {
        &self.semantic
    }

    /// Get access to the procedural memory subsystem.
    #[must_use]
    pub fn procedural_memory(&self) -> &ProceduralMemory {
        &self.procedural
    }

    /// Get access to the long-term memory subsystem.
    #[must_use]
    pub fn long_term_memory(&self) -> &LongTermMemory {
        &self.long_term
    }

    /// Get the retrieval engine.
    #[must_use]
    pub fn retrieval_engine(&self) -> &RetrievalEngine {
        &self.retrieval
    }

    /// Get the embedding integration.
    #[must_use]
    pub fn embedding(&self) -> &EmbeddingIntegration {
        &self.embedding
    }

    /// Get the indexes.
    #[must_use]
    pub fn indexes(&self) -> &MemoryIndexes {
        &self.indexes
    }

    /// Get the security manager.
    #[must_use]
    pub fn security(&self) -> &MemorySecurity {
        &self.security
    }

    /// Get the analytics engine.
    #[must_use]
    pub fn analytics_engine(&self) -> &MemoryAnalytics {
        &self.analytics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU64;

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_config() -> (tempfile::TempDir, UnifiedMemoryConfig) {
        let dir = tempfile::tempdir().unwrap();
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = dir.path().join(format!("sled-{id}")).to_str().unwrap().to_string();
        let config = UnifiedMemoryConfig {
            persistence: PersistenceConfig {
                path,
                ..PersistenceConfig::default()
            },
            ..UnifiedMemoryConfig::default()
        };
        (dir, config)
    }

    #[test]
    fn create_manager() {
        let (_dir, config) = test_config();
        let manager = CognitiveMemoryManager::new(config);
        assert!(manager.is_ok());
    }

    #[test]
    fn store_and_recall() {
        let (_dir, config) = test_config();
        let manager = CognitiveMemoryManager::new(config).unwrap();
        let response = manager
            .store(StoreRequest {
                tier: MemoryTier::LongTerm,
                content: serde_json::json!("test memory"),
                tags: vec!["test".to_string()],
                importance: Some(0.8),
                priority: None,
                namespace: None,
                ttl_secs: None,
                source: None,
            })
            .unwrap();

        let id = MemoryId::from(
            uuid::Uuid::parse_str(&response.id).unwrap(),
        );
        let recalled = manager.recall(id);
        assert!(recalled.is_some());
    }

    #[test]
    fn search_memories() {
        let (_dir, config) = test_config();
        let manager = CognitiveMemoryManager::new(config).unwrap();

        manager
            .store(StoreRequest {
                tier: MemoryTier::LongTerm,
                content: serde_json::json!("Rust programming language"),
                tags: vec!["rust".to_string()],
                importance: Some(0.7),
                priority: None,
                namespace: None,
                ttl_secs: None,
                source: None,
            })
            .unwrap();

        let response = manager
            .search(SearchRequest {
                query: Some("rust".to_string()),
                limit: Some(10),
                ..SearchRequest::default()
            })
            .unwrap();

        assert!(response.total > 0);
    }

    #[test]
    fn update_memory() {
        let (_dir, config) = test_config();
        let manager = CognitiveMemoryManager::new(config).unwrap();
        let response = manager
            .store(StoreRequest {
                tier: MemoryTier::LongTerm,
                content: serde_json::json!("original"),
                tags: vec![],
                importance: Some(0.5),
                priority: None,
                namespace: None,
                ttl_secs: None,
                source: None,
            })
            .unwrap();

        let id = MemoryId::from(
            uuid::Uuid::parse_str(&response.id).unwrap(),
        );

        manager
            .update(
                id,
                UpdateRequest {
                    content: Some(serde_json::json!("updated")),
                    tags: None,
                    importance: Some(0.9),
                    priority: None,
                },
            )
            .unwrap();
    }

    #[test]
    fn delete_memory() {
        let (_dir, config) = test_config();
        let manager = CognitiveMemoryManager::new(config).unwrap();
        let response = manager
            .store(StoreRequest {
                tier: MemoryTier::LongTerm,
                content: serde_json::json!("to delete"),
                tags: vec![],
                importance: None,
                priority: None,
                namespace: None,
                ttl_secs: None,
                source: None,
            })
            .unwrap();

        let id = MemoryId::from(
            uuid::Uuid::parse_str(&response.id).unwrap(),
        );

        let deleted = manager.delete(id).unwrap();
        assert!(deleted);
    }

    #[test]
    fn health_check() {
        let (_dir, config) = test_config();
        let manager = CognitiveMemoryManager::new(config).unwrap();
        let health = manager.health();
        assert_eq!(health.status, "healthy");
    }

    #[test]
    fn build_context() {
        let (_dir, config) = test_config();
        let manager = CognitiveMemoryManager::new(config).unwrap();

        manager
            .store(StoreRequest {
                tier: MemoryTier::LongTerm,
                content: serde_json::json!("relevant context"),
                tags: vec!["context".to_string()],
                importance: Some(0.8),
                priority: None,
                namespace: None,
                ttl_secs: None,
                source: None,
            })
            .unwrap();

        let ctx = manager.build_context(None, 4096);
        assert!(ctx.max_tokens == 4096);
    }

    #[test]
    fn merge_memories() {
        let (_dir, config) = test_config();
        let manager = CognitiveMemoryManager::new(config).unwrap();

        let r1 = manager
            .store(StoreRequest {
                tier: MemoryTier::LongTerm,
                content: serde_json::json!("first"),
                tags: vec!["a".to_string()],
                importance: Some(0.3),
                priority: None,
                namespace: None,
                ttl_secs: None,
                source: None,
            })
            .unwrap();

        let r2 = manager
            .store(StoreRequest {
                tier: MemoryTier::LongTerm,
                content: serde_json::json!("second"),
                tags: vec!["b".to_string()],
                importance: Some(0.8),
                priority: None,
                namespace: None,
                ttl_secs: None,
                source: None,
            })
            .unwrap();

        let response = manager
            .merge(MergeRequest {
                ids: vec![r1.id, r2.id],
                strategy: MergeStrategy::HighestImportance,
            })
            .unwrap();

        assert_eq!(response.entries_merged, 2);
    }

    #[test]
    fn export_import() {
        let (_dir, config) = test_config();
        let manager = CognitiveMemoryManager::new(config).unwrap();

        manager
            .store(StoreRequest {
                tier: MemoryTier::LongTerm,
                content: serde_json::json!("export test"),
                tags: vec!["export".to_string()],
                importance: None,
                priority: None,
                namespace: None,
                ttl_secs: None,
                source: None,
            })
            .unwrap();

        let exported = manager
            .export(ExportRequest {
                tiers: None,
                namespace: None,
                format: ExportFormat::Json,
            })
            .unwrap();

        assert!(exported.count > 0);

        let imported = manager
            .import(ImportRequest {
                data: exported.data,
                format: ExportFormat::Json,
                namespace: None,
            })
            .unwrap();

        assert!(imported.count > 0);
    }

    #[test]
    fn fact_management() {
        let (_dir, config) = test_config();
        let manager = CognitiveMemoryManager::new(config).unwrap();

        let fact = SemanticFact::new("Neo", "is_a", serde_json::json!("AGI System"))
            .with_confidence(0.95);

        let fact_id = manager.add_fact(fact).unwrap();

        let facts = manager.query_facts("Neo");
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].subject, "Neo");
    }

    #[test]
    fn procedure_management() {
        let (_dir, config) = test_config();
        let manager = CognitiveMemoryManager::new(config).unwrap();

        let mut proc = Procedure::new("Test Procedure", "A test procedure");
        proc.tags = vec!["test".to_string()];

        let proc_id = manager.store_procedure(proc).unwrap();

        let results = manager.search_procedures("Test");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn summarization() {
        let (_dir, config) = test_config();
        let manager = CognitiveMemoryManager::new(config).unwrap();

        let r1 = manager
            .store(StoreRequest {
                tier: MemoryTier::LongTerm,
                content: serde_json::json!("First memory"),
                tags: vec![],
                importance: Some(0.8),
                priority: None,
                namespace: None,
                ttl_secs: None,
                source: None,
            })
            .unwrap();

        let r2 = manager
            .store(StoreRequest {
                tier: MemoryTier::LongTerm,
                content: serde_json::json!("Second memory"),
                tags: vec![],
                importance: Some(0.3),
                priority: None,
                namespace: None,
                ttl_secs: None,
                source: None,
            })
            .unwrap();

        let response = manager
            .summarize(SummarizeRequest {
                ids: vec![r1.id, r2.id],
                max_length: Some(200),
            })
            .unwrap();

        assert!(response.entries_summarized == 2);
        assert!(!response.summary.is_empty());
    }
}
