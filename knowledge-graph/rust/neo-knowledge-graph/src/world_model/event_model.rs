use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::entity::{Entity, EntityId};

/// An event in the world model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEntity {
    pub id: EntityId,
    pub name: String,
    pub event_type: String,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub location_id: Option<EntityId>,
    pub participant_ids: Vec<EntityId>,
    pub properties: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl EventEntity {
    #[must_use]
    pub fn from_entity(entity: &Entity) -> Self {
        Self {
            id: entity.id,
            name: entity.label.clone(),
            event_type: entity
                .get_property("event_type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            start_time: entity
                .get_property("start_time")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok()),
            end_time: entity
                .get_property("end_time")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok()),
            location_id: entity
                .get_property("location_id")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .map(EntityId),
            participant_ids: Vec::new(),
            properties: entity.properties.clone(),
            created_at: entity.created_at,
        }
    }
}
