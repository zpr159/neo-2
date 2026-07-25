use std::collections::{BTreeSet, HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;

use crate::error::{AgentError, AgentResult};
use crate::types::{AgentId, Conversation, ConversationId, MessagePriority};

// ---------------------------------------------------------------------------
// AgentMessage
// ---------------------------------------------------------------------------

/// A message exchanged between agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    /// Unique message identifier.
    pub id: Uuid,
    /// Sender agent identifier.
    pub from: AgentId,
    /// Recipient agent identifier (None for broadcasts).
    pub to: Option<AgentId>,
    /// The type of message.
    pub message_type: MessageType,
    /// Message payload.
    pub payload: serde_json::Value,
    /// When the message was created.
    pub timestamp: DateTime<Utc>,
    /// If this is a reply, the ID of the message being replied to.
    pub reply_to: Option<Uuid>,
    /// Message priority.
    pub priority: MessagePriority,
    /// Conversation ID this message belongs to.
    pub conversation_id: Option<ConversationId>,
    /// Time-to-live in seconds. None means no expiry.
    pub ttl_secs: Option<u64>,
    /// Correlation ID for request/reply matching.
    pub correlation_id: Option<Uuid>,
    /// Whether delivery acknowledgement is requested.
    pub requires_ack: bool,
}

impl AgentMessage {
    /// Create a new message.
    #[must_use]
    pub fn new(
        from: AgentId,
        to: AgentId,
        message_type: MessageType,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            from,
            to: Some(to),
            message_type,
            payload,
            timestamp: Utc::now(),
            reply_to: None,
            priority: MessagePriority::Normal,
            conversation_id: None,
            ttl_secs: None,
            correlation_id: None,
            requires_ack: false,
        }
    }

    /// Create a broadcast message.
    #[must_use]
    pub fn broadcast(from: AgentId, message_type: MessageType, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            from,
            to: None,
            message_type,
            payload,
            timestamp: Utc::now(),
            reply_to: None,
            priority: MessagePriority::Normal,
            conversation_id: None,
            ttl_secs: None,
            correlation_id: None,
            requires_ack: false,
        }
    }

    /// Create a reply to this message.
    #[must_use]
    pub fn reply(&self, from: AgentId, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            from,
            to: Some(self.from),
            message_type: MessageType::Response,
            payload,
            timestamp: Utc::now(),
            reply_to: Some(self.id),
            priority: self.priority,
            conversation_id: self.conversation_id,
            ttl_secs: None,
            correlation_id: self.correlation_id,
            requires_ack: false,
        }
    }

    /// Set the priority.
    #[must_use]
    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set the conversation ID.
    #[must_use]
    pub fn with_conversation(mut self, conversation_id: ConversationId) -> Self {
        self.conversation_id = Some(conversation_id);
        self
    }

    /// Set the correlation ID for request/reply.
    #[must_use]
    pub fn with_correlation_id(mut self, correlation_id: Uuid) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    /// Set the TTL.
    #[must_use]
    pub fn with_ttl(mut self, ttl_secs: u64) -> Self {
        self.ttl_secs = Some(ttl_secs);
        self
    }

    /// Request delivery acknowledgement.
    #[must_use]
    pub fn with_ack(mut self) -> Self {
        self.requires_ack = true;
        self
    }

    /// Check if this message has expired based on its TTL.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl_secs {
            let elapsed = Utc::now()
                .signed_duration_since(self.timestamp)
                .num_seconds() as u64;
            elapsed > ttl
        } else {
            false
        }
    }
}

// ---------------------------------------------------------------------------
// MessageType
// ---------------------------------------------------------------------------

/// Types of messages agents can exchange.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageType {
    /// A request for information or action.
    Request,
    /// A response to a request.
    Response,
    /// A one-way notification.
    Notification,
    /// A broadcast to all agents.
    Broadcast,
    /// A periodic heartbeat.
    Heartbeat,
    /// A task assignment.
    TaskAssignment,
    /// A task result.
    TaskResult,
    /// A status update.
    StatusUpdate,
    /// An error report.
    Error,
    /// An acknowledgement of receipt.
    Ack,
    /// A delegation message (one agent delegates to another).
    Delegation,
    /// A consensus request.
    ConsensusRequest,
    /// A consensus response.
    ConsensusResponse,
    /// A vote.
    Vote,
    /// An escalation to a supervisor.
    Escalation,
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Request => write!(f, "request"),
            Self::Response => write!(f, "response"),
            Self::Notification => write!(f, "notification"),
            Self::Broadcast => write!(f, "broadcast"),
            Self::Heartbeat => write!(f, "heartbeat"),
            Self::TaskAssignment => write!(f, "task_assignment"),
            Self::TaskResult => write!(f, "task_result"),
            Self::StatusUpdate => write!(f, "status_update"),
            Self::Error => write!(f, "error"),
            Self::Ack => write!(f, "ack"),
            Self::Delegation => write!(f, "delegation"),
            Self::ConsensusRequest => write!(f, "consensus_request"),
            Self::ConsensusResponse => write!(f, "consensus_response"),
            Self::Vote => write!(f, "vote"),
            Self::Escalation => write!(f, "escalation"),
        }
    }
}

