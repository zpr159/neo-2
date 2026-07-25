use std::collections::{HashSet, VecDeque};

use crate::core::entity::EntityId;
use crate::storage::graph_store::GraphStore;

/// Analysis of connected components.
#[derive(Debug, Clone)]
pub struct ComponentInfo {
    /// Component size (number of entities).
    pub size: usize,
    /// Whether this is the largest component.
    pub is_largest: bool,
    /// Entity ids in this component.
    pub members: Vec<EntityId>,
}

/// Analyzes connected components in the graph.
pub struct ConnectedComponentAnalyzer;

impl ConnectedComponentAnalyzer {
    /// Create a new analyzer.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Find all connected components.
    #[must_use]
    pub fn find_components(&self, store: &GraphStore) -> Vec<ComponentInfo> {
        let all_ids: HashSet<EntityId> = store.all_entity_ids().into_iter().collect();
        let mut visited = HashSet::new();
        let mut components = Vec::new();

        for &start in &all_ids {
            if visited.contains(&start) {
                continue;
            }
            let mut members = Vec::new();
            let mut queue = VecDeque::new();
            queue.push_back(start);
            visited.insert(start);

            while let Some(current) = queue.pop_front() {
                members.push(current);
                for neighbor in store.neighbors(current) {
                    if all_ids.contains(&neighbor) && visited.insert(neighbor) {
                        queue.push_back(neighbor);
                    }
                }
            }
            components.push(members);
        }

        let max_size = components.iter().map(|c| c.len()).max().unwrap_or(0);

        components
            .into_iter()
            .enumerate()
            .map(|(i, members)| ComponentInfo {
                size: members.len(),
                is_largest: members.len() == max_size,
                members,
            })
            .collect()
    }

    /// Count the number of connected components.
    #[must_use]
    pub fn count(&self, store: &GraphStore) -> usize {
        self.find_components(store).len()
    }

    /// Get the size of the largest connected component.
    #[must_use]
    pub fn largest_component_size(&self, store: &GraphStore) -> usize {
        self.find_components(store)
            .iter()
            .map(|c| c.size)
            .max()
            .unwrap_or(0)
    }
}

impl Default for ConnectedComponentAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
