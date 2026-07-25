use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::entity::{Entity, EntityId};

/// An organization in the world model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationEntity {
    pub id: EntityId,
    pub name: String,
    pub org_type: String,
    pub industry: Option<String>,
    pub member_ids: Vec<EntityId>,
    pub properties: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl OrganizationEntity {
    #[must_use]
    pub fn from_entity(entity: &Entity) -> Self {
        Self {
            id: entity.id,
            name: entity.label.clone(),
            org_type: entity
                .get_property("org_type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            industry: entity.get_property("industry").and_then(|v| v.as_str()).map(String::from),
            member_ids: Vec::new(),
            properties: entity.properties.clone(),
            created_at: entity.created_at,
        }
    }
}