// ---------------------------------------------------------------------------
// MessageEnvelope
// ---------------------------------------------------------------------------

/// Wraps a message with delivery metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope {
    /// The wrapped message.
    pub message: AgentMessage,
    /// Delivery attempt count.
    pub delivery_attempts: u32,
    /// When the envelope was created.
    pub created_at: DateTime<Utc>,
    /// When the envelope was last attempted.
    pub last_attempt: Option<DateTime<Utc>>,
    /// Whether the message has been delivered.
    pub delivered: bool,
    /// Whether the message has been processed by the recipient.
    pub processed: bool,
}

impl MessageEnvelope {
    /// Create a new envelope for a message.
    #[must_use]
    pub fn new(message: AgentMessage) -> Self {
        Self {
            message,
            delivery_attempts: 0,
            created_at: Utc::now(),
            last_attempt: None,
            delivered: false,
            processed: false,
        }
    }

    /// Record a delivery attempt.
    pub fn record_attempt(&mut self) {
        self.delivery_attempts += 1;
        self.last_attempt = Some(Utc::now());
    }

    /// Mark as delivered.
    pub fn mark_delivered(&mut self) {
        self.delivered = true;
    }

    /// Mark as processed.
    pub fn mark_processed(&mut self) {
        self.processed = true;
    }
}

// ---------------------------------------------------------------------------
// MessageChannel
// ---------------------------------------------------------------------------

/// A bounded, async message channel between agents.
pub struct MessageChannel {
    /// Channel capacity.
    capacity: usize,
    /// Internal queue.
    queue: Arc<RwLock<VecDeque<AgentMessage>>>,
    /// Current pending count.
    pending_count: Arc<AtomicUsize>,
    /// Whether the channel is closed.
    closed: Arc<AtomicBool>,
}

impl MessageChannel {
    /// Create a new message channel with the given capacity.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            queue: Arc::new(RwLock::new(VecDeque::with_capacity(capacity))),
            pending_count: Arc::new(AtomicUsize::new(0)),
            closed: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Send a message into the channel.
    pub async fn send(&self, msg: AgentMessage) -> AgentResult<()> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(AgentError::MessageDeliveryFailed(
                "channel is closed".into(),
            ));
        }

        // Check TTL before sending
        if msg.is_expired() {
            return Err(AgentError::MessageDeliveryFailed(
                "message expired before delivery".into(),
            ));
        }

        let mut queue = self.queue.write().await;
        if queue.len() >= self.capacity {
            return Err(AgentError::QuotaExceeded(format!(
                "channel capacity {} reached",
                self.capacity
            )));
        }
        queue.push_back(msg);
        self.pending_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    /// Receive a message from the channel, returning None if empty.
    pub async fn receive(&self) -> Option<AgentMessage> {
        let mut queue = self.queue.write().await;
        let msg = queue.pop_front();
        if msg.is_some() {
            self.pending_count.fetch_sub(1, Ordering::SeqCst);
        }
        msg
    }

    /// Receive a message, waiting up to `timeout_ms` milliseconds.
    pub async fn receive_timeout(&self, timeout_ms: u64) -> Option<AgentMessage> {
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout_ms);

        loop {
            if let Some(msg) = self.receive().await {
                return Some(msg);
            }
            if tokio::time::Instant::now() >= deadline {
                return None;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    }

    /// Peek at the next message without removing it.
    pub async fn peek(&self) -> Option<AgentMessage> {
        let queue = self.queue.read().await;
        queue.front().cloned()
    }

    /// Return the number of pending messages.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.pending_count.load(Ordering::SeqCst)
    }

    /// Return the channel capacity.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Check if the channel is empty.
    pub async fn is_empty(&self) -> bool {
        let queue = self.queue.read().await;
        queue.is_empty()
    }

    /// Close the channel, rejecting further sends.
    pub fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
    }

    /// Drain all messages from the channel.
    pub async fn drain(&self) -> Vec<AgentMessage> {
        let mut queue = self.queue.write().await;
        let msgs: Vec<_> = queue.drain(..).collect();
        self.pending_count.store(0, Ordering::SeqCst);
        msgs
    }
}

