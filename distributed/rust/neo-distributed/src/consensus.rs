//! Consensus engine — Raft-inspired leader election, log replication, and
//! cluster agreement for distributed state.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{DistributedError, NeoResult};
use crate::types::{NodeId, NodeState};

// ---------------------------------------------------------------------------
// ConsensusEngine
// ---------------------------------------------------------------------------

/// Core consensus engine implementing a Raft-like protocol.
pub struct ConsensusEngine {
    /// Current Raft term.
    term: RwLock<u64>,
    /// Which node we voted for in the current term.
    voted_for: RwLock<Option<NodeId>>,
    /// Current leader.
    leader: RwLock<Option<NodeId>>,
    /// Commit index — highest log entry known to be committed.
    commit_index: RwLock<u64>,
    /// Last applied index.
    last_applied: RwLock<u64>,
    /// Log entries.
    log: RwLock<Vec<LogEntry>>,
    /// Votes received in the current election.
    votes_received: RwLock<Vec<NodeId>>,
    /// Election start time.
    election_started: RwLock<Option<DateTime<Utc>>>,
    /// Cluster node IDs for quorum calculation.
    cluster_nodes: RwLock<Vec<NodeId>>,
    /// Our own node ID.
    node_id: NodeId,
    /// Election timeout in milliseconds.
    election_timeout_ms: u64,
}

impl ConsensusEngine {
    /// Create a new consensus engine for the given node.
    pub fn new(node_id: NodeId, election_timeout_ms: u64) -> Self {
        tracing::info!(
            node_id = %node_id,
            election_timeout_ms = election_timeout_ms,
            "consensus engine created"
        );
        Self {
            term: RwLock::new(0),
            voted_for: RwLock::new(None),
            leader: RwLock::new(None),
            commit_index: RwLock::new(0),
            last_applied: RwLock::new(0),
            log: RwLock::new(Vec::new()),
            votes_received: RwLock::new(Vec::new()),
            election_started: RwLock::new(None),
            cluster_nodes: RwLock::new(Vec::new()),
            node_id,
            election_timeout_ms,
        }
    }

    // -- State queries --

    /// Current term.
    pub fn term(&self) -> u64 {
        *self.term.read()
    }

    /// Current leader.
    pub fn leader(&self) -> Option<NodeId> {
        *self.leader.read()
    }

    /// Whether this node is the current leader.
    pub fn is_leader(&self) -> bool {
        *self.leader.read() == Some(self.node_id)
    }

    /// Current commit index.
    pub fn commit_index(&self) -> u64 {
        *self.commit_index.read()
    }

    /// Last applied index.
    pub fn last_applied(&self) -> u64 {
        *self.last_applied.read()
    }

    /// Log length.
    pub fn log_len(&self) -> usize {
        self.log.read().len()
    }

    /// Node ID of this engine.
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    // -- Cluster membership --

    /// Set the cluster node list (for quorum calculation).
    pub fn set_cluster_nodes(&self, nodes: Vec<NodeId>) {
        *self.cluster_nodes.write() = nodes;
    }

    /// Get cluster nodes.
    pub fn cluster_nodes(&self) -> Vec<NodeId> {
        self.cluster_nodes.read().clone()
    }

    /// Calculate quorum size (majority).
    pub fn quorum_size(&self) -> usize {
        let n = self.cluster_nodes.read().len();
        n / 2 + 1
    }

    /// Current cluster size.
    pub fn cluster_size(&self) -> usize {
        self.cluster_nodes.read().len()
    }

    // -- Leader election --

    /// Start a new election (transition to candidate state).
    pub fn start_election(&self) -> NeoResult<()> {
        let mut term = self.term.write();
        *term += 1;
        let current_term = *term;

        // Vote for ourselves.
        *self.voted_for.write() = Some(self.node_id);
        *self.votes_received.write() = vec![self.node_id];
        *self.election_started.write() = Some(Utc::now());
        *self.leader.write() = None;

        tracing::info!(
            node_id = %self.node_id,
            term = current_term,
            "election started"
        );

        Ok(())
    }

