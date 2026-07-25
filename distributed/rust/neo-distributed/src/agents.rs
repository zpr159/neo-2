//! Distributed agents — remote agent execution, migration, placement,
//! replication, checkpointing, and restoration across cluster nodes.

use std::collections::HashMap;
use std::time::Instant;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{DistributedError, NeoResult};
use crate::types::{NodeCapabilities, NodeId, TaskPriority};

// ---------------------------------------------------------------------------
// AgentState
// ---------------------------------------------------------------------------

/// State of a distributed agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DistributedAgentState {
    /// Agent is being created.
    Initializing,
    /// Agent is running.
    Running,
    /// Agent is paused.
    Paused,
    /// Agent is being migrated.
    Migrating,
    /// Agent is checkpointed and stopped.
    Checkpointed,
    /// Agent has failed.
    Failed,
    /// Agent has been stopped.
    Stopped,
}

impl std::fmt::Display for DistributedAgentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Initializing => write!(f, "initializing"),
            Self::Running => write!(f, "running"),
            Self::Paused => write!(f, "paused"),
            Self::Migrating => write!(f, "migrating"),
            Self::Checkpointed => write!(f, "checkpointed"),
            Self::Failed => write!(f, "failed"),
            Self::Stopped => write!(f, "stopped"),
        }
    }
}

// ---------------------------------------------------------------------------
// RemoteAgent
// ---------------------------------------------------------------------------

/// A distributed agent that can execute on different nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteAgent {
    /// Unique agent identifier.
    pub id: Uuid,
    /// Agent name.
    pub name: String,
    /// Agent type / role.
    pub agent_type: String,
    /// Current state.
    pub state: DistributedAgentState,
    /// Current node hosting this agent.
    pub current_node: NodeId,
    /// Required capabilities.
    pub required_capabilities: Vec<String>,
    /// Priority.
    pub priority: TaskPriority,
    /// When the agent was created.
    pub created_at: DateTime<Utc>,
    /// When the agent last checkpointed.
    pub last_checkpoint: Option<DateTime<Utc>>,
    /// Checkpoint data (serialized agent state).
    pub checkpoint_data: Option<Vec<u8>>,
    /// Agent metadata.
    pub metadata: HashMap<String, String>,
    /// Number of migrations performed.
    pub migration_count: u32,
    /// Number of restarts.
    pub restart_count: u32,
}

impl RemoteAgent {
    /// Create a new remote agent.
    pub fn new(name: String, agent_type: String, node_id: NodeId) -> Self {
        tracing::info!(
            name = %name,
            agent_type = %agent_type,
            node_id = %node_id,
            "remote agent created"
        );
        Self {
            id: Uuid::new_v4(),
            name,
            agent_type,
            state: DistributedAgentState::Initializing,
            current_node: node_id,
            required_capabilities: Vec::new(),
            priority: TaskPriority::NORMAL,
            created_at: Utc::now(),
            last_checkpoint: None,
            checkpoint_data: None,
            metadata: HashMap::new(),
            migration_count: 0,
            restart_count: 0,
        }
    }

    /// Set required capabilities.
    #[must_use]
    pub fn with_capabilities(mut self, caps: Vec<String>) -> Self {
        self.required_capabilities = caps;
        self
    }

    /// Set priority.
    #[must_use]
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Check if the agent can run on a node with the given capabilities.
    pub fn can_run_on(&self, capabilities: &NodeCapabilities) -> bool {
        self.required_capabilities
            .iter()
            .all(|c| capabilities.supports_capability(c))
    }
}

// ---------------------------------------------------------------------------
// AgentCheckpoint
// ---------------------------------------------------------------------------

/// Checkpoint data for agent migration/restore.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCheckpoint {
    /// Agent ID.
    pub agent_id: Uuid,
    /// Serialized agent state.
    pub state: Vec<u8>,
    /// Checkpoint timestamp.
    pub timestamp: DateTime<Utc>,
    /// Source node.
    pub source_node: NodeId,
    /// Checkpoint version.
    pub version: u64,
}

// ---------------------------------------------------------------------------
// AgentMigration
// ---------------------------------------------------------------------------

/// Tracks an in-progress agent migration.
#[derive(Debug, Clone)]
pub struct AgentMigration {
    /// Agent being migrated.
    pub agent_id: Uuid,
    /// Source node.
    pub source_node: NodeId,
    /// Destination node.
    pub destination_node: NodeId,
    /// When the migration started.
    pub started_at: Instant,
    /// Migration phase.
    pub phase: MigrationPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationPhase {
    Checkpointing,
    Transferring,
    Restoring,
    Complete,
}

// ---------------------------------------------------------------------------
// AgentPlacement
// ---------------------------------------------------------------------------

/// Placement information for an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPlacement {
    /// Agent ID.
    pub agent_id: Uuid,
    /// Current node.
    pub node_id: NodeId,
    /// When placed.
    pub placed_at: DateTime<Utc>,
    /// Placement score (higher = better).
    pub score: f64,
    /// Reasons for this placement.
    pub reasons: Vec<String>,
}

