use crate::entity::WorldEntity;
use crate::relationships::{Relationship, RelationshipType, RelationshipStrength};
use crate::types::{
    AttributeValue, Confidence, EntityId, EntityType,
    EntityState,
};

/// Fluent builder for constructing entities.
pub struct EntityBuilder {
    name: String,
    entity_type: EntityType,
    state: EntityState,
    confidence: Confidence,
    tags: Vec<String>,
    attributes: Vec<(String, AttributeValue)>,
    location_id: Option<String>,
    source_system: String,
}

impl EntityBuilder {
    pub fn new(name: impl Into<String>, entity_type: EntityType) -> Self {
        Self {
            name: name.into(),
            entity_type,
            state: EntityState::Created,
            confidence: Confidence::MEDIUM,
            tags: Vec::new(),
            attributes: Vec::new(),
            location_id: None,
            source_system: String::new(),
        }
    }

    pub fn state(mut self, state: EntityState) -> Self {
        self.state = state;
        self
    }

    pub fn confidence(mut self, confidence: Confidence) -> Self {
        self.confidence = confidence;
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn attribute(mut self, key: impl Into<String>, value: AttributeValue) -> Self {
        self.attributes.push((key.into(), value));
        self
    }

    pub fn location(mut self, location_id: impl Into<String>) -> Self {
        self.location_id = Some(location_id.into());
        self
    }

    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source_system = source.into();
        self
    }

    pub fn build(self) -> WorldEntity {
        let mut entity = WorldEntity::new(self.name, self.entity_type);
        entity.state = self.state;
        entity.confidence = self.confidence;
        entity.tags = self.tags;
        entity.location_id = self.location_id;
        entity.source_system = self.source_system;

        for (key, value) in self.attributes {
            entity.set_attribute(key, value);
        }

        entity
    }
}

/// Fluent builder for constructing relationships.
pub struct RelationshipBuilder {
    source: EntityId,
    target: EntityId,
    relationship_type: RelationshipType,
    strength: RelationshipStrength,
    confidence: Confidence,
    source_system: String,
}

impl RelationshipBuilder {
    pub fn new(source: EntityId, target: EntityId, relationship_type: RelationshipType) -> Self {
        Self {
            source,
            target,
            relationship_type,
            strength: RelationshipStrength::Normal,
            confidence: Confidence::MEDIUM,
            source_system: String::new(),
        }
    }

    pub fn strength(mut self, strength: RelationshipStrength) -> Self {
        self.strength = strength;
        self
    }

    pub fn confidence(mut self, confidence: Confidence) -> Self {
        self.confidence = confidence;
        self
    }

    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source_system = source.into();
        self
    }

    pub fn build(self) -> Relationship {
        let mut rel = Relationship::new(self.source, self.target, self.relationship_type);
        rel.strength = self.strength;
        rel.confidence = self.confidence;
        rel.source_system = self.source_system;
        rel
    }
}
