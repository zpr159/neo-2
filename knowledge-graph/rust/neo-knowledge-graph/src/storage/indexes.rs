use std::collections::HashMap;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::core::entity::EntityId;
use crate::core::relation::RelationId;

/// Types of indexes maintained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IndexType {
    Label,
    Type,
    RelationType,
    Temporal,
    Confidence,
    Namespace,
    Property,
}

/// Statistics about an index.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexStats {
    /// Number of entries in the index.
    pub entry_count: usize,
    /// Number of indexed items.
    pub item_count: usize,
    /// Whether the index is up to date.
    pub up_to_date: bool,
}

/// Maintains various indexes for fast knowledge graph queries.
pub struct GraphIndexes {
    /// Label -> entity ids.
    pub label_index: DashMap<String, Vec<EntityId>>,
    /// Entity type -> entity ids.
    pub type_index: DashMap<String, Vec<EntityId>>,
    /// Relation type -> relation ids.
    pub relation_type_index: DashMap<String, Vec<RelationId>>,
    /// Namespace -> entity ids.
    pub namespace_index: DashMap<String, Vec<EntityId>>,
    /// Confidence bucket (0.0-0.2, 0.2-0.4, ...) -> entity ids.
    pub confidence_index: DashMap<String, Vec<EntityId>>,
    /// Property key-value -> entity ids.
    pub property_index: DashMap<String, Vec<EntityId>>,
    /// Timestamp hour bucket -> entity ids.
    pub temporal_index: DashMap<String, Vec<EntityId>>,
    /// Index stats.
    stats: DashMap<IndexType, IndexStats>,
}

impl GraphIndexes {
    /// Create new empty indexes.
    #[must_use]
    pub fn new() -> Self {
        Self {
            label_index: DashMap::new(),
            type_index: DashMap::new(),
            relation_type_index: DashMap::new(),
            namespace_index: DashMap::new(),
            confidence_index: DashMap::new(),
            property_index: DashMap::new(),
            temporal_index: DashMap::new(),
            stats: DashMap::new(),
        }
    }

    /// Index an entity.
    pub fn index_entity(&self, entity_id: EntityId, label: &str, entity_type: &str, namespace: &str, confidence: f32) {
        // Label index
        self.label_index
            .entry(label.to_lowercase())
            .or_default()
            .push(entity_id);

        // Type index
        self.type_index
            .entry(entity_type.to_string())
            .or_default()
            .push(entity_id);

        // Namespace index
        self.namespace_index
            .entry(namespace.to_string())
            .or_default()
            .push(entity_id);

        // Confidence bucket
        let bucket = confidence_bucket(confidence);
        self.confidence_index
            .entry(bucket)
            .or_default()
            .push(entity_id);

        // Temporal index (hour bucket)
        let hour_bucket = chrono::Utc::now().format("%Y-%m-%dT%H").to_string();
        self.temporal_index
            .entry(hour_bucket)
            .or_default()
            .push(entity_id);
    }

    /// Index a relation.
    pub fn index_relation(&self, relation_id: RelationId, relation_type: &str) {
        self.relation_type_index
            .entry(relation_type.to_string())
            .or_default()
            .push(relation_id);
    }

    /// Index a property on an entity.
    pub fn index_property(&self, entity_id: EntityId, key: &str, value: &str) {
        let composite = format!("{}={}", key, value);
        self.property_index
            .entry(composite)
            .or_default()
            .push(entity_id);
    }

    /// Lookup by label.
    #[must_use]
    pub fn by_label(&self, label: &str) -> Vec<EntityId> {
        self.label_index
            .get(&label.to_lowercase())
            .map(|v| v.clone())
            .unwrap_or_default()
    }

    /// Lookup by type.
    #[must_use]
    pub fn by_type(&self, entity_type: &str) -> Vec<EntityId> {
        self.type_index
            .get(entity_type)
            .map(|v| v.clone())
            .unwrap_or_default()
    }

    /// Lookup by relation type.
    #[must_use]
    pub fn by_relation_type(&self, relation_type: &str) -> Vec<RelationId> {
        self.relation_type_index
            .get(relation_type)
            .map(|v| v.clone())
            .unwrap_or_default()
    }

    /// Lookup by namespace.
    #[must_use]
    pub fn by_namespace(&self, namespace: &str) -> Vec<EntityId> {
        self.namespace_index
            .get(namespace)
            .map(|v| v.clone())
            .unwrap_or_default()
    }

    /// Get stats for an index type.
    #[must_use]
    pub fn stats(&self, index_type: IndexType) -> IndexStats {
        self.stats
            .get(&index_type)
            .map(|s| s.value().clone())
            .unwrap_or_default()
    }

    /// Clear all indexes.
    pub fn clear(&self) {
        self.label_index.clear();
        self.type_index.clear();
        self.relation_type_index.clear();
        self.namespace_index.clear();
        self.confidence_index.clear();
        self.property_index.clear();
        self.temporal_index.clear();
        self.stats.clear();
    }
}

impl Default for GraphIndexes {
    fn default() -> Self {
        Self::new()
    }
}

fn confidence_bucket(confidence: f32) -> String {
    let bucket = (confidence * 5.0).floor() as u32;
    format!("{}-{}", bucket as f32 / 5.0, (bucket + 1) as f32 / 5.0)
}
