use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::{mpsc, RwLock};

use crate::error::{AgentError, AgentResult};
use crate::types::{
    is_valid_transition, AgentConfiguration, AgentHealth, AgentId, AgentMetrics, AgentRole,
    AgentStatus, AgentType, AgentVersion,
};

// ---------------------------------------------------------------------------
// AgentRuntimeHandle
// ---------------------------------------------------------------------------

/// Handle used to communicate with a running agent.
#[derive(Debug, Clone)]
pub struct AgentRuntimeHandle {
    /// The agent's identifier.
    pub id: AgentId,
    /// Channel to send commands to the agent.
    command_tx: mpsc::Sender<AgentCommand>,
}

impl AgentRuntimeHandle {
    /// Create a new runtime handle.
    #[must_use]
    pub fn new(id: AgentId, command_tx: mpsc::Sender<AgentCommand>) -> Self {
        Self { id, command_tx }
    }

    /// Send a command to the agent, returning an error if the channel is closed.
    pub async fn send_command(&self, cmd: AgentCommand) -> AgentResult<()> {
        self.command_tx
            .send(cmd)
            .await
            .map_err(|_| AgentError::Internal("agent command channel closed".into()))
    }

    /// Request the agent to stop gracefully.
    pub async fn stop(&self) -> AgentResult<()> {
        self.send_command(AgentCommand::Stop).await
    }

    /// Request the agent to pause.
    pub async fn pause(&self) -> AgentResult<()> {
        self.send_command(AgentCommand::Pause).await
    }

    /// Request the agent to resume.
    pub async fn resume(&self) -> AgentResult<()> {
        self.send_command(AgentCommand::Resume).await
    }
}

// ---------------------------------------------------------------------------
// AgentCommand
// ---------------------------------------------------------------------------

/// Commands that can be sent to a running agent.
#[derive(Debug, Clone)]
pub enum AgentCommand {
    /// Gracefully stop the agent.
    Stop,
    /// Pause the agent (stops processing but retains state).
    Pause,
    /// Resume a paused agent.
    Resume,
    /// Update the agent's configuration.
    UpdateConfig(Box<AgentConfiguration>),
    /// Trigger a health check.
    HealthCheck,
    /// Trigger a heartbeat.
    Heartbeat,
    /// Shutdown the agent immediately.
    Shutdown,
}

// ---------------------------------------------------------------------------
// AgentContext
// ---------------------------------------------------------------------------

