use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use crate::types::{Confidence, EntityId, RelationshipId};

/// Directional relationship between two entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub id: RelationshipId,
    pub source: EntityId,
    pub target: EntityId,
    pub relationship_type: RelationshipType,
    pub strength: RelationshipStrength,
    pub confidence: Confidence,
    pub properties: HashMap<String, crate::types::AttributeValue>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub source_system: String,
    pub history: Vec<RelationshipChange>,
}

impl Relationship {
    pub fn new(
        source: EntityId,
        target: EntityId,
        relationship_type: RelationshipType,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: RelationshipId::random(),
            source,
            target,
            relationship_type,
            strength: RelationshipStrength::Normal,
            confidence: Confidence::MEDIUM,
            properties: HashMap::new(),
            created_at: now,
            updated_at: now,
            source_system: String::new(),
            history: Vec::new(),
        }
    }

    /// Record a change to this relationship.
    pub fn record_change(&mut self, change_type: ChangeType, description: impl Into<String>) {
        self.history.push(RelationshipChange {
            change_type,
            description: description.into(),
            timestamp: Utc::now(),
        });
        self.updated_at = Utc::now();
    }
}

/// Types of directional relationships.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationshipType {
    Owns,
    Contains,
    CreatedBy,
    Uses,
    LocatedAt,
    DependsOn,
    CommunicatesWith,
    Caused,
    DerivedFrom,
    ExecutedBy,
    PlannedBy,
    MemberOf,
    ParentOf,
    ChildOf,
    ConnectedTo,
    Near,
    Inside,
    Outside,
    References,
    Custom(String),
}

impl RelationshipType {
    #[must_use]
    pub fn label(&self) -> &str {
        match self {
            Self::Owns => "owns",
            Self::Contains => "contains",
            Self::CreatedBy => "created_by",
            Self::Uses => "uses",
            Self::LocatedAt => "located_at",
            Self::DependsOn => "depends_on",
            Self::CommunicatesWith => "communicates_with",
            Self::Caused => "caused",
            Self::DerivedFrom => "derived_from",
            Self::ExecutedBy => "executed_by",
            Self::PlannedBy => "planned_by",
            Self::MemberOf => "member_of",
            Self::ParentOf => "parent_of",
            Self::ChildOf => "child_of",
            Self::ConnectedTo => "connected_to",
            Self::Near => "near",
            Self::Inside => "inside",
            Self::Outside => "outside",
            Self::References => "references",
            Self::Custom(name) => name,
        }
    }
}

impl fmt::Display for RelationshipType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Strength of a relationship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RelationshipStrength {
    Weak,
    Normal,
    Strong,
    Critical,
}

impl RelationshipStrength {
    #[must_use]
    pub fn as_f32(self) -> f32 {
        match self {
            Self::Weak => 0.25,
            Self::Normal => 0.5,
            Self::Strong => 0.75,
            Self::Critical => 1.0,
        }
    }
}

impl fmt::Display for RelationshipStrength {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Weak => write!(f, "weak"),
            Self::Normal => write!(f, "normal"),
            Self::Strong => write!(f, "strong"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

/// A change record for relationship history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipChange {
    pub change_type: ChangeType,
    pub description: String,
    pub timestamp: DateTime<Utc>,
}

/// Type of relationship change.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChangeType {
    Created,
    StrengthChanged,
    ConfidenceChanged,
    PropertyUpdated,
    TypeChanged,
    Deleted,
    Restored,
}

impl fmt::Display for ChangeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::StrengthChanged => write!(f, "strength_changed"),
            Self::ConfidenceChanged => write!(f, "confidence_changed"),
            Self::PropertyUpdated => write!(f, "property_updated"),
            Self::TypeChanged => write!(f, "type_changed"),
            Self::Deleted => write!(f, "deleted"),
            Self::Restored => write!(f, "restored"),
        }
    }
}

