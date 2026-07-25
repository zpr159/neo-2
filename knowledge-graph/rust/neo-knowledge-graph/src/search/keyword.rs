use crate::core::entity::Entity;

/// Keyword-based search over entity labels, descriptions, and properties.
pub struct KeywordSearch;

impl KeywordSearch {
    /// Create a new keyword search.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Check if an entity matches a query.
    #[must_use]
    pub fn matches(&self, entity: &Entity, query: &str) -> bool {
        let q = query.to_lowercase();
        entity.label.to_lowercase().contains(&q)
            || entity.description.to_lowercase().contains(&q)
            || entity.aliases.iter().any(|a| a.to_lowercase().contains(&q))
            || entity
                .properties
                .values()
                .any(|v| v.to_string().to_lowercase().contains(&q))
    }

    /// Score how well an entity matches a query (0.0 - 1.0).
    #[must_use]
    pub fn score(&self, entity: &Entity, query: &str) -> f32 {
        let q = query.to_lowercase();
        let mut score = 0.0;

        if entity.label.to_lowercase().contains(&q) {
            score += 0.5;
        }
        if entity.description.to_lowercase().contains(&q) {
            score += 0.2;
        }
        if entity.aliases.iter().any(|a| a.to_lowercase().contains(&q)) {
            score += 0.2;
        }
        if entity
            .properties
            .values()
            .any(|v| v.to_string().to_lowercase().contains(&q))
        {
            score += 0.1;
        }

        let final_score: f32 = score;
        final_score.min(1.0)
    }

    /// Search a list of entities by keyword.
    #[must_use]
    pub fn search_all(&self, entities: &[Entity], query: &str) -> Vec<(Entity, f32)> {
        let mut results: Vec<(Entity, f32)> = entities
            .iter()
            .filter(|e| e.active && self.matches(e, query))
            .map(|e| (e.clone(), self.score(e, query)))
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }
}

impl Default for KeywordSearch {
    fn default() -> Self {
        Self::new()
    }
}
