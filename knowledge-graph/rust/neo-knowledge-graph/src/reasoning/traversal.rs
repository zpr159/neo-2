use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::core::entity::{Entity, EntityId};
use crate::core::relation::{Relation, RelationType};
use crate::storage::graph_store::GraphStore;

/// Configuration for graph traversal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalConfig {
    /// Maximum depth.
    pub max_depth: u32,
    /// Maximum number of results.
    pub max_results: usize,
    /// Filter by relation types.
    pub edge_filter: Option<Vec<RelationType>>,
    /// Filter by entity types (as strings).
    pub node_type_filter: Option<Vec<String>>,
}

impl Default for TraversalConfig {
    fn default() -> Self {
        Self {
            max_depth: 5,
            max_results: 100,
            edge_filter: None,
            node_type_filter: None,
        }
    }
}

/// Result of a graph traversal.
#[derive(Debug, Clone)]
pub struct TraversalResult {
    /// Entities visited.
    pub entities: Vec<Entity>,
    /// Relations traversed.
    pub relations: Vec<Relation>,
    /// BFS paths from start.
    pub paths: Vec<Vec<EntityId>>,
    /// Visit order.
    pub visit_order: Vec<EntityId>,
}

/// Performs graph traversals (BFS, DFS).
pub struct GraphTraversal;

impl GraphTraversal {
    /// Create a new traversal engine.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// BFS traversal from a start entity.
    #[must_use]
    pub fn bfs(
        &self,
        store: &GraphStore,
        start: EntityId,
        config: &TraversalConfig,
    ) -> TraversalResult {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut entities = Vec::new();
        let mut relations = Vec::new();
        let mut visit_order = Vec::new();

        queue.push_back((start, 0u32));
        visited.insert(start);

        while let Some((current, depth)) = queue.pop_front() {
            if depth > config.max_depth || entities.len() >= config.max_results {
                continue;
            }

            if let Some(entity) = store.get_entity(current) {
                let matches_type = config
                    .node_type_filter
                    .as_ref()
                    .map_or(true, |types| types.contains(&entity.entity_type.to_string()));
                if matches_type {
                    entities.push(entity);
                    visit_order.push(current);
                }
            }

            // Process outgoing
            for relation in store.get_outgoing_relations(current) {
                let matches_edge = config.edge_filter.as_ref().map_or(true, |types| {
                    types.contains(&relation.relation_type)
                });
                if matches_edge {
                    relations.push(relation.clone());
                    if visited.insert(relation.target) {
                        queue.push_back((relation.target, depth + 1));
                    }
                }
            }

            // Process incoming
            for relation in store.get_incoming_relations(current) {
                let matches_edge = config.edge_filter.as_ref().map_or(true, |types| {
                    types.contains(&relation.relation_type)
                });
                if matches_edge {
                    if visited.insert(relation.source) {
                        queue.push_back((relation.source, depth + 1));
                    }
                }
            }
        }

        TraversalResult {
            entities,
            relations,
            paths: Vec::new(),
            visit_order,
        }
    }

    /// DFS traversal from a start entity.
    #[must_use]
    pub fn dfs(
        &self,
        store: &GraphStore,
        start: EntityId,
        config: &TraversalConfig,
    ) -> TraversalResult {
        let mut visited = HashSet::new();
        let mut entities = Vec::new();
        let mut relations = Vec::new();
        let mut visit_order = Vec::new();

        self.dfs_recursive(
            store, start, config, 0, &mut visited, &mut entities, &mut relations, &mut visit_order,
        );

        TraversalResult {
            entities,
            relations,
            paths: Vec::new(),
            visit_order,
        }
    }

    fn dfs_recursive(
        &self,
        store: &GraphStore,
        current: EntityId,
        config: &TraversalConfig,
        depth: u32,
        visited: &mut HashSet<EntityId>,
        entities: &mut Vec<Entity>,
        relations: &mut Vec<Relation>,
        visit_order: &mut Vec<EntityId>,
    ) {
        if depth > config.max_depth || entities.len() >= config.max_results {
            return;
        }

        if !visited.insert(current) {
            return;
        }

        if let Some(entity) = store.get_entity(current) {
            let matches_type = config
                .node_type_filter
                .as_ref()
                .map_or(true, |types| types.contains(&entity.entity_type.to_string()));
            if matches_type {
                entities.push(entity);
                visit_order.push(current);
            }
        }

        for relation in store.get_outgoing_relations(current) {
            let matches_edge = config.edge_filter.as_ref().map_or(true, |types| {
                types.contains(&relation.relation_type)
            });
            if matches_edge {
                relations.push(relation.clone());
                self.dfs_recursive(
                    store, relation.target, config, depth + 1,
                    visited, entities, relations, visit_order,
                );
            }
        }
    }
}

impl Default for GraphTraversal {
    fn default() -> Self {
        Self::new()
    }
}
