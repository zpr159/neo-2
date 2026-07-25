use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::node::{NodeId, NodeProperties};

/// Unique identifier for an edge in the knowledge graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EdgeId(pub Uuid);

impl EdgeId {
    /// Create a new random EdgeId.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for EdgeId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Uuid> for EdgeId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<EdgeId> for Uuid {
    fn from(id: EdgeId) -> Self {
        id.0
    }
}

/// Semantic type of an edge.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeType {
    IsA,
    HasProperty,
    RelatedTo,
    Causes,
    PartOf,
    DependsOn,
    Custom(String),
}

/// A directed edge connecting two nodes in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    /// Unique identifier.
    pub id: EdgeId,
    /// Source (origin) node.
    pub source: NodeId,
    /// Target (destination) node.
    pub target: NodeId,
    /// Semantic type of this edge.
    pub edge_type: EdgeType,
    /// Key-value properties.
    pub properties: NodeProperties,
    /// Strength or weight of this edge (0.0 – 1.0).
    pub weight: f32,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

impl Edge {
    /// Create a new edge between two nodes.
    #[must_use]
    pub fn new(source: NodeId, target: NodeId, edge_type: EdgeType) -> Self {
        Self {
            id: EdgeId::new(),
            source,
            target,
            edge_type,
            properties: NodeProperties::new(),
            weight: 1.0,
            created_at: Utc::now(),
        }
    }

    /// Set the weight of this edge.
    pub fn set_weight(&mut self, weight: f32) {
        self.weight = weight.clamp(0.0, 1.0);
    }
}
