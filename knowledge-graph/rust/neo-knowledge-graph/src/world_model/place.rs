use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::entity::{Entity, EntityId};

/// A place in the world model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceEntity {
    pub id: EntityId,
    pub name: String,
    pub address: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub place_type: String,
    pub properties: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl PlaceEntity {
    #[must_use]
    pub fn from_entity(entity: &Entity) -> Self {
        Self {
            id: entity.id,
            name: entity.label.clone(),
            address: entity.get_property("address").and_then(|v| v.as_str()).map(String::from),
            latitude: entity.get_property("latitude").and_then(|v| v.as_f64()),
            longitude: entity.get_property("longitude").and_then(|v| v.as_f64()),
            place_type: entity
                .get_property("place_type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            properties: entity.properties.clone(),
            created_at: entity.created_at,
        }
    }
}
