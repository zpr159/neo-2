use std::collections::{HashMap, HashSet, VecDeque};

use crate::core::entity::EntityId;
use crate::storage::graph_store::GraphStore;

/// A detected community (set of entity ids).
#[derive(Debug, Clone)]
pub struct Community {
    pub members: Vec<EntityId>,
    pub density: f32,
}

/// Detects communities in the graph using label propagation.
pub struct CommunityDetector;

impl CommunityDetector {
    /// Create a new detector.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Detect communities using connected components (simple BFS).
    #[must_use]
    pub fn detect(&self, store: &GraphStore) -> Vec<Community> {
        let all_ids: HashSet<EntityId> = store.all_entity_ids().into_iter().collect();
        let mut visited = HashSet::new();
        let mut communities = Vec::new();

        for &start_id in &all_ids {
            if visited.contains(&start_id) {
                continue;
            }

            let mut members = Vec::new();
            let mut queue = VecDeque::new();
            queue.push_back(start_id);
            visited.insert(start_id);

            while let Some(current) = queue.pop_front() {
                members.push(current);
                for neighbor in store.neighbors(current) {
                    if visited.insert(neighbor) {
                        queue.push_back(neighbor);
                    }
                }
            }

            let internal_edges = self.count_internal_edges(store, &members);
            let max_edges = if members.len() > 1 {
                members.len() * (members.len() - 1) / 2
            } else {
                1
            };
            let density = internal_edges as f32 / max_edges as f32;

            communities.push(Community { members, density });
        }

        communities.sort_by(|a, b| b.members.len().cmp(&a.members.len()));
        communities
    }

    fn count_internal_edges(&self, store: &GraphStore, members: &[EntityId]) -> usize {
        let member_set: HashSet<EntityId> = members.iter().copied().collect();
        let mut count = 0;
        for &id in members {
            for neighbor in store.neighbors(id) {
                if member_set.contains(&neighbor) && id.0 < neighbor.0 {
                    count += 1;
                }
            }
        }
        count
    }
}

impl Default for CommunityDetector {
    fn default() -> Self {
        Self::new()
    }
}
