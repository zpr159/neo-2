use crate::core::entity::Entity;
use crate::core::relation::Relation;
use crate::storage::graph_store::GraphStore;

/// Enriches context with knowledge graph information.
pub struct ContextEnricher;

impl ContextEnricher {
    /// Create a new enricher.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Enrich an entity with its neighbors' information.
    #[must_use]
    pub fn enrich_entity(&self, store: &GraphStore, entity: &Entity) -> EnrichedContext {
        let outgoing = store.get_outgoing_relations(entity.id);
        let incoming = store.get_incoming_relations(entity.id);

        let mut related_facts = Vec::new();

        for rel in &outgoing {
            if let Some(target) = store.get_entity(rel.target) {
                related_facts.push(format!(
                    "{} --[{}]--> {}",
                    entity.label, rel.relation_type, target.label
                ));
            }
        }

        for rel in &incoming {
            if let Some(source) = store.get_entity(rel.source) {
                related_facts.push(format!(
                    "{} --[{}]--> {}",
                    source.label, rel.relation_type, entity.label
                ));
            }
        }

        EnrichedContext {
            entity: entity.clone(),
            related_facts,
            properties: entity.properties.clone(),
            confidence: entity.confidence,
        }
    }

    /// Build context around a query from the graph.
    #[must_use]
    pub fn build_query_context(
        &self,
        store: &GraphStore,
        query: &str,
        max_entities: usize,
    ) -> Vec<EnrichedContext> {
        let all_entities = store.all_entities();
        let matching: Vec<&Entity> = all_entities
            .iter()
            .filter(|e| e.active && e.matches_query(query))
            .take(max_entities)
            .collect();

        matching
            .into_iter()
            .map(|e| self.enrich_entity(store, e))
            .collect()
    }
}

/// Context enriched with knowledge graph data.
#[derive(Debug, Clone)]
pub struct EnrichedContext {
    /// The primary entity.
    pub entity: Entity,
    /// Related facts derived from graph edges.
    pub related_facts: Vec<String>,
    /// Entity properties.
    pub properties: std::collections::HashMap<String, serde_json::Value>,
    /// Confidence score.
    pub confidence: f32,
}

impl Default for ContextEnricher {
    fn default() -> Self {
        Self::new()
    }
}
