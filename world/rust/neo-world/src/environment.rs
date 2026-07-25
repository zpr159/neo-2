use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::{Confidence, EnvironmentId, EnvironmentType, EntityId, AttributeValue};

/// An environment in the world model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub id: EnvironmentId,
    pub name: String,
    pub environment_type: EnvironmentType,
    pub description: String,
    pub entities: Vec<EntityId>,
    pub properties: HashMap<String, AttributeValue>,
    pub parent_id: Option<EnvironmentId>,
    pub confidence: Confidence,
    pub is_active: bool,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Environment {
    pub fn new(name: impl Into<String>, environment_type: EnvironmentType) -> Self {
        let now = Utc::now();
        Self {
            id: EnvironmentId::random(),
            name: name.into(),
            environment_type,
            description: String::new(),
            entities: Vec::new(),
            properties: HashMap::new(),
            parent_id: None,
            confidence: Confidence::MEDIUM,
            is_active: true,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add_entity(&mut self, entity_id: EntityId) {
        if !self.entities.contains(&entity_id) {
            self.entities.push(entity_id);
            self.updated_at = Utc::now();
        }
    }

    pub fn remove_entity(&mut self, entity_id: &EntityId) {
        self.entities.retain(|e| e != entity_id);
        self.updated_at = Utc::now();
    }
}

/// Manages environments.
pub struct EnvironmentManager {
    environments: dashmap::DashMap<EnvironmentId, Environment>,
}

impl EnvironmentManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            environments: dashmap::DashMap::new(),
        }
    }

    pub fn add(&self, env: Environment) -> EnvironmentId {
        let id = env.id.clone();
        self.environments.insert(id.clone(), env);
        id
    }

    pub fn get(&self, id: &EnvironmentId) -> Option<Environment> {
        self.environments.get(id).map(|e| e.value().clone())
    }

    pub fn find_by_name(&self, name: &str) -> Vec<Environment> {
        let lower = name.to_lowercase();
        self.environments
            .iter()
            .filter(|e| e.value().name.to_lowercase().contains(&lower))
            .map(|e| e.value().clone())
            .collect()
    }

    pub fn by_type(&self, env_type: &EnvironmentType) -> Vec<Environment> {
        self.environments
            .iter()
            .filter(|e| &e.value().environment_type == env_type)
            .map(|e| e.value().clone())
            .collect()
    }

    pub fn active(&self) -> Vec<Environment> {
        self.environments
            .iter()
            .filter(|e| e.value().is_active)
            .map(|e| e.value().clone())
            .collect()
    }

    pub fn children_of(&self, parent_id: &EnvironmentId) -> Vec<Environment> {
        self.environments
            .iter()
            .filter(|e| e.value().parent_id.as_ref() == Some(parent_id))
            .map(|e| e.value().clone())
            .collect()
    }

    pub fn remove(&self, id: &EnvironmentId) -> bool {
        self.environments.remove(id).is_some()
    }

    pub fn count(&self) -> usize {
        self.environments.len()
    }
}

impl Default for EnvironmentManager {
    fn default() -> Self {
        Self::new()
    }
}
