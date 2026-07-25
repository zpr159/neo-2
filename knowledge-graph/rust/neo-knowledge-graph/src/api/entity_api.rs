use crate::core::entity::{Entity, EntityBuilder, EntityId, EntityType};
use crate::error::{KnowledgeError, KnowledgeResult};
use crate::storage::graph_store::GraphStore;

/// API for CRUD operations on entities.
pub struct EntityApi<'a> {
    store: &'a GraphStore,
}

impl<'a> EntityApi<'a> {
    /// Create a new entity API.
    #[must_use]
    pub fn new(store: &'a GraphStore) -> Self {
        Self { store }
    }

    /// Create a new entity.
    pub fn create(
        &self,
        entity_type: EntityType,
        label: impl Into<String>,
    ) -> Entity {
        let entity = Entity::new(entity_type, label.into());
        let id = entity.id;
        self.store.insert_entity(entity.clone());
        entity
    }

    /// Create an entity with builder pattern.
    pub fn create_with(&self, builder: EntityBuilder) -> Entity {
        let entity = builder.build();
        self.store.insert_entity(entity.clone());
        entity
    }

    /// Get an entity by id.
    #[must_use]
    pub fn get(&self, id: EntityId) -> Option<Entity> {
        self.store.get_entity(id)
    }

    /// Update an entity.
    pub fn update(
        &self,
        id: EntityId,
        updater: impl FnOnce(&mut Entity),
    ) -> KnowledgeResult<()> {
        self.store.update_entity(id, updater)
    }

    /// Delete (deactivate) an entity.
    pub fn delete(&self, id: EntityId) -> KnowledgeResult<()> {
        self.store.deactivate_entity(id)
    }

    /// Hard remove an entity and its relations.
    pub fn remove(&self, id: EntityId) -> KnowledgeResult<bool> {
        self.store.remove_entity(id)
    }

    /// Search entities by label.
    #[must_use]
    pub fn search_by_label(&self, label: &str) -> Vec<Entity> {
        self.store.find_entities_by_label(label)
    }

    /// Search entities by type.
    #[must_use]
    pub fn search_by_type(&self, entity_type: &EntityType) -> Vec<Entity> {
        self.store.find_entities_by_type(entity_type)
    }

    /// Count entities.
    #[must_use]
    pub fn count(&self) -> usize {
        self.store.active_entity_count()
    }
}
