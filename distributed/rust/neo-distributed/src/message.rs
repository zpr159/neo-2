//! Cluster message types for inter-node communication, heartbeats, task
//! assignment, state synchronization, and consensus.

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::{
    ClusterMetadata, ClusterState, NodeHealth, NodeId, NodeInfo, NodeResources, NodeState,
    TaskPriority,
};

// ---------------------------------------------------------------------------
// MessageType
// ---------------------------------------------------------------------------

/// Discriminant for cluster messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageType {
    // -- Lifecycle --
    /// Node join request.
    Join,
    /// Node leave request.
    Leave,
    /// Node deregistration acknowledgment.
    LeaveAck,
    // -- Heartbeat --
    /// Periodic heartbeat.
    Heartbeat,
    /// Heartbeat response with health info.
    HeartbeatAck,
    // -- Consensus --
    /// Leader election: vote request.
    VoteRequest,
    /// Leader election: vote response.
    VoteResponse,
    /// Append entries (Raft-style).
    AppendEntries,
    /// Append entries acknowledgment.
    AppendEntriesAck,
    // -- Task --
    /// Assign a task to a node.
    TaskAssign,
    /// Report task completion.
    TaskComplete,
    /// Report task failure.
    TaskFail,
    /// Cancel a task.
    TaskCancel,
    // -- State --
    /// Full state synchronization.
    StateSync,
    /// Incremental state update.
    StateUpdate,
    // -- Discovery --
    /// Discovery probe (multicast / broadcast).
    DiscoveryProbe,
    /// Discovery response.
    DiscoveryResponse,
    // -- General --
    /// Ping.
    Ping,
    /// Pong.
    Pong,
    /// Generic error notification.
    Error,
}

// ---------------------------------------------------------------------------
// ClusterMessage
// ---------------------------------------------------------------------------

/// A message exchanged between cluster nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMessage {
    /// Unique message identifier.
    pub id: Uuid,
    /// Sender node ID.
    pub from: NodeId,
    /// Recipient node ID (`None` = broadcast).
    pub to: Option<NodeId>,
    /// Message type.
    pub msg_type: MessageType,
    /// Message payload.
    pub payload: MessagePayload,
    /// Timestamp when the message was created.
    pub timestamp: DateTime<Utc>,
    /// Monotonic sequence number for ordering.
    pub sequence: u64,
    /// TTL in hops (decremented by relays).
    pub ttl: u8,
    /// Optional correlation ID for request/reply matching.
    pub correlation_id: Option<Uuid>,
}

impl ClusterMessage {
    /// Create a new directed message.
    pub fn new(
        from: NodeId,
        to: Option<NodeId>,
        msg_type: MessageType,
        payload: MessagePayload,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            from,
            to,
            msg_type,
            payload,
            timestamp: Utc::now(),
            sequence: 0,
            ttl: 16,
            correlation_id: None,
        }
    }

    /// Create a broadcast message.
    pub fn broadcast(from: NodeId, msg_type: MessageType, payload: MessagePayload) -> Self {
        Self::new(from, None, msg_type, payload)
    }

    /// Set the sequence number.
    #[must_use]
    pub fn with_sequence(mut self, seq: u64) -> Self {
        self.sequence = seq;
        self
    }

    /// Set the TTL.
    #[must_use]
    pub fn with_ttl(mut self, ttl: u8) -> Self {
        self.ttl = ttl;
        self
    }

    /// Set a correlation ID.
    #[must_use]
    pub fn with_correlation(mut self, id: Uuid) -> Self {
        self.correlation_id = Some(id);
        self
    }

    /// Whether this is a broadcast message.
    pub fn is_broadcast(&self) -> bool {
        self.to.is_none()
    }

    /// Whether this is a directed message.
    pub fn is_directed(&self) -> bool {
        self.to.is_some()
    }

    /// Decrement TTL and return `true` if still alive.
    pub fn decrement_ttl(&mut self) -> bool {
        self.ttl = self.ttl.saturating_sub(1);
        self.ttl > 0
    }
}

// ---------------------------------------------------------------------------
// MessagePayload
// ---------------------------------------------------------------------------

/// Typed payload carried by a `ClusterMessage`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessagePayload {
    /// Empty payload.
    Empty,
    /// Heartbeat payload.
    Heartbeat(HeartbeatPayload),
    /// Heartbeat acknowledgment.
    HeartbeatAck(HeartbeatAckPayload),
    /// Join request payload.
    Join(JoinPayload),
    /// Task assignment payload.
    TaskAssign(TaskAssignPayload),
    /// Task completion payload.
    TaskComplete(TaskCompletePayload),
    /// Task failure payload.
    TaskFail(TaskFailPayload),
    /// State synchronization payload.
    StateSync(StateSyncPayload),
    /// Vote request payload.
    VoteRequest(VoteRequestPayload),
    /// Vote response payload.
    VoteResponse(VoteResponsePayload),
    /// Append entries payload.
    AppendEntries(AppendEntriesPayload),
    /// Append entries acknowledgment.
    AppendEntriesAck(AppendEntriesAckPayload),
    /// Discovery probe.
    DiscoveryProbe(DiscoveryProbePayload),
    /// Discovery response.
    DiscoveryResponse(DiscoveryResponsePayload),
    /// Generic JSON payload.
    Json(serde_json::Value),
}

// ---------------------------------------------------------------------------
// Payload structs
// ---------------------------------------------------------------------------

/// Heartbeat sent periodically by each node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatPayload {
    /// Node ID of sender.
    pub node_id: NodeId,
    /// Current node state.
    pub state: NodeState,
    /// Current resource utilization.
    pub resources: NodeResources,
    /// Sequence number for ordering.
    pub seq: u64,
}

