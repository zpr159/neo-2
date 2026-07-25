use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::conversation::error::ConversationResult;
use crate::conversation::evidence::Evidence;
use crate::conversation::types::ConversationContext;

/// Result from knowledge graph retrieval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeResult {
    pub entities: Vec<Entity>,
    pub relationships: Vec<Relationship>,
    pub facts: Vec<Fact>,
    pub provenance: Vec<Provenance>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub name: String,
    pub entity_type: String,
    pub attributes: std::collections::HashMap<String, String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub source: String,
    pub target: String,
    pub relationship_type: String,
    pub weight: f32,
    pub confidence: f32,
    pub attributes: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f32,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    pub fact_id: String,
    pub source_system: String,
    pub timestamp: crate::time::Timestamp,
    pub chain: Vec<String>,
}

/// Bridge between the Knowledge Graph subsystem and the Conversation layer.
#[async_trait]
pub trait KnowledgeConversationBridge: Send + Sync {
    /// Look up entities by name or identifier.
    async fn entity_lookup(
        &self,
        context: &ConversationContext,
        query: &str,
    ) -> ConversationResult<Vec<Entity>>;

    /// Traverse relationships from a starting entity.
    async fn relationship_traversal(
        &self,
        context: &ConversationContext,
        entity_id: &str,
        max_depth: usize,
    ) -> ConversationResult<Vec<Relationship>>;

    /// Expand the ontology with new types and relationships.
    async fn ontology_expansion(
        &self,
        context: &ConversationContext,
        new_types: &[String],
        new_relationships: &[(String, String, String)],
    ) -> ConversationResult<()>;

    /// Search the knowledge graph.
    async fn graph_search(
        &self,
        context: &ConversationContext,
        query: &str,
        limit: usize,
    ) -> ConversationResult<KnowledgeResult>;

    /// Retrieve the semantic neighborhood of an entity.
    async fn semantic_neighborhood(
        &self,
        context: &ConversationContext,
        entity_id: &str,
        radius: usize,
    ) -> ConversationResult<KnowledgeResult>;

    /// Verify a fact against the knowledge graph.
    async fn verify_fact(
        &self,
        context: &ConversationContext,
        subject: &str,
        predicate: &str,
        object: &str,
    ) -> ConversationResult<(bool, f32)>;

    /// Retrieve facts relevant to a query, returned as evidence.
    async fn retrieve_evidence(
        &self,
        context: &ConversationContext,
        query: &str,
        limit: usize,
    ) -> ConversationResult<Vec<Evidence>>;
}

/// Mock implementation for testing.
pub struct MockKnowledgeBridge;

#[async_trait]
impl KnowledgeConversationBridge for MockKnowledgeBridge {
    async fn entity_lookup(
        &self,
        _context: &ConversationContext,
        _query: &str,
    ) -> ConversationResult<Vec<Entity>> {
        Ok(Vec::new())
    }

    async fn relationship_traversal(
        &self,
        _context: &ConversationContext,
        _entity_id: &str,
        _max_depth: usize,
    ) -> ConversationResult<Vec<Relationship>> {
        Ok(Vec::new())
    }

    async fn ontology_expansion(
        &self,
        _context: &ConversationContext,
        _new_types: &[String],
        _new_relationships: &[(String, String, String)],
    ) -> ConversationResult<()> {
        Ok(())
    }

    async fn graph_search(
        &self,
        _context: &ConversationContext,
        _query: &str,
        _limit: usize,
    ) -> ConversationResult<KnowledgeResult> {
        Ok(KnowledgeResult {
            entities: Vec::new(),
            relationships: Vec::new(),
            facts: Vec::new(),
            provenance: Vec::new(),
            confidence: 0.0,
        })
    }

    async fn semantic_neighborhood(
        &self,
        _context: &ConversationContext,
        _entity_id: &str,
        _radius: usize,
    ) -> ConversationResult<KnowledgeResult> {
        Ok(KnowledgeResult {
            entities: Vec::new(),
            relationships: Vec::new(),
            facts: Vec::new(),
            provenance: Vec::new(),
            confidence: 0.0,
        })
    }

    async fn verify_fact(
        &self,
        _context: &ConversationContext,
        _subject: &str,
        _predicate: &str,
        _object: &str,
    ) -> ConversationResult<(bool, f32)> {
        Ok((false, 0.0))
    }

    async fn retrieve_evidence(
        &self,
        _context: &ConversationContext,
        _query: &str,
        _limit: usize,
    ) -> ConversationResult<Vec<Evidence>> {
        Ok(Vec::new())
    }
}
