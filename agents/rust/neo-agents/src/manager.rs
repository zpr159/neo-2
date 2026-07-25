use std::collections::HashMap;
use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::{mpsc, RwLock};

use crate::agent::{Agent, AgentCommand, AgentContext, AgentRuntimeHandle};
use crate::communication::{AgentMessage, MessageChannelRegistry};
use crate::error::{AgentError, AgentResult};
use crate::types::{AgentConfiguration, AgentId, AgentRole, AgentSnapshot, AgentStatus, AgentType};

// ---------------------------------------------------------------------------
// AgentRuntime
// ---------------------------------------------------------------------------

/// The runtime state for a single running agent, combining the agent with
/// its communication channels and context.
pub struct AgentRuntime {
    /// The stateful agent instance.
    pub agent: Agent,
    /// Runtime handle for sending commands.
    pub handle: AgentRuntimeHandle,
    /// The agent's execution context.
    pub context: AgentContext,
    /// Command receiver (used by the agent's run loop).
    pub command_rx: mpsc::Receiver<AgentCommand>,
}

impl AgentRuntime {
    /// Create a new runtime from an agent.
    #[must_use]
    pub fn new(agent: Agent, context: AgentContext) -> (Self, AgentRuntimeHandle) {
        let (command_tx, command_rx) = mpsc::channel(64);
        let handle = AgentRuntimeHandle::new(agent.id(), command_tx);

        let runtime = Self {
            agent,
            handle: handle.clone(),
            context,
            command_rx,
        };

        (runtime, handle)
    }
}

// ---------------------------------------------------------------------------
// AgentRegistry
// ---------------------------------------------------------------------------

/// Thread-safe registry of all agents managed by the system.
pub struct AgentRegistry {
    /// All registered agents by ID.
    agents: DashMap<AgentId, Arc<RwLock<Agent>>>,
    /// Name-to-ID index for lookup by name.
    name_index: DashMap<String, AgentId>,
    /// Role index: role -> set of agent IDs.
    role_index: DashMap<AgentRole, Vec<AgentId>>,
    /// Type index: type -> set of agent IDs.
    type_index: DashMap<AgentType, Vec<AgentId>>,
    /// Status index: status -> set of agent IDs.
    status_index: DashMap<AgentStatus, Vec<AgentId>>,
}

