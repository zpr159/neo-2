use crate::error::ConversationResult;

/// Interface to the world model subsystem.
///
/// Provides environmental awareness, entity tracking,
/// and causal reasoning about the external world.
pub trait WorldModelInterface: Send + Sync {
    /// Get current world state observations.
    fn observe(&self) -> ConversationResult<Vec<WorldObservation>>;

    /// Query specific aspects of the world model.
    fn query(&self, query: &str) -> ConversationResult<Vec<WorldObservation>>;

    /// Get entities known to the world model.
    fn entities(&self) -> ConversationResult<Vec<WorldEntity>>;
}

/// An observation from the world model.
#[derive(Debug, Clone)]
pub struct WorldObservation {
    pub content: String,
    pub confidence: f64,
    pub source: String,
    pub timestamp: Option<String>,
}

/// An entity tracked by the world model.
#[derive(Debug, Clone)]
pub struct WorldEntity {
    pub name: String,
    pub entity_type: String,
    pub state: std::collections::HashMap<String, String>,
}

/// Default world model interface.
pub struct DefaultWorldModel;

impl WorldModelInterface for DefaultWorldModel {
    fn observe(&self) -> ConversationResult<Vec<WorldObservation>> {
        Ok(Vec::new())
    }

    fn query(&self, _query: &str) -> ConversationResult<Vec<WorldObservation>> {
        Ok(Vec::new())
    }

    fn entities(&self) -> ConversationResult<Vec<WorldEntity>> {
        Ok(Vec::new())
    }
}
