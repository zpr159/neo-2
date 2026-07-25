use crate::storage::graph_store::GraphStore;

/// Statistics about graph density.
#[derive(Debug, Clone)]
pub struct DensityStats {
    /// Number of entities.
    pub entity_count: usize,
    /// Number of relations.
    pub relation_count: usize,
    /// Maximum possible relations.
    pub max_possible_relations: usize,
    /// Actual density (0.0 - 1.0).
    pub density: f32,
    /// Average degree.
    pub avg_degree: f32,
    /// Maximum degree.
    pub max_degree: usize,
    /// Minimum degree (of non-isolated nodes).
    pub min_degree: usize,
}

/// Analyzes the density of the knowledge graph.
pub struct DensityAnalyzer;

impl DensityAnalyzer {
    /// Create a new analyzer.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Compute density statistics.
    #[must_use]
    pub fn analyze(&self, store: &GraphStore) -> DensityStats {
        let entity_count = store.active_entity_count();
        let relation_count = store.active_relation_count();

        let max_possible = if entity_count > 1 {
            entity_count * (entity_count - 1)
        } else {
            0
        };

        let density = if max_possible > 0 {
            relation_count as f32 / max_possible as f32
        } else {
            0.0
        };

        let all_ids = store.all_entity_ids();
        let mut degrees: Vec<usize> = all_ids
            .iter()
            .map(|&id| store.neighbors(id).len())
            .collect();

        degrees.sort_unstable();

        let avg_degree = if !degrees.is_empty() {
            degrees.iter().sum::<usize>() as f32 / degrees.len() as f32
        } else {
            0.0
        };

        let max_degree = degrees.last().copied().unwrap_or(0);
        let min_degree = degrees.first().copied().unwrap_or(0);

        DensityStats {
            entity_count,
            relation_count,
            max_possible_relations: max_possible,
            density,
            avg_degree,
            max_degree,
            min_degree,
        }
    }
}

impl Default for DensityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
