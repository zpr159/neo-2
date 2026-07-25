use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::entity::{Entity, EntityId};

/// A project in the world model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectEntity {
    pub id: EntityId,
    pub name: String,
    pub description: String,
    pub status: String,
    pub member_ids: Vec<EntityId>,
    pub task_ids: Vec<EntityId>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub properties: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl ProjectEntity {
    #[must_use]
    pub fn from_entity(entity: &Entity) -> Self {
        Self {
            id: entity.id,
            name: entity.label.clone(),
            description: entity.description.clone(),
            status: entity
                .get_property("status")
                .and_then(|v| v.as_str())
                .unwrap_or("active")
                .to_string(),
            member_ids: Vec::new(),
            task_ids: Vec::new(),
            start_date: entity
                .get_property("start_date")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok()),
            end_date: entity
                .get_property("end_date")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok()),
            properties: entity.properties.clone(),
            created_at: entity.created_at,
        }
    }
}