/// Response to a heartbeat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatAckPayload {
    /// Node ID of responder.
    pub node_id: NodeId,
    /// Measured round-trip latency.
    pub latency_ms: f64,
    /// Current cluster state.
    pub cluster_state: ClusterState,
}

/// Node join request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinPayload {
    /// Node information.
    pub info: NodeInfo,
    /// Requested role.
    pub requested_role: Option<String>,
}

/// Task assignment from scheduler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAssignPayload {
    /// Task identifier.
    pub task_id: Uuid,
    /// Task type / capability required.
    pub task_type: String,
    /// Task priority.
    pub priority: TaskPriority,
    /// Estimated duration.
    pub estimated_duration: Duration,
    /// Serialized task data.
    pub data: Vec<u8>,
}

/// Task completion report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCompletePayload {
    /// Task identifier.
    pub task_id: Uuid,
    /// Execution duration.
    pub duration: Duration,
    /// Serialized result.
    pub result: Vec<u8>,
}

/// Task failure report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFailPayload {
    /// Task identifier.
    pub task_id: Uuid,
    /// Error message.
    pub error: String,
    /// Whether the task is retryable.
    pub retryable: bool,
}

/// Full state synchronization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSyncPayload {
    /// Cluster metadata.
    pub cluster: ClusterMetadata,
    /// All node infos.
    pub nodes: Vec<NodeInfo>,
    /// All node states.
    pub node_states: Vec<(NodeId, NodeState)>,
}

/// Raft-style vote request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteRequestPayload {
    /// Current term.
    pub term: u64,
    /// Candidate node ID.
    pub candidate_id: NodeId,
    /// Last log index.
    pub last_log_index: u64,
    /// Last log term.
    pub last_log_term: u64,
}

/// Raft-style vote response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteResponsePayload {
    /// Current term (for leader to update itself).
    pub term: u64,
    /// Whether the vote was granted.
    pub vote_granted: bool,
    /// Responder node ID.
    pub voter_id: NodeId,
}

/// Raft-style append entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesPayload {
    /// Current term.
    pub term: u64,
    /// Leader node ID.
    pub leader_id: NodeId,
    /// Previous log index.
    pub prev_log_index: u64,
    /// Previous log term.
    pub prev_log_term: u64,
    /// Log entries to append.
    pub entries: Vec<LogEntry>,
    /// Leader's commit index.
    pub leader_commit: u64,
}

/// A single Raft log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Term when the entry was received.
    pub term: u64,
    /// Log index.
    pub index: u64,
    /// Command type.
    pub command: String,
    /// Serialized command data.
    pub data: Vec<u8>,
}

/// Acknowledgment for append entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesAckPayload {
    /// Whether the append was successful.
    pub success: bool,
    /// Current term.
    pub term: u64,
    /// Match index (highest log index replicated).
    pub match_index: u64,
}

/// Discovery probe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryProbePayload {
    /// Cluster name to discover.
    pub cluster_name: String,
    /// Sender node info.
    pub info: NodeInfo,
}

/// Discovery response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResponsePayload {
    /// Responder node info.
    pub info: NodeInfo,
    /// Known peers.
    pub peers: Vec<NodeInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::NodeCapabilities;

    fn test_node_id() -> NodeId {
        NodeId::new()
    }

    #[test]
    fn message_creation() {
        let from = test_node_id();
        let msg = ClusterMessage::new(
            from,
            None,
            MessageType::Heartbeat,
            MessagePayload::Empty,
        );
        assert!(msg.is_broadcast());
        assert!(!msg.is_directed());
    }

    #[test]
    fn directed_message() {
        let from = test_node_id();
        let to = test_node_id();
        let msg = ClusterMessage::new(
            from,
            Some(to),
            MessageType::TaskAssign,
            MessagePayload::Empty,
        );
        assert!(!msg.is_broadcast());
        assert!(msg.is_directed());
        assert_eq!(msg.to, Some(to));
    }

    #[test]
    fn message_with_sequence() {
        let from = test_node_id();
        let msg = ClusterMessage::broadcast(from, MessageType::Ping, MessagePayload::Empty)
            .with_sequence(42);
        assert_eq!(msg.sequence, 42);
    }

    #[test]
    fn message_ttl() {
        let from = test_node_id();
        let mut msg = ClusterMessage::broadcast(from, MessageType::Ping, MessagePayload::Empty)
            .with_ttl(2);
        assert!(msg.decrement_ttl());
        assert!(msg.decrement_ttl());
        assert!(!msg.decrement_ttl());
    }

    #[test]
    fn heartbeat_payload() {
        let payload = HeartbeatPayload {
            node_id: test_node_id(),
            state: NodeState::Ready,
            resources: NodeResources::default(),
            seq: 1,
        };
        assert_eq!(payload.state, NodeState::Ready);
    }

    #[test]
    fn task_assign_payload() {
        let payload = TaskAssignPayload {
            task_id: Uuid::new_v4(),
            task_type: "inference".to_string(),
            priority: TaskPriority::HIGH,
            estimated_duration: Duration::from_secs(10),
            data: vec![],
        };
        assert_eq!(payload.priority, TaskPriority::HIGH);
    }

    #[test]
    fn message_correlation() {
        let from = test_node_id();
        let corr = Uuid::new_v4();
        let msg = ClusterMessage::broadcast(from, MessageType::Ping, MessagePayload::Empty)
            .with_correlation(corr);
        assert_eq!(msg.correlation_id, Some(corr));
    }
}
