//! Message bus with typed messages, serialization, routing, request/reply,
//! and streaming.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::config::MessageBusConfig;
use crate::error::{RuntimeError, RuntimeErrorKind};

/// Unique message identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(pub Uuid);

impl MessageId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

/// A message flowing through the bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub topic: String,
    pub payload: Vec<u8>,
    pub content_type: String,
    pub timestamp_ms: u64,
    pub reply_to: Option<String>,
    pub correlation_id: Option<Uuid>,
    pub headers: HashMap<String, String>,
}

impl Message {
    /// Create a new message on the given topic.
    pub fn new(topic: impl Into<String>, payload: Vec<u8>) -> Self {
        Self {
            id: MessageId::new(),
            topic: topic.into(),
            payload,
            content_type: "application/octet-stream".to_string(),
            timestamp_ms: now_ms(),
            reply_to: None,
            correlation_id: None,
            headers: HashMap::new(),
        }
    }

    /// Set the content type.
    #[must_use]
    pub fn with_content_type(mut self, ct: impl Into<String>) -> Self {
        self.content_type = ct.into();
        self
    }

    /// Set a reply-to topic.
    #[must_use]
    pub fn with_reply_to(mut self, reply_to: impl Into<String>) -> Self {
        self.reply_to = Some(reply_to.into());
        self
    }

    /// Set a correlation ID for request/reply matching.
    #[must_use]
    pub fn with_correlation_id(mut self, id: Uuid) -> Self {
        self.correlation_id = Some(id);
        self
    }

    /// Add a header.
    #[must_use]
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Serialize a typed payload into the message.
    pub fn from_json<T: Serialize>(topic: impl Into<String>, value: &T) -> Result<Self, RuntimeError> {
        let payload = serde_json::to_vec(value)
            .map_err(|e| RuntimeError::new(RuntimeErrorKind::Unknown, format!("json serialize: {}", e)))?;
        Ok(Self::new(topic, payload).with_content_type("application/json"))
    }

    /// Deserialize the payload as JSON.
    pub fn to_json<T: for<'de> Deserialize<'de>>(&self) -> Result<T, RuntimeError> {
        serde_json::from_slice(&self.payload)
            .map_err(|e| RuntimeError::new(RuntimeErrorKind::Unknown, format!("json deserialize: {}", e)))
    }
}

/// A topic with subscriber channels.
struct TopicChannel {
    sender: mpsc::Sender<Message>,
    subscriber_count: AtomicUsize,
}

/// Routing key for message delivery.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RouteKey {
    pub topic: String,
    pub tag: Option<String>,
}

/// A route entry.
#[derive(Debug, Clone)]
pub struct Route {
    pub key: RouteKey,
    pub target_topic: String,
    pub transform: Option<String>,
}

/// Request/reply context.
pub struct RequestContext {
    pub request_id: Uuid,
    pub reply_rx: oneshot::Receiver<Message>,
    pub timeout: Duration,
}

impl RequestContext {
    /// Wait for the reply, respecting the timeout.
    pub async fn await_reply(self) -> Result<Message, RuntimeError> {
        tokio::time::timeout(self.timeout, self.reply_rx)
            .await
            .map_err(|_| RuntimeError::timeout("request/reply timed out"))?
            .map_err(|_| RuntimeError::new(
                RuntimeErrorKind::Unknown,
                "reply channel closed",
            ))
    }
}

/// Streaming receiver for a topic.
pub struct StreamingReceiver {
    rx: mpsc::Receiver<Message>,
}

impl StreamingReceiver {
    /// Receive the next message, or None if the channel is closed.
    pub async fn next(&mut self) -> Option<Message> {
        self.rx.recv().await
    }
}

/// Message bus statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MessageBusStatistics {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub messages_routed: u64,
    pub request_replies: u64,
    pub active_topics: usize,
    pub total_subscribers: usize,
}

/// Thread-safe message bus.
pub struct MessageBus {
    topics: DashMap<String, Arc<TopicChannel>>,
    routes: RwLock<Vec<Route>>,
    config: MessageBusConfig,
    stats: RwLock<MessageBusStatistics>,
}

impl MessageBus {
    /// Create a new message bus with the given configuration.
    pub fn new(config: MessageBusConfig) -> Self {
        Self {
            topics: DashMap::new(),
            routes: RwLock::new(Vec::new()),
            config,
            stats: RwLock::new(MessageBusStatistics::default()),
        }
    }

    /// Register a topic and get a receiver for it.
    pub fn register_topic(&self, topic: &str) -> mpsc::Receiver<Message> {
        let (tx, rx) = mpsc::channel(self.config.message_buffer_size);
        let channel = Arc::new(TopicChannel {
            sender: tx,
            subscriber_count: AtomicUsize::new(0),
        });
        self.topics.insert(topic.to_string(), channel);
        rx
    }

    /// Subscribe to a topic. Returns a channel receiver.
    pub fn subscribe(&self, topic: &str) -> Result<mpsc::Receiver<Message>, RuntimeError> {
        let (tx, rx) = mpsc::channel(self.config.message_buffer_size);

        if let Some(channel) = self.topics.get(topic) {
            channel.subscriber_count.fetch_add(1, Ordering::Relaxed);
        } else {
            return Err(RuntimeError::new(
                RuntimeErrorKind::Service(format!("topic '{}' not registered", topic)),
                "subscribe failed",
            ));
        }

        let sender = self.topics.get(topic).unwrap().sender.clone();
        let topic_clone = topic.to_string();
        let config_size = self.config.message_buffer_size;

        let (forward_tx, mut forward_rx) = mpsc::channel::<Message>(config_size);

        tokio::spawn(async move {
            while let Some(msg) = forward_rx.recv().await {
                let _ = tx.send(msg).await;
            }
        });

        Ok(rx)
    }

