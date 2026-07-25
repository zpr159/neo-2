//! Event bus with publish/subscribe, broadcast, filtering, priority events,
//! persistent events, and replay.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::config::EventBusConfig;

/// Unique event identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(pub Uuid);

impl EventId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

/// Priority level for events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EventPriority {
    Background = 0,
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

impl Default for EventPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// A published event with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub topic: String,
    pub payload: serde_json::Value,
    pub priority: EventPriority,
    pub timestamp_ms: u64,
    pub source: String,
    pub persistent: bool,
}

impl Event {
    /// Create a new event.
    pub fn new(
        topic: impl Into<String>,
        payload: serde_json::Value,
        source: impl Into<String>,
    ) -> Self {
        Self {
            id: EventId::new(),
            topic: topic.into(),
            payload,
            priority: EventPriority::default(),
            timestamp_ms: now_ms(),
            source: source.into(),
            persistent: false,
        }
    }

    /// Set the event priority.
    #[must_use]
    pub fn with_priority(mut self, priority: EventPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Mark the event as persistent.
    #[must_use]
    pub fn persistent(mut self) -> Self {
        self.persistent = true;
        self
    }
}

/// Filter for selecting events.
#[derive(Debug, Clone, Default)]
pub struct EventFilter {
    pub topics: Option<Vec<String>>,
    pub min_priority: Option<EventPriority>,
    pub source_filter: Option<String>,
}

impl EventFilter {
    /// Check whether an event matches this filter.
    pub fn matches(&self, event: &Event) -> bool {
        if let Some(ref topics) = self.topics {
            if !topics.iter().any(|t| t == &event.topic) {
                return false;
            }
        }
        if let Some(min) = self.min_priority {
            if event.priority < min {
                return false;
            }
        }
        if let Some(ref source) = self.source_filter {
            if event.source != *source {
                return false;
            }
        }
        true
    }
}

/// Unique subscription identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubscriptionId(pub Uuid);

impl SubscriptionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SubscriptionId {
    fn default() -> Self {
        Self::new()
    }
}

/// A subscription record.
#[derive(Debug, Clone)]
pub struct Subscription {
    pub id: SubscriptionId,
    pub filter: EventFilter,
    pub name: String,
}

/// Event bus statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventStatistics {
    pub events_published: u64,
    pub events_delivered: u64,
    pub events_dropped: u64,
    pub active_subscriptions: usize,
    pub persistent_events_stored: usize,
}

/// Thread-safe event bus.
pub struct EventBus {
    sender: broadcast::Sender<Event>,
    subscriptions: RwLock<HashMap<SubscriptionId, Subscription>>,
    persistent_store: RwLock<VecDeque<Event>>,
    stats: EventStatistics,
    config: EventBusConfig,
}

impl EventBus {
    /// Create a new event bus with the given configuration.
    pub fn new(config: EventBusConfig) -> Self {
        let (sender, _) = broadcast::channel(config.broadcast_capacity);
        Self {
            sender,
            subscriptions: RwLock::new(HashMap::new()),
            persistent_store: RwLock::new(VecDeque::new()),
            stats: EventStatistics::default(),
            config,
        }
    }

    /// Publish an event to all subscribers.
    pub fn publish(&mut self, event: Event) -> Result<(), String> {
        if event.persistent {
            let mut store = self.persistent_store.write();
            if store.len() >= self.config.persistent_event_limit {
                store.pop_front();
            }
            store.push_back(event.clone());
        }

        self.sender
            .send(event)
            .map_err(|e| format!("publish failed: {}", e))?;
        self.stats.events_published += 1;
        Ok(())
    }

    /// Subscribe to events matching the given filter.
    pub fn subscribe(&self, name: impl Into<String>, filter: EventFilter) -> SubscriptionId {
        let id = SubscriptionId::new();
        let sub = Subscription {
            id,
            filter,
            name: name.into(),
        };
        self.subscriptions.write().insert(id, sub);
        id
    }

    /// Subscribe to all events.
    pub fn subscribe_all(&self, name: impl Into<String>) -> SubscriptionId {
        self.subscribe(name, EventFilter::default())
    }

    /// Unsubscribe by ID.
    pub fn unsubscribe(&self, id: SubscriptionId) -> bool {
        self.subscriptions.write().remove(&id).is_some()
    }

    /// Get a broadcast receiver for raw event consumption.
    pub fn receiver(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }

    /// Replay all persistent events.
    pub fn replay_persistent(&self) -> Vec<Event> {
        self.persistent_store.read().iter().cloned().collect()
    }

