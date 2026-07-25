use crate::core::entity::EntityId;
use crate::core::relation::RelationType;
use crate::reasoning::expansion::NeighborExpander;
use crate::reasoning::path::{PathSearcher, SearchResult};
use crate::reasoning::traversal::{GraphTraversal, TraversalConfig, TraversalResult};
use crate::storage::graph_store::GraphStore;

/// API for graph traversal and path queries.
pub struct TraverseApi<'a> {
    store: &'a GraphStore,
    expander: NeighborExpander,
    path_searcher: PathSearcher,
    traversal: GraphTraversal,
}

impl<'a> TraverseApi<'a> {
    /// Create a new traversal API.
    #[must_use]
    pub fn new(store: &'a GraphStore) -> Self {
        Self {
            store,
            expander: NeighborExpander::new(),
            path_searcher: PathSearcher::new(),
            traversal: GraphTraversal::new(),
        }
    }

    /// Expand neighbors of an entity up to a given depth.
    pub fn expand_neighbors(
        &self,
        entity_id: EntityId,
        depth: u32,
    ) -> Vec<EntityId> {
        self.expander.n_hop_neighbors(self.store, entity_id, depth)
    }

    /// Find shortest path between two entities.
    pub fn shortest_path(&self, from: EntityId, to: EntityId) -> SearchResult {
        self.path_searcher.shortest_path(self.store, from, to)
    }

    /// Find weighted path between two entities.
    pub fn weighted_path(&self, from: EntityId, to: EntityId) -> SearchResult {
        self.path_searcher.weighted_path(self.store, from, to)
    }

    /// Find all paths within max depth.
    pub fn all_paths(&self, from: EntityId, to: EntityId, max_depth: u32) -> Vec<Vec<EntityId>> {
        self.path_searcher.all_paths(self.store, from, to, max_depth)
    }

    /// BFS traversal.
    pub fn bfs(&self, start: EntityId, config: TraversalConfig) -> TraversalResult {
        self.traversal.bfs(self.store, start, &config)
    }

    /// DFS traversal.
    pub fn dfs(&self, start: EntityId, config: TraversalConfig) -> TraversalResult {
        self.traversal.dfs(self.store, start, &config)
    }
}
