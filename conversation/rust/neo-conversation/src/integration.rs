use std::sync::Arc;

use crate::error::ConversationResult;
use crate::metrics::ConversationMetrics;
use crate::persistence::{InMemoryPersistence, PersistenceBackend};
use crate::types::CognitiveContext;

/// Integrates the conversation subsystem with all cognitive subsystems.
///
/// Acts as the glue layer that connects the conversation pipeline
/// to memory, knowledge, reasoning, planning, executive, workflow,
/// and agent interfaces.
pub struct ConversationIntegration {
    pub metrics: ConversationMetrics,
    pub persistence: Arc<dyn PersistenceBackend>,
}

impl ConversationIntegration {
    pub fn new() -> Self {
        Self {
            metrics: ConversationMetrics::new(),
            persistence: Arc::new(InMemoryPersistence::new()),
        }
    }

    pub fn with_persistence(persistence: Arc<dyn PersistenceBackend>) -> Self {
        Self {
            metrics: ConversationMetrics::new(),
            persistence,
        }
    }

    /// Gather cognitive context from all subsystems for a given query.
    ///
    /// In a full implementation, this would call each cognitive interface
    /// and merge results. Currently returns an empty context scaffold.
    pub fn gather_context(&self, _query: &str) -> ConversationResult<CognitiveContext> {
        let _ = &self.metrics;
        Ok(CognitiveContext::empty())
    }
}

impl Default for ConversationIntegration {
    fn default() -> Self {
        Self::new()
    }
}
