use crate::core::entity::{Entity, EntityId};
use crate::core::relation::{Relation, RelationType};
use crate::storage::graph_store::GraphStore;

/// Discovered relationship between entities.
#[derive(Debug, Clone)]
pub struct DiscoveredRelation {
    /// Source entity id.
    pub source: EntityId,
    /// Target entity id.
    pub target: EntityId,
    /// Suggested relation type.
    pub relation_type: RelationType,
    /// Confidence in the discovery.
    pub confidence: f32,
    /// Reason for the discovery.
    pub reason: String,
}

/// Automatically discovers new relationships based on graph structure.
pub struct RelationshipDiscovery;

impl RelationshipDiscovery {
    /// Create a new discovery engine.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Discover implicit relationships based on shared neighbors.
    #[must_use]
    pub fn discover_by_shared_neighbors(
        &self,
        store: &GraphStore,
        min_shared: usize,
        confidence_threshold: f32,
    ) -> Vec<DiscoveredRelation> {
        let mut discoveries = Vec::new();
        let entities = store.all_entities();

        for (i, entity_a) in entities.iter().enumerate() {
            if !entity_a.active {
                continue;
            }
            let neighbors_a: std::collections::HashSet<EntityId> =
                store.neighbors(entity_a.id).into_iter().collect();

            for entity_b in entities.iter().skip(i + 1) {
                if !entity_b.active || entity_a.id == entity_b.id {
                    continue;
                }
                let neighbors_b: std::collections::HashSet<EntityId> =
                    store.neighbors(entity_b.id).into_iter().collect();

                let shared: Vec<&EntityId> = neighbors_a
                    .intersection(&neighbors_b)
                    .filter(|n| **n != entity_a.id && **n != entity_b.id)
                    .collect();

                if shared.len() >= min_shared {
                    let total = neighbors_a.len().max(neighbors_b.len()) as f32;
                    let confidence = (shared.len() as f32 / total).min(1.0);

                    if confidence >= confidence_threshold {
                        // Check if this relation already exists
                        let existing = store.find_relations_by_type(&RelationType::RelatedTo);
                        let already_exists = existing.iter().any(|r| {
                            (r.source == entity_a.id && r.target == entity_b.id)
                                || (r.source == entity_b.id && r.target == entity_a.id)
                        });

                        if !already_exists {
                            discoveries.push(DiscoveredRelation {
                                source: entity_a.id,
                                target: entity_b.id,
                                relation_type: RelationType::RelatedTo,
                                confidence,
                                reason: format!(
                                    "Share {} common neighbors",
                                    shared.len()
                                ),
                            });
                        }
                    }
                }
            }
        }

        discoveries
    }
}

impl Default for RelationshipDiscovery {
    fn default() -> Self {
        Self::new()
    }
}
