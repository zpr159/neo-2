use crate::core::entity::{Entity, EntityId, EntityType};
use crate::error::KnowledgeResult;

/// Result of splitting a concept.
#[derive(Debug, Clone)]
pub struct SplitOutcome {
    /// The original entity id.
    pub original: EntityId,
    /// The new entity ids created.
    pub created: Vec<EntityId>,
    /// Description.
    pub description: String,
}

/// Splits a concept into multiple more specific concepts.
pub struct ConceptSplitter;

impl ConceptSplitter {
    /// Create a new splitter.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Split an entity into two based on a criterion (label parts).
    #[must_use]
    pub fn split_by_labels(
        &self,
        original: &Entity,
        label_a: &str,
        label_b: &str,
    ) -> SplitOutcome {
        let new_a = Entity::builder(original.entity_type.clone(), label_a)
            .description(format!("Split from '{}'", original.label))
            .confidence(original.confidence * 0.9)
            .build();

        let new_b = Entity::builder(original.entity_type.clone(), label_b)
            .description(format!("Split from '{}'", original.label))
            .confidence(original.confidence * 0.9)
            .build();

        SplitOutcome {
            original: original.id,
            created: vec![new_a.id, new_b.id],
            description: format!(
                "Split '{}' into '{}' and '{}'",
                original.label, label_a, label_b
            ),
        }
    }
}

impl Default for ConceptSplitter {
    fn default() -> Self {
        Self::new()
    }
}
