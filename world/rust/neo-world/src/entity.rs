use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::confidence::Evidence;
use crate::lifecycle::{is_valid_transition, LifecycleManager};
use crate::types::{
    AttributeValue, AttributeSource, Confidence, EntityAttribute, EntityId, EntityState,
    EntityType, EntityVersion, RelationshipId, WorldVersion,
};

/// A persistent entity tracked in the world model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldEntity {
    pub id: EntityId,
    pub name: String,
    pub entity_type: EntityType,
    pub state: EntityState,
    pub confidence: Confidence,
    pub attributes: Vec<EntityAttribute>,
    pub tags: Vec<String>,
    pub location_id: Option<String>,
    pub relationships: Vec<RelationshipId>,
    pub version: u64,
    pub version_history: Vec<EntityVersion>,
    pub source_system: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub observation_count: u64,
    pub last_observed_at: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub evidence: Vec<Evidence>,
}

impl WorldEntity {
    pub fn new(name: impl Into<String>, entity_type: EntityType) -> Self {
        let now = Utc::now();
        Self {
            id: EntityId::random(),
            name: name.into(),
            entity_type,
            state: EntityState::Created,
            confidence: Confidence::MEDIUM,
            attributes: Vec::new(),
            tags: Vec::new(),
            location_id: None,
            relationships: Vec::new(),
            version: 1,
            version_history: Vec::new(),
            source_system: String::new(),
            created_at: now,
            updated_at: now,
            observation_count: 0,
            last_observed_at: None,
            metadata: HashMap::new(),
            evidence: Vec::new(),
        }
    }

