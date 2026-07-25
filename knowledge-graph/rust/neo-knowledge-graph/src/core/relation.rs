use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::entity::EntityId;

/// Unique identifier for a relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RelationId(pub Uuid);

impl RelationId {
    /// Create a new random RelationId.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for RelationId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Uuid> for RelationId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<RelationId> for Uuid {
    fn from(id: RelationId) -> Self {
        id.0
    }
}

impl std::fmt::Display for RelationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "relation:{}", self.0)
    }
}

/// Semantic type of a relation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationType {
    IsA,
    HasA,
    PartOf,
    RelatedTo,
    Causes,
    Enables,
    Prevents,
    DependsOn,
    LocatedAt,
    MemberOf,
    AuthorOf,
    CreatedBy,
    Uses,
    InheritsFrom,
    Implements,
    Contradicts,
    Supports,
    TemporallyFollows,
    SpatiallyNear,
    Custom(String),
}

impl RelationType {
    /// Returns the string representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::IsA => "is_a",
            Self::HasA => "has_a",
            Self::PartOf => "part_of",
            Self::RelatedTo => "related_to",
            Self::Causes => "causes",
            Self::Enables => "enables",
            Self::Prevents => "prevents",
            Self::DependsOn => "depends_on",
            Self::LocatedAt => "located_at",
            Self::MemberOf => "member_of",
            Self::AuthorOf => "author_of",
            Self::CreatedBy => "created_by",
            Self::Uses => "uses",
            Self::InheritsFrom => "inherits_from",
            Self::Implements => "implements",
            Self::Contradicts => "contradicts",
            Self::Supports => "supports",
            Self::TemporallyFollows => "temporally_follows",
            Self::SpatiallyNear => "spatially_near",
            Self::Custom(s) => s,
        }
    }
}

impl std::fmt::Display for RelationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Whether a relation is directed or undirected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Directedness {
    Directed,
    Undirected,
}

/// A relation (typed, weighted edge) between two entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    /// Unique identifier.
    pub id: RelationId,
    /// Semantic type.
    pub relation_type: RelationType,
    /// Source entity.
    pub source: EntityId,
    /// Target entity.
    pub target: EntityId,
    /// Whether this relation is directed.
    pub directedness: Directedness,
    /// Weight/strength of this relation (0.0 - 1.0).
    pub weight: f32,
    /// Confidence in this relation (0.0 - 1.0).
    pub confidence: f32,
    /// Key-value properties.
    pub properties: HashMap<String, serde_json::Value>,
    /// Human-readable label.
    pub label: String,
    /// Source attributions.
    pub sources: Vec<String>,
    /// Namespace.
    pub namespace: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp.
    pub updated_at: DateTime<Utc>,
    /// Version counter.
    pub version: u64,
    /// Whether this relation is active.
    pub active: bool,
}

impl Relation {
    /// Create a new relation.
    #[must_use]
    pub fn new(
        relation_type: RelationType,
        source: EntityId,
        target: EntityId,
        label: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: RelationId::new(),
            relation_type,
            source,
            target,
            directedness: Directedness::Directed,
            weight: 1.0,
            confidence: 1.0,
            properties: HashMap::new(),
            label,
            sources: Vec::new(),
            namespace: "default".to_string(),
            created_at: now,
            updated_at: now,
            version: 0,
            active: true,
        }
    }

    /// Set or overwrite a property.
    pub fn set_property(&mut self, key: String, value: serde_json::Value) {
        self.properties.insert(key, value);
        self.touch();
    }

    /// Touch the updated_at timestamp and increment version.
    pub fn touch(&mut self) {
        self.version += 1;
        self.updated_at = Utc::now();
    }

    /// Check if this relation connects the given entities (ignoring direction for undirected).
    #[must_use]
    pub fn connects(&self, a: EntityId, b: EntityId) -> bool {
        match self.directedness {
            Directedness::Directed => self.source == a && self.target == b,
            Directedness::Undirected => {
                (self.source == a && self.target == b) || (self.source == b && self.target == a)
            }
        }
    }

    /// Get the other end of the relation.
    #[must_use]
    pub fn other_end(&self, id: EntityId) -> Option<EntityId> {
        if self.source == id {
            Some(self.target)
        } else if self.target == id {
            Some(self.source)
        } else {
            None
        }
    }

    /// Build a relation using the builder pattern.
    #[must_use]
    pub fn builder(
        relation_type: RelationType,
        source: EntityId,
        target: EntityId,
        label: impl Into<String>,
    ) -> RelationBuilder {
        RelationBuilder::new(relation_type, source, target, label.into())
    }
}

/// Builder for constructing relations with fluent API.
pub struct RelationBuilder {
    relation: Relation,
}

impl RelationBuilder {
    /// Create a new RelationBuilder.
    #[must_use]
    pub fn new(
        relation_type: RelationType,
        source: EntityId,
        target: EntityId,
        label: String,
    ) -> Self {
        Self {
            relation: Relation::new(relation_type, source, target, label),
        }
    }

    /// Set as undirected.
    #[must_use]
    pub fn undirected(mut self) -> Self {
        self.relation.directedness = Directedness::Undirected;
        self
    }

    /// Set the weight.
    #[must_use]
    pub fn weight(mut self, w: f32) -> Self {
        self.relation.weight = w.clamp(0.0, 1.0);
        self
    }

    /// Set the confidence.
    #[must_use]
    pub fn confidence(mut self, c: f32) -> Self {
        self.relation.confidence = c.clamp(0.0, 1.0);
        self
    }

    /// Set the namespace.
    #[must_use]
    pub fn namespace(mut self, ns: impl Into<String>) -> Self {
        self.relation.namespace = ns.into();
        self
    }

    /// Add a source.
    #[must_use]
    pub fn source_attr(mut self, source: impl Into<String>) -> Self {
        self.relation.sources.push(source.into());
        self
    }

    /// Set a property.
    #[must_use]
    pub fn property(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.relation.properties.insert(key.into(), value);
        self
    }

    /// Build the relation.
    #[must_use]
    pub fn build(self) -> Relation {
        self.relation
    }
}
