use crate::core::entity::Entity;
use crate::storage::graph_store::GraphStore;

/// Retrieves relevant facts from the knowledge graph for a given query.
pub struct FactRetriever;

impl FactRetriever {
    /// Create a new retriever.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Retrieve facts relevant to a query string.
    #[must_use]
    pub fn retrieve(&self, store: &GraphStore, query: &str, max_facts: usize) -> Vec<RetrievedFact> {
        let all_entities = store.all_entities();
        let all_relations = store.all_relations();

        let mut facts: Vec<RetrievedFact> = Vec::new();

        // Entity-based facts
        for entity in &all_entities {
            if !entity.active {
                continue;
            }
            if entity.matches_query(query) {
                let fact_text = format!(
                    "{} is a {}",
                    entity.label, entity.entity_type
                );
                facts.push(RetrievedFact {
                    text: fact_text,
                    confidence: entity.confidence,
                    source_id: entity.id.to_string(),
                    fact_type: FactType::EntityAttribute,
                });

                if !entity.description.is_empty() {
                    facts.push(RetrievedFact {
                        text: format!("{}. Description: {}", entity.label, entity.description),
                        confidence: entity.confidence * 0.9,
                        source_id: entity.id.to_string(),
                        fact_type: FactType::EntityDescription,
                    });
                }
            }
        }

        // Relation-based facts
        for relation in &all_relations {
            if !relation.active {
                continue;
            }
            if let (Some(source), Some(target)) = (
                store.get_entity(relation.source),
                store.get_entity(relation.target),
            ) {
                if source.matches_query(query) || target.matches_query(query) {
                    facts.push(RetrievedFact {
                        text: format!(
                            "{} --[{} (w={:.2})]--> {}",
                            source.label, relation.relation_type, relation.weight, target.label
                        ),
                        confidence: relation.confidence,
                        source_id: relation.id.to_string(),
                        fact_type: FactType::Relation,
                    });
                }
            }
        }

        // Sort by confidence and limit
        facts.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        facts.truncate(max_facts);
        facts
    }
}

/// A retrieved fact.
#[derive(Debug, Clone)]
pub struct RetrievedFact {
    /// The fact text.
    pub text: String,
    /// Confidence.
    pub confidence: f32,
    /// Source entity/relation id.
    pub source_id: String,
    /// Type of fact.
    pub fact_type: FactType,
}

/// Type of fact.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FactType {
    EntityAttribute,
    EntityDescription,
    Relation,
    Property,
}

impl Default for FactRetriever {
    fn default() -> Self {
        Self::new()
    }
}