    /// Transition entity to a new state.
    pub fn transition(&mut self, to: EntityState, reason: impl Into<String>, _version: WorldVersion) -> bool {
        if is_valid_transition(&self.state, &to) {
            let from = self.state.clone();
            self.version += 1;
            self.version_history.push(EntityVersion {
                version: self.version,
                timestamp: Utc::now(),
                snapshot: serde_json::to_value(&self.attributes).unwrap_or_default(),
                change_description: format!("State: {from} -> {to}: {}", reason.into()),
            });
            self.state = to;
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Set a typed attribute.
    pub fn set_attribute(&mut self, key: impl Into<String>, value: AttributeValue) {
        let key = key.into();
        let now = Utc::now();
        if let Some(attr) = self.attributes.iter_mut().find(|a| a.key == key) {
            attr.value = value;
            attr.updated_at = now;
        } else {
            self.attributes.push(EntityAttribute {
                key,
                value,
                source: AttributeSource::default(),
                confidence: 0.5,
                updated_at: now,
            });
        }
        self.updated_at = now;
        self.version += 1;
    }

    /// Get a typed attribute.
    pub fn get_attribute(&self, key: &str) -> Option<&AttributeValue> {
        self.attributes.iter().find(|a| a.key == key).map(|a| &a.value)
    }

    /// Get attribute as string.
    pub fn attribute_as_str(&self, key: &str) -> Option<&str> {
        self.get_attribute(key).and_then(|v| match v {
            AttributeValue::Text(s) => Some(s.as_str()),
            _ => None,
        })
    }

    /// Get attribute as f64.
    pub fn attribute_as_f64(&self, key: &str) -> Option<f64> {
        self.get_attribute(key).and_then(|v| match v {
            AttributeValue::Float(f) => Some(*f),
            AttributeValue::Integer(i) => Some(*i as f64),
            _ => None,
        })
    }

    /// Remove an attribute.
    pub fn remove_attribute(&mut self, key: &str) -> bool {
        let len_before = self.attributes.len();
        self.attributes.retain(|a| a.key != key);
        if self.attributes.len() < len_before {
            self.updated_at = Utc::now();
            self.version += 1;
            true
        } else {
            false
        }
    }

    /// Record an observation.
    pub fn observe(&mut self) {
        self.observation_count += 1;
        self.last_observed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Add a tag.
    pub fn tag(&mut self, tag: impl Into<String>) {
        let tag = tag.into();
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    /// Remove a tag.
    pub fn untag(&mut self, tag: &str) -> bool {
        let len = self.tags.len();
        self.tags.retain(|t| t != tag);
        self.tags.len() < len
    }

    /// Check if entity has a tag.
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Activate the entity.
    pub fn activate(&mut self, version: WorldVersion) {
        self.transition(EntityState::Active, "activated", version);
    }

    /// Archive the entity.
    pub fn archive(&mut self, version: WorldVersion) {
        self.transition(EntityState::Archived, "archived", version);
    }

    /// Soft-delete the entity.
    pub fn delete(&mut self, version: WorldVersion) {
        self.transition(EntityState::Deleted, "deleted", version);
    }

    /// Check if entity is active.
    pub fn is_active(&self) -> bool {
        self.state.is_active()
    }

    /// Check if entity is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }

    /// Create a version snapshot.
    pub fn snapshot_version(&mut self, description: impl Into<String>) {
        self.version += 1;
        self.version_history.push(EntityVersion {
            version: self.version,
            timestamp: Utc::now(),
            snapshot: serde_json::to_value(&self.attributes).unwrap_or_default(),
            change_description: description.into(),
        });
    }
}

/// Tracks all entities in the world model.
pub struct EntityTracker {
    entities: dashmap::DashMap<EntityId, WorldEntity>,
    lifecycle: LifecycleManager,
}

impl EntityTracker {
    #[must_use]
    pub fn new() -> Self {
        Self {
            entities: dashmap::DashMap::new(),
            lifecycle: LifecycleManager::new(),
        }
    }

    /// Add an entity.
    pub fn add(&self, entity: WorldEntity) -> EntityId {
        let id = entity.id.clone();
        self.entities.insert(id.clone(), entity);
        id
    }

    /// Get an entity.
    pub fn get(&self, id: &EntityId) -> Option<WorldEntity> {
        self.entities.get(id).map(|e| e.value().clone())
    }

    /// Get a mutable reference to an entity.
    pub fn get_mut(&self, id: &EntityId) -> Option<dashmap::mapref::one::RefMut<'_, EntityId, WorldEntity>> {
        self.entities.get_mut(id)
    }

    /// Remove an entity.
    pub fn remove(&self, id: &EntityId) -> bool {
        self.entities.remove(id).is_some()
    }

    /// Check if an entity exists.
    pub fn contains(&self, id: &EntityId) -> bool {
        self.entities.contains_key(id)
    }

    /// Total entity count.
    pub fn count(&self) -> usize {
        self.entities.len()
    }

    /// All entity IDs.
    pub fn all_ids(&self) -> Vec<EntityId> {
        self.entities.iter().map(|e| e.key().clone()).collect()
    }

    /// All entities.
    pub fn all(&self) -> Vec<WorldEntity> {
        self.entities.iter().map(|e| e.value().clone()).collect()
    }

    /// Find by type.
    pub fn by_type(&self, entity_type: &EntityType) -> Vec<WorldEntity> {
        self.entities
            .iter()
            .filter(|e| &e.value().entity_type == entity_type)
            .map(|e| e.value().clone())
            .collect()
    }

    /// Find by name (case-insensitive substring).
    pub fn by_name(&self, name: &str) -> Vec<WorldEntity> {
        let lower = name.to_lowercase();
        self.entities
            .iter()
            .filter(|e| e.value().name.to_lowercase().contains(&lower))
            .map(|e| e.value().clone())
            .collect()
    }

    /// Find by tag.
    pub fn by_tag(&self, tag: &str) -> Vec<WorldEntity> {
        self.entities
            .iter()
            .filter(|e| e.value().has_tag(tag))
            .map(|e| e.value().clone())
            .collect()
    }

    /// Find by state.
    pub fn by_state(&self, state: &EntityState) -> Vec<WorldEntity> {
        self.entities
            .iter()
            .filter(|e| &e.value().state == state)
            .map(|e| e.value().clone())
            .collect()
    }

    /// Active entities.
    pub fn active(&self) -> Vec<WorldEntity> {
        self.entities
            .iter()
            .filter(|e| e.value().is_active())
            .map(|e| e.value().clone())
            .collect()
    }

    /// Access lifecycle manager.
    pub fn lifecycle(&self) -> &LifecycleManager {
        &self.lifecycle
    }

    /// Deactivate entities not observed within the decay window.
    pub fn apply_decay(&self, decay_secs: u64, version: WorldVersion) -> usize {
        let mut deactivated = 0;
        let now = Utc::now();
        for mut entry in self.entities.iter_mut() {
            let entity = entry.value_mut();
            if entity.is_active() {
                if let Some(last) = entity.last_observed_at {
                    let elapsed = (now - last).num_seconds() as u64;
                    if elapsed > decay_secs {
                        entity.transition(EntityState::Suspended, "decay: not observed", version);
                        deactivated += 1;
                    }
                }
            }
        }
        deactivated
    }
}

impl Default for EntityTracker {
    fn default() -> Self {
        Self::new()
    }
}
