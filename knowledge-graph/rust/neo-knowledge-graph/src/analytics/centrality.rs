use std::collections::{HashMap, VecDeque};

use crate::core::entity::{Entity, EntityId};
use crate::storage::graph_store::GraphStore;

/// Computes centrality metrics for entities in the graph.
pub struct CentralityAnalyzer;

impl CentralityAnalyzer {
    /// Create a new analyzer.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Compute degree centrality for all entities.
    #[must_use]
    pub fn degree_centrality(&self, store: &GraphStore) -> HashMap<EntityId, f32> {
        let all_ids = store.all_entity_ids();
        let n = all_ids.len() as f32;
        if n <= 1.0 {
            return HashMap::new();
        }

        let max_degree = (n - 1.0);
        all_ids
            .into_iter()
            .map(|id| {
                let degree = store.neighbors(id).len() as f32;
                let centrality = if max_degree > 0.0 { degree / max_degree } else { 0.0 };
                (id, centrality)
            })
            .collect()
    }

    /// Compute betweenness centrality approximation using BFS.
    #[must_use]
    pub fn betweenness_centrality(&self, store: &GraphStore) -> HashMap<EntityId, f32> {
        let all_ids = store.all_entity_ids();
        let mut betweenness: HashMap<EntityId, f32> = HashMap::new();
        for &id in &all_ids {
            betweenness.insert(id, 0.0);
        }

        let n = all_ids.len() as f32;
        if n <= 1.0 {
            return betweenness;
        }

        for &source in all_ids.iter() {
            // BFS from source
            let mut visited: HashMap<EntityId, u32> = HashMap::new();
            let mut queue = VecDeque::new();
            let mut predecessors: HashMap<EntityId, Vec<EntityId>> = HashMap::new();
            let mut sigma: HashMap<EntityId, f32> = HashMap::new();
            let mut distance: HashMap<EntityId, i32> = HashMap::new();

            visited.insert(source, 0);
            sigma.insert(source, 1.0);
            distance.insert(source, 0);
            queue.push_back(source);

            let mut stack = Vec::new();

            while let Some(v) = queue.pop_front() {
                stack.push(v);
                let dist_v = *distance.get(&v).unwrap_or(&0);

                for neighbor in store.neighbors(v) {
                    if !visited.contains_key(&neighbor) {
                        visited.insert(neighbor, 0);
                        distance.insert(neighbor, dist_v + 1);
                        queue.push_back(neighbor);
                    }
                    if *distance.get(&neighbor).unwrap_or(&0) == dist_v + 1 {
                        let sigma_v = *sigma.get(&v).unwrap_or(&0.0);
                        *sigma.entry(neighbor).or_insert(0.0) += sigma_v;
                        predecessors.entry(neighbor).or_default().push(v);
                    }
                }
            }

            // Back-propagation
            let mut delta: HashMap<EntityId, f32> = HashMap::new();
            while let Some(w) = stack.pop() {
                if let Some(preds) = predecessors.get(&w) {
                    for &p in preds {
                        let sigma_p = *sigma.get(&p).unwrap_or(&0.0);
                        let sigma_w = *sigma.get(&w).unwrap_or(&1.0);
                        let delta_w = *delta.get(&w).unwrap_or(&0.0);
                        let contrib = if sigma_w > 0.0 { sigma_p / sigma_w * (1.0 + delta_w) } else { 0.0 };
                        *delta.entry(p).or_insert(0.0) += contrib;
                    }
                }
                if w != source {
                    let delta_w = *delta.get(&w).unwrap_or(&0.0);
                    *betweenness.entry(w).or_insert(0.0) += delta_w;
                }
            }
        }

        // Normalize
        let norm = if n > 2.0 { (n - 1.0) * (n - 2.0) } else { 1.0 };
        if norm > 0.0 {
            for val in betweenness.values_mut() {
                *val /= norm;
            }
        }

        betweenness
    }

    /// Find the most central entities by degree.
    #[must_use]
    pub fn top_by_degree(&self, store: &GraphStore, k: usize) -> Vec<(EntityId, f32, String)> {
        let centrality = self.degree_centrality(store);
        let mut pairs: Vec<(EntityId, f32, String)> = centrality
            .into_iter()
            .filter_map(|(id, score)| {
                store.get_entity(id).map(|e| (id, score, e.label))
            })
            .collect();
        pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        pairs.truncate(k);
        pairs
    }
}

impl Default for CentralityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
