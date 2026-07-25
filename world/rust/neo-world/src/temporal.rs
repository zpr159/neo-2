use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::{
    Confidence, EntityId, EventId, EventType, AttributeValue,
};

/// A temporal event in the world model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalEvent {
    pub id: EventId,
    pub description: String,
    pub event_type: EventType,
    pub occurred_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub participants: Vec<EntityId>,
    pub location: Option<String>,
    pub properties: HashMap<String, AttributeValue>,
    pub confidence: Confidence,
    pub caused_by: Vec<EventId>,
    pub causes: Vec<EventId>,
    pub recorded_at: DateTime<Utc>,
    pub source: String,
}

impl TemporalEvent {
    pub fn new(description: impl Into<String>, event_type: EventType) -> Self {
        let now = Utc::now();
        Self {
            id: EventId::random(),
            description: description.into(),
            event_type,
            occurred_at: now,
            ended_at: None,
            participants: Vec::new(),
            location: None,
            properties: HashMap::new(),
            confidence: Confidence::MEDIUM,
            caused_by: Vec::new(),
            causes: Vec::new(),
            recorded_at: now,
            source: String::new(),
        }
    }

    pub fn duration_secs(&self) -> Option<f64> {
        self.ended_at.map(|end| {
            end.signed_duration_since(self.occurred_at).num_milliseconds() as f64 / 1000.0
        })
    }
}

/// A time window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeWindow {
    pub start: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
    pub label: String,
}

impl TimeWindow {
    pub fn open(label: impl Into<String>, start: DateTime<Utc>) -> Self {
        Self { start, end: None, label: label.into() }
    }

    pub fn closed(label: impl Into<String>, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self { start, end: Some(end), label: label.into() }
    }

    pub fn contains(&self, dt: &DateTime<Utc>) -> bool {
        if *dt < self.start {
            return false;
        }
        match &self.end {
            Some(end) => dt <= end,
            None => true,
        }
    }

    pub fn duration_secs(&self) -> Option<f64> {
        self.end.map(|e| {
            e.signed_duration_since(self.start).num_milliseconds() as f64 / 1000.0
        })
    }
}

/// An entry in a timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEntry {
    pub event_id: EventId,
    pub timestamp: DateTime<Utc>,
    pub description: String,
    pub event_type: EventType,
    pub importance: f32,
}

/// A chronological timeline of events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timeline {
    pub name: String,
    pub entries: Vec<TimelineEntry>,
}

impl Timeline {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            entries: Vec::new(),
        }
    }

    pub fn add(&mut self, event: &TemporalEvent) {
        self.entries.push(TimelineEntry {
            event_id: event.id.clone(),
            timestamp: event.occurred_at,
            description: event.description.clone(),
            event_type: event.event_type.clone(),
            importance: event.confidence.value(),
        });
        self.entries.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    }

    pub fn events_in_window(&self, window: &TimeWindow) -> Vec<&TimelineEntry> {
        self.entries.iter().filter(|e| window.contains(&e.timestamp)).collect()
    }
}

/// Historical state of the world at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalState {
    pub timestamp: DateTime<Utc>,
    pub version: u64,
    pub entity_count: usize,
    pub event_count: usize,
    pub snapshot: serde_json::Value,
}

/// Manages temporal knowledge.
pub struct TemporalModel {
    events: dashmap::DashMap<EventId, TemporalEvent>,
    timeline: Vec<EventId>,
    timelines: dashmap::DashMap<String, Timeline>,
    historical_states: Vec<HistoricalState>,
}

impl TemporalModel {
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: dashmap::DashMap::new(),
            timeline: Vec::new(),
            timelines: dashmap::DashMap::new(),
            historical_states: Vec::new(),
        }
    }

    pub fn record_event(&mut self, event: TemporalEvent) -> EventId {
        let id = event.id.clone();
        self.events.insert(id.clone(), event);
        self.timeline.push(id.clone());
        id
    }

    pub fn get_event(&self, id: &EventId) -> Option<TemporalEvent> {
        self.events.get(id).map(|e| e.value().clone())
    }

    pub fn chronological(&self) -> Vec<TemporalEvent> {
        let mut events: Vec<TemporalEvent> = self
            .timeline
            .iter()
            .filter_map(|id| self.events.get(id).map(|e| e.value().clone()))
            .collect();
        events.sort_by(|a, b| a.occurred_at.cmp(&b.occurred_at));
        events
    }

    pub fn events_in_window(&self, window: &TimeWindow) -> Vec<TemporalEvent> {
        self.events
            .iter()
            .filter(|e| window.contains(&e.value().occurred_at))
            .map(|e| e.value().clone())
            .collect()
    }

    pub fn events_for_entity(&self, entity_id: &EntityId) -> Vec<TemporalEvent> {
        self.events
            .iter()
            .filter(|e| e.value().participants.contains(entity_id))
            .map(|e| e.value().clone())
            .collect()
    }

    pub fn events_by_type(&self, event_type: &EventType) -> Vec<TemporalEvent> {
        self.events
            .iter()
            .filter(|e| &e.value().event_type == event_type)
            .map(|e| e.value().clone())
            .collect()
    }

    pub fn recent(&self, count: usize) -> Vec<TemporalEvent> {
        let mut events = self.chronological();
        events.reverse();
        events.into_iter().take(count).collect()
    }

    pub fn get_or_create_timeline(&self, name: &str) -> dashmap::mapref::one::RefMut<'_, String, Timeline> {
        self.timelines
            .entry(name.to_string())
            .or_insert_with(|| Timeline::new(name))
    }

    pub fn record_historical_state(&mut self, state: HistoricalState) {
        self.historical_states.push(state);
    }

    pub fn historical_states(&self) -> &[HistoricalState] {
        &self.historical_states
    }

    pub fn count(&self) -> usize {
        self.events.len()
    }
}

impl Default for TemporalModel {
    fn default() -> Self {
        Self::new()
    }
}
