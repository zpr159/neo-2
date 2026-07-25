use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::{EntityState, WorldVersion};

/// Valid state transitions for entities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleTransition {
    pub from: EntityState,
    pub to: EntityState,
    pub valid: bool,
}

impl LifecycleTransition {
    pub const fn new(from: EntityState, to: EntityState) -> Self {
        Self { from, to, valid: true }
    }
}

/// Check if a state transition is valid.
pub fn is_valid_transition(from: &EntityState, to: &EntityState) -> bool {
    matches!(
        (from, to),
        (EntityState::Created, EntityState::Active)
            | (EntityState::Created, EntityState::Deleted)
            | (EntityState::Active, EntityState::Suspended)
            | (EntityState::Active, EntityState::Updating)
            | (EntityState::Active, EntityState::Migrating)
            | (EntityState::Active, EntityState::Archived)
            | (EntityState::Active, EntityState::Deleted)
            | (EntityState::Suspended, EntityState::Active)
            | (EntityState::Suspended, EntityState::Deleted)
            | (EntityState::Updating, EntityState::Active)
            | (EntityState::Updating, EntityState::Suspended)
            | (EntityState::Migrating, EntityState::Active)
            | (EntityState::Migrating, EntityState::Suspended)
            | (EntityState::Archived, EntityState::Active)
            | (EntityState::Archived, EntityState::Deleted)
    )
}

/// A lifecycle event for an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleEvent {
    pub from_state: EntityState,
    pub to_state: EntityState,
    pub timestamp: DateTime<Utc>,
    pub reason: String,
    pub version: WorldVersion,
}

impl LifecycleEvent {
    pub fn new(from: EntityState, to: EntityState, reason: impl Into<String>, version: WorldVersion) -> Self {
        Self {
            from_state: from,
            to_state: to,
            timestamp: Utc::now(),
            reason: reason.into(),
            version,
        }
    }
}

/// Manages entity lifecycle transitions.
pub struct LifecycleManager {
    events: dashmap::DashMap<String, Vec<LifecycleEvent>>,
}

impl LifecycleManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: dashmap::DashMap::new(),
        }
    }

    /// Record a lifecycle event.
    pub fn record_transition(&self, entity_id: &str, event: LifecycleEvent) {
        self.events
            .entry(entity_id.to_string())
            .or_default()
            .push(event);
    }

    /// Get the lifecycle history for an entity.
    pub fn history(&self, entity_id: &str) -> Vec<LifecycleEvent> {
        self.events
            .get(entity_id)
            .map(|h| h.value().clone())
            .unwrap_or_default()
    }

    /// Get the first lifecycle event for an entity.
    pub fn created_at(&self, entity_id: &str) -> Option<DateTime<Utc>> {
        self.events
            .get(entity_id)
            .and_then(|h| h.value().first().map(|e| e.timestamp))
    }

    /// Get the last state transition.
    pub fn last_transition(&self, entity_id: &str) -> Option<LifecycleEvent> {
        self.events
            .get(entity_id)
            .and_then(|h| h.value().last().cloned())
    }

    /// Count transitions for an entity.
    pub fn transition_count(&self, entity_id: &str) -> usize {
        self.events
            .get(entity_id)
            .map(|h| h.value().len())
            .unwrap_or(0)
    }
}

impl Default for LifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}