    /// Cast a vote (respond to a vote request).
    pub fn handle_vote_request(
        &self,
        term: u64,
        candidate_id: NodeId,
        last_log_index: u64,
        last_log_term: u64,
    ) -> NeoResult<bool> {
        let current_term = *self.term.read();

        // Reject if candidate's term is behind.
        if term < current_term {
            return Ok(false);
        }

        // Update term if candidate is ahead.
        if term > current_term {
            *self.term.write() = term;
            *self.voted_for.write() = None;
            *self.leader.write() = None;
        }

        // Grant vote if we haven't voted for someone else.
        let voted_for = *self.voted_for.read();
        if voted_for.is_none() || voted_for == Some(candidate_id) {
            // Check log up-to-date.
            let our_last_term = self.last_log_term();
            let our_last_index = self.log_len() as u64;
            let log_ok =
                last_log_term > our_last_term
                    || (last_log_term == our_last_term && last_log_index >= our_last_index);

            if log_ok {
                *self.voted_for.write() = Some(candidate_id);
                tracing::debug!(
                    term = term,
                    candidate = %candidate_id,
                    "vote granted"
                );
                return Ok(true);
            }
        }

        tracing::debug!(
            term = term,
            candidate = %candidate_id,
            "vote denied"
        );
        Ok(false)
    }

    /// Receive a vote from another node.
    pub fn receive_vote(&self, voter_id: NodeId) -> NeoResult<bool> {
        let mut votes = self.votes_received.write();
        if votes.contains(&voter_id) {
            return Ok(false);
        }
        votes.push(voter_id);

        let quorum = self.quorum_size();
        let received = votes.len();

        tracing::debug!(
            voter = %voter_id,
            votes_received = received,
            quorum = quorum,
            "vote received"
        );

        // Check if we have a quorum.
        if received >= quorum {
            // We win the election.
            drop(votes);
            self.become_leader()?;
            return Ok(true);
        }

        Ok(false)
    }

    /// Become the leader.
    fn become_leader(&self) -> NeoResult<()> {
        *self.leader.write() = Some(self.node_id);
        *self.election_started.write() = None;

        tracing::info!(
            node_id = %self.node_id,
            term = self.term(),
            "became leader"
        );

        Ok(())
    }

    /// Check if the election has timed out.
    pub fn is_election_timeout(&self) -> bool {
        if let Some(started) = *self.election_started.read() {
            let elapsed = Utc::now()
                .signed_duration_since(started)
                .num_milliseconds() as u64;
            elapsed >= self.election_timeout_ms
        } else {
            false
        }
    }

    // -- Log replication --

    /// Append a new log entry.
    pub fn append_entry(&self, command: String, data: Vec<u8>) -> NeoResult<u64> {
        let term = *self.term.read();
        let index = self.log_len() as u64 + 1;

        let entry = LogEntry {
            term,
            index,
            command,
            data,
        };

        self.log.write().push(entry);

        tracing::debug!(
            index = index,
            term = term,
            "log entry appended"
        );

        Ok(index)
    }

    /// Handle an append entries request from the leader.
    pub fn handle_append_entries(
        &self,
        term: u64,
        leader_id: NodeId,
        prev_log_index: u64,
        prev_log_term: u64,
        entries: Vec<LogEntry>,
        leader_commit: u64,
    ) -> NeoResult<bool> {
        let current_term = *self.term.read();

        // Reject if leader's term is behind.
        if term < current_term {
            return Ok(false);
        }

        // Accept the leader.
        *self.term.write() = term;
        *self.leader.write() = Some(leader_id);

        // Check log consistency.
        if prev_log_index > 0 {
            let log = self.log.read();
            if prev_log_index > log.len() as u64 {
                return Ok(false);
            }
            if prev_log_index > 0 {
                if let Some(entry) = log.get((prev_log_index - 1) as usize) {
                    if entry.term != prev_log_term {
                        return Ok(false);
                    }
                }
            }
        }

        // Append entries.
        {
            let mut log = self.log.write();
            for entry in entries {
                let idx = (entry.index - 1) as usize;
                if idx < log.len() {
                    if log[idx].term != entry.term {
                        log.truncate(idx);
                        log.push(entry);
                    }
                } else {
                    log.push(entry);
                }
            }
        }

        // Update commit index.
        if leader_commit > *self.commit_index.read() {
            let new_commit = leader_commit.min(self.log_len() as u64);
            *self.commit_index.write() = new_commit;
        }

        Ok(true)
    }

