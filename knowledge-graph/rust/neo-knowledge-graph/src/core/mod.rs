pub mod entity;
pub mod relation;
pub mod attribute;
pub mod namespace;
pub mod versioning;
pub mod knowledge_id;

pub use entity::{Entity, EntityId, EntityType, EntityBuilder};
pub use relation::{Relation, RelationId, RelationType, RelationBuilder, Directedness};
pub use attribute::{Attribute, AttributeId, AttributeType, AttributeValue};
pub use namespace::{KnowledgeNamespace, NamespaceRegistry, NamespaceConfig};
pub use versioning::{VersionVector, VersionTracker, VersionedChange, ChangeType};
pub use knowledge_id::{KnowledgeId, IdType};