/// Manages all relationships in the world model.
pub struct RelationshipManager {
    relationships: dashmap::DashMap<RelationshipId, Relationship>,
    /// Index: source entity -> relationship IDs.
    source_index: dashmap::DashMap<EntityId, Vec<RelationshipId>>,
    /// Index: target entity -> relationship IDs.
    target_index: dashmap::DashMap<EntityId, Vec<RelationshipId>>,
    /// Index: entity pair -> relationship IDs.
    pair_index: dashmap::DashMap<(EntityId, EntityId), Vec<RelationshipId>>,
}

impl RelationshipManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            relationships: dashmap::DashMap::new(),
            source_index: dashmap::DashMap::new(),
            target_index: dashmap::DashMap::new(),
            pair_index: dashmap::DashMap::new(),
        }
    }

    /// Add a relationship and update all indexes.
    pub fn add(&self, relationship: Relationship) -> RelationshipId {
        let id = relationship.id.clone();
        self.source_index
            .entry(relationship.source.clone())
            .or_default()
            .push(id.clone());
        self.target_index
            .entry(relationship.target.clone())
            .or_default()
            .push(id.clone());
        self.pair_index
            .entry((relationship.source.clone(), relationship.target.clone()))
            .or_default()
            .push(id.clone());
        self.relationships.insert(id.clone(), relationship);
        id
    }

    /// Get a relationship by ID.
    pub fn get(&self, id: &RelationshipId) -> Option<Relationship> {
        self.relationships.get(id).map(|r| r.value().clone())
    }

    /// Get all relationships from a source entity.
    pub fn from_source(&self, source: &EntityId) -> Vec<Relationship> {
        self.source_index
            .get(source)
            .map(|ids| {
                ids.value()
                    .iter()
                    .filter_map(|id| self.relationships.get(id).map(|r| r.value().clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all relationships to a target entity.
    pub fn to_target(&self, target: &EntityId) -> Vec<Relationship> {
        self.target_index
            .get(target)
            .map(|ids| {
                ids.value()
                    .iter()
                    .filter_map(|id| self.relationships.get(id).map(|r| r.value().clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all relationships involving an entity (as source or target).
    pub fn involving(&self, entity: &EntityId) -> Vec<Relationship> {
        let mut result = self.from_source(entity);
        result.extend(self.to_target(entity));
        result
    }

    /// Get relationships between two specific entities.
    pub fn between(&self, source: &EntityId, target: &EntityId) -> Vec<Relationship> {
        self.pair_index
            .get(&(source.clone(), target.clone()))
            .map(|ids| {
                ids.value()
                    .iter()
                    .filter_map(|id| self.relationships.get(id).map(|r| r.value().clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all relationships of a given type.
    pub fn by_type(&self, rel_type: &RelationshipType) -> Vec<Relationship> {
        self.relationships
            .iter()
            .filter(|r| &r.value().relationship_type == rel_type)
            .map(|r| r.value().clone())
            .collect()
    }

    /// Remove a relationship.
    pub fn remove(&self, id: &RelationshipId) -> bool {
        if let Some((_, rel)) = self.relationships.remove(id) {
            self.remove_from_indexes(&rel);
            true
        } else {
            false
        }
    }

    fn remove_from_indexes(&self, rel: &Relationship) {
        if let Some(mut ids) = self.source_index.get_mut(&rel.source) {
            ids.retain(|i| i != &rel.id);
        }
        if let Some(mut ids) = self.target_index.get_mut(&rel.target) {
            ids.retain(|i| i != &rel.id);
        }
        if let Some(mut ids) = self.pair_index.get_mut(&(rel.source.clone(), rel.target.clone())) {
            ids.retain(|i| i != &rel.id);
        }
    }

    /// Total number of relationships.
    #[must_use]
    pub fn count(&self) -> usize {
        self.relationships.len()
    }

    /// Get all relationship IDs.
    pub fn all_ids(&self) -> Vec<RelationshipId> {
        self.relationships.iter().map(|r| r.key().clone()).collect()
    }
}

impl Default for RelationshipManager {
    fn default() -> Self {
        Self::new()
    }
}
