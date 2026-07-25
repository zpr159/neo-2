use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(pub Uuid);

impl EntityId {
    /// Create a new random EntityId.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for EntityId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Uuid> for EntityId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<EntityId> for Uuid {
    fn from(id: EntityId) -> Self {
        id.0
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "entity:{}", self.0)
    }
}

/// Semantic type of an entity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityType {
    Person,
    Place,
    Organization,
    Object,
    Event,
    Concept,
    Task,
    Goal,
    Skill,
    Project,
    Document,
    Idea,
    Rule,
    Custom(String),
}

impl EntityType {
    /// Returns the string representation of the entity type.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Person => "person",
            Self::Place => "place",
            Self::Organization => "organization",
            Self::Object => "object",
            Self::Event => "event",
            Self::Concept => "concept",
            Self::Task => "task",
            Self::Goal => "goal",
            Self::Skill => "skill",
            Self::Project => "project",
            Self::Document => "document",
            Self::Idea => "idea",
            Self::Rule => "rule",
            Self::Custom(s) => s,
        }
    }
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// An entity in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// Unique identifier.
    pub id: EntityId,
    /// Semantic type.
    pub entity_type: EntityType,
    /// Human-readable label/name.
    pub label: String,
    /// Description of the entity.
    pub description: String,
    /// Key-value properties.
    pub properties: HashMap<String, serde_json::Value>,
    /// Aliases or alternative names.
    pub aliases: Vec<String>,
    /// Namespace this entity belongs to.
    pub namespace: String,
    /// Confidence score (0.0 - 1.0).
    pub confidence: f32,
    /// Importance score (0.0 - 1.0).
    pub importance: f32,
    /// Source attributions.
    pub sources: Vec<String>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp.
    pub updated_at: DateTime<Utc>,
    /// Version counter.
    pub version: u64,
    /// Whether this entity is active (not deleted/pruned).
    pub active: bool,
}

impl Entity {
    /// Create a new entity with the given type and label.
    #[must_use]
    pub fn new(entity_type: EntityType, label: String) -> Self {
        let now = Utc::now();
        Self {
            id: EntityId::new(),
            entity_type,
            label,
            description: String::new(),
            properties: HashMap::new(),
            aliases: Vec::new(),
            namespace: "default".to_string(),
            confidence: 1.0,
            importance: 0.5,
            sources: Vec::new(),
            created_at: now,
            updated_at: now,
            version: 0,
            active: true,
        }
    }

    /// Set or overwrite a property.
    pub fn set_property(&mut self, key: String, value: serde_json::Value) {
        self.properties.insert(key, value);
        self.touch();
    }

    /// Retrieve a property by key.
    #[must_use]
    pub fn get_property(&self, key: &str) -> Option<&serde_json::Value> {
        self.properties.get(key)
    }

    /// Add an alias.
    pub fn add_alias(&mut self, alias: String) {
        if !self.aliases.contains(&alias) {
            self.aliases.push(alias);
            self.touch();
        }
    }

    /// Add a source attribution.
    pub fn add_source(&mut self, source: String) {
        if !self.sources.contains(&source) {
            self.sources.push(source);
            self.touch();
        }
    }

    /// Touch the updated_at timestamp and increment version.
    pub fn touch(&mut self) {
        self.version += 1;
        self.updated_at = Utc::now();
    }

    /// Check if this entity matches a query string (label, aliases, description).
    #[must_use]
    pub fn matches_query(&self, query: &str) -> bool {
        let q = query.to_lowercase();
        self.label.to_lowercase().contains(&q)
            || self.description.to_lowercase().contains(&q)
            || self.aliases.iter().any(|a| a.to_lowercase().contains(&q))
    }
}

impl Entity {
    /// Build an entity using the builder pattern.
    #[must_use]
    pub fn builder(entity_type: EntityType, label: impl Into<String>) -> EntityBuilder {
        EntityBuilder::new(entity_type, label.into())
    }
}

/// Builder for constructing entities with fluent API.
pub struct EntityBuilder {
    entity: Entity,
}

impl EntityBuilder {
    /// Create a new EntityBuilder.
    #[must_use]
    pub fn new(entity_type: EntityType, label: String) -> Self {
        Self {
            entity: Entity::new(entity_type, label),
        }
    }

    /// Set the description.
    #[must_use]
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.entity.description = desc.into();
        self
    }

    /// Set the namespace.
    #[must_use]
    pub fn namespace(mut self, ns: impl Into<String>) -> Self {
        self.entity.namespace = ns.into();
        self
    }

    /// Set the confidence.
    #[must_use]
    pub fn confidence(mut self, c: f32) -> Self {
        self.entity.confidence = c.clamp(0.0, 1.0);
        self
    }

    /// Set the importance.
    #[must_use]
    pub fn importance(mut self, i: f32) -> Self {
        self.entity.importance = i.clamp(0.0, 1.0);
        self
    }

    /// Add an alias.
    #[must_use]
    pub fn alias(mut self, alias: impl Into<String>) -> Self {
        self.entity.aliases.push(alias.into());
        self
    }

    /// Add a source.
    #[must_use]
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.entity.sources.push(source.into());
        self
    }

    /// Set a property.
    #[must_use]
    pub fn property(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.entity.properties.insert(key.into(), value);
        self
    }

    /// Build the entity.
    #[must_use]
    pub fn build(self) -> Entity {
        self.entity
    }
}
