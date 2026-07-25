use crate::error::ConversationResult;

/// Interface to the knowledge graph subsystem.
///
/// Provides fact retrieval, entity lookup, and relationship traversal.
pub trait KnowledgeInterface: Send + Sync {
    /// Query the knowledge graph for facts relevant to the query.
    fn query(&self, query: &str, limit: usize) -> ConversationResult<Vec<KnowledgeResult>>;

    /// Look up a specific entity.
    fn lookup_entity(&self, entity: &str) -> ConversationResult<Option<EntityInfo>>;

    /// Get relationships for an entity.
    fn relationships(&self, entity: &str) -> ConversationResult<Vec<Relationship>>;
}

/// A knowledge graph query result.
#[derive(Debug, Clone)]
pub struct KnowledgeResult {
    pub fact: String,
    pub confidence: f64,
    pub source: String,
    pub entities: Vec<String>,
}

/// Information about an entity.
#[derive(Debug, Clone)]
pub struct EntityInfo {
    pub name: String,
    pub entity_type: String,
    pub properties: std::collections::HashMap<String, String>,
}

/// A relationship between entities.
#[derive(Debug, Clone)]
pub struct Relationship {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f64,
}

/// Default knowledge interface.
pub struct DefaultKnowledge;

impl KnowledgeInterface for DefaultKnowledge {
    fn query(&self, _query: &str, _limit: usize) -> ConversationResult<Vec<KnowledgeResult>> {
        Ok(Vec::new())
    }

    fn lookup_entity(&self, _entity: &str) -> ConversationResult<Option<EntityInfo>> {
        Ok(None)
    }

    fn relationships(&self, _entity: &str) -> ConversationResult<Vec<Relationship>> {
        Ok(Vec::new())
    }
}
