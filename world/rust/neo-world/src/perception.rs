use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::observation::Observation;
use crate::types::{Confidence, EntityId, LocationId, PerceptionId, ObservationSource, AttributeValue};

/// A processed perception derived from observations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Perception {
    pub id: PerceptionId,
    pub content: String,
    pub location: Option<LocationId>,
    pub entities: Vec<EntityId>,
    pub relationships: Vec<PerceivedRelationship>,
    pub events: Vec<String>,
    pub properties: HashMap<String, AttributeValue>,
    pub source: ObservationSource,
    pub confidence: Confidence,
    pub observed_at: DateTime<Utc>,
    pub recorded_at: DateTime<Utc>,
    pub raw_data: Option<serde_json::Value>,
    pub fused_from: Vec<String>,
}

/// A relationship perceived between entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceivedRelationship {
    pub source: EntityId,
    pub target: EntityId,
    pub relationship_type: String,
    pub confidence: Confidence,
}

impl Perception {
    pub fn from_observation(obs: &Observation) -> Self {
        Self {
            id: PerceptionId::random(),
            content: obs.content.clone(),
            location: obs.locations_mentioned.first().cloned(),
            entities: obs.entities_mentioned.clone(),
            relationships: Vec::new(),
            events: obs.events_mentioned.iter().map(|e| e.to_string()).collect(),
            properties: obs.properties.clone(),
            source: obs.source.clone(),
            confidence: obs.confidence,
            observed_at: obs.observed_at,
            recorded_at: Utc::now(),
            raw_data: obs.raw_data.clone(),
            fused_from: vec![obs.id.0.clone()],
        }
    }
}

/// Processes observations into perceptions.
pub struct PerceptionProcessor;

impl PerceptionProcessor {
    pub fn extract_entity_mentions(content: &str) -> Vec<String> {
        let mut mentions = Vec::new();
        for word in content.split_whitespace() {
            if word.starts_with('@') && word.len() > 1 {
                mentions.push(word[1..].to_string());
            }
        }
        mentions
    }

    pub fn extract_location_mentions(content: &str) -> Vec<String> {
        let mut mentions = Vec::new();
        for word in content.split_whitespace() {
            if word.starts_with('#') && word.len() > 1 {
                mentions.push(word[1..].to_string());
            }
        }
        mentions
    }

    pub fn source_confidence(source: &ObservationSource) -> Confidence {
        match source {
            ObservationSource::ToolResult | ObservationSource::KnowledgeGraph => Confidence::HIGH,
            ObservationSource::AgentCommunication | ObservationSource::Conversation => Confidence::MEDIUM,
            ObservationSource::Memory | ObservationSource::Reasoning => Confidence::LOW,
            _ => Confidence::MEDIUM,
        }
    }
}

/// Buffer and deduplication for perceptions.
pub struct PerceptionBuffer {
    buffer: dashmap::DashMap<String, Perception>,
    history: Vec<Perception>,
    max_history: usize,
}

impl PerceptionBuffer {
    pub fn new(max_history: usize) -> Self {
        Self {
            buffer: dashmap::DashMap::new(),
            history: Vec::new(),
            max_history,
        }
    }

    pub fn add(&mut self, perception: Perception) {
        let id = perception.id.0.clone();
        self.buffer.insert(id, perception.clone());
        self.history.push(perception);
        if self.history.len() > self.max_history {
            self.history.drain(..self.history.len() - self.max_history);
        }
    }

    pub fn recent(&self, count: usize) -> Vec<&Perception> {
        self.history.iter().rev().take(count).collect()
    }

    pub fn pending_count(&self) -> usize {
        self.buffer.len()
    }

    pub fn clear_processed(&mut self) {
        self.buffer.clear();
    }
}

/// Fuses multiple perceptions into one.
pub struct PerceptionFusion;

impl PerceptionFusion {
    pub fn fuse(perceptions: &[Perception]) -> Option<Perception> {
        if perceptions.is_empty() {
            return None;
        }

        let first = &perceptions[0];
        let mut fused = first.clone();
        fused.id = PerceptionId::random();
        fused.fused_from = perceptions.iter().map(|p| p.id.0.clone()).collect();

        let mut all_entities: Vec<EntityId> = perceptions.iter().flat_map(|p| p.entities.clone()).collect();
        all_entities.dedup();
        fused.entities = all_entities;

        let mut all_events: Vec<String> = perceptions.iter().flat_map(|p| p.events.clone()).collect();
        all_events.dedup();
        fused.events = all_events;

        let avg_confidence: f32 = perceptions.iter().map(|p| p.confidence.value()).sum::<f32>()
            / perceptions.len() as f32;
        fused.confidence = Confidence(avg_confidence);

        Some(fused)
    }

    pub fn remove_duplicates(perceptions: &mut Vec<Perception>) {
        let mut seen = std::collections::HashSet::new();
        perceptions.retain(|p| seen.insert(p.content.clone()));
    }
}

impl Default for PerceptionBuffer {
    fn default() -> Self {
        Self::new(1000)
    }
}
