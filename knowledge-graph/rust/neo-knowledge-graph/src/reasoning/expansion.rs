use std::collections::{HashMap, HashSet, VecDeque};

use crate::core::entity::{Entity, EntityId};
use crate::core::relation::{Relation, RelationType};
use crate::storage::graph_store::GraphStore;

/// Expands neighborhoods around entities in the graph.
pub struct NeighborExpander;

impl NeighborExpander {
    /// Create a new expander.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Expand neighbors up to a given depth, returning entities and relations.
    #[must_use]
    pub fn expand(
        &self,
        store: &GraphStore,
        start: EntityId,
        depth: u32,
        edge_filter: Option<&[RelationType]>,
    ) -> (Vec<Entity>, Vec<Relation>) {
        let mut visited_entities: HashSet<EntityId> = HashSet::new();
        let mut visited_relations: HashSet<crate::core::relation::RelationId> = HashSet::new();
        let mut result_entities = Vec::new();
        let mut result_relations = Vec::new();

        let mut queue: VecDeque<(EntityId, u32)> = VecDeque::new();
        queue.push_back((start, 0));
        visited_entities.insert(start);

        while let Some((current, current_depth)) = queue.pop_front() {
            if current_depth >= depth {
                continue;
            }

            if let Some(entity) = store.get_entity(current) {
                result_entities.push(entity);
            }

            // Outgoing
            for relation in store.get_outgoing_relations(current) {
                if let Some(filter) = edge_filter {
                    if !filter.contains(&relation.relation_type) {
                        continue;
                    }
                }
                if visited_relations.insert(relation.id) {
                    result_relations.push(relation.clone());
                }
                if visited_entities.insert(relation.target) {
                    queue.push_back((relation.target, current_depth + 1));
                }
            }

            // Incoming
            for relation in store.get_incoming_relations(current) {
                if let Some(filter) = edge_filter {
                    if !filter.contains(&relation.relation_type) {
                        continue;
                    }
                }
                if visited_relations.insert(relation.id) {
                    result_relations.push(relation.clone());
                }
                if visited_entities.insert(relation.source) {
                    queue.push_back((relation.source, current_depth + 1));
                }
            }
        }

        (result_entities, result_relations)
    }

    /// Get all neighbors reachable within N hops (excluding start).
    #[must_use]
    pub fn n_hop_neighbors(
        &self,
        store: &GraphStore,
        start: EntityId,
        hops: u32,
    ) -> Vec<EntityId> {
        let mut visited = HashSet::new();
        let mut current_level = HashSet::new();
        current_level.insert(start);

        for _ in 0..hops {
            let mut next_level = HashSet::new();
            for &entity_id in &current_level {
                for neighbor in store.neighbors(entity_id) {
                    if visited.insert(neighbor) {
                        next_level.insert(neighbor);
                    }
                }
            }
            current_level = next_level;
        }

        // Include all visited nodes (everything reachable within hops, minus start)
        visited.into_iter().collect()
    }
}

impl Default for NeighborExpander {
    fn default() -> Self {
        Self::new()
    }
}