// ---------------------------------------------------------------------------
// DistributedAgentManager
// ---------------------------------------------------------------------------

/// Manages the lifecycle of distributed agents.
pub struct DistributedAgentManager {
    /// All agents.
    agents: RwLock<HashMap<Uuid, RemoteAgent>>,
    /// Active migrations.
    migrations: RwLock<HashMap<Uuid, AgentMigration>>,
    /// Agent placements.
    placements: RwLock<HashMap<Uuid, AgentPlacement>>,
    /// Checkpoints.
    checkpoints: RwLock<HashMap<Uuid, Vec<AgentCheckpoint>>>,
    /// Total migrations performed.
    total_migrations: std::sync::atomic::AtomicU64,
    /// Total checkpoints.
    total_checkpoints: std::sync::atomic::AtomicU64,
}

impl DistributedAgentManager {
    /// Create a new agent manager.
    pub fn new() -> Self {
        tracing::info!("distributed agent manager created");
        Self {
            agents: RwLock::new(HashMap::new()),
            migrations: RwLock::new(HashMap::new()),
            placements: RwLock::new(HashMap::new()),
            checkpoints: RwLock::new(HashMap::new()),
            total_migrations: std::sync::atomic::AtomicU64::new(0),
            total_checkpoints: std::sync::atomic::AtomicU64::new(0),
        }
    }

    // -- Agent lifecycle --

    /// Register a new agent.
    pub fn register(&self, agent: RemoteAgent) -> NeoResult<Uuid> {
        let id = agent.id;
        self.agents.write().insert(id, agent);
        tracing::info!(agent_id = %id, "agent registered");
        Ok(id)
    }

    /// Get an agent by ID.
    pub fn get(&self, id: Uuid) -> Option<RemoteAgent> {
        self.agents.read().get(&id).cloned()
    }

    /// Update agent state.
    pub fn set_state(&self, id: Uuid, state: DistributedAgentState) -> NeoResult<()> {
        let mut agents = self.agents.write();
        let agent = agents
            .get_mut(&id)
            .ok_or_else(|| DistributedError::node(format!("agent not found: {id}")))?;
        agent.state = state;
        Ok(())
    }

    /// Remove an agent.
    pub fn remove(&self, id: Uuid) -> Option<RemoteAgent> {
        self.agents.write().remove(&id)
    }

    /// Get all agents.
    pub fn list(&self) -> Vec<RemoteAgent> {
        self.agents.read().values().cloned().collect()
    }

    /// Get agents on a specific node.
    pub fn on_node(&self, node_id: NodeId) -> Vec<RemoteAgent> {
        self.agents
            .read()
            .values()
            .filter(|a| a.current_node == node_id)
            .cloned()
            .collect()
    }

    /// Get agents in a specific state.
    pub fn in_state(&self, state: DistributedAgentState) -> Vec<RemoteAgent> {
        self.agents
            .read()
            .values()
            .filter(|a| a.state == state)
            .cloned()
            .collect()
    }

    /// Total agent count.
    pub fn count(&self) -> usize {
        self.agents.read().len()
    }

    // -- Checkpointing --

    /// Create a checkpoint for an agent.
    pub fn checkpoint(&self, agent_id: Uuid, source_node: NodeId, data: Vec<u8>) -> NeoResult<()> {
        let checkpoint = AgentCheckpoint {
            agent_id,
            state: data,
            timestamp: Utc::now(),
            source_node,
            version: self
                .checkpoints
                .read()
                .get(&agent_id)
                .map_or(1, |v| v.len() as u64 + 1),
        };

        self.checkpoints
            .write()
            .entry(agent_id)
            .or_default()
            .push(checkpoint);

        if let Some(agent) = self.agents.write().get_mut(&agent_id) {
            agent.last_checkpoint = Some(Utc::now());
        }

        self.total_checkpoints
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        tracing::info!(agent_id = %agent_id, "agent checkpointed");
        Ok(())
    }

    /// Get latest checkpoint for an agent.
    pub fn latest_checkpoint(&self, agent_id: Uuid) -> Option<AgentCheckpoint> {
        self.checkpoints
            .read()
            .get(&agent_id)
            .and_then(|v| v.last())
            .cloned()
    }

    // -- Migration --

    /// Start migrating an agent to a new node.
    pub fn start_migration(
        &self,
        agent_id: Uuid,
        source: NodeId,
        destination: NodeId,
    ) -> NeoResult<()> {
        let migration = AgentMigration {
            agent_id,
            source_node: source,
            destination_node: destination,
            started_at: Instant::now(),
            phase: MigrationPhase::Checkpointing,
        };

        self.migrations.write().insert(agent_id, migration);
        self.set_state(agent_id, DistributedAgentState::Migrating)?;

        tracing::info!(
            agent_id = %agent_id,
            from = %source,
            to = %destination,
            "agent migration started"
        );

        Ok(())
    }

