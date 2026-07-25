use crate::core::entity::{Entity, EntityId, EntityType};
use crate::storage::graph_store::GraphStore;
use crate::world_model::{person::PersonEntity, place::PlaceEntity, organization::OrganizationEntity, task::TaskEntity, goal::GoalEntity, skill::SkillEntity, project::ProjectEntity, event_model::EventEntity};

/// Manages world model entities and provides typed access.
pub struct WorldModelManager<'a> {
    store: &'a GraphStore,
}

impl<'a> WorldModelManager<'a> {
    /// Create a new world model manager.
    #[must_use]
    pub fn new(store: &'a GraphStore) -> Self {
        Self { store }
    }

    /// Get all people in the world model.
    #[must_use]
    pub fn people(&self) -> Vec<PersonEntity> {
        self.store
            .find_entities_by_type(&EntityType::Person)
            .iter()
            .map(PersonEntity::from_entity)
            .collect()
    }

    /// Get all places.
    #[must_use]
    pub fn places(&self) -> Vec<PlaceEntity> {
        self.store
            .find_entities_by_type(&EntityType::Place)
            .iter()
            .map(PlaceEntity::from_entity)
            .collect()
    }

    /// Get all organizations.
    #[must_use]
    pub fn organizations(&self) -> Vec<OrganizationEntity> {
        self.store
            .find_entities_by_type(&EntityType::Organization)
            .iter()
            .map(OrganizationEntity::from_entity)
            .collect()
    }

    /// Get all tasks.
    #[must_use]
    pub fn tasks(&self) -> Vec<TaskEntity> {
        self.store
            .find_entities_by_type(&EntityType::Task)
            .iter()
            .map(TaskEntity::from_entity)
            .collect()
    }

    /// Get all goals.
    #[must_use]
    pub fn goals(&self) -> Vec<GoalEntity> {
        self.store
            .find_entities_by_type(&EntityType::Goal)
            .iter()
            .map(GoalEntity::from_entity)
            .collect()
    }

    /// Get all skills.
    #[must_use]
    pub fn skills(&self) -> Vec<SkillEntity> {
        self.store
            .find_entities_by_type(&EntityType::Skill)
            .iter()
            .map(SkillEntity::from_entity)
            .collect()
    }

    /// Get all projects.
    #[must_use]
    pub fn projects(&self) -> Vec<ProjectEntity> {
        self.store
            .find_entities_by_type(&EntityType::Project)
            .iter()
            .map(ProjectEntity::from_entity)
            .collect()
    }

    /// Get all events.
    #[must_use]
    pub fn events(&self) -> Vec<EventEntity> {
        self.store
            .find_entities_by_type(&EntityType::Event)
            .iter()
            .map(EventEntity::from_entity)
            .collect()
    }

    /// Get a person by id.
    #[must_use]
    pub fn get_person(&self, id: EntityId) -> Option<PersonEntity> {
        self.store.get_entity(id).and_then(|e| {
            if e.entity_type == EntityType::Person {
                Some(PersonEntity::from_entity(&e))
            } else {
                None
            }
        })
    }

    /// Get a task by id.
    #[must_use]
    pub fn get_task(&self, id: EntityId) -> Option<TaskEntity> {
        self.store.get_entity(id).and_then(|e| {
            if e.entity_type == EntityType::Task {
                Some(TaskEntity::from_entity(&e))
            } else {
                None
            }
        })
    }

    /// Get a goal by id.
    #[must_use]
    pub fn get_goal(&self, id: EntityId) -> Option<GoalEntity> {
        self.store.get_entity(id).and_then(|e| {
            if e.entity_type == EntityType::Goal {
                Some(GoalEntity::from_entity(&e))
            } else {
                None
            }
        })
    }

    /// Count entities by type.
    #[must_use]
    pub fn count_by_type(&self) -> std::collections::HashMap<String, usize> {
        let mut counts = std::collections::HashMap::new();
        for entity in self.store.all_entities() {
            if entity.active {
                *counts.entry(entity.entity_type.to_string()).or_insert(0) += 1;
            }
        }
        counts
    }
}
