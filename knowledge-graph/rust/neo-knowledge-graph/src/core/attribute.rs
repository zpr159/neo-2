use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::entity::EntityId;
use super::relation::RelationId;

/// Unique identifier for an attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AttributeId(pub Uuid);

impl AttributeId {
    /// Create a new random AttributeId.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for AttributeId {
    fn default() -> Self {
        Self::new()
    }
}

/// The type of value an attribute holds.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttributeType {
    String,
    Integer,
    Float,
    Boolean,
    DateTime,
    Json,
    Binary,
}

/// A typed value for an attribute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttributeValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    DateTime(DateTime<Utc>),
    Json(serde_json::Value),
    Binary(Vec<u8>),
}

impl AttributeValue {
    /// Returns the type of this value.
    #[must_use]
    pub fn value_type(&self) -> AttributeType {
        match self {
            Self::String(_) => AttributeType::String,
            Self::Integer(_) => AttributeType::Integer,
            Self::Float(_) => AttributeType::Float,
            Self::Boolean(_) => AttributeType::Boolean,
            Self::DateTime(_) => AttributeType::DateTime,
            Self::Json(_) => AttributeType::Json,
            Self::Binary(_) => AttributeType::Binary,
        }
    }

    /// Convert to a string representation.
    #[must_use]
    pub fn to_string_value(&self) -> String {
        match self {
            Self::String(s) => s.clone(),
            Self::Integer(i) => i.to_string(),
            Self::Float(f) => f.to_string(),
            Self::Boolean(b) => b.to_string(),
            Self::DateTime(dt) => dt.to_rfc3339(),
            Self::Json(v) => v.to_string(),
            Self::Binary(_) => "<binary>".to_string(),
        }
    }
}

/// An attribute attached to an entity or relation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribute {
    /// Unique identifier.
    pub id: AttributeId,
    /// Name of the attribute.
    pub name: String,
    /// The value.
    pub value: AttributeValue,
    /// Confidence in this attribute (0.0 - 1.0).
    pub confidence: f32,
    /// The entity this attribute belongs to, if any.
    pub entity_id: Option<EntityId>,
    /// The relation this attribute belongs to, if any.
    pub relation_id: Option<RelationId>,
    /// Source attribution.
    pub source: Option<String>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp.
    pub updated_at: DateTime<Utc>,
}

impl Attribute {
    /// Create a new attribute with a string value.
    #[must_use]
    pub fn new_string(name: impl Into<String>, value: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: AttributeId::new(),
            name: name.into(),
            value: AttributeValue::String(value.into()),
            confidence: 1.0,
            entity_id: None,
            relation_id: None,
            source: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new attribute with an integer value.
    #[must_use]
    pub fn new_integer(name: impl Into<String>, value: i64) -> Self {
        let now = Utc::now();
        Self {
            id: AttributeId::new(),
            name: name.into(),
            value: AttributeValue::Integer(value),
            confidence: 1.0,
            entity_id: None,
            relation_id: None,
            source: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new attribute with a float value.
    #[must_use]
    pub fn new_float(name: impl Into<String>, value: f64) -> Self {
        let now = Utc::now();
        Self {
            id: AttributeId::new(),
            name: name.into(),
            value: AttributeValue::Float(value),
            confidence: 1.0,
            entity_id: None,
            relation_id: None,
            source: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new attribute with a boolean value.
    #[must_use]
    pub fn new_boolean(name: impl Into<String>, value: bool) -> Self {
        let now = Utc::now();
        Self {
            id: AttributeId::new(),
            name: name.into(),
            value: AttributeValue::Boolean(value),
            confidence: 1.0,
            entity_id: None,
            relation_id: None,
            source: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new attribute with a JSON value.
    #[must_use]
    pub fn new_json(name: impl Into<String>, value: serde_json::Value) -> Self {
        let now = Utc::now();
        Self {
            id: AttributeId::new(),
            name: name.into(),
            value: AttributeValue::Json(value),
            confidence: 1.0,
            entity_id: None,
            relation_id: None,
            source: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Attach to an entity.
    #[must_use]
    pub fn for_entity(mut self, entity_id: EntityId) -> Self {
        self.entity_id = Some(entity_id);
        self
    }

    /// Attach to a relation.
    #[must_use]
    pub fn for_relation(mut self, relation_id: RelationId) -> Self {
        self.relation_id = Some(relation_id);
        self
    }

    /// Set the source.
    #[must_use]
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Update the value.
    pub fn update_value(&mut self, value: AttributeValue) {
        self.value = value;
        self.updated_at = Utc::now();
    }
}
