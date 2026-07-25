use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::entity::{Entity, EntityId, EntityType};

/// A person in the world model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonEntity {
    /// Entity id.
    pub id: EntityId,
    /// Full name.
    pub name: String,
    /// Role or title.
    pub role: Option<String>,
    /// Organization id.
    pub organization_id: Option<EntityId>,
    /// Email.
    pub email: Option<String>,
    /// Skills.
    pub skills: Vec<String>,
    /// Relationships.
    pub relationships: Vec<EntityId>,
    /// Additional properties.
    pub properties: HashMap<String, serde_json::Value>,
    /// Created at.
    pub created_at: DateTime<Utc>,
}

impl PersonEntity {
    /// Create from a generic Entity.
    #[must_use]
    pub fn from_entity(entity: &Entity) -> Self {
        Self {
            id: entity.id,
            name: entity.label.clone(),
            role: entity.get_property("role").and_then(|v| v.as_str()).map(String::from),
            organization_id: entity
                .get_property("organization_id")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .map(EntityId),
            email: entity.get_property("email").and_then(|v| v.as_str()).map(String::from),
            skills: entity
                .get_property("skills")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            relationships: Vec::new(),
            properties: entity.properties.clone(),
            created_at: entity.created_at,
        }
    }
}