/// Shared context available to an agent during execution.
#[derive(Debug, Clone)]
pub struct AgentContext {
    /// The agent's own identifier.
    pub agent_id: AgentId,
    /// The agent's current configuration.
    pub config: AgentConfiguration,
    /// Shared properties across the agent system.
    pub shared_properties: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

impl AgentContext {
    /// Create a new agent context.
    #[must_use]
    pub fn new(agent_id: AgentId, config: AgentConfiguration) -> Self {
        Self {
            agent_id,
            config,
            shared_properties: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

// ---------------------------------------------------------------------------
// Agent (stateful runtime representation)
// ---------------------------------------------------------------------------

/// A fully stateful agent runtime instance.
///
/// Agents are the primary execution unit in the Neo AGI Operating System.
/// Each agent runs as an independent async task, processing commands from
/// its command channel and executing work according to its configuration
/// and role.
pub struct Agent {
    /// Unique agent identifier.
    id: AgentId,
    /// Current configuration.
    config: AgentConfiguration,
    /// Current status.
    status: AgentStatus,
    /// Current health.
    health: AgentHealth,
    /// When the agent was created.
    created_at: DateTime<Utc>,
    /// When the agent was last updated.
    updated_at: DateTime<Utc>,
    /// When the agent last sent a heartbeat.
    last_heartbeat: DateTime<Utc>,
    /// Runtime metrics.
    metrics: AgentMetrics,
    /// Recovery attempt counter.
    recovery_attempts: u32,
    /// Current error message, if any.
    error: Option<String>,
    /// Version of this agent instance.
    version: AgentVersion,
    /// Task IDs currently assigned to this agent.
    assigned_tasks: Vec<crate::types::TaskId>,
}

impl Agent {
    /// Create a new agent from a configuration.
    ///
    /// The agent starts in `Created` status.
    #[must_use]
    pub fn new(config: AgentConfiguration) -> Self {
        let now = Utc::now();
        Self {
            id: AgentId::new(),
            config,
            status: AgentStatus::Created,
            health: AgentHealth::Unknown,
            created_at: now,
            updated_at: now,
            last_heartbeat: now,
            metrics: AgentMetrics::default(),
            recovery_attempts: 0,
            error: None,
            version: AgentVersion::initial(),
            assigned_tasks: Vec::new(),
        }
    }

    /// Create a new agent with a pre-specified identifier.
    #[must_use]
    pub fn with_id(id: AgentId, config: AgentConfiguration) -> Self {
        let now = Utc::now();
        Self {
            id,
            config,
            status: AgentStatus::Created,
            health: AgentHealth::Unknown,
            created_at: now,
            updated_at: now,
            last_heartbeat: now,
            metrics: AgentMetrics::default(),
            recovery_attempts: 0,
            error: None,
            version: AgentVersion::initial(),
            assigned_tasks: Vec::new(),
        }
    }

    /// Create a new `AgentBuilder` for constructing an agent with a fluent API.
    #[must_use]
    pub fn builder() -> crate::sdk::AgentBuilder {
        crate::sdk::AgentBuilder::new()
    }

    /// Return the agent's identifier.
    #[must_use]
    pub fn id(&self) -> AgentId {
        self.id
    }

    /// Return the agent's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.config.name
    }

    /// Return the agent's current status.
    #[must_use]
    pub fn status(&self) -> AgentStatus {
        self.status
    }

    /// Return the agent's current health.
    #[must_use]
    pub fn health(&self) -> AgentHealth {
        self.health
    }

    /// Return the agent's configuration.
    #[must_use]
    pub fn config(&self) -> &AgentConfiguration {
        &self.config
    }

    /// Return a mutable reference to the agent's configuration.
    pub fn config_mut(&mut self) -> &mut AgentConfiguration {
        &mut self.config
    }

    /// Return the agent's type.
    #[must_use]
    pub fn agent_type(&self) -> AgentType {
        self.config.agent_type.clone()
    }

    /// Return the agent's role.
    #[must_use]
    pub fn role(&self) -> &AgentRole {
        &self.config.role
    }

    /// Return the creation timestamp.
    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Return the last update timestamp.
    #[must_use]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Return the last heartbeat timestamp.
    #[must_use]
    pub fn last_heartbeat(&self) -> DateTime<Utc> {
        self.last_heartbeat
    }

    /// Return a reference to the agent's metrics.
    #[must_use]
    pub fn metrics(&self) -> &AgentMetrics {
        &self.metrics
    }

    /// Return a mutable reference to the agent's metrics.
    pub fn metrics_mut(&mut self) -> &mut AgentMetrics {
        &mut self.metrics
    }

    /// Return the current error message, if any.
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Return the recovery attempt count.
    #[must_use]
    pub fn recovery_attempts(&self) -> u32 {
        self.recovery_attempts
    }

    /// Return the agent version.
    #[must_use]
    pub fn version(&self) -> AgentVersion {
        self.version
    }

    /// Return the list of assigned task IDs.
    #[must_use]
    pub fn assigned_tasks(&self) -> &[crate::types::TaskId] {
        &self.assigned_tasks
    }

    /// Assign a task to this agent.
    pub fn assign_task(&mut self, task_id: crate::types::TaskId) {
        self.assigned_tasks.push(task_id);
        self.metrics.tasks_active = self.assigned_tasks.len() as u64;
    }

    /// Remove a completed task from the agent's assignment list.
    pub fn complete_task(&mut self, task_id: &crate::types::TaskId) {
        self.assigned_tasks.retain(|t| t != task_id);
        self.metrics.tasks_active = self.assigned_tasks.len() as u64;
        self.metrics.tasks_completed += 1;
    }

    /// Remove a failed task from the agent's assignment list.
    pub fn fail_task(&mut self, task_id: &crate::types::TaskId) {
        self.assigned_tasks.retain(|t| t != task_id);
        self.metrics.tasks_active = self.assigned_tasks.len() as u64;
        self.metrics.tasks_failed += 1;
    }

    /// Attempt to transition to a new status.
    ///
    /// Returns `Ok(())` if the transition is valid, or an error describing
    /// why it is not.
    pub fn transition_to(&mut self, new_status: AgentStatus) -> AgentResult<()> {
        if !is_valid_transition(self.status, new_status) {
            return Err(AgentError::InvalidState(format!(
                "cannot transition from {} to {}",
                self.status, new_status
            )));
        }
        self.status = new_status;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Initialize the agent (Created -> Initializing -> Ready).
    pub fn initialize(&mut self) -> AgentResult<()> {
        self.transition_to(AgentStatus::Initializing)?;
        self.transition_to(AgentStatus::Ready)?;
        self.health = AgentHealth::Healthy;
        Ok(())
    }

    /// Start the agent (Ready -> Running).
    pub fn start(&mut self) -> AgentResult<()> {
        self.transition_to(AgentStatus::Running)?;
        self.health = AgentHealth::Healthy;
        Ok(())
    }

    /// Stop the agent gracefully.
    pub fn stop(&mut self) -> AgentResult<()> {
        match self.status {
            AgentStatus::Running | AgentStatus::Ready | AgentStatus::Waiting => {
                self.transition_to(AgentStatus::Stopping)?;
                self.transition_to(AgentStatus::Stopped)?;
                self.metrics.tasks_active = 0;
                Ok(())
            }
            AgentStatus::Paused => {
                self.transition_to(AgentStatus::Stopping)?;
                self.transition_to(AgentStatus::Stopped)?;
                self.metrics.tasks_active = 0;
                Ok(())
            }
            other => Err(AgentError::InvalidState(format!(
                "cannot stop agent in {other} state"
            ))),
        }
    }

    /// Pause the agent.
    pub fn pause(&mut self) -> AgentResult<()> {
        self.transition_to(AgentStatus::Paused)
    }

    /// Resume the agent from a paused state.
    pub fn resume(&mut self) -> AgentResult<()> {
        self.transition_to(AgentStatus::Running)
    }

    /// Mark the agent as failed.
    pub fn fail(&mut self, error: impl Into<String>) {
        let _ = self.transition_to(AgentStatus::Failed);
        self.error = Some(error.into());
        self.health = AgentHealth::Unhealthy;
    }

    /// Attempt recovery (Failed -> Recovering).
    pub fn attempt_recovery(&mut self) -> AgentResult<()> {
        if self.recovery_attempts >= self.config.max_recovery_attempts {
            return Err(AgentError::MaxRetriesExceeded(format!(
                "agent {} exceeded max recovery attempts ({})",
                self.id, self.config.max_recovery_attempts
            )));
        }
        self.recovery_attempts += 1;
        self.status = AgentStatus::Recovering;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Complete recovery (Recovering -> Ready/Running).
    pub fn complete_recovery(&mut self) -> AgentResult<()> {
        self.transition_to(AgentStatus::Ready)?;
        self.error = None;
        self.health = AgentHealth::Healthy;
        Ok(())
    }

    /// Restart the agent (Stopped -> Restarting -> Initializing).
    pub fn restart(&mut self) -> AgentResult<()> {
        self.transition_to(AgentStatus::Restarting)?;
        self.transition_to(AgentStatus::Initializing)?;
        self.transition_to(AgentStatus::Ready)?;
        self.recovery_attempts = 0;
        self.error = None;
        self.health = AgentHealth::Healthy;
        Ok(())
    }

    /// Terminate the agent permanently.
    pub fn terminate(&mut self) -> AgentResult<()> {
        match self.status {
            AgentStatus::Stopped | AgentStatus::Failed => {
                self.transition_to(AgentStatus::Terminated)?;
                Ok(())
            }
            AgentStatus::Running | AgentStatus::Ready | AgentStatus::Waiting => {
                self.transition_to(AgentStatus::Stopping)?;
                self.transition_to(AgentStatus::Stopped)?;
                self.transition_to(AgentStatus::Terminated)?;
                Ok(())
            }
            AgentStatus::Terminated => Ok(()),
            other => Err(AgentError::InvalidState(format!(
                "cannot terminate agent in {other} state"
            ))),
        }
    }

    /// Send a heartbeat.
    pub fn heartbeat(&mut self) {
        self.last_heartbeat = Utc::now();
        self.metrics.uptime_secs = (Utc::now() - self.created_at).num_seconds() as u64;
    }

    /// Update the agent's health.
    pub fn set_health(&mut self, health: AgentHealth) {
        self.health = health;
        self.updated_at = Utc::now();
    }

    /// Set a custom error message.
    pub fn set_error(&mut self, error: Option<String>) {
        self.error = error;
        self.updated_at = Utc::now();
    }

    /// Build a point-in-time snapshot.
    #[must_use]
    pub fn snapshot(&self) -> crate::types::AgentSnapshot {
        crate::types::AgentSnapshot {
            id: self.id,
            name: self.config.name.clone(),
            agent_type: self.config.agent_type.clone(),
            role: self.config.role.clone(),
            status: self.status,
            health: self.health,
            metrics: self.metrics.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            last_heartbeat: self.last_heartbeat,
            error: self.error.clone(),
            metadata: self.config.metadata.properties.clone(),
        }
    }

    /// Update the configuration.
    pub fn update_config(&mut self, config: AgentConfiguration) {
        self.config = config;
        self.updated_at = Utc::now();
    }

    /// Increment message sent counter.
    pub fn record_message_sent(&mut self) {
        self.metrics.messages_sent += 1;
    }

    /// Increment message received counter.
    pub fn record_message_received(&mut self) {
        self.metrics.messages_received += 1;
    }

    /// Record an error occurrence.
    pub fn record_error(&mut self) {
        self.metrics.error_count += 1;
    }

    /// Record a recovery.
    pub fn record_recovery(&mut self) {
        self.metrics.recovery_count += 1;
    }

    /// Update memory usage.
    pub fn set_memory_usage(&mut self, bytes: u64) {
        self.metrics.memory_used_bytes = bytes;
    }

    /// Update CPU time.
    pub fn add_cpu_time(&mut self, ms: u64) {
        self.metrics.cpu_time_ms += ms;
    }

    /// Check if the heartbeat has expired.
    #[must_use]
    pub fn heartbeat_expired(&self) -> bool {
        let elapsed = Utc::now()
            .signed_duration_since(self.last_heartbeat)
            .num_seconds() as u64;
        elapsed > self.config.heartbeat_interval_secs * 3
    }
}

impl std::fmt::Debug for Agent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Agent")
            .field("id", &self.id)
            .field("name", &self.config.name)
            .field("status", &self.status)
            .field("health", &self.health)
            .field("role", &self.config.role)
            .field("agent_type", &self.config.agent_type)
            .field("created_at", &self.created_at)
            .field("recovery_attempts", &self.recovery_attempts)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> AgentConfiguration {
        AgentConfiguration::new("test-agent")
            .with_role(AgentRole::Executor)
            .with_max_retries(3)
            .with_heartbeat_interval(10)
    }

    #[test]
    fn test_agent_creation() {
        let agent = Agent::new(test_config());
        assert_eq!(agent.name(), "test-agent");
        assert_eq!(agent.status(), AgentStatus::Created);
        assert_eq!(agent.health(), AgentHealth::Unknown);
    }

    #[test]
    fn test_agent_lifecycle() {
        let mut agent = Agent::new(test_config());
        agent.initialize().expect("init failed");
        assert_eq!(agent.status(), AgentStatus::Ready);

        agent.start().expect("start failed");
        assert_eq!(agent.status(), AgentStatus::Running);

        agent.stop().expect("stop failed");
        assert_eq!(agent.status(), AgentStatus::Stopped);
    }

    #[test]
    fn test_agent_pause_resume() {
        let mut agent = Agent::new(test_config());
        agent.initialize().unwrap();
        agent.start().unwrap();

        agent.pause().unwrap();
        assert_eq!(agent.status(), AgentStatus::Paused);

        agent.resume().unwrap();
        assert_eq!(agent.status(), AgentStatus::Running);
    }

    #[test]
    fn test_agent_failure_and_recovery() {
        let mut agent = Agent::new(test_config());
        agent.initialize().unwrap();
        agent.start().unwrap();

        agent.fail("test error");
        assert_eq!(agent.status(), AgentStatus::Failed);
        assert_eq!(agent.error(), Some("test error"));

        agent.attempt_recovery().unwrap();
        assert_eq!(agent.status(), AgentStatus::Recovering);

        agent.complete_recovery().unwrap();
        assert_eq!(agent.status(), AgentStatus::Ready);
        assert!(agent.error().is_none());
    }

    #[test]
    fn test_agent_terminate() {
        let mut agent = Agent::new(test_config());
        agent.initialize().unwrap();
        agent.start().unwrap();
        agent.terminate().unwrap();
        assert_eq!(agent.status(), AgentStatus::Terminated);
    }

    #[test]
    fn test_invalid_transition() {
        let mut agent = Agent::new(test_config());
        let result = agent.transition_to(AgentStatus::Running);
        assert!(result.is_err());
    }

    #[test]
    fn test_agent_snapshot() {
        let mut agent = Agent::new(test_config());
        agent.initialize().unwrap();
        let snap = agent.snapshot();
        assert_eq!(snap.name, "test-agent");
        assert_eq!(snap.status, AgentStatus::Ready);
    }

    #[test]
    fn test_task_assignment() {
        let mut agent = Agent::new(test_config());
        let tid = crate::types::TaskId::new();
        agent.assign_task(tid);
        assert_eq!(agent.assigned_tasks().len(), 1);

        agent.complete_task(&tid);
        assert_eq!(agent.assigned_tasks().len(), 0);
        assert_eq!(agent.metrics().tasks_completed, 1);
    }

    #[test]
    fn test_heartbeat() {
        let mut agent = Agent::new(test_config());
        agent.heartbeat();
        assert!(!agent.heartbeat_expired());
    }

    #[test]
    fn test_max_recovery_exceeded() {
        let mut agent = Agent::new(test_config());
        agent.config.max_recovery_attempts = 3;
        agent.initialize().unwrap();
        agent.start().unwrap();
        for _ in 0..3 {
            agent.fail("error");
            agent.attempt_recovery().unwrap();
            agent.complete_recovery().unwrap();
            agent.start().unwrap();
        }
        agent.fail("error");
        let result = agent.attempt_recovery();
        assert!(result.is_err());
    }
}
