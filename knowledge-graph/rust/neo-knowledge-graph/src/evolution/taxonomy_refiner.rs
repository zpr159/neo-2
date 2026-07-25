use crate::core::entity::Entity;
use crate::storage::graph_store::GraphStore;
use crate::ontology::taxonomy::TaxonomyTree;

/// Refines the taxonomy tree based on observed entity relationships.
pub struct TaxonomyRefiner;

impl TaxonomyRefiner {
    /// Create a new refiner.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Suggest taxonomy refinements based on IsA relationships in the graph.
    #[must_use]
    pub fn suggest_refinements(
        &self,
        store: &GraphStore,
        taxonomy: &TaxonomyTree,
    ) -> Vec<TaxonomySuggestion> {
        let mut suggestions = Vec::new();

        let isa_relations = store.find_relations_by_type(&crate::core::relation::RelationType::IsA);

        for relation in &isa_relations {
            if let (Some(child), Some(parent)) = (
                store.get_entity(relation.source),
                store.get_entity(relation.target),
            ) {
                let child_type = child.entity_type.to_string();
                let parent_type = parent.entity_type.to_string();

                // Check if the taxonomy already knows about this relationship
                if !taxonomy.is_ancestor(&parent_type, &child_type) {
                    suggestions.push(TaxonomySuggestion {
                        child_type: child_type.clone(),
                        parent_type: parent_type.clone(),
                        confidence: relation.confidence,
                        reason: format!(
                            "IsA relation found between '{}' and '{}' not in taxonomy",
                            child.label, parent.label
                        ),
                    });
                }
            }
        }

        suggestions
    }
}

impl Default for TaxonomyRefiner {
    fn default() -> Self {
        Self::new()
    }
}

/// A suggested taxonomy change.
#[derive(Debug, Clone)]
pub struct TaxonomySuggestion {
    /// Child type to add.
    pub child_type: String,
    /// Parent type.
    pub parent_type: String,
    /// Confidence in the suggestion.
    pub confidence: f32,
    /// Reason for the suggestion.
    pub reason: String,
}
