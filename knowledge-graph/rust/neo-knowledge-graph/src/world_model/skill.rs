use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::entity::{Entity, EntityId};

/// A skill in the world model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEntity {
    pub id: EntityId,
    pub name: String,
    pub description: String,
    pub skill_type: String,
    pub proficiency: f32,
    pub related_skills: Vec<EntityId>,
    pub properties: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl SkillEntity {
    #[must_use]
    pub fn from_entity(entity: &Entity) -> Self {
        Self {
            id: entity.id,
            name: entity.label.clone(),
            description: entity.description.clone(),
            skill_type: entity
                .get_property("skill_type")
                .and_then(|v| v.as_str())
                .unwrap_or("general")
                .to_string(),
            proficiency: entity
                .get_property("proficiency")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.5) as f32,
            related_skills: Vec::new(),
            properties: entity.properties.clone(),
            created_at: entity.created_at,
        }
    }
}
