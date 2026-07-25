use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::EntityType;

/// A registered entity type with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityTypeEntry {
    pub entity_type: EntityType,
    pub label: String,
    pub description: String,
    pub parent_type: Option<EntityType>,
    pub allowed_attributes: Vec<String>,
    pub required_attributes: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Extensible registry of entity types.
pub struct EntityTypeRegistry {
    types: HashMap<EntityType, EntityTypeEntry>,
}

impl EntityTypeRegistry {
    /// Create a registry pre-populated with all built-in types.
    #[must_use]
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register_defaults();
        registry
    }

    #[must_use]
    pub fn new() -> Self {
        Self {
            types: HashMap::new(),
        }
    }

    /// Register all built-in entity types.
    pub fn register_defaults(&mut self) {
        let builtins = vec![
            EntityType::Human,
            EntityType::User,
            EntityType::Agent,
            EntityType::Tool,
            EntityType::Capability,
            EntityType::Workflow,
            EntityType::Task,
            EntityType::Location,
            EntityType::Object,
            EntityType::File,
            EntityType::Document,
            EntityType::Image,
            EntityType::Audio,
            EntityType::Video,
            EntityType::Conversation,
            EntityType::Goal,
            EntityType::Memory,
            EntityType::Knowledge,
            EntityType::Environment,
            EntityType::Vehicle,
            EntityType::Device,
            EntityType::Container,
            EntityType::Sensor,
            EntityType::Service,
            EntityType::Organization,
            EntityType::Concept,
            EntityType::System,
        ];

        for et in builtins {
            let entry = EntityTypeEntry {
                entity_type: et.clone(),
                label: et.label().to_string(),
                description: format!("Built-in entity type: {}", et.label()),
                parent_type: None,
                allowed_attributes: Vec::new(),
                required_attributes: Vec::new(),
                metadata: HashMap::new(),
            };
            self.types.insert(et, entry);
        }
    }

    /// Register a new entity type.
    pub fn register(&mut self, entry: EntityTypeEntry) {
        self.types.insert(entry.entity_type.clone(), entry);
    }

    /// Look up an entity type.
    pub fn get(&self, entity_type: &EntityType) -> Option<&EntityTypeEntry> {
        self.types.get(entity_type)
    }

    /// Check if an entity type is registered.
    pub fn is_registered(&self, entity_type: &EntityType) -> bool {
        self.types.contains_key(entity_type)
    }

    /// Get all registered types.
    pub fn all_types(&self) -> Vec<&EntityTypeEntry> {
        self.types.values().collect()
    }

    /// Get all registered type labels.
    pub fn all_labels(&self) -> Vec<&str> {
        self.types.values().map(|e| e.label.as_str()).collect()
    }

    /// Find types by parent.
    pub fn children_of(&self, parent: &EntityType) -> Vec<&EntityTypeEntry> {
        self.types
            .values()
            .filter(|e| e.parent_type.as_ref() == Some(parent))
            .collect()
    }

    /// Total number of registered types.
    #[must_use]
    pub fn count(&self) -> usize {
        self.types.len()
    }
}

impl Default for EntityTypeRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}