    /// Complete an agent migration.
    pub fn complete_migration(&self, agent_id: Uuid) -> NeoResult<()> {
        if let Some(mut migration) = self.migrations.write().remove(&agent_id) {
            migration.phase = MigrationPhase::Complete;
        }

        if let Some(agent) = self.agents.write().get_mut(&agent_id) {
            agent.migration_count += 1;
        }

        self.set_state(agent_id, DistributedAgentState::Running)?;
        self.total_migrations
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        tracing::info!(agent_id = %agent_id, "agent migration completed");
        Ok(())
    }

    /// Get active migrations.
    pub fn active_migrations(&self) -> Vec<AgentMigration> {
        self.migrations.read().values().cloned().collect()
    }

    // -- Placement --

    /// Place an agent on a node.
    pub fn place(&self, agent_id: Uuid, placement: AgentPlacement) -> NeoResult<()> {
        if let Some(agent) = self.agents.write().get_mut(&agent_id) {
            agent.current_node = placement.node_id;
        }
        self.placements.write().insert(agent_id, placement);
        Ok(())
    }

    /// Get placement for an agent.
    pub fn get_placement(&self, agent_id: Uuid) -> Option<AgentPlacement> {
        self.placements.read().get(&agent_id).cloned()
    }

    // -- Restore --

    /// Restore an agent from a checkpoint.
    pub fn restore(&self, agent_id: Uuid, target_node: NodeId) -> NeoResult<Vec<u8>> {
        let checkpoint = self
            .latest_checkpoint(agent_id)
            .ok_or_else(|| DistributedError::execution(format!("no checkpoint for agent {agent_id}")))?;

        if let Some(agent) = self.agents.write().get_mut(&agent_id) {
            agent.current_node = target_node;
            agent.state = DistributedAgentState::Running;
            agent.restart_count += 1;
        }

        tracing::info!(
            agent_id = %agent_id,
            node = %target_node,
            "agent restored from checkpoint"
        );

        Ok(checkpoint.state)
    }

    // -- Statistics --

    pub fn stats(&self) -> AgentManagerStats {
        let agents = self.agents.read();
        let running = agents.values().filter(|a| a.state == DistributedAgentState::Running).count();
        let migrating = agents.values().filter(|a| a.state == DistributedAgentState::Migrating).count();

        AgentManagerStats {
            total_agents: agents.len(),
            running,
            migrating,
            total_migrations: self.total_migrations.load(std::sync::atomic::Ordering::Relaxed),
            total_checkpoints: self.total_checkpoints.load(std::sync::atomic::Ordering::Relaxed),
        }
    }
}

impl Default for DistributedAgentManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentManagerStats {
    pub total_agents: usize,
    pub running: usize,
    pub migrating: usize,
    pub total_migrations: u64,
    pub total_checkpoints: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_agent() {
        let agent = RemoteAgent::new("test".to_string(), "worker".to_string(), NodeId::new());
        assert_eq!(agent.state, DistributedAgentState::Initializing);
    }

    #[test]
    fn register_and_list() {
        let mgr = DistributedAgentManager::new();
        let agent = RemoteAgent::new("a1".to_string(), "worker".to_string(), NodeId::new());
        let id = agent.id;
        mgr.register(agent).unwrap();
        assert_eq!(mgr.count(), 1);
        assert!(mgr.get(id).is_some());
    }

    #[test]
    fn checkpoint_and_restore() {
        let mgr = DistributedAgentManager::new();
        let node = NodeId::new();
        let agent = RemoteAgent::new("a1".to_string(), "worker".to_string(), node);
        let id = agent.id;
        mgr.register(agent).unwrap();

        mgr.checkpoint(id, node, vec![1, 2, 3]).unwrap();
        let cp = mgr.latest_checkpoint(id).unwrap();
        assert_eq!(cp.state, vec![1, 2, 3]);

        let data = mgr.restore(id, NodeId::new()).unwrap();
        assert_eq!(data, vec![1, 2, 3]);
    }

    #[test]
    fn migration() {
        let mgr = DistributedAgentManager::new();
        let src = NodeId::new();
        let dst = NodeId::new();
        let agent = RemoteAgent::new("a1".to_string(), "worker".to_string(), src);
        let id = agent.id;
        mgr.register(agent).unwrap();

        mgr.start_migration(id, src, dst).unwrap();
        assert_eq!(mgr.active_migrations().len(), 1);

        mgr.complete_migration(id).unwrap();
        assert_eq!(mgr.active_migrations().len(), 0);
    }

    #[test]
    fn agent_can_run_on() {
        let agent = RemoteAgent::new("a1".to_string(), "worker".to_string(), NodeId::new())
            .with_capabilities(vec!["inference".to_string()]);

        let caps_no_gpu = NodeCapabilities {
            capabilities: vec!["inference".to_string()],
            ..Default::default()
        };
        assert!(agent.can_run_on(&caps_no_gpu));

        let caps_wrong = NodeCapabilities {
            capabilities: vec!["ocr".to_string()],
            ..Default::default()
        };
        assert!(!agent.can_run_on(&caps_wrong));
    }
}
