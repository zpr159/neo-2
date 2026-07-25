use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::entity::{Entity, EntityId};

/// A goal in the world model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalEntity {
    pub id: EntityId,
    pub name: String,
    pub description: String,
    pub goal_type: String,
    pub progress: f32,
    pub target_date: Option<DateTime<Utc>>,
    pub subgoal_ids: Vec<EntityId>,
    pub properties: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl GoalEntity {
    #[must_use]
    pub fn from_entity(entity: &Entity) -> Self {
        Self {
            id: entity.id,
            name: entity.label.clone(),
            description: entity.description.clone(),
            goal_type: entity
                .get_property("goal_type")
                .and_then(|v| v.as_str())
                .unwrap_or("general")
                .to_string(),
            progress: entity
                .get_property("progress")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32,
            target_date: entity
                .get_property("target_date")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok()),
            subgoal_ids: Vec::new(),
            properties: entity.properties.clone(),
            created_at: entity.created_at,
        }
    }
}
