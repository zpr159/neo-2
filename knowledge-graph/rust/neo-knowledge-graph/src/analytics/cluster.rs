use std::collections::{HashMap, HashSet};

use crate::core::entity::EntityId;
use crate::storage::graph_store::GraphStore;

/// Cluster of entities with high inter-connectivity.
#[derive(Debug, Clone)]
pub struct Cluster {
    pub center: EntityId,
    pub members: Vec<EntityId>,
    pub internal_density: f32,
}

/// Analyzes clusters in the graph.
pub struct ClusterAnalyzer;

impl ClusterAnalyzer {
    /// Create a new analyzer.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Find clusters using a simple k-core decomposition.
    #[must_use]
    pub fn find_k_cores(&self, store: &GraphStore, min_degree: usize) -> Vec<Vec<EntityId>> {
        let all_ids: Vec<EntityId> = store.all_entity_ids();
        let mut degree: HashMap<EntityId, usize> = HashMap::new();
        let mut alive: HashSet<EntityId> = all_ids.iter().copied().collect();

        for &id in &all_ids {
            degree.insert(id, store.neighbors(id).len());
        }

        let mut changed = true;
        while changed {
            changed = false;
            let to_remove: Vec<EntityId> = alive
                .iter()
                .filter(|&&id| *degree.get(&id).unwrap_or(&0) < min_degree)
                .copied()
                .collect();
            for id in to_remove {
                alive.remove(&id);
                for neighbor in store.neighbors(id) {
                    if let Some(d) = degree.get_mut(&neighbor) {
                        *d = d.saturating_sub(1);
                    }
                }
                changed = true;
            }
        }

        if alive.is_empty() {
            return Vec::new();
        }

        // Extract connected components from alive nodes
        let mut visited = HashSet::new();
        let mut cores = Vec::new();

        for &start in &alive {
            if visited.contains(&start) {
                continue;
            }
            let mut component = Vec::new();
            let mut queue = std::collections::VecDeque::new();
            queue.push_back(start);
            visited.insert(start);

            while let Some(current) = queue.pop_front() {
                component.push(current);
                for neighbor in store.neighbors(current) {
                    if alive.contains(&neighbor) && visited.insert(neighbor) {
                        queue.push_back(neighbor);
                    }
                }
            }
            cores.push(component);
        }

        cores
    }
}

impl Default for ClusterAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
