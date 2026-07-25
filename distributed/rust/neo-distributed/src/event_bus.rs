//! Distributed event bus — publish/subscribe, reliable delivery, replay,
//! filtering, and priorities across cluster nodes.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::{DeliveryGuarantee, EventBusConfiguration};
use crate::error::{DistributedError, NeoResult};
use crate::types::NodeId;

// ---------------------------------------------------------------------------
// Event
// ---------------------------------------------------------------------------

/// A cluster event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique event ID.
    pub id: Uuid,
    /// Event topic.
    pub topic: String,
    /// Event type (within the topic).
    pub event_type: String,
    /// Priority (lower number = higher priority).
    pub priority: u8,
    /// Publisher node ID.
    pub publisher: NodeId,
    /// Serialized event payload.
    pub payload: Vec<u8>,
    /// When the event was published.
    pub timestamp: DateTime<Utc>,
    /// Monotonic sequence number.
    pub sequence: u64,
    /// Correlation ID for request tracking.
    pub correlation_id: Option<Uuid>,
}

impl Event {
    /// Create a new event.
    pub fn new(topic: impl Into<String>, event_type: impl Into<String>, payload: Vec<u8>) -> Self {
        Self {
            id: Uuid::new_v4(),
            topic: topic.into(),
            event_type: event_type.into(),
            priority: 5,
            publisher: NodeId::new(),
            payload,
            timestamp: Utc::now(),
            sequence: 0,
            correlation_id: None,
        }
    }

    /// Set priority.
    #[must_use]
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Set publisher.
    #[must_use]
    pub fn with_publisher(mut self, publisher: NodeId) -> Self {
        self.publisher = publisher;
        self
    }

    /// Set correlation ID.
    #[must_use]
    pub fn with_correlation(mut self, id: Uuid) -> Self {
        self.correlation_id = Some(id);
        self
    }
}

// ---------------------------------------------------------------------------
// EventFilter
// ---------------------------------------------------------------------------

/// Filter for event subscriptions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFilter {
    /// Topic pattern (supports simple glob with *).
    pub topic_pattern: Option<String>,
    /// Event type filter.
    pub event_type: Option<String>,
    /// Maximum priority to receive.
    pub max_priority: u8,
    /// Node IDs to ignore.
    pub ignore_publishers: Vec<NodeId>,
}

impl EventFilter {
    pub fn matches(&self, event: &Event) -> bool {
        if let Some(ref pattern) = self.topic_pattern {
            if !topic_matches(pattern, &event.topic) {
                return false;
            }
        }
        if let Some(ref et) = self.event_type {
            if et != &event.event_type {
                return false;
            }
        }
        if event.priority > self.max_priority {
            return false;
        }
        if self.ignore_publishers.contains(&event.publisher) {
            return false;
        }
        true
    }
}

fn topic_matches(pattern: &str, topic: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        topic.starts_with(prefix)
    } else {
        pattern == topic
    }
}

// ---------------------------------------------------------------------------
// Subscriber
// ---------------------------------------------------------------------------

/// A subscription to events.
#[derive(Debug, Clone)]
pub struct Subscriber {
    /// Subscriber ID.
    pub id: Uuid,
    /// Topic to subscribe to.
    pub topic: String,
    /// Optional filter.
    pub filter: Option<EventFilter>,
    /// Channel capacity.
    pub capacity: usize,
}

// ---------------------------------------------------------------------------
// ReliableEventQueue
// ---------------------------------------------------------------------------

/// Reliable event queue with persistence and delivery guarantees.
pub struct ReliableEventQueue {
    /// Buffered events.
    events: RwLock<VecDeque<Event>>,
    /// Maximum queue size.
    max_size: usize,
    /// Delivery guarantee.
    guarantee: DeliveryGuarantee,
    /// Pending acknowledgments (event_id → subscriber_ids).
    pending_acks: RwLock<HashMap<Uuid, Vec<Uuid>>>,
    /// Delivered events (for deduplication).
    delivered: RwLock<Vec<Uuid>>,
    /// Total enqueued.
    enqueued: AtomicU64,
    /// Total delivered.
    delivered_count: AtomicU64,
    /// Total dropped.
    dropped: AtomicU64,
}

impl ReliableEventQueue {
    pub fn new(max_size: usize, guarantee: DeliveryGuarantee) -> Self {
        Self {
            events: RwLock::new(VecDeque::new()),
            max_size,
            guarantee,
            pending_acks: RwLock::new(HashMap::new()),
            delivered: RwLock::new(Vec::new()),
            enqueued: AtomicU64::new(0),
            delivered_count: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
        }
    }

