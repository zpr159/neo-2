use std::collections::{HashMap, HashSet, VecDeque};

use crate::core::entity::{Entity, EntityId};
use crate::core::relation::Relation;
use crate::storage::graph_store::GraphStore;

/// Extracts subgraphs from the main graph based on various criteria.
pub struct SubgraphExtractor;

impl SubgraphExtractor {
    /// Create a new extractor.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Extract the subgraph around a set of seed entities within a given radius.
    #[must_use]
    pub fn ego_network(
        &self,
        store: &GraphStore,
        seeds: &[EntityId],
        radius: u32,
    ) -> (Vec<Entity>, Vec<Relation>) {
        let mut visited_entities = HashSet::new();
        let mut visited_relations = HashSet::new();
        let mut result_entities = Vec::new();
        let mut result_relations = Vec::new();
        let mut queue: VecDeque<(EntityId, u32)> = VecDeque::new();

        for &seed in seeds {
            queue.push_back((seed, 0));
            visited_entities.insert(seed);
        }

        while let Some((current, depth)) = queue.pop_front() {
            if depth > radius {
                continue;
            }

            if let Some(entity) = store.get_entity(current) {
                result_entities.push(entity);
            }

            for relation in store.get_outgoing_relations(current) {
                if visited_relations.insert(relation.id) {
                    result_relations.push(relation.clone());
                }
                if visited_entities.insert(relation.target) {
                    queue.push_back((relation.target, depth + 1));
                }
            }

            for relation in store.get_incoming_relations(current) {
                if visited_relations.insert(relation.id) {
                    result_relations.push(relation.clone());
                }
                if visited_entities.insert(relation.source) {
                    queue.push_back((relation.source, depth + 1));
                }
            }
        }

        (result_entities, result_relations)
    }

    /// Extract a subgraph containing only entities of specific types.
    #[must_use]
    pub fn by_entity_types(
        &self,
        store: &GraphStore,
        types: &[String],
    ) -> (Vec<Entity>, Vec<Relation>) {
        let target_set: HashSet<&String> = types.iter().collect();
        let mut type_entities = Vec::new();
        let mut type_relations = Vec::new();
        let mut entity_ids = HashSet::new();

        for entity in store.all_entities() {
            let type_str = entity.entity_type.to_string();
            if target_set.contains(&type_str) {
                entity_ids.insert(entity.id);
                type_entities.push(entity);
            }
        }

        for relation in store.all_relations() {
            if entity_ids.contains(&relation.source) && entity_ids.contains(&relation.target) {
                type_relations.push(relation);
            }
        }

        (type_entities, type_relations)
    }

    /// Extract the strongly connected component containing a given entity.
    #[must_use]
    pub fn connected_component(
        &self,
        store: &GraphStore,
        seed: EntityId,
    ) -> (Vec<Entity>, Vec<Relation>) {
        let mut visited = HashSet::new();
        let mut result_entities = Vec::new();
        let mut result_relations = Vec::new();
        let mut queue = VecDeque::new();

        queue.push_back(seed);
        visited.insert(seed);

        while let Some(current) = queue.pop_front() {
            if let Some(entity) = store.get_entity(current) {
                result_entities.push(entity);
            }

            for relation in store.get_outgoing_relations(current) {
                result_relations.push(relation.clone());
                if visited.insert(relation.target) {
                    queue.push_back(relation.target);
                }
            }

            for relation in store.get_incoming_relations(current) {
                if visited.insert(relation.source) {
                    result_relations.push(relation.clone());
                    queue.push_back(relation.source);
                }
            }
        }

        (result_entities, result_relations)
    }
}

impl Default for SubgraphExtractor {
    fn default() -> Self {
        Self::new()
    }
}
