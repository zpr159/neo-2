use crate::core::entity::{Entity, EntityId};
use crate::core::relation::{Relation, RelationId};
use crate::error::KnowledgeResult;
use crate::storage::graph_store::GraphStore;

/// Result of a merge operation.
#[derive(Debug, Clone)]
pub struct MergeResult {
    /// The surviving entity id.
    pub surviving_entity_id: EntityId,
    /// Entity ids that were merged (and removed).
    pub merged_entity_ids: Vec<EntityId>,
    /// Number of relations redirected.
    pub relations_redirected: usize,
    /// Number of relations removed (duplicates).
    pub relations_removed: usize,
}

/// Merges duplicate entities and relations in the graph.
pub struct DuplicateMerger;

impl DuplicateMerger {
    /// Create a new merger.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Find and merge duplicate entities based on label similarity.
    pub fn merge_duplicates(
        &self,
        store: &GraphStore,
        similarity_threshold: f32,
    ) -> KnowledgeResult<Vec<MergeResult>> {
        let mut results = Vec::new();
        let all_entities = store.all_entities();
        let mut visited = std::collections::HashSet::new();

        for (i, entity_a) in all_entities.iter().enumerate() {
            if visited.contains(&entity_a.id) || !entity_a.active {
                continue;
            }

            let mut merge_group = vec![entity_a.clone()];

            for entity_b in all_entities.iter().skip(i + 1) {
                if visited.contains(&entity_b.id) || !entity_b.active {
                    continue;
                }

                if entity_a.entity_type == entity_b.entity_type {
                    let similarity = self.label_similarity(&entity_a.label, &entity_b.label);
                    if similarity >= similarity_threshold {
                        merge_group.push(entity_b.clone());
                    }
                }
            }

            if merge_group.len() > 1 {
                let result = self.merge_group(store, &merge_group)?;
                for merged in &merge_group[1..] {
                    visited.insert(merged.id);
                }
                results.push(result);
            }
        }

        Ok(results)
    }

    /// Merge a group of duplicate entities, keeping the one with highest confidence.
    fn merge_group(
        &self,
        store: &GraphStore,
        group: &[Entity],
    ) -> KnowledgeResult<MergeResult> {
        let surviving = group
            .iter()
            .max_by(|a, b| {
                a.confidence
                    .partial_cmp(&b.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .expect("merge group must not be empty");

        let surviving_id = surviving.id;
        let mut merged_ids = Vec::new();
        let mut relations_redirected = 0;
        let mut relations_removed = 0;

        for other in group {
            if other.id == surviving_id {
                continue;
            }

            // Redirect all relations from/to merged entity to surviving entity
            let out_relations: Vec<Relation> = store.get_outgoing_relations(other.id);
            let in_relations: Vec<Relation> = store.get_incoming_relations(other.id);

            for mut rel in out_relations {
                if rel.source == other.id {
                    rel.source = surviving_id;
                    store.upsert_relation(&rel)?;
                    relations_redirected += 1;
                }
            }

            for mut rel in in_relations {
                if rel.target == other.id {
                    rel.target = surviving_id;
                    store.upsert_relation(&rel)?;
                    relations_redirected += 1;
                }
            }

            // Remove the merged entity
            store.deactivate_entity(other.id)?;
            merged_ids.push(other.id);
        }

        Ok(MergeResult {
            surviving_entity_id: surviving_id,
            merged_entity_ids: merged_ids,
            relations_redirected,
            relations_removed,
        })
    }

    /// Simple label similarity using Jaccard on words.
    fn label_similarity(&self, a: &str, b: &str) -> f32 {
        let a_words: std::collections::HashSet<String> = a
            .to_lowercase()
            .split_whitespace()
            .map(String::from)
            .collect();
        let b_words: std::collections::HashSet<String> = b
            .to_lowercase()
            .split_whitespace()
            .map(String::from)
            .collect();

        if a_words.is_empty() || b_words.is_empty() {
            return 0.0;
        }

        let intersection = a_words.intersection(&b_words).count() as f32;
        let union = a_words.union(&b_words).count() as f32;

        if union > 0.0 {
            intersection / union
        } else {
            0.0
        }
    }
}

impl Default for DuplicateMerger {
    fn default() -> Self {
        Self::new()
    }
}
