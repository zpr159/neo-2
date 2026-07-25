use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::{Confidence, EntityId, EventId, LocationId, ObservationId, ObservationSource, AttributeValue};

/// An observation about the world from any source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub id: ObservationId,
    pub content: String,
    pub observation_type: ObservationType,
    pub source: ObservationSource,
    pub source_id: Option<String>,
    pub confidence: Confidence,
    pub entities_mentioned: Vec<EntityId>,
    pub events_mentioned: Vec<EventId>,
    pub locations_mentioned: Vec<LocationId>,
    pub properties: HashMap<String, AttributeValue>,
    pub observed_at: DateTime<Utc>,
    pub recorded_at: DateTime<Utc>,
    pub raw_data: Option<serde_json::Value>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Type of observation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObservationType {
    StateReport,
    EventReport,
    EntityUpdate,
    RelationshipUpdate,
    LocationUpdate,
    ErrorReport,
    StatusChange,
    Measurement,
    Custom(String),
}

impl std::fmt::Display for ObservationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StateReport => write!(f, "state_report"),
            Self::EventReport => write!(f, "event_report"),
            Self::EntityUpdate => write!(f, "entity_update"),
            Self::RelationshipUpdate => write!(f, "relationship_update"),
            Self::LocationUpdate => write!(f, "location_update"),
            Self::ErrorReport => write!(f, "error_report"),
            Self::StatusChange => write!(f, "status_change"),
            Self::Measurement => write!(f, "measurement"),
            Self::Custom(name) => write!(f, "{name}"),
        }
    }
}

impl Observation {
    pub fn new(
        content: impl Into<String>,
        observation_type: ObservationType,
        source: ObservationSource,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: ObservationId::random(),
            content: content.into(),
            observation_type,
            source,
            source_id: None,
            confidence: Confidence::MEDIUM,
            entities_mentioned: Vec::new(),
            events_mentioned: Vec::new(),
            locations_mentioned: Vec::new(),
            properties: HashMap::new(),
            observed_at: now,
            recorded_at: now,
            raw_data: None,
            metadata: HashMap::new(),
        }
    }
}

/// Pipeline stage for processing observations.
pub struct ObservationPipeline {
    buffer: dashmap::DashMap<String, Observation>,
    processed_count: std::sync::atomic::AtomicU64,
}

impl ObservationPipeline {
    #[must_use]
    pub fn new() -> Self {
        Self {
            buffer: dashmap::DashMap::new(),
            processed_count: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn submit(&self, observation: Observation) {
        let id = observation.id.0.clone();
        self.buffer.insert(id, observation);
    }

    pub fn next_batch(&self, max: usize) -> Vec<Observation> {
        let mut batch = Vec::new();
        let keys: Vec<String> = self.buffer.iter().take(max).map(|e| e.key().clone()).collect();
        for key in keys {
            if let Some((_, obs)) = self.buffer.remove(&key) {
                batch.push(obs);
                self.processed_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
        batch
    }

    pub fn pending_count(&self) -> usize {
        self.buffer.len()
    }

    pub fn processed_count(&self) -> u64 {
        self.processed_count.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl Default for ObservationPipeline {
    fn default() -> Self {
        Self::new()
    }
}