    /// Handle append entries acknowledgment.
    pub fn handle_append_entries_ack(
        &self,
        success: bool,
        match_index: u64,
    ) -> NeoResult<()> {
        if success {
            // Update the match index for this follower.
            tracing::debug!(
                match_index = match_index,
                "append entries acknowledged"
            );
        } else {
            tracing::debug!("append entries rejected");
        }
        Ok(())
    }

    /// Get the last log term.
    fn last_log_term(&self) -> u64 {
        let log = self.log.read();
        log.last().map_or(0, |e| e.term)
    }

    /// Get log entries in a range.
    pub fn get_entries(&self, from: u64, to: u64) -> Vec<LogEntry> {
        let log = self.log.read();
        log.iter()
            .filter(|e| e.index >= from && e.index <= to)
            .cloned()
            .collect()
    }

    // -- Leadership --

    /// Step down as leader.
    pub fn step_down(&self) {
        *self.leader.write() = None;
        *self.voted_for.write() = None;
        *self.votes_received.write() = Vec::new();
        *self.election_started.write() = None;
        tracing::info!(node_id = %self.node_id, "stepped down as leader");
    }

    /// Update the commit index after applying entries.
    pub fn advance_commit_index(&self, index: u64) {
        let mut ci = self.commit_index.write();
        if index > *ci {
            *ci = index;
        }
    }

    /// Apply committed entries.
    pub fn apply_committed(&self) -> Vec<LogEntry> {
        let mut applied = Vec::new();
        let mut last_applied = self.last_applied.write();
        let commit_index = *self.commit_index.read();
        let log = self.log.read();

        for entry in log.iter() {
            if entry.index > *last_applied && entry.index <= commit_index {
                applied.push(entry.clone());
                *last_applied = entry.index;
            }
        }

        applied
    }

    /// Get consensus state snapshot.
    pub fn snapshot(&self) -> ConsensusSnapshot {
        ConsensusSnapshot {
            term: self.term(),
            leader: self.leader(),
            commit_index: self.commit_index(),
            last_applied: self.last_applied(),
            log_length: self.log_len(),
            cluster_size: self.cluster_size(),
            quorum_size: self.quorum_size(),
        }
    }
}

impl std::fmt::Debug for ConsensusEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConsensusEngine")
            .field("node_id", &self.node_id)
            .field("term", &self.term())
            .field("leader", &self.leader())
            .field("log_len", &self.log_len())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// LogEntry
// ---------------------------------------------------------------------------

/// A single consensus log entry.
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

// ---------------------------------------------------------------------------
// ConsensusSnapshot
// ---------------------------------------------------------------------------

/// Read-only snapshot of consensus state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusSnapshot {
    pub term: u64,
    pub leader: Option<NodeId>,
    pub commit_index: u64,
    pub last_applied: u64,
    pub log_length: usize,
    pub cluster_size: usize,
    pub quorum_size: usize,
}

// ---------------------------------------------------------------------------
// ConsensusProtocol (trait)
// ---------------------------------------------------------------------------

/// Trait for pluggable consensus implementations.
#[async_trait::async_trait]
pub trait ConsensusProtocol: Send + Sync + std::fmt::Debug {
    /// Start the consensus protocol.
    async fn start(&self) -> NeoResult<()>;

    /// Stop the consensus protocol.
    async fn stop(&self) -> NeoResult<()>;

    /// Propose a value for consensus.
    async fn propose(&self, command: String, data: Vec<u8>) -> NeoResult<u64>;

    /// Get the current leader.
    fn leader(&self) -> Option<NodeId>;

    /// Check if this node is the leader.
    fn is_leader(&self) -> bool;

    /// Get consensus state.
    fn state(&self) -> ConsensusSnapshot;
}

// ---------------------------------------------------------------------------
// InMemoryConsensus
// ---------------------------------------------------------------------------

/// Simple in-memory consensus for single-node or testing.
#[derive(Debug)]
pub struct InMemoryConsensus {
    engine: Arc<ConsensusEngine>,
}

impl InMemoryConsensus {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            engine: Arc::new(ConsensusEngine::new(node_id, 5000)),
        }
    }

    pub fn engine(&self) -> &Arc<ConsensusEngine> {
        &self.engine
    }
}

#[async_trait::async_trait]
impl ConsensusProtocol for InMemoryConsensus {
    async fn start(&self) -> NeoResult<()> {
        self.engine.start_election()?;
        // In single-node mode, we immediately become leader.
        self.engine.receive_vote(self.engine.node_id())?;
        tracing::info!("in-memory consensus started, self-elected as leader");
        Ok(())
    }