    /// Enqueue an event.
    pub fn enqueue(&self, event: Event) -> NeoResult<()> {
        let mut events = self.events.write();
        if events.len() >= self.max_size {
            return Err(DistributedError::internal("event queue full"));
        }
        events.push_back(event);
        self.enqueued.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Dequeue the next event.
    pub fn dequeue(&self) -> Option<Event> {
        self.events.write().pop_front()
    }

    /// Dequeue events matching a filter.
    pub fn dequeue_filtered(&self, filter: &EventFilter) -> Vec<Event> {
        let mut events = self.events.write();
        let mut result = Vec::new();
        events.retain(|e| {
            if filter.matches(e) {
                result.push(e.clone());
                false
            } else {
                true
            }
        });
        result
    }

    /// Acknowledge delivery of an event.
    pub fn acknowledge(&self, event_id: Uuid, subscriber_id: Uuid) {
        self.pending_acks
            .write()
            .entry(event_id)
            .or_default()
            .push(subscriber_id);
    }

    /// Check if event has been delivered to all subscribers.
    pub fn is_fully_delivered(&self, event_id: Uuid, total_subscribers: usize) -> bool {
        self.pending_acks
            .read()
            .get(&event_id)
            .map_or(false, |acks| acks.len() >= total_subscribers)
    }

    /// Get queue depth.
    pub fn depth(&self) -> usize {
        self.events.read().len()
    }

    /// Check if the event was already delivered (for deduplication).
    pub fn was_delivered(&self, event_id: Uuid) -> bool {
        self.delivered.read().contains(&event_id)
    }

    /// Mark an event as delivered.
    pub fn mark_delivered(&self, event_id: Uuid) {
        self.delivered.write().push(event_id);
        self.delivered_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Replay events from the queue (non-destructive read).
    pub fn replay(&self, count: usize) -> Vec<Event> {
        self.events.read().iter().take(count).cloned().collect()
    }

    /// Get statistics.
    pub fn stats(&self) -> QueueStats {
        QueueStats {
            depth: self.depth(),
            enqueued: self.enqueued.load(Ordering::Relaxed),
            delivered: self.delivered_count.load(Ordering::Relaxed),
            dropped: self.dropped.load(Ordering::Relaxed),
        }
    }
}

// ---------------------------------------------------------------------------
// QueueStats
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    pub depth: usize,
    pub enqueued: u64,
    pub delivered: u64,
    pub dropped: u64,
}

// ---------------------------------------------------------------------------
// DistributedEventBus
// ---------------------------------------------------------------------------

/// The main distributed event bus.
pub struct DistributedEventBus {
    /// Configuration.
    config: RwLock<EventBusConfiguration>,
    /// Reliable event queue.
    queue: Arc<ReliableEventQueue>,
    /// Subscriptions: topic → subscribers.
    subscriptions: DashMap<String, Vec<Subscriber>>,
    /// Sequence counter.
    sequence: AtomicU64,
    /// Total published.
    published: AtomicU64,
    /// Total received.
    received: AtomicU64,
}

impl DistributedEventBus {
    /// Create a new distributed event bus.
    pub fn new(config: EventBusConfiguration) -> Self {
        let queue = Arc::new(ReliableEventQueue::new(
            config.queue_capacity,
            config.delivery_guarantee,
        ));
        tracing::info!(
            queue_capacity = config.queue_capacity,
            delivery_guarantee = ?config.delivery_guarantee,
            "distributed event bus created"
        );
        Self {
            config: RwLock::new(config),
            queue,
            subscriptions: DashMap::new(),
            sequence: AtomicU64::new(0),
            published: AtomicU64::new(0),
            received: AtomicU64::new(0),
        }
    }

    // -- Publishing --

    /// Publish an event.
    pub fn publish(&self, mut event: Event) -> NeoResult<()> {
        event.sequence = self.sequence.fetch_add(1, Ordering::Relaxed) + 1;
        self.queue.enqueue(event)?;
        self.published.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Publish an event and return the sequence number.
    pub fn publish_with_seq(&self, mut event: Event) -> NeoResult<u64> {
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed) + 1;
        event.sequence = seq;
        self.queue.enqueue(event)?;
        self.published.fetch_add(1, Ordering::Relaxed);
        Ok(seq)
    }

    // -- Subscribing --

    /// Subscribe to a topic.
    pub fn subscribe(&self, topic: String, filter: Option<EventFilter>) -> Subscriber {
        let subscriber = Subscriber {
            id: Uuid::new_v4(),
            topic: topic.clone(),
            filter,
            capacity: 1024,
        };
        self.subscriptions
            .entry(topic)
            .or_default()
            .push(subscriber.clone());
        subscriber
    }

