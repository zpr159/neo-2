use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Identifies what kind of ID this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IdType {
    Entity,
    Relation,
    Attribute,
    Concept,
    Event,
}

impl fmt::Display for IdType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Entity => write!(f, "entity"),
            Self::Relation => write!(f, "relation"),
            Self::Attribute => write!(f, "attribute"),
            Self::Concept => write!(f, "concept"),
            Self::Event => write!(f, "event"),
        }
    }
}

/// A typed unique identifier for any knowledge graph element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KnowledgeId {
    pub id: Uuid,
    pub id_type: IdType,
}

impl KnowledgeId {
    /// Create a new random KnowledgeId of the given type.
    #[must_use]
    pub fn new(id_type: IdType) -> Self {
        Self {
            id: Uuid::new_v4(),
            id_type,
        }
    }

    /// Create a KnowledgeId from an existing UUID and type.
    #[must_use]
    pub fn from_uuid(id: Uuid, id_type: IdType) -> Self {
        Self { id, id_type }
    }
}

impl fmt::Display for KnowledgeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.id_type, self.id)
    }
}