    async fn stop(&self) -> NeoResult<()> {
        self.engine.step_down();
        Ok(())
    }

    async fn propose(&self, command: String, data: Vec<u8>) -> NeoResult<u64> {
        if !self.engine.is_leader() {
            return Err(DistributedError::consensus("not the leader"));
        }
        let index = self.engine.append_entry(command, data)?;
        self.engine.advance_commit_index(index);
        Ok(index)
    }

    fn leader(&self) -> Option<NodeId> {
        self.engine.leader()
    }

    fn is_leader(&self) -> bool {
        self.engine.is_leader()
    }

    fn state(&self) -> ConsensusSnapshot {
        self.engine.snapshot()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node_ids(n: usize) -> Vec<NodeId> {
        (0..n).map(|_| NodeId::new()).collect()
    }

    #[test]
    fn election_single_node() {
        let ids = node_ids(1);
        let engine = ConsensusEngine::new(ids[0], 5000);
        engine.set_cluster_nodes(ids.clone());

        engine.start_election().unwrap();
        engine.receive_vote(ids[0]).unwrap();
        assert!(engine.is_leader());
    }

    #[test]
    fn election_majority() {
        let ids = node_ids(3);
        let engine = ConsensusEngine::new(ids[0], 5000);
        engine.set_cluster_nodes(ids.clone());

        engine.start_election().unwrap();
        engine.receive_vote(ids[0]).unwrap(); // Self vote.
        let won = engine.receive_vote(ids[1]).unwrap(); // 2 votes = quorum.
        assert!(won);
        assert!(engine.is_leader());
    }

    #[test]
    fn vote_request() {
        let ids = node_ids(3);
        let engine = ConsensusEngine::new(ids[0], 5000);
        engine.set_cluster_nodes(ids.clone());

        engine.start_election().unwrap();

        let granted = engine
            .handle_vote_request(1, ids[1], 0, 0)
            .unwrap();
        // We already voted for ourselves, so deny.
        assert!(!granted);
    }

    #[test]
    fn append_and_apply() {
        let ids = node_ids(1);
        let engine = ConsensusEngine::new(ids[0], 5000);
        engine.set_cluster_nodes(ids.clone());
        engine.start_election().unwrap();
        engine.receive_vote(ids[0]).unwrap();

        let index = engine.append_entry("test".to_string(), vec![1, 2, 3]).unwrap();
        assert_eq!(index, 1);
        engine.advance_commit_index(index);

        let applied = engine.apply_committed();
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0].command, "test");
    }

    #[test]
    fn consensus_snapshot() {
        let ids = node_ids(3);
        let engine = ConsensusEngine::new(ids[0], 5000);
        engine.set_cluster_nodes(ids);

        let snap = engine.snapshot();
        assert_eq!(snap.term, 0);
        assert_eq!(snap.log_length, 0);
    }

    #[tokio::test]
    async fn in_memory_consensus() {
        let id = NodeId::new();
        let consensus = InMemoryConsensus::new(id);
        consensus.start().await.unwrap();
        assert!(consensus.is_leader());

        let index = consensus
            .propose("cmd".to_string(), vec![1])
            .await
            .unwrap();
        assert_eq!(index, 1);

        let state = consensus.state();
        assert_eq!(state.term, 1);
    }

    #[test]
    fn quorum_calculation() {
        let ids = node_ids(5);
        let engine = ConsensusEngine::new(ids[0], 5000);
        engine.set_cluster_nodes(ids);
        assert_eq!(engine.quorum_size(), 3);
        assert_eq!(engine.cluster_size(), 5);
    }

    #[test]
    fn election_timeout() {
        let ids = node_ids(1);
        let engine = ConsensusEngine::new(ids[0], 0); // Immediate timeout.
        engine.start_election().unwrap();
        assert!(engine.is_election_timeout());
    }

    #[test]
    fn step_down() {
        let ids = node_ids(1);
        let engine = ConsensusEngine::new(ids[0], 5000);
        engine.set_cluster_nodes(ids.clone());
        engine.start_election().unwrap();
        engine.receive_vote(ids[0]).unwrap();
        assert!(engine.is_leader());

        engine.step_down();
        assert!(!engine.is_leader());
        assert!(engine.leader().is_none());
    }
}
