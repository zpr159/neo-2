use crate::core::entity::{Entity, EntityId};
use crate::core::relation::Relation;
use crate::error::KnowledgeResult;
use crate::storage::graph_store::GraphStore;

/// Result of a merge operation.
#[derive(Debug, Clone)]
pub struct MergeOutcome {
    /// The surviving entity.
    pub surviving: EntityId,
    /// Entities that were merged and removed.
    pub merged: Vec<EntityId>,
    /// Description of what happened.
    pub description: String,
}

/// Merges similar concepts into unified concepts.
pub struct ConceptMerger;

impl ConceptMerger {
    /// Create a new merger.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Merge two specific entities, keeping the one with higher confidence.
    #[must_use]
    pub fn merge_pair(&self, a: &Entity, b: &Entity) -> MergeOutcome {
        let (survivor, absorbed) = if a.confidence >= b.confidence {
            (a, b)
        } else {
            (b, a)
        };

        MergeOutcome {
            surviving: survivor.id,
            merged: vec![absorbed.id],
            description: format!(
                "Merged '{}' into '{}'",
                absorbed.label, survivor.label
            ),
        }
    }

    /// Find and merge duplicate entities in the graph.
    pub fn merge_duplicates(
        &self,
        store: &GraphStore,
        threshold: f32,
    ) -> KnowledgeResult<Vec<MergeOutcome>> {
        let entities = store.all_entities();
        let mut outcomes = Vec::new();
        let mut merged_ids = std::collections::HashSet::new();

        for (i, entity_a) in entities.iter().enumerate() {
            if merged_ids.contains(&entity_a.id) || !entity_a.active {
                continue;
            }

            for entity_b in entities.iter().skip(i + 1) {
                if merged_ids.contains(&entity_b.id) || !entity_b.active {
                    continue;
                }

                if entity_a.entity_type == entity_b.entity_type {
                    let sim = label_similarity(&entity_a.label, &entity_b.label);
                    if sim >= threshold {
                        let outcome = self.merge_pair(entity_a, entity_b);
                        merged_ids.insert(outcome.merged[0]);
                        outcomes.push(outcome);
                    }
                }
            }
        }

        Ok(outcomes)
    }
}

fn label_similarity(a: &str, b: &str) -> f32 {
    let words_a: std::collections::HashSet<String> = a
        .to_lowercase()
        .split_whitespace()
        .map(String::from)
        .collect();
    let words_b: std::collections::HashSet<String> = b
        .to_lowercase()
        .split_whitespace()
        .map(String::from)
        .collect();

    if words_a.is_empty() || words_b.is_empty() {
        return 0.0;
    }

    let intersection = words_a.intersection(&words_b).count() as f32;
    let union = words_a.union(&words_b).count() as f32;
    if union > 0.0 { intersection / union } else { 0.0 }
}

impl Default for ConceptMerger {
    fn default() -> Self {
        Self::new()
    }
}