    /// Send a message to a topic.
    pub fn send(&self, message: Message) -> Result<(), RuntimeError> {
        let topic = &message.topic;
        let channel = self
            .topics
            .get(topic)
            .ok_or_else(|| RuntimeError::new(
                RuntimeErrorKind::Service(format!("topic '{}' not found", topic)),
                "send failed",
            ))?;

        channel
            .sender
            .try_send(message)
            .map_err(|e| RuntimeError::new(
                RuntimeErrorKind::Scheduler(crate::error::SchedulerErrorKind::QueueFull),
                format!("send failed: {}", e),
            ))?;

        self.stats.write().messages_sent += 1;
        Ok(())
    }

    /// Send a message asynchronously.
    pub async fn send_async(&self, message: Message) -> Result<(), RuntimeError> {
        let topic = &message.topic;
        let channel = self
            .topics
            .get(topic)
            .ok_or_else(|| RuntimeError::new(
                RuntimeErrorKind::Service(format!("topic '{}' not found", topic)),
                "send_async failed",
            ))?;

        channel
            .sender
            .send(message)
            .await
            .map_err(|e| RuntimeError::new(
                RuntimeErrorKind::Unknown,
                format!("send_async failed: {}", e),
            ))?;

        self.stats.write().messages_sent += 1;
        Ok(())
    }

    /// Send a request and wait for a reply.
    pub async fn request_reply(
        &self,
        request: Message,
        reply_topic: &str,
    ) -> Result<Message, RuntimeError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let correlation_id = Uuid::new_v4();

        let request = request
            .with_reply_to(reply_topic)
            .with_correlation_id(correlation_id);

        let timeout = Duration::from_millis(self.config.request_reply_timeout_ms);

        self.send_async(request).await?;

        tokio::time::timeout(timeout, reply_rx)
            .await
            .map_err(|_| RuntimeError::timeout("request/reply timed out"))?
            .map_err(|_| RuntimeError::new(
                RuntimeErrorKind::Unknown,
                "reply channel closed",
            ))
    }

    /// Add a routing rule.
    pub fn add_route(&self, route: Route) {
        self.routes.write().push(route);
        self.stats.write().messages_routed += 1;
    }

    /// Get a streaming receiver for a topic.
    pub fn stream(&self, topic: &str) -> Result<StreamingReceiver, RuntimeError> {
        let rx = self.register_topic(topic);
        Ok(StreamingReceiver { rx })
    }

    /// Get the number of registered topics.
    pub fn topic_count(&self) -> usize {
        self.topics.len()
    }

    /// Get statistics.
    pub fn statistics(&self) -> MessageBusStatistics {
        let mut stats = self.stats.read().clone();
        stats.active_topics = self.topics.len();
        let mut total_subs = 0;
        for entry in self.topics.iter() {
            total_subs += entry.subscriber_count.load(Ordering::Relaxed);
        }
        stats.total_subscribers = total_subs;
        stats
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new(MessageBusConfig::default())
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
    fn message_creation() {
        let msg = Message::new("topic", vec![1, 2, 3]);
        assert_eq!(msg.topic, "topic");
        assert_eq!(msg.payload, vec![1, 2, 3]);
    }

    #[test]
    fn message_with_headers() {
        let msg = Message::new("t", vec![])
            .with_header("k1", "v1")
            .with_header("k2", "v2");
        assert_eq!(msg.headers.len(), 2);
        assert_eq!(msg.headers["k1"], "v1");
    }

    #[test]
    fn json_message_roundtrip() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct TestData {
            name: String,
            value: u32,
        }

        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };

        let msg = Message::from_json("topic", &data).unwrap();
        assert_eq!(msg.content_type, "application/json");

        let recovered: TestData = msg.to_json().unwrap();
        assert_eq!(recovered, data);
    }

    #[test]
    fn register_topic() {
        let bus = MessageBus::new(MessageBusConfig::default());
        let _rx = bus.register_topic("test");
        assert_eq!(bus.topic_count(), 1);
    }

    #[test]
    fn send_message() {
        let bus = MessageBus::new(MessageBusConfig::default());
        let _rx = bus.register_topic("test");
        let msg = Message::new("test", vec![1, 2, 3]);
        bus.send(msg).unwrap();

        let stats = bus.statistics();
        assert_eq!(stats.messages_sent, 1);
    }

    #[test]
    fn send_to_unregistered_topic() {
        let bus = MessageBus::new(MessageBusConfig::default());
        let msg = Message::new("missing", vec![]);
        let result = bus.send(msg);
        assert!(result.is_err());
    }

    #[test]
    fn route_registration() {
        let bus = MessageBus::new(MessageBusConfig::default());
        let route = Route {
            key: RouteKey {
                topic: "input".to_string(),
                tag: None,
            },
            target_topic: "output".to_string(),
            transform: None,
        };
        bus.add_route(route);
        let stats = bus.statistics();
        assert_eq!(stats.messages_routed, 1);
    }

    #[test]
    fn statistics() {
        let bus = MessageBus::new(MessageBusConfig::default());
        bus.register_topic("a");
        bus.register_topic("b");
        let stats = bus.statistics();
        assert_eq!(stats.active_topics, 2);
    }
}