// ---------------------------------------------------------------------------
// MessageQueue
// ---------------------------------------------------------------------------

/// A priority message queue that holds messages sorted by priority.
pub struct MessageQueue {
    /// Priority-indexed queues.
    queues: Arc<RwLock<HashMap<MessagePriority, VecDeque<AgentMessage>>>>,
    /// Total pending count.
    pending_count: Arc<AtomicUsize>,
}

impl MessageQueue {
    /// Create a new empty priority message queue.
    #[must_use]
    pub fn new() -> Self {
        let mut queues = HashMap::new();
        queues.insert(MessagePriority::Background, VecDeque::new());
        queues.insert(MessagePriority::Low, VecDeque::new());
        queues.insert(MessagePriority::Normal, VecDeque::new());
        queues.insert(MessagePriority::High, VecDeque::new());
        queues.insert(MessagePriority::Critical, VecDeque::new());

        Self {
            queues: Arc::new(RwLock::new(queues)),
            pending_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Enqueue a message at its priority level.
    pub async fn enqueue(&self, msg: AgentMessage) {
        let mut queues = self.queues.write().await;
        if let Some(queue) = queues.get_mut(&msg.priority) {
            queue.push_back(msg);
            self.pending_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// Dequeue the highest-priority message.
    pub async fn dequeue(&self) -> Option<AgentMessage> {
        let mut queues = self.queues.write().await;
        // Iterate from highest to lowest priority
        let priorities = [
            MessagePriority::Critical,
            MessagePriority::High,
            MessagePriority::Normal,
            MessagePriority::Low,
            MessagePriority::Background,
        ];
        for priority in &priorities {
            if let Some(queue) = queues.get_mut(priority) {
                if let Some(msg) = queue.pop_front() {
                    self.pending_count.fetch_sub(1, Ordering::SeqCst);
                    return Some(msg);
                }
            }
        }
        None
    }

    /// Return the total number of pending messages.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.pending_count.load(Ordering::SeqCst)
    }

    /// Check if the queue is empty.
    pub async fn is_empty(&self) -> bool {
        let queues = self.queues.read().await;
        queues.values().all(std::collections::VecDeque::is_empty)
    }

    /// Return the count per priority level.
    pub async fn counts_by_priority(&self) -> HashMap<MessagePriority, usize> {
        let queues = self.queues.read().await;
        let mut result = HashMap::new();
        for (priority, queue) in queues.iter() {
            result.insert(*priority, queue.len());
        }
        result
    }
}

impl Default for MessageQueue {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// MessageChannelRegistry
// ---------------------------------------------------------------------------

/// Registry of all message channels between agents.
pub struct MessageChannelRegistry {
    /// Direct channels between pairs of agents: (sender, receiver) -> channel.
    channels: DashMap<(AgentId, AgentId), Arc<MessageChannel>>,
    /// Broadcast channels per topic.
    broadcast_channels: DashMap<String, broadcast::Sender<AgentMessage>>,
    /// Agent inbox channels: agent_id -> sender.
    pub(crate) inboxes: DashMap<AgentId, mpsc::Sender<AgentMessage>>,
    /// Default channel capacity.
    default_capacity: usize,
}

impl MessageChannelRegistry {
    /// Create a new channel registry.
    #[must_use]
    pub fn new(default_capacity: usize) -> Self {
        Self {
            channels: DashMap::new(),
            broadcast_channels: DashMap::new(),
            inboxes: DashMap::new(),
            default_capacity,
        }
    }

    /// Register an agent's inbox channel.
    pub fn register_inbox(&self, agent_id: AgentId, sender: mpsc::Sender<AgentMessage>) {
        self.inboxes.insert(agent_id, sender);
    }

    /// Unregister an agent's inbox.
    pub fn unregister_inbox(&self, agent_id: &AgentId) {
        self.inboxes.remove(agent_id);
    }

    /// Get or create a direct channel between two agents.
    #[must_use]
    pub fn get_or_create_channel(&self, from: AgentId, to: AgentId) -> Arc<MessageChannel> {
        self.channels
            .entry((from, to))
            .or_insert_with(|| Arc::new(MessageChannel::new(self.default_capacity)))
            .value()
            .clone()
    }

    /// Send a direct message from one agent to another.
    pub async fn send_direct(&self, msg: AgentMessage) -> AgentResult<()> {
        let to = msg.to.ok_or_else(|| {
            AgentError::InvalidConfiguration("direct message requires a recipient".into())
        })?;

        if let Some(sender) = self.inboxes.get(&to) {
            sender.send(msg).await.map_err(|_| {
                AgentError::MessageDeliveryFailed(format!("inbox full for agent {to}"))
            })?;
            Ok(())
        } else {
            Err(AgentError::NotFound(format!(
                "agent {to} has no registered inbox"
            )))
        }
    }

    /// Create or get a broadcast channel for a topic.
    pub fn get_or_create_broadcast(&self, topic: &str) -> broadcast::Sender<AgentMessage> {
        self.broadcast_channels
            .entry(topic.to_string())
            .or_insert_with(|| {
                let (tx, _) = broadcast::channel(256);
                tx
            })
            .value()
            .clone()
    }

    /// Subscribe to a broadcast channel.
    pub fn subscribe_broadcast(
        &self,
        topic: &str,
    ) -> AgentResult<broadcast::Receiver<AgentMessage>> {
        let tx = self.get_or_create_broadcast(topic);
        Ok(tx.subscribe())
    }

    /// Publish a message to a broadcast topic.
    pub async fn publish_broadcast(&self, topic: &str, msg: AgentMessage) -> AgentResult<()> {
        let tx = self.get_or_create_broadcast(topic);
        tx.send(msg).map_err(|_| {
            AgentError::MessageDeliveryFailed(format!("no subscribers for topic {topic}"))
        })?;
        Ok(())
    }

    /// Remove all channels involving a given agent.
    pub fn remove_agent(&self, agent_id: &AgentId) {
        self.channels
            .retain(|(a, b), _| a != agent_id && b != agent_id);
        self.inboxes.remove(agent_id);
    }

    /// Return the total number of registered direct channels.
    #[must_use]
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    /// Return the number of registered inboxes.
    #[must_use]
    pub fn inbox_count(&self) -> usize {
        self.inboxes.len()
    }
}

impl Default for MessageChannelRegistry {
    fn default() -> Self {
        Self::new(256)
    }
}

// ---------------------------------------------------------------------------
// ConversationManager
// ---------------------------------------------------------------------------

/// Manages conversations between agents.
pub struct ConversationManager {
    /// Active conversations.
    conversations: DashMap<ConversationId, Conversation>,
    /// Conversation history: conversation_id -> messages.
    history: DashMap<ConversationId, Vec<AgentMessage>>,
    /// Index: agent_id -> conversation IDs.
    agent_conversations: DashMap<AgentId, BTreeSet<ConversationId>>,
}

impl ConversationManager {
    /// Create a new conversation manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            conversations: DashMap::new(),
            history: DashMap::new(),
            agent_conversations: DashMap::new(),
        }
    }

    /// Start a new conversation.
    pub fn start_conversation(
        &self,
        participants: Vec<AgentId>,
        subject: String,
    ) -> ConversationId {
        let conv = Conversation::new(participants.clone(), subject);
        let id = conv.id;

        // Index by participants
        for agent_id in &participants {
            self.agent_conversations
                .entry(*agent_id)
                .or_default()
                .insert(id);
        }

        self.conversations.insert(id, conv);
        self.history.insert(id, Vec::new());
        id
    }

    /// Record a message in a conversation.
    pub fn record_message(&self, conversation_id: ConversationId, msg: AgentMessage) {
        if let Some(mut conv) = self.conversations.get_mut(&conversation_id) {
            conv.last_activity = Utc::now();
            conv.message_count += 1;
        }
        if let Some(mut history) = self.history.get_mut(&conversation_id) {
            history.push(msg);
        }
    }

    /// End a conversation.
    pub fn end_conversation(&self, conversation_id: ConversationId) {
        if let Some(mut conv) = self.conversations.get_mut(&conversation_id) {
            conv.is_active = false;
        }
    }

    /// Get a conversation by ID.
    pub fn get_conversation(&self, conversation_id: ConversationId) -> Option<Conversation> {
        self.conversations.get(&conversation_id).map(|c| c.clone())
    }

    /// Get all conversations for an agent.
    #[must_use]
    pub fn agent_conversations(&self, agent_id: &AgentId) -> Vec<ConversationId> {
        self.agent_conversations
            .get(agent_id)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Get message history for a conversation.
    pub fn get_history(&self, conversation_id: ConversationId) -> Vec<AgentMessage> {
        self.history
            .get(&conversation_id)
            .map(|h| h.clone())
            .unwrap_or_default()
    }

    /// Return the total number of active conversations.
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.conversations
            .iter()
            .filter(|c| c.value().is_active)
            .count()
    }
}

impl Default for ConversationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(_name: &str) -> AgentId {
        AgentId::new()
    }

    #[tokio::test]
    async fn test_message_channel() {
        let ch = MessageChannel::new(10);
        let from = agent("a");
        let to = agent("b");
        let msg = AgentMessage::new(from, to, MessageType::Request, serde_json::json!("hello"));
        ch.send(msg.clone()).await.unwrap();
        assert_eq!(ch.pending_count(), 1);

        let received = ch.receive().await.unwrap();
        assert_eq!(received.id, msg.id);
        assert_eq!(ch.pending_count(), 0);
    }

    #[tokio::test]
    async fn test_message_channel_capacity() {
        let ch = MessageChannel::new(2);
        let from = agent("a");
        let to = agent("b");
        let msg = || AgentMessage::new(from, to, MessageType::Request, serde_json::json!("x"));
        ch.send(msg()).await.unwrap();
        ch.send(msg()).await.unwrap();
        let result = ch.send(msg()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_priority_message_queue() {
        let q = MessageQueue::new();
        let from = agent("a");
        let to = agent("b");

        q.enqueue(
            AgentMessage::new(from, to, MessageType::Request, serde_json::json!("low"))
                .with_priority(MessagePriority::Low),
        )
        .await;
        q.enqueue(
            AgentMessage::new(
                from,
                to,
                MessageType::Request,
                serde_json::json!("critical"),
            )
            .with_priority(MessagePriority::Critical),
        )
        .await;
        q.enqueue(
            AgentMessage::new(from, to, MessageType::Request, serde_json::json!("normal"))
                .with_priority(MessagePriority::Normal),
        )
        .await;

        let msg1 = q.dequeue().await.unwrap();
        assert_eq!(msg1.payload, serde_json::json!("critical"));

        let msg2 = q.dequeue().await.unwrap();
        assert_eq!(msg2.payload, serde_json::json!("normal"));

        let msg3 = q.dequeue().await.unwrap();
        assert_eq!(msg3.payload, serde_json::json!("low"));

        assert!(q.dequeue().await.is_none());
    }

    #[tokio::test]
    async fn test_message_expiry() {
        let mut msg = AgentMessage::new(
            agent("a"),
            agent("b"),
            MessageType::Request,
            serde_json::json!("test"),
        );
        msg.ttl_secs = Some(0);
        // Ensure time passes for expiry check
        let now = Utc::now();
        msg.timestamp = now - chrono::Duration::seconds(1);
        assert!(msg.is_expired());
    }

    #[test]
    fn test_message_reply() {
        let from = agent("a");
        let to = agent("b");
        let original = AgentMessage::new(from, to, MessageType::Request, serde_json::json!("?"));
        let reply = original.reply(to, serde_json::json!("!"));
        assert_eq!(reply.to, Some(from));
        assert_eq!(reply.from, to);
        assert_eq!(reply.reply_to, Some(original.id));
    }

    #[test]
    fn test_conversation_manager() {
        let mgr = ConversationManager::new();
        let a1 = agent("a");
        let a2 = agent("b");

        let cid = mgr.start_conversation(vec![a1, a2], "test".to_string());
        assert_eq!(mgr.active_count(), 1);

        let conv = mgr.get_conversation(cid).unwrap();
        assert!(conv.is_active);
        assert_eq!(conv.participants.len(), 2);

        mgr.end_conversation(cid);
        let conv = mgr.get_conversation(cid).unwrap();
        assert!(!conv.is_active);
    }

    #[test]
    fn test_channel_registry() {
        let reg = MessageChannelRegistry::new(64);
        let a1 = agent("a");
        let a2 = agent("b");

        let ch = reg.get_or_create_channel(a1, a2);
        assert_eq!(ch.capacity(), 64);
        assert_eq!(reg.channel_count(), 1);

        // Same pair returns same channel
        let ch2 = reg.get_or_create_channel(a1, a2);
        assert_eq!(ch.capacity(), ch2.capacity());
        assert_eq!(reg.channel_count(), 1);

        // Different pair creates new channel
        let _ch3 = reg.get_or_create_channel(a2, a1);
        assert_eq!(reg.channel_count(), 2);
    }
}