    /// Replay persistent events matching a filter.
    pub fn replay_filtered(&self, filter: &EventFilter) -> Vec<Event> {
        self.persistent_store
            .read()
            .iter()
            .filter(|e| filter.matches(e))
            .cloned()
            .collect()
    }

    /// Clear the persistent event store.
    pub fn clear_persistent(&mut self) {
        self.persistent_store.write().clear();
    }

    /// Get statistics.
    pub fn statistics(&self) -> EventStatistics {
        let mut stats = self.stats.clone();
        stats.active_subscriptions = self.subscriptions.read().len();
        stats.persistent_events_stored = self.persistent_store.read().len();
        stats
    }

    /// Get the number of active subscriptions.
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.read().len()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(EventBusConfig::default())
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_creation() {
        let event = Event::new("test.topic", serde_json::json!({"key": "value"}), "source");
        assert_eq!(event.topic, "test.topic");
        assert!(event.persistent);
        assert!(!event.persistent);
    }

    #[test]
    fn publish_and_receive() {
        let mut bus = EventBus::new(EventBusConfig::default());
        let mut rx = bus.receiver();

        let event = Event::new("test", serde_json::json!(42), "src");
        bus.publish(event).unwrap();

        let received = rx.try_recv();
        assert!(received.is_ok());
        assert_eq!(received.unwrap().topic, "test");
    }

    #[test]
    fn filter_matching() {
        let filter = EventFilter {
            topics: Some(vec!["topic.a".to_string(), "topic.b".to_string()]),
            min_priority: Some(EventPriority::High),
            source_filter: None,
        };

        let high_event = Event::new(
            "topic.a",
            serde_json::json!(null),
            "src",
        )
        .with_priority(EventPriority::High);
        assert!(filter.matches(&high_event));

        let low_event = Event::new(
            "topic.a",
            serde_json::json!(null),
            "src",
        )
        .with_priority(EventPriority::Low);
        assert!(!filter.matches(&low_event));

        let wrong_topic = Event::new(
            "topic.c",
            serde_json::json!(null),
            "src",
        )
        .with_priority(EventPriority::Critical);
        assert!(!filter.matches(&wrong_topic));
    }

    #[test]
    fn subscribe_and_unsubscribe() {
        let bus = EventBus::new(EventBusConfig::default());
        let id = bus.subscribe("test", EventFilter::default());
        assert_eq!(bus.subscription_count(), 1);

        bus.unsubscribe(id);
        assert_eq!(bus.subscription_count(), 0);
    }

    #[test]
    fn persistent_events() {
        let mut bus = EventBus::new(EventBusConfig::default());

        let event = Event::new("topic", serde_json::json!(1), "src").persistent();
        bus.publish(event).unwrap();

        let persistent = bus.replay_persistent();
        assert_eq!(persistent.len(), 1);
        assert_eq!(persistent[0].topic, "topic");
    }

    #[test]
    fn replay_filtered() {
        let mut bus = EventBus::new(EventBusConfig::default());

        bus.publish(Event::new("a", serde_json::json!(1), "src").persistent())
            .unwrap();
        bus.publish(Event::new("b", serde_json::json!(2), "src").persistent())
            .unwrap();
        bus.publish(Event::new("a", serde_json::json!(3), "src").persistent())
            .unwrap();

        let filter = EventFilter {
            topics: Some(vec!["a".to_string()]),
            ..EventFilter::default()
        };
        let replayed = bus.replay_filtered(&filter);
        assert_eq!(replayed.len(), 2);
    }

    #[test]
    fn statistics() {
        let mut bus = EventBus::new(EventBusConfig::default());
        bus.subscribe("s1", EventFilter::default());
        bus.subscribe("s2", EventFilter::default());
        bus.publish(Event::new("t", serde_json::json!(null), "src"))
            .unwrap();

        let stats = bus.statistics();
        assert_eq!(stats.events_published, 1);
        assert_eq!(stats.active_subscriptions, 2);
    }

    #[test]
    fn event_priority() {
        let e1 = Event::new("t", serde_json::json!(null), "s")
            .with_priority(EventPriority::Critical);
        let e2 = Event::new("t", serde_json::json!(null), "s")
            .with_priority(EventPriority::Low);
        assert!(e1.priority > e2.priority);
    }

    #[test]
    fn clear_persistent() {
        let mut bus = EventBus::new(EventBusConfig::default());
        bus.publish(Event::new("t", serde_json::json!(null), "s").persistent())
            .unwrap();
        assert_eq!(bus.replay_persistent().len(), 1);

        bus.clear_persistent();
        assert_eq!(bus.replay_persistent().len(), 0);
    }
}
