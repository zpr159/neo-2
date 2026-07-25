use serde::{Deserialize, Serialize};

use crate::validation::contradiction::{ContradictionDetector, DetectedContradiction};
use crate::core::entity::Entity;

/// Strategy for resolving conflicts.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResolutionStrategy {
    /// Keep the entity with highest confidence.
    HighestConfidence,
    /// Keep the most recently updated entity.
    MostRecent,
    /// Merge the two entities.
    Merge,
    /// Keep both and mark as conflicting.
    KeepBoth,
    /// Remove both.
    RemoveBoth,
}

/// Result of a conflict resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionResult {
    /// The strategy used.
    pub strategy: ResolutionStrategy,
    /// Entity ids that survived.
    pub kept: Vec<String>,
    /// Entity ids that were removed.
    pub removed: Vec<String>,
    /// Description of what was done.
    pub description: String,
}

/// Resolves contradictions in the knowledge graph.
pub struct ConflictResolver {
    default_strategy: ResolutionStrategy,
}

impl ConflictResolver {
    /// Create a new resolver with a default strategy.
    #[must_use]
    pub fn new(strategy: ResolutionStrategy) -> Self {
        Self {
            default_strategy: strategy,
        }
    }

    /// Resolve a contradiction between two entities.
    #[must_use]
    pub fn resolve_contradiction(
        &self,
        entity_a: &Entity,
        entity_b: &Entity,
        strategy: Option<&ResolutionStrategy>,
    ) -> ResolutionResult {
        let strategy = strategy.unwrap_or(&self.default_strategy);

        match strategy {
            ResolutionStrategy::HighestConfidence => {
                if entity_a.confidence >= entity_b.confidence {
                    ResolutionResult {
                        strategy: ResolutionStrategy::HighestConfidence,
                        kept: vec![entity_a.id.to_string()],
                        removed: vec![entity_b.id.to_string()],
                        description: format!(
                            "Kept '{}' (confidence {:.2}) over '{}' (confidence {:.2})",
                            entity_a.label, entity_a.confidence, entity_b.label, entity_b.confidence
                        ),
                    }
                } else {
                    ResolutionResult {
                        strategy: ResolutionStrategy::HighestConfidence,
                        kept: vec![entity_b.id.to_string()],
                        removed: vec![entity_a.id.to_string()],
                        description: format!(
                            "Kept '{}' (confidence {:.2}) over '{}' (confidence {:.2})",
                            entity_b.label, entity_b.confidence, entity_a.label, entity_a.confidence
                        ),
                    }
                }
            }
            ResolutionStrategy::MostRecent => {
                if entity_a.updated_at >= entity_b.updated_at {
                    ResolutionResult {
                        strategy: ResolutionStrategy::MostRecent,
                        kept: vec![entity_a.id.to_string()],
                        removed: vec![entity_b.id.to_string()],
                        description: format!(
                            "Kept '{}' (updated {}) over '{}' (updated {})",
                            entity_a.label, entity_a.updated_at, entity_b.label, entity_b.updated_at
                        ),
                    }
                } else {
                    ResolutionResult {
                        strategy: ResolutionStrategy::MostRecent,
                        kept: vec![entity_b.id.to_string()],
                        removed: vec![entity_a.id.to_string()],
                        description: format!(
                            "Kept '{}' (updated {}) over '{}' (updated {})",
                            entity_b.label, entity_b.updated_at, entity_a.label, entity_a.updated_at
                        ),
                    }
                }
            }
            ResolutionStrategy::KeepBoth => ResolutionResult {
                strategy: ResolutionStrategy::KeepBoth,
                kept: vec![entity_a.id.to_string(), entity_b.id.to_string()],
                removed: Vec::new(),
                description: format!(
                    "Kept both '{}' and '{}' despite contradiction",
                    entity_a.label, entity_b.label
                ),
            },
            ResolutionStrategy::Merge => ResolutionResult {
                strategy: ResolutionStrategy::Merge,
                kept: vec![entity_a.id.to_string()],
                removed: vec![entity_b.id.to_string()],
                description: format!(
                    "Merged '{}' and '{}' into '{}'",
                    entity_a.label, entity_b.label, entity_a.label
                ),
            },
            ResolutionStrategy::RemoveBoth => ResolutionResult {
                strategy: ResolutionStrategy::RemoveBoth,
                kept: Vec::new(),
                removed: vec![entity_a.id.to_string(), entity_b.id.to_string()],
                description: format!(
                    "Removed both '{}' and '{}' due to contradiction",
                    entity_a.label, entity_b.label
                ),
            },
        }
    }

    /// Resolve all detected contradictions.
    #[must_use]
    pub fn resolve_all(
        &self,
        contradictions: &[DetectedContradiction],
        entities: &[Entity],
        strategy: Option<&ResolutionStrategy>,
    ) -> Vec<ResolutionResult> {
        let mut results = Vec::new();
        let entity_map: std::collections::HashMap<String, &Entity> = entities
            .iter()
            .map(|e| (e.id.to_string(), e))
            .collect();

        for contradiction in contradictions {
            if let (Some(a), Some(b)) = (
                entity_map.get(&contradiction.first_id),
                entity_map.get(&contradiction.second_id),
            ) {
                results.push(self.resolve_contradiction(a, b, strategy));
            }
        }

        results
    }
}

impl Default for ConflictResolver {
    fn default() -> Self {
        Self::new(ResolutionStrategy::HighestConfidence)
    }
}
