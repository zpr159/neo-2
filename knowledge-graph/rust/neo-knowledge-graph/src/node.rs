use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a node in the knowledge graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub Uuid);

impl NodeId {
    /// Create a new random NodeId.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Uuid> for NodeId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<NodeId> for Uuid {
    fn from(id: NodeId) -> Self {
        id.0
    }
}

/// Semantic type of a node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType {
    Entity,
    Concept,
    Event,
    Relation,
    Attribute,
    Custom(String),
}

/// Arbitrary properties attached to a node or edge.
pub type NodeProperties = HashMap<String, serde_json::Value>;

/// A node in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// Unique identifier.
    pub id: NodeId,
    /// Semantic type.
    pub node_type: NodeType,
    /// Human-readable label.
    pub label: String,
    /// Key-value properties.
    pub properties: NodeProperties,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp.
    pub updated_at: DateTime<Utc>,
    /// Monotonically increasing version counter.
    pub version: u64,
}

impl Node {
    /// Create a new node with the given type and label.
    #[must_use]
    pub fn new(node_type: NodeType, label: String) -> Self {
        let now = Utc::now();
        Self {
            id: NodeId::new(),
            node_type,
            label,
            properties: NodeProperties::new(),
            created_at: now,
            updated_at: now,
            version: 0,
        }
    }

    /// Set or overwrite a property on this node.
    pub fn set_property(&mut self, key: String, value: serde_json::Value) {
        self.properties.insert(key, value);
        self.updated_at = Utc::now();
        self.version += 1;
    }

    /// Retrieve a property by key.
    #[must_use]
    pub fn get_property(&self, key: &str) -> Option<&serde_json::Value> {
        self.properties.get(key)
    }

    /// Increment the version counter and update the timestamp.
    pub fn increment_version(&mut self) {
        self.version += 1;
        self.updated_at = Utc::now();
    }
}
