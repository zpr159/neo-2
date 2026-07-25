use serde::{Deserialize, Serialize};

use crate::core::entity::Entity;
use crate::core::relation::Relation;
use crate::error::KnowledgeResult;
use crate::storage::graph_store::GraphStore;

/// Configuration for pruning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruningConfig {
    /// Minimum confidence to keep.
    pub min_confidence: f32,
    /// Minimum importance to keep.
    pub min_importance: f32,
    /// Whether to preserve entities with incoming relations.
    pub preserve_connected: bool,
}

impl Default for PruningConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.1,
            min_importance: 0.1,
            preserve_connected: true,
        }
    }
}

/// Result of a pruning operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruningResult {
    /// Number of entities pruned.
    pub entities_pruned: usize,
    /// Number of relations pruned.
    pub relations_pruned: usize,
    /// Ids of pruned entities.
    pub pruned_entity_ids: Vec<String>,
}

/// Prunes low-quality knowledge from the graph.
pub struct KnowledgePruner {
    config: PruningConfig,
}

impl KnowledgePruner {
    /// Create a new pruner.
    #[must_use]
    pub fn new(config: PruningConfig) -> Self {
        Self { config }
    }

    /// Identify entities that should be pruned.
    #[must_use]
    pub fn candidates(&self, entities: &[Entity], store: &GraphStore) -> Vec<String> {
        entities
            .iter()
            .filter(|e| {
                if !e.active {
                    return false;
                }
                if e.confidence < self.config.min_confidence
                    || e.importance < self.config.min_importance
                {
                    if self.config.preserve_connected {
                        let out = store.get_outgoing_relation_ids(e.id);
                        let in_rel = store.get_incoming_relation_ids(e.id);
                        out.is_empty() && in_rel.is_empty()
                    } else {
                        true
                    }
                } else {
                    false
                }
            })
            .map(|e| e.id.to_string())
            .collect()
    }
}

impl Default for KnowledgePruner {
    fn default() -> Self {
        Self::new(PruningConfig::default())
    }
}
