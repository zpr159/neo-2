use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::entity::{Entity, EntityId};

/// An object in the world model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectEntity {
    pub id: EntityId,
    pub name: String,
    pub object_type: String,
    pub location_id: Option<EntityId>,
    pub properties: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl ObjectEntity {
    #[must_use]
    pub fn from_entity(entity: &Entity) -> Self {
        Self {
            id: entity.id,
            name: entity.label.clone(),
            object_type: entity
                .get_property("object_type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            location_id: entity
                .get_property("location_id")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .map(EntityId),
            properties: entity.properties.clone(),
            created_at: entity.created_at,
        }
    }
}