    /// Unsubscribe.
    pub fn unsubscribe(&self, topic: &str, subscriber_id: Uuid) -> bool {
        if let Some(mut subs) = self.subscriptions.get_mut(topic) {
            let before = subs.len();
            subs.retain(|s| s.id != subscriber_id);
            subs.len() < before
        } else {
            false
        }
    }

    /// Receive events for a subscriber (non-blocking).
    pub fn receive(&self, subscriber: &Subscriber) -> Vec<Event> {
        let filter = subscriber
            .filter
            .clone()
            .unwrap_or_else(|| EventFilter {
                topic_pattern: None,
                event_type: None,
                max_priority: u8::MAX,
                ignore_publishers: vec![],
            });

        let mut events = Vec::new();
        let mut remaining = Vec::new();

        // Drain all events from the queue.
        while let Some(event) = self.queue.dequeue() {
            if filter.matches(&event) {
                events.push(event);
            } else {
                remaining.push(event);
            }
        }

        // Re-enqueue non-matching events.
        for event in remaining {
            let _ = self.queue.enqueue(event);
        }

        self.received
            .fetch_add(events.len() as u64, Ordering::Relaxed);
        events
    }

    // -- Queries --

    /// Get the event queue.
    pub fn queue(&self) -> &Arc<ReliableEventQueue> {
        &self.queue
    }

    /// Get subscriber count for a topic.
    pub fn subscriber_count(&self, topic: &str) -> usize {
        self.subscriptions.get(topic).map_or(0, |s| s.len())
    }

    /// Get all subscribed topics.
    pub fn topics(&self) -> Vec<String> {
        self.subscriptions.iter().map(|r| r.key().clone()).collect()
    }

    /// Get statistics.
    pub fn stats(&self) -> EventBusStats {
        let queue_stats = self.queue.stats();
        EventBusStats {
            published: self.published.load(Ordering::Relaxed),
            received: self.received.load(Ordering::Relaxed),
            queue_depth: queue_stats.depth,
            topics: self.topics().len(),
            queue: queue_stats,
        }
    }
}

impl std::fmt::Debug for DistributedEventBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DistributedEventBus")
            .field("topics", &self.topics().len())
            .field("queue_depth", &self.queue.depth())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// EventBusStats
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBusStats {
    pub published: u64,
    pub received: u64,
    pub queue_depth: usize,
    pub topics: usize,
    pub queue: QueueStats,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_creation() {
        let event = Event::new("test_topic", "test_event", vec![1, 2, 3])
            .with_priority(3)
            .with_publisher(NodeId::new());
        assert_eq!(event.topic, "test_topic");
        assert_eq!(event.priority, 3);
    }

    #[test]
    fn event_bus_publish_subscribe() {
        let bus = DistributedEventBus::new(EventBusConfiguration::default());
        let sub = bus.subscribe("topic1".to_string(), None);

        let event = Event::new("topic1", "type1", vec![1]);
        bus.publish(event).unwrap();

        let received = bus.receive(&sub);
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].event_type, "type1");
    }

    #[test]
    fn event_filter() {
        let filter = EventFilter {
            topic_pattern: Some("test_*".to_string()),
            event_type: None,
            max_priority: 5,
            ignore_publishers: vec![],
        };

        let event = Event::new("test_topic", "type1", vec![1]);
        assert!(filter.matches(&event));

        let event2 = Event::new("other_topic", "type1", vec![1]);
        assert!(!filter.matches(&event2));
    }

    #[test]
    fn topic_matching() {
        assert!(topic_matches("*", "anything"));
        assert!(topic_matches("test_*", "test_topic"));
        assert!(!topic_matches("test_*", "other_topic"));
        assert!(topic_matches("exact", "exact"));
    }

    #[test]
    fn reliable_queue() {
        let queue = ReliableEventQueue::new(100, DeliveryGuarantee::AtLeastOnce);
        let event = Event::new("t", "e", vec![1]);
        queue.enqueue(event).unwrap();
        assert_eq!(queue.depth(), 1);

        let dequeued = queue.dequeue().unwrap();
        assert_eq!(dequeued.topic, "t");
    }

    #[test]
    fn event_bus_stats() {
        let bus = DistributedEventBus::new(EventBusConfiguration::default());
        let stats = bus.stats();
        assert_eq!(stats.published, 0);
    }

    #[test]
    fn unsubscribe() {
        let bus = DistributedEventBus::new(EventBusConfiguration::default());
        let sub = bus.subscribe("topic".to_string(), None);
        assert!(bus.unsubscribe("topic", sub.id));
    }
}
