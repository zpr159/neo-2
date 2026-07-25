use std::collections::HashMap;

use crate::core::entity::{Entity, EntityId};

/// Searches entities by metadata properties.
pub struct MetadataSearch;

impl MetadataSearch {
    /// Create a new metadata search.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Search entities by a property key-value match.
    #[must_use]
    pub fn search_by_property(
        &self,
        entities: &[Entity],
        key: &str,
        value: &serde_json::Value,
    ) -> Vec<Entity> {
        entities
            .iter()
            .filter(|e| {
                e.active
                    && e.properties
                        .get(key)
                        .map_or(false, |v| v == value)
            })
            .cloned()
            .collect()
    }

    /// Search entities by namespace.
    #[must_use]
    pub fn search_by_namespace(&self, entities: &[Entity], namespace: &str) -> Vec<Entity> {
        entities
            .iter()
            .filter(|e| e.active && e.namespace == namespace)
            .cloned()
            .collect()
    }

    /// Search entities by minimum confidence.
    #[must_use]
    pub fn search_by_min_confidence(&self, entities: &[Entity], min_confidence: f32) -> Vec<Entity> {
        entities
            .iter()
            .filter(|e| e.active && e.confidence >= min_confidence)
            .cloned()
            .collect()
    }

    /// Search entities by minimum importance.
    #[must_use]
    pub fn search_by_min_importance(&self, entities: &[Entity], min_importance: f32) -> Vec<Entity> {
        entities
            .iter()
            .filter(|e| e.active && e.importance >= min_importance)
            .cloned()
            .collect()
    }

    /// Search entities that have a specific property key (regardless of value).
    #[must_use]
    pub fn search_by_property_key(&self, entities: &[Entity], key: &str) -> Vec<Entity> {
        entities
            .iter()
            .filter(|e| e.active && e.properties.contains_key(key))
            .cloned()
            .collect()
    }
}

impl Default for MetadataSearch {
    fn default() -> Self {
        Self::new()
    }
}
