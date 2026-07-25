use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::conversation::error::ConversationResult;
use crate::conversation::evidence::Evidence;
use crate::conversation::types::ConversationContext;

/// Memory types for retrieval.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    Working,
    Episodic,
    Semantic,
    LongTerm,
    Vector,
}

/// Retrieval strategy for memory access.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalMethod {
    SimilaritySearch,
    KeywordSearch,
    SemanticRetrieval,
    HybridRetrieval,
    TimeBasedRecall,
    ImportanceRanking,
    ContextExpansion,
}

/// A memory retrieval query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQuery {
    pub text: String,
    pub memory_types: Vec<MemoryType>,
    pub retrieval_methods: Vec<RetrievalMethod>,
    pub limit: usize,
    pub confidence_threshold: f32,
    pub time_range: Option<TimeRange>,
    pub importance_threshold: f32,
    pub expand_context: bool,
    pub context_hops: usize,
}

impl Default for MemoryQuery {
    fn default() -> Self {
        Self {
            text: String::new(),
            memory_types: vec![MemoryType::Working, MemoryType::Episodic, MemoryType::Semantic],
            retrieval_methods: vec![RetrievalMethod::HybridRetrieval],
            limit: 10,
            confidence_threshold: 0.3,
            time_range: None,
            importance_threshold: 0.0,
            expand_context: true,
            context_hops: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: crate::time::Timestamp,
    pub end: crate::time::Timestamp,
}

/// Result from memory retrieval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRetrievalResult {
    pub evidence: Vec<Evidence>,
    pub total_retrieved: usize,
    pub memory_types_used: Vec<MemoryType>,
    pub retrieval_methods_used: Vec<RetrievalMethod>,
    pub average_confidence: f32,
}

/// A memory item to store after interaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConsolidationItem {
    pub content: String,
    pub memory_type: MemoryType,
    pub importance: f32,
    pub context: std::collections::HashMap<String, String>,
    pub source_conversation_id: uuid::Uuid,
    pub timestamp: crate::time::Timestamp,
}

/// Bridge between the Memory subsystem and the Conversation layer.
#[async_trait]
pub trait MemoryConversationBridge: Send + Sync {
    /// Retrieve memories matching the query.
    async fn retrieve(
        &self,
        context: &ConversationContext,
        query: &MemoryQuery,
    ) -> ConversationResult<MemoryRetrievalResult>;

    /// Store a new memory item.
    async fn store(
        &self,
        context: &ConversationContext,
        item: &MemoryConsolidationItem,
    ) -> ConversationResult<()>;

    /// Consolidate memories from a completed interaction.
    async fn consolidate(
        &self,
        context: &ConversationContext,
        items: &[MemoryConsolidationItem],
    ) -> ConversationResult<()>;

    /// Search by similarity.
    async fn similarity_search(
        &self,
        context: &ConversationContext,
        query: &str,
        limit: usize,
    ) -> ConversationResult<Vec<Evidence>>;

    /// Search by keywords.
    async fn keyword_search(
        &self,
        context: &ConversationContext,
        keywords: &[String],
        limit: usize,
    ) -> ConversationResult<Vec<Evidence>>;

    /// Search by semantic meaning.
    async fn semantic_search(
        &self,
        context: &ConversationContext,
        query: &str,
        limit: usize,
    ) -> ConversationResult<Vec<Evidence>>;
}

/// Mock implementation for testing.
pub struct MockMemoryBridge;

#[async_trait]
impl MemoryConversationBridge for MockMemoryBridge {
    async fn retrieve(
        &self,
        _context: &ConversationContext,
        query: &MemoryQuery,
    ) -> ConversationResult<MemoryRetrievalResult> {
        Ok(MemoryRetrievalResult {
            evidence: Vec::new(),
            total_retrieved: 0,
            memory_types_used: query.memory_types.clone(),
            retrieval_methods_used: query.retrieval_methods.clone(),
            average_confidence: 0.0,
        })
    }

    async fn store(
        &self,
        _context: &ConversationContext,
        _item: &MemoryConsolidationItem,
    ) -> ConversationResult<()> {
        Ok(())
    }

    async fn consolidate(
        &self,
        _context: &ConversationContext,
        _items: &[MemoryConsolidationItem],
    ) -> ConversationResult<()> {
        Ok(())
    }

    async fn similarity_search(
        &self,
        _context: &ConversationContext,
        _query: &str,
        _limit: usize,
    ) -> ConversationResult<Vec<Evidence>> {
        Ok(Vec::new())
    }

    async fn keyword_search(
        &self,
        _context: &ConversationContext,
        _keywords: &[String],
        _limit: usize,
    ) -> ConversationResult<Vec<Evidence>> {
        Ok(Vec::new())
    }

    async fn semantic_search(
        &self,
        _context: &ConversationContext,
        _query: &str,
        _limit: usize,
    ) -> ConversationResult<Vec<Evidence>> {
        Ok(Vec::new())
    }
}
