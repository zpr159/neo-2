use crate::core::entity::EntityId;
use crate::core::relation::{Relation, RelationId, RelationType};
use crate::error::{KnowledgeError, KnowledgeResult};
use crate::storage::graph_store::GraphStore;

/// API for CRUD operations on relations.
pub struct RelationApi<'a> {
    store: &'a GraphStore,
}

impl<'a> RelationApi<'a> {
    /// Create a new relation API.
    #[must_use]
    pub fn new(store: &'a GraphStore) -> Self {
        Self { store }
    }

    /// Create a new relation.
    pub fn create(
        &self,
        relation_type: RelationType,
        source: EntityId,
        target: EntityId,
        label: impl Into<String>,
    ) -> KnowledgeResult<Relation> {
        if self.store.get_entity(source).is_none() {
            return Err(KnowledgeError::EntityNotFound(source.to_string()));
        }
        if self.store.get_entity(target).is_none() {
            return Err(KnowledgeError::EntityNotFound(target.to_string()));
        }

        let relation = Relation::new(relation_type, source, target, label.into());
        self.store.insert_relation(relation.clone());
        Ok(relation)
    }

    /// Get a relation by id.
    #[must_use]
    pub fn get(&self, id: RelationId) -> Option<Relation> {
        self.store.get_relation(id)
    }

    /// Remove a relation.
    pub fn remove(&self, id: RelationId) -> KnowledgeResult<bool> {
        self.store.remove_relation(id)
    }

    /// Get outgoing relations for an entity.
    #[must_use]
    pub fn outgoing(&self, entity_id: EntityId) -> Vec<Relation> {
        self.store.get_outgoing_relations(entity_id)
    }

    /// Get incoming relations for an entity.
    #[must_use]
    pub fn incoming(&self, entity_id: EntityId) -> Vec<Relation> {
        self.store.get_incoming_relations(entity_id)
    }

    /// Get all relations of a type.
    #[must_use]
    pub fn by_type(&self, relation_type: &RelationType) -> Vec<Relation> {
        self.store.find_relations_by_type(relation_type)
    }

    /// Count relations.
    #[must_use]
    pub fn count(&self) -> usize {
        self.store.active_relation_count()
    }
}
