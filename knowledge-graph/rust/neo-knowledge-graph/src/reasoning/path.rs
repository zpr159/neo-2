use std::collections::{HashMap, HashSet, VecDeque};

use crate::core::entity::{Entity, EntityId};
use crate::core::relation::Relation;
use crate::error::KnowledgeResult;
use crate::storage::graph_store::GraphStore;

/// Result of a path search.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// The path found (sequence of entity ids).
    pub path: Vec<EntityId>,
    /// Total path weight (sum of edge weights).
    pub total_weight: f32,
    /// Number of hops.
    pub hops: usize,
    /// Whether a path was found.
    pub found: bool,
}

impl SearchResult {
    /// Create a "not found" result.
    #[must_use]
    pub fn not_found() -> Self {
        Self {
            path: Vec::new(),
            total_weight: 0.0,
            hops: 0,
            found: false,
        }
    }
}

/// Searches for paths between entities in the graph.
pub struct PathSearcher;

impl PathSearcher {
    /// Create a new path searcher.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Find the shortest path (fewest hops) between two entities using BFS.
    #[must_use]
    pub fn shortest_path(
        &self,
        store: &GraphStore,
        from: EntityId,
        to: EntityId,
    ) -> SearchResult {
        if from == to {
            return SearchResult {
                path: vec![from],
                total_weight: 0.0,
                hops: 0,
                found: true,
            };
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut parent: HashMap<EntityId, EntityId> = HashMap::new();

        queue.push_back(from);
        visited.insert(from);

        while let Some(current) = queue.pop_front() {
            for neighbor in store.neighbors(current) {
                if visited.insert(neighbor) {
                    parent.insert(neighbor, current);
                    if neighbor == to {
                        let mut path = vec![to];
                        let mut cur = to;
                        while let Some(&p) = parent.get(&cur) {
                            path.push(p);
                            cur = p;
                        }
                        path.reverse();
                        let hops = path.len().saturating_sub(1);
                        return SearchResult {
                            path,
                            total_weight: 0.0,
                            hops,
                            found: true,
                        };
                    }
                    queue.push_back(neighbor);
                }
            }
        }

        SearchResult::not_found()
    }

    /// Find the highest-weight path using Dijkstra-like approach.
    #[must_use]
    pub fn weighted_path(
        &self,
        store: &GraphStore,
        from: EntityId,
        to: EntityId,
    ) -> SearchResult {
        if from == to {
            return SearchResult {
                path: vec![from],
                total_weight: 0.0,
                hops: 0,
                found: true,
            };
        }

        // Use a priority queue approach (simplified)
        let mut best_score: HashMap<EntityId, f32> = HashMap::new();
        let mut parent: HashMap<EntityId, EntityId> = HashMap::new();
        let mut visited = HashSet::new();

        best_score.insert(from, 0.0);
        let mut queue: VecDeque<(EntityId, f32)> = VecDeque::new();
        queue.push_back((from, 0.0));

        while let Some((current, current_score)) = queue.pop_front() {
            if !visited.insert(current) {
                continue;
            }

            if current == to {
                let mut path = vec![to];
                let mut cur = to;
                while let Some(&p) = parent.get(&cur) {
                    path.push(p);
                    cur = p;
                }
                path.reverse();
                let hops = path.len().saturating_sub(1);
                return SearchResult {
                    path,
                    total_weight: current_score,
                    hops,
                    found: true,
                };
            }

            for relation in store.get_outgoing_relations(current) {
                let next_score = current_score + relation.weight;
                let dominated = best_score
                    .get(&relation.target)
                    .map_or(true, |&best| next_score < best);
                if dominated {
                    best_score.insert(relation.target, next_score);
                    parent.insert(relation.target, current);
                    queue.push_back((relation.target, next_score));
                }
            }
        }

        SearchResult::not_found()
    }

    /// Find all paths within a maximum depth.
    #[must_use]
    pub fn all_paths(
        &self,
        store: &GraphStore,
        from: EntityId,
        to: EntityId,
        max_depth: u32,
    ) -> Vec<Vec<EntityId>> {
        let mut results = Vec::new();
        let mut path = vec![from];
        let mut visited = HashSet::new();
        visited.insert(from);
        self.dfs_paths(store, from, to, max_depth, 0, &mut path, &mut visited, &mut results);
        results
    }

    fn dfs_paths(
        &self,
        store: &GraphStore,
        current: EntityId,
        target: EntityId,
        max_depth: u32,
        depth: u32,
        path: &mut Vec<EntityId>,
        visited: &mut HashSet<EntityId>,
        results: &mut Vec<Vec<EntityId>>,
    ) {
        if current == target {
            results.push(path.clone());
            return;
        }
        if depth >= max_depth {
            return;
        }

        for neighbor in store.neighbors(current) {
            if !visited.contains(&neighbor) {
                visited.insert(neighbor);
                path.push(neighbor);
                self.dfs_paths(
                    store, neighbor, target, max_depth, depth + 1, path, visited, results,
                );
                path.pop();
                visited.remove(&neighbor);
            }
        }
    }
}

impl Default for PathSearcher {
    fn default() -> Self {
        Self::new()
    }
}