impl AgentRegistry {
    /// Create a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            agents: DashMap::new(),
            name_index: DashMap::new(),
            role_index: DashMap::new(),
            type_index: DashMap::new(),
            status_index: DashMap::new(),
        }
    }

    /// Register an agent in the registry.
    pub fn register(&self, agent: Agent) -> AgentResult<AgentId> {
        let id = agent.id();
        let name = agent.name().to_string();
        let role = agent.role().clone();
        let agent_type = agent.agent_type();
        let status = agent.status();

        if self.name_index.contains_key(&name) {
            return Err(AgentError::AlreadyRegistered(format!(
                "agent with name '{name}' already exists"
            )));
        }

        self.agents.insert(id, Arc::new(RwLock::new(agent)));
        self.name_index.insert(name, id);
        self.role_index.entry(role).or_default().push(id);
        self.type_index.entry(agent_type).or_default().push(id);
        self.status_index.entry(status).or_default().push(id);

        Ok(id)
    }

    /// Unregister an agent from the registry.
    pub fn unregister(&self, id: &AgentId) -> AgentResult<()> {
        if let Some((_, agent_arc)) = self.agents.remove(id) {
            if let Ok(agent) = agent_arc.try_read() {
                let name = agent.name().to_string();
                drop(agent);
                self.name_index.remove(&name);
            }

            // Clean up role index
            for mut entry in self.role_index.iter_mut() {
                entry.value_mut().retain(|a| a != id);
            }

            // Clean up type index
            for mut entry in self.type_index.iter_mut() {
                entry.value_mut().retain(|a| a != id);
            }

            // Clean up status index
            for mut entry in self.status_index.iter_mut() {
                entry.value_mut().retain(|a| a != id);
            }

            Ok(())
        } else {
            Err(AgentError::NotFound(format!("agent {id} not found")))
        }
    }

    /// Get a read reference to an agent.
    pub fn get(&self, id: &AgentId) -> Option<AgentSnapshot> {
        self.agents
            .get(id)
            .and_then(|agent| agent.try_read().ok().map(|a| a.snapshot()))
    }

    /// Get a snapshot of an agent (async-safe).
    pub fn get_async(&self, id: &AgentId) -> Option<AgentSnapshot> {
        self.agents
            .get(id)
            .and_then(|agent| agent.try_read().ok().map(|a| a.snapshot()))
    }

    /// Get a write handle to an agent.
    pub fn get_agent(&self, id: &AgentId) -> Option<Arc<RwLock<Agent>>> {
        self.agents.get(id).map(|a| a.value().clone())
    }

    /// Look up an agent by name.
    pub fn get_by_name(&self, name: &str) -> Option<AgentId> {
        self.name_index.get(name).map(|id| *id)
    }

    /// List all registered agent IDs.
    #[must_use]
    pub fn list_all(&self) -> Vec<AgentId> {
        self.agents.iter().map(|entry| *entry.key()).collect()
    }

    /// List agents matching a specific status.
    #[must_use]
    pub fn list_by_status(&self, status: AgentStatus) -> Vec<AgentId> {
        self.status_index
            .get(&status)
            .map(|ids| ids.clone())
            .unwrap_or_default()
    }

    /// List agents matching a specific role.
    #[must_use]
    pub fn list_by_role(&self, role: &AgentRole) -> Vec<AgentId> {
        self.role_index
            .get(role)
            .map(|ids| ids.clone())
            .unwrap_or_default()
    }

    /// List agents matching a specific type.
    #[must_use]
    pub fn list_by_type(&self, agent_type: &AgentType) -> Vec<AgentId> {
        self.type_index
            .get(agent_type)
            .map(|ids| ids.clone())
            .unwrap_or_default()
    }

    /// Return the total number of registered agents.
    #[must_use]
    pub fn count(&self) -> usize {
        self.agents.len()
    }

    /// Return the number of agents in a specific status.
    #[must_use]
    pub fn count_by_status(&self, status: AgentStatus) -> usize {
        self.status_index
            .get(&status)
            .map(|ids| ids.len())
            .unwrap_or(0)
    }

    /// Take a snapshot of all agents.
    #[must_use]
    pub fn snapshot_all(&self) -> Vec<AgentSnapshot> {
        self.agents
            .iter()
            .filter_map(|entry| entry.value().try_read().ok().map(|a| a.snapshot()))
            .collect()
    }

    /// Update the status index when an agent changes status.
    pub fn update_status_index(
        &self,
        id: &AgentId,
        old_status: AgentStatus,
        new_status: AgentStatus,
    ) {
        // Remove from old status
        if let Some(mut old_list) = self.status_index.get_mut(&old_status) {
            old_list.retain(|a| a != id);
        }
        // Add to new status
        self.status_index.entry(new_status).or_default().push(*id);
    }

    /// Clear the entire registry.
    pub fn clear(&self) {
        self.agents.clear();
        self.name_index.clear();
        self.role_index.clear();
        self.type_index.clear();
        self.status_index.clear();
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// AgentManager
// ---------------------------------------------------------------------------

/// The central manager responsible for creating, managing, and coordinating
/// all agents in the Neo AGI Operating System.
///
/// The `AgentManager` is the primary API surface for the agent framework.
/// It owns the registry of agents, manages their lifecycles, coordinates
/// communication, and provides monitoring capabilities.
pub struct AgentManager {
    /// The agent registry.
    registry: Arc<AgentRegistry>,
    /// Message channel registry for inter-agent communication.
    pub message_channels: Arc<MessageChannelRegistry>,
    /// Runtime handles for running agents: agent_id -> handle.
    handles: DashMap<AgentId, AgentRuntimeHandle>,
    /// Agent contexts.
    contexts: DashMap<AgentId, AgentContext>,
    /// Maximum number of concurrent agents.
    max_agents: usize,
    /// Global shared properties.
    shared_properties: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    /// Whether the manager has been shut down.
    is_shutdown: Arc<std::sync::atomic::AtomicBool>,
}

impl AgentManager {
    /// Create a new agent manager with the specified maximum agent count.
    #[must_use]
    pub fn new(max_agents: usize) -> Self {
        Self {
            registry: Arc::new(AgentRegistry::new()),
            message_channels: Arc::new(MessageChannelRegistry::new(256)),
            handles: DashMap::new(),
            contexts: DashMap::new(),
            max_agents,
            shared_properties: Arc::new(RwLock::new(HashMap::new())),
            is_shutdown: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Create a new agent manager with a builder pattern.
    #[must_use]
    pub fn builder() -> AgentManagerBuilder {
        AgentManagerBuilder::default()
    }

    /// Create a new agent from a configuration and register it.
    pub async fn create_agent(&self, config: AgentConfiguration) -> AgentResult<AgentId> {
        if self.is_shutdown.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(AgentError::Internal("manager is shut down".into()));
        }

        if self.registry.count() >= self.max_agents {
            return Err(AgentError::QuotaExceeded(format!(
                "maximum agent count {} reached",
                self.max_agents
            )));
        }

        let mut agent = Agent::new(config.clone());
        agent.initialize()?;

        let id = agent.id();
        let context = AgentContext::new(id, config.clone());

        // Create command channel
        let (command_tx, _command_rx) = mpsc::channel(64);
        let handle = AgentRuntimeHandle::new(id, command_tx);

        // Register agent inbox for messaging
        let (inbox_tx, _inbox_rx) = mpsc::channel(256);
        self.message_channels.register_inbox(id, inbox_tx);

        self.registry.register(agent)?;
        self.handles.insert(id, handle.clone());
        self.contexts.insert(id, context);

        tracing::info!("Created agent {id} ({})", config.name);
        Ok(id)
    }

    /// Start a registered agent.
    pub async fn start_agent(&self, id: AgentId) -> AgentResult<()> {
        let agent_arc = self
            .registry
            .get_agent(&id)
            .ok_or_else(|| AgentError::NotFound(format!("agent {id} not found")))?;

        let mut agent = agent_arc.write().await;
        let old_status = agent.status();
        agent.start()?;
        let new_status = agent.status();
        drop(agent);

        self.registry
            .update_status_index(&id, old_status, new_status);
        tracing::info!("Started agent {id}");

        // Notify via handle if available
        if let Some(handle) = self.handles.get(&id) {
            let _ = handle.send_command(AgentCommand::Heartbeat).await;
        }

        Ok(())
    }

    /// Stop a running agent gracefully.
    pub async fn stop_agent(&self, id: AgentId) -> AgentResult<()> {
        let agent_arc = self
            .registry
            .get_agent(&id)
            .ok_or_else(|| AgentError::NotFound(format!("agent {id} not found")))?;

        let mut agent = agent_arc.write().await;
        let old_status = agent.status();
        agent.stop()?;
        let new_status = agent.status();
        drop(agent);

        self.registry
            .update_status_index(&id, old_status, new_status);
        tracing::info!("Stopped agent {id}");
        Ok(())
    }

    /// Pause a running agent.
    pub async fn pause_agent(&self, id: AgentId) -> AgentResult<()> {
        let agent_arc = self
            .registry
            .get_agent(&id)
            .ok_or_else(|| AgentError::NotFound(format!("agent {id} not found")))?;

        let mut agent = agent_arc.write().await;
        let old_status = agent.status();
        agent.pause()?;
        let new_status = agent.status();
        drop(agent);

        self.registry
            .update_status_index(&id, old_status, new_status);
        tracing::info!("Paused agent {id}");
        Ok(())
    }

    /// Resume a paused agent.
    pub async fn resume_agent(&self, id: AgentId) -> AgentResult<()> {
        let agent_arc = self
            .registry
            .get_agent(&id)
            .ok_or_else(|| AgentError::NotFound(format!("agent {id} not found")))?;

        let mut agent = agent_arc.write().await;
        let old_status = agent.status();
        agent.resume()?;
        let new_status = agent.status();
        drop(agent);

        self.registry
            .update_status_index(&id, old_status, new_status);
        tracing::info!("Resumed agent {id}");
        Ok(())
    }

    /// Restart an agent.
    pub async fn restart_agent(&self, id: AgentId) -> AgentResult<()> {
        let agent_arc = self
            .registry
            .get_agent(&id)
            .ok_or_else(|| AgentError::NotFound(format!("agent {id} not found")))?;

        let mut agent = agent_arc.write().await;
        let old_status = agent.status();
        agent.restart()?;
        let new_status = agent.status();
        drop(agent);

        self.registry
            .update_status_index(&id, old_status, new_status);
        tracing::info!("Restarted agent {id}");
        Ok(())
    }

    /// Terminate an agent permanently and remove it from the registry.
    pub async fn terminate_agent(&self, id: AgentId) -> AgentResult<()> {
        let agent_arc = self
            .registry
            .get_agent(&id)
            .ok_or_else(|| AgentError::NotFound(format!("agent {id} not found")))?;

        {
            let mut agent = agent_arc.write().await;
            agent.terminate()?;
        }

        // Remove from all structures
        self.handles.remove(&id);
        self.contexts.remove(&id);
        self.message_channels.remove_agent(&id);
        self.registry.unregister(&id)?;

        tracing::info!("Terminated agent {id}");
        Ok(())
    }

    /// Get a snapshot of an agent.
    pub async fn inspect_agent(&self, id: AgentId) -> AgentResult<AgentSnapshot> {
        self.registry
            .get_async(&id)
            .ok_or_else(|| AgentError::NotFound(format!("agent {id} not found")))
    }

    /// List all agents, optionally filtered by status.
    #[must_use]
    pub fn list_agents(&self, status_filter: Option<AgentStatus>) -> Vec<AgentId> {
        match status_filter {
            Some(status) => self.registry.list_by_status(status),
            None => self.registry.list_all(),
        }
    }

    /// List agents by role.
    #[must_use]
    pub fn list_agents_by_role(&self, role: &AgentRole) -> Vec<AgentId> {
        self.registry.list_by_role(role)
    }

    /// Get the total agent count.
    #[must_use]
    pub fn agent_count(&self) -> usize {
        self.registry.count()
    }

    /// Get aggregate statistics.
    #[must_use]
    pub fn statistics(&self) -> crate::types::AgentStatistics {
        let snapshots = self.registry.snapshot_all();
        let total = snapshots.len() as u64;
        let active = snapshots.iter().filter(|s| s.status.is_active()).count() as u64;
        let failed = snapshots
            .iter()
            .filter(|s| s.status == AgentStatus::Failed)
            .count() as u64;
        let total_tasks_completed: u64 = snapshots.iter().map(|s| s.metrics.tasks_completed).sum();
        let total_tasks_failed: u64 = snapshots.iter().map(|s| s.metrics.tasks_failed).sum();
        let total_messages_sent: u64 = snapshots.iter().map(|s| s.metrics.messages_sent).sum();
        let total_messages_received: u64 =
            snapshots.iter().map(|s| s.metrics.messages_received).sum();

        crate::types::AgentStatistics {
            total_agents_created: total,
            active_agents: active,
            failed_agents: failed,
            total_tasks_assigned: total_tasks_completed + total_tasks_failed,
            total_tasks_completed,
            total_tasks_failed,
            total_messages_sent,
            total_messages_received,
            avg_agent_uptime_secs: if total > 0 {
                snapshots.iter().map(|s| s.metrics.uptime_secs).sum::<u64>() as f64 / total as f64
            } else {
                0.0
            },
            avg_task_completion_ms: 0.0,
            system_utilization: if self.max_agents > 0 {
                active as f64 / self.max_agents as f64
            } else {
                0.0
            },
        }
    }

    /// Send a message from one agent to another.
    pub async fn send_message(&self, msg: AgentMessage) -> AgentResult<()> {
        self.message_channels.send_direct(msg).await
    }

    /// Broadcast a message to all agents.
    pub async fn broadcast(&self, msg: AgentMessage) -> AgentResult<()> {
        let agent_ids = self.registry.list_all();
        for agent_id in &agent_ids {
            if let Some(sender) = self.message_channels.inboxes.get(agent_id) {
                let _ = sender.send(msg.clone()).await;
            }
        }
        Ok(())
    }

    /// Get a reference to the registry.
    #[must_use]
    pub fn registry(&self) -> &AgentRegistry {
        &self.registry
    }

    /// Get a shared property value.
    pub async fn get_shared_property(&self, key: &str) -> Option<serde_json::Value> {
        let props = self.shared_properties.read().await;
        props.get(key).cloned()
    }

    /// Set a shared property value.
    pub async fn set_shared_property(&self, key: String, value: serde_json::Value) {
        let mut props = self.shared_properties.write().await;
        props.insert(key, value);
    }

    /// Shutdown all agents and the manager.
    pub async fn shutdown(&self) -> AgentResult<()> {
        self.is_shutdown
            .store(true, std::sync::atomic::Ordering::SeqCst);

        let agent_ids: Vec<AgentId> = self.registry.list_all();
        for id in agent_ids {
            let _ = self.stop_agent(id).await;
        }

        self.handles.clear();
        self.contexts.clear();
        self.registry.clear();

        tracing::info!("AgentManager shut down");
        Ok(())
    }

    /// Check if the manager has been shut down.
    #[must_use]
    pub fn is_shutdown(&self) -> bool {
        self.is_shutdown.load(std::sync::atomic::Ordering::SeqCst)
    }
}

// ---------------------------------------------------------------------------
// AgentManagerBuilder
// ---------------------------------------------------------------------------

/// Builder for constructing an `AgentManager`.
#[derive(Debug, Default)]
pub struct AgentManagerBuilder {
    max_agents: Option<usize>,
}

impl AgentManagerBuilder {
    /// Set the maximum number of agents.
    #[must_use]
    pub fn with_max_agents(mut self, max: usize) -> Self {
        self.max_agents = Some(max);
        self
    }

    /// Build the `AgentManager`.
    #[must_use]
    pub fn build(self) -> AgentManager {
        AgentManager::new(self.max_agents.unwrap_or(64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(name: &str) -> AgentConfiguration {
        AgentConfiguration::new(name).with_role(AgentRole::Executor)
    }

    #[tokio::test]
    async fn test_create_and_start_agent() {
        let mgr = AgentManager::new(10);
        let id = mgr.create_agent(test_config("agent-1")).await.unwrap();
        mgr.start_agent(id).await.unwrap();

        let snap = mgr.inspect_agent(id).await.unwrap();
        assert_eq!(snap.status, AgentStatus::Running);
        assert_eq!(snap.name, "agent-1");

        mgr.stop_agent(id).await.unwrap();
        let snap = mgr.inspect_agent(id).await.unwrap();
        assert_eq!(snap.status, AgentStatus::Stopped);
    }

    #[tokio::test]
    async fn test_agent_lifecycle_through_manager() {
        let mgr = AgentManager::new(10);
        let id = mgr.create_agent(test_config("lifecycle")).await.unwrap();

        mgr.start_agent(id).await.unwrap();
        assert_eq!(
            mgr.inspect_agent(id).await.unwrap().status,
            AgentStatus::Running
        );

        mgr.pause_agent(id).await.unwrap();
        assert_eq!(
            mgr.inspect_agent(id).await.unwrap().status,
            AgentStatus::Paused
        );

        mgr.resume_agent(id).await.unwrap();
        assert_eq!(
            mgr.inspect_agent(id).await.unwrap().status,
            AgentStatus::Running
        );

        mgr.restart_agent(id).await.unwrap();
        assert_eq!(
            mgr.inspect_agent(id).await.unwrap().status,
            AgentStatus::Ready
        );
    }

    #[tokio::test]
    async fn test_max_agents_enforced() {
        let mgr = AgentManager::new(2);
        mgr.create_agent(test_config("a1")).await.unwrap();
        mgr.create_agent(test_config("a2")).await.unwrap();
        let result = mgr.create_agent(test_config("a3")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_terminate_removes_agent() {
        let mgr = AgentManager::new(10);
        let id = mgr.create_agent(test_config("to-terminate")).await.unwrap();
        mgr.start_agent(id).await.unwrap();
        mgr.terminate_agent(id).await.unwrap();

        assert_eq!(mgr.agent_count(), 0);
        assert!(mgr.inspect_agent(id).await.is_err());
    }

    #[tokio::test]
    async fn test_list_agents() {
        let mgr = AgentManager::new(10);
        let id1 = mgr.create_agent(test_config("a1")).await.unwrap();
        let id2 = mgr.create_agent(test_config("a2")).await.unwrap();
        mgr.start_agent(id1).await.unwrap();

        let running = mgr.list_agents(Some(AgentStatus::Running));
        assert_eq!(running.len(), 1);
        assert!(running.contains(&id1));

        let ready = mgr.list_agents(Some(AgentStatus::Ready));
        assert_eq!(ready.len(), 1);
        assert!(ready.contains(&id2));

        let all = mgr.list_agents(None);
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_statistics() {
        let mgr = AgentManager::new(10);
        let id1 = mgr.create_agent(test_config("s1")).await.unwrap();
        let id2 = mgr.create_agent(test_config("s2")).await.unwrap();

        let stats = mgr.statistics();
        assert_eq!(stats.total_agents_created, 2);
        assert_eq!(stats.active_agents, 2);

        mgr.terminate_agent(id1).await.unwrap();
        mgr.terminate_agent(id2).await.unwrap();

        let stats = mgr.statistics();
        assert_eq!(stats.active_agents, 0);
    }

    #[tokio::test]
    async fn test_shutdown() {
        let mgr = AgentManager::new(10);
        mgr.create_agent(test_config("a1")).await.unwrap();
        mgr.create_agent(test_config("a2")).await.unwrap();

        mgr.shutdown().await.unwrap();
        assert!(mgr.is_shutdown());
        assert_eq!(mgr.agent_count(), 0);
    }

    #[tokio::test]
    async fn test_registry_by_name() {
        let mgr = AgentManager::new(10);
        let id = mgr.create_agent(test_config("named-agent")).await.unwrap();
        let found = mgr.registry().get_by_name("named-agent");
        assert_eq!(found, Some(id));
    }

    #[test]
    fn test_manager_builder() {
        let mgr = AgentManager::builder().with_max_agents(128).build();
        assert_eq!(mgr.max_agents, 128);
    }
}
