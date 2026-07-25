use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Re-export core identity types for convenience
// ---------------------------------------------------------------------------

pub use neo_core::id::AgentId;
pub use neo_core::id::TaskId;

// ---------------------------------------------------------------------------
// AgentType
// ---------------------------------------------------------------------------

/// The architectural type of an agent.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentType {
    /// A standard autonomous agent.
    #[default]
    Autonomous,
    /// A reactive agent that only responds to stimuli.
    Reactive,
    /// A deliberative agent that plans before acting.
    Deliberative,
    /// A hybrid agent combining reactive and deliberative.
    Hybrid,
    /// A BDI (Belief-Desire-Intention) agent.
    Bdi,
    /// A learning agent that improves over time.
    Learning,
    /// A multi-agent system coordinator.
    Coordinator,
    /// A meta-agent that supervises other agents.
    Meta,
    /// A custom agent type.
    Custom(String),
}

impl fmt::Display for AgentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Autonomous => write!(f, "autonomous"),
            Self::Reactive => write!(f, "reactive"),
            Self::Deliberative => write!(f, "deliberative"),
            Self::Hybrid => write!(f, "hybrid"),
            Self::Bdi => write!(f, "bdi"),
            Self::Learning => write!(f, "learning"),
            Self::Coordinator => write!(f, "coordinator"),
            Self::Meta => write!(f, "meta"),
            Self::Custom(name) => write!(f, "custom:{name}"),
        }
    }
}

impl FromStr for AgentType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "autonomous" => Ok(Self::Autonomous),
            "reactive" => Ok(Self::Reactive),
            "deliberative" => Ok(Self::Deliberative),
            "hybrid" => Ok(Self::Hybrid),
            "bdi" => Ok(Self::Bdi),
            "learning" => Ok(Self::Learning),
            "coordinator" => Ok(Self::Coordinator),
            "meta" => Ok(Self::Meta),
            other => {
                if let Some(name) = other.strip_prefix("custom:") {
                    Ok(Self::Custom(name.to_string()))
                } else {
                    Err(format!("unknown agent type: {other}"))
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// AgentRole
// ---------------------------------------------------------------------------

/// Built-in roles that agents can assume within the system.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentRole {
    /// Decomposes goals into sub-goals and creates plans.
    Planner,
    /// Gathers information from external sources.
    Researcher,
    /// Applies reasoning strategies to solve problems.
    Reasoner,
    /// Executes actions and capabilities.
    Executor,
    /// Writes and manages code.
    Coder,
    /// Analyzes data and produces insights.
    Analyst,
    /// Reviews outputs for quality and correctness.
    Reviewer,
    /// Manages agent memory systems.
    MemoryManager,
    /// Manages the knowledge graph.
    KnowledgeManager,
    /// Orchestrates workflow execution.
    WorkflowManager,
    /// Manages capability discovery and invocation.
    CapabilityManager,
    /// A general-purpose system agent.
    #[default]
    SystemAgent,
    /// Monitors and supervises other agents.
    Supervisor,
    /// Coordinates multi-agent collaboration.
    Coordinator,
    /// A custom role.
    Custom(String),
}

impl fmt::Display for AgentRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Planner => write!(f, "planner"),
            Self::Researcher => write!(f, "researcher"),
            Self::Reasoner => write!(f, "reasoner"),
            Self::Executor => write!(f, "executor"),
            Self::Coder => write!(f, "coder"),
            Self::Analyst => write!(f, "analyst"),
            Self::Reviewer => write!(f, "reviewer"),
            Self::MemoryManager => write!(f, "memory_manager"),
            Self::KnowledgeManager => write!(f, "knowledge_manager"),
            Self::WorkflowManager => write!(f, "workflow_manager"),
            Self::CapabilityManager => write!(f, "capability_manager"),
            Self::SystemAgent => write!(f, "system"),
            Self::Supervisor => write!(f, "supervisor"),
            Self::Coordinator => write!(f, "coordinator"),
            Self::Custom(name) => write!(f, "custom:{name}"),
        }
    }
}

impl FromStr for AgentRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "planner" => Ok(Self::Planner),
            "researcher" => Ok(Self::Researcher),
            "reasoner" => Ok(Self::Reasoner),
            "executor" => Ok(Self::Executor),
            "coder" => Ok(Self::Coder),
            "analyst" => Ok(Self::Analyst),
            "reviewer" => Ok(Self::Reviewer),
            "memory_manager" => Ok(Self::MemoryManager),
            "knowledge_manager" => Ok(Self::KnowledgeManager),
            "workflow_manager" => Ok(Self::WorkflowManager),
            "capability_manager" => Ok(Self::CapabilityManager),
            "system" => Ok(Self::SystemAgent),
            "supervisor" => Ok(Self::Supervisor),
            "coordinator" => Ok(Self::Coordinator),
            other => {
                if let Some(name) = other.strip_prefix("custom:") {
                    Ok(Self::Custom(name.to_string()))
                } else {
                    Err(format!("unknown agent role: {other}"))
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// AgentStatus
// ---------------------------------------------------------------------------

/// Detailed operational status of an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentStatus {
    /// Agent is newly created, not yet initialized.
    Created,
    /// Agent is initializing resources and state.
    Initializing,
    /// Agent is ready to accept tasks.
    Ready,
    /// Agent is actively executing tasks.
    Running,
    /// Agent is waiting for external input or events.
    Waiting,
    /// Agent has been temporarily suspended by a supervisor.
    Suspended,
    /// Agent has been manually paused.
    Paused,
    /// Agent is in the process of stopping.
    Stopping,
    /// Agent has stopped and released resources.
    Stopped,
    /// Agent is restarting after a failure or manual restart.
    Restarting,
    /// Agent is recovering from a failure.
    Recovering,
    /// Agent has failed and cannot operate.
    Failed,
    /// Agent has been permanently terminated.
    Terminated,
}

impl fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::Initializing => write!(f, "initializing"),
            Self::Ready => write!(f, "ready"),
            Self::Running => write!(f, "running"),
            Self::Waiting => write!(f, "waiting"),
            Self::Suspended => write!(f, "suspended"),
            Self::Paused => write!(f, "paused"),
            Self::Stopping => write!(f, "stopping"),
            Self::Stopped => write!(f, "stopped"),
            Self::Restarting => write!(f, "restarting"),
            Self::Recovering => write!(f, "recovering"),
            Self::Failed => write!(f, "failed"),
            Self::Terminated => write!(f, "terminated"),
        }
    }
}

impl AgentStatus {
    /// Returns `true` if the agent can accept new tasks.
    #[must_use]
    pub fn can_accept_tasks(&self) -> bool {
        matches!(self, Self::Ready | Self::Running | Self::Waiting)
    }

    /// Returns `true` if the agent is in a terminal state.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Terminated | Self::Failed)
    }

    /// Returns `true` if the agent is active (not stopped or terminated).
    #[must_use]
    pub fn is_active(&self) -> bool {
        !matches!(self, Self::Stopped | Self::Terminated | Self::Failed)
    }
}

impl std::str::FromStr for AgentStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "created" => Ok(Self::Created),
            "initializing" => Ok(Self::Initializing),
            "ready" => Ok(Self::Ready),
            "running" => Ok(Self::Running),
            "waiting" => Ok(Self::Waiting),
            "suspended" => Ok(Self::Suspended),
            "paused" => Ok(Self::Paused),
            "stopping" => Ok(Self::Stopping),
            "stopped" => Ok(Self::Stopped),
            "restarting" => Ok(Self::Restarting),
            "recovering" => Ok(Self::Recovering),
            "failed" => Ok(Self::Failed),
            "terminated" => Ok(Self::Terminated),
            other => Err(format!("unknown agent status: {other}")),
        }
    }
}

// ---------------------------------------------------------------------------
// AgentHealth
// ---------------------------------------------------------------------------

/// Health status of an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentHealth {
    /// Agent is operating normally.
    Healthy,
    /// Agent is degraded but still functional.
    Degraded,
    /// Agent is unhealthy and needs attention.
    Unhealthy,
    /// Agent health is unknown (not enough data).
    Unknown,
}

impl fmt::Display for AgentHealth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Healthy => write!(f, "healthy"),
            Self::Degraded => write!(f, "degraded"),
            Self::Unhealthy => write!(f, "unhealthy"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

// ---------------------------------------------------------------------------
// AgentMetrics
// ---------------------------------------------------------------------------

/// Runtime metrics collected from an agent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentMetrics {
    /// Total seconds the agent has been alive.
    pub uptime_secs: u64,
    /// Number of messages sent.
    pub messages_sent: u64,
    /// Number of messages received.
    pub messages_received: u64,
    /// Number of tasks completed successfully.
    pub tasks_completed: u64,
    /// Number of tasks that failed.
    pub tasks_failed: u64,
    /// Number of tasks currently active.
    pub tasks_active: u64,
    /// Memory used in bytes.
    pub memory_used_bytes: u64,
    /// CPU time consumed in milliseconds.
    pub cpu_time_ms: u64,
    /// Average response latency in milliseconds.
    pub avg_response_latency_ms: f64,
    /// Current message queue depth.
    pub queue_depth: usize,
    /// Number of errors encountered.
    pub error_count: u64,
    /// Number of recoveries from failures.
    pub recovery_count: u64,
    /// Throughput in tasks per second.
    pub throughput_tasks_per_sec: f64,
}

// ---------------------------------------------------------------------------
// AgentStatistics
// ---------------------------------------------------------------------------

/// Aggregate statistics across all agents managed by a single manager.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentStatistics {
    /// Total agents ever created.
    pub total_agents_created: u64,
    /// Currently active agents.
    pub active_agents: u64,
    /// Currently failed agents.
    pub failed_agents: u64,
    /// Total tasks assigned across all agents.
    pub total_tasks_assigned: u64,
    /// Total tasks completed across all agents.
    pub total_tasks_completed: u64,
    /// Total tasks failed across all agents.
    pub total_tasks_failed: u64,
    /// Total messages sent across all agents.
    pub total_messages_sent: u64,
    /// Total messages received across all agents.
    pub total_messages_received: u64,
    /// Average agent uptime in seconds.
    pub avg_agent_uptime_secs: f64,
    /// Average task completion time in milliseconds.
    pub avg_task_completion_ms: f64,
    /// Current system-wide resource utilization (0.0–1.0).
    pub system_utilization: f64,
}

// ---------------------------------------------------------------------------
// AgentSnapshot
// ---------------------------------------------------------------------------

/// A point-in-time snapshot of an agent's state for inspection and persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSnapshot {
    /// Agent identifier.
    pub id: AgentId,
    /// Agent name.
    pub name: String,
    /// Agent type.
    pub agent_type: AgentType,
    /// Current role.
    pub role: AgentRole,
    /// Current status.
    pub status: AgentStatus,
    /// Current health.
    pub health: AgentHealth,
    /// Current metrics.
    pub metrics: AgentMetrics,
    /// When the agent was created.
    pub created_at: DateTime<Utc>,
    /// When the agent was last updated.
    pub updated_at: DateTime<Utc>,
    /// When the agent last sent a heartbeat.
    pub last_heartbeat: DateTime<Utc>,
    /// Current error message, if any.
    pub error: Option<String>,
    /// Agent-level metadata.
    pub metadata: HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// AgentVersion
// ---------------------------------------------------------------------------

/// Semantic version for an agent definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentVersion {
    /// Major version (breaking changes).
    pub major: u32,
    /// Minor version (new features).
    pub minor: u32,
    /// Patch version (bug fixes).
    pub patch: u32,
}

impl AgentVersion {
    /// Create a new agent version.
    #[must_use]
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Initial version (0.1.0).
    #[must_use]
    pub fn initial() -> Self {
        Self::new(0, 1, 0)
    }
}

impl fmt::Display for AgentVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Default for AgentVersion {
    fn default() -> Self {
        Self::initial()
    }
}

// ---------------------------------------------------------------------------
// AgentMetadata
// ---------------------------------------------------------------------------

/// Metadata associated with an agent definition.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentMetadata {
    /// Human-readable description of the agent.
    pub description: String,
    /// Tags for categorization and search.
    pub tags: Vec<String>,
    /// Arbitrary key-value metadata.
    pub properties: HashMap<String, String>,
    /// Author of the agent definition.
    pub author: Option<String>,
    /// Version of the agent definition.
    pub version: AgentVersion,
    /// The agent type.
    pub agent_type: AgentType,
    /// The agent role.
    pub role: AgentRole,
}

// ---------------------------------------------------------------------------
// MessagePriority
// ---------------------------------------------------------------------------

/// Priority level for agent messages.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum MessagePriority {
    /// Lowest priority, best-effort delivery.
    Background = 0,
    /// Low priority.
    Low = 1,
    /// Normal priority.
    #[default]
    Normal = 2,
    /// High priority.
    High = 3,
    /// Critical priority, guaranteed delivery.
    Critical = 4,
}

impl fmt::Display for MessagePriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Background => write!(f, "background"),
            Self::Low => write!(f, "low"),
            Self::Normal => write!(f, "normal"),
            Self::High => write!(f, "high"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

// ---------------------------------------------------------------------------
// TaskPriority
// ---------------------------------------------------------------------------

/// Priority level for tasks.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum TaskPriority {
    /// Background tasks, run when idle.
    Background = 0,
    /// Low priority.
    Low = 1,
    /// Normal priority.
    #[default]
    Normal = 2,
    /// High priority.
    High = 3,
    /// Critical, highest priority.
    Critical = 4,
}

impl fmt::Display for TaskPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Background => write!(f, "background"),
            Self::Low => write!(f, "low"),
            Self::Normal => write!(f, "normal"),
            Self::High => write!(f, "high"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

impl FromStr for TaskPriority {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "background" => Ok(Self::Background),
            "low" => Ok(Self::Low),
            "normal" => Ok(Self::Normal),
            "high" => Ok(Self::High),
            "critical" => Ok(Self::Critical),
            other => Err(format!("unknown task priority: {other}")),
        }
    }
}

// ---------------------------------------------------------------------------
// ConversationId
// ---------------------------------------------------------------------------

/// Identifier for a conversation between agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ConversationId(pub Uuid);

impl ConversationId {
    /// Create a new conversation identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ConversationId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ConversationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// Conversation
// ---------------------------------------------------------------------------

/// A conversation between two or more agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    /// Unique conversation identifier.
    pub id: ConversationId,
    /// Participating agent IDs.
    pub participants: Vec<AgentId>,
    /// Subject or topic of the conversation.
    pub subject: String,
    /// When the conversation started.
    pub started_at: DateTime<Utc>,
    /// When the conversation last had activity.
    pub last_activity: DateTime<Utc>,
    /// Whether the conversation is still active.
    pub is_active: bool,
    /// Message count in this conversation.
    pub message_count: u64,
}

impl Conversation {
    /// Create a new conversation.
    #[must_use]
    pub fn new(participants: Vec<AgentId>, subject: String) -> Self {
        let now = Utc::now();
        Self {
            id: ConversationId::new(),
            participants,
            subject,
            started_at: now,
            last_activity: now,
            is_active: true,
            message_count: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// AgentConfiguration
// ---------------------------------------------------------------------------

/// Configuration for creating an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfiguration {
    /// Human-readable name.
    pub name: String,
    /// Agent type.
    pub agent_type: AgentType,
    /// Agent role.
    pub role: AgentRole,
    /// Metadata.
    pub metadata: AgentMetadata,
    /// Maximum memory in bytes.
    pub max_memory_bytes: u64,
    /// Heartbeat interval in seconds.
    pub heartbeat_interval_secs: u64,
    /// Task timeout in milliseconds.
    pub task_timeout_ms: u64,
    /// Maximum concurrent tasks.
    pub max_concurrent_tasks: usize,
    /// Maximum retry attempts for failed tasks.
    pub max_retries: u32,
    /// Base delay between retries in milliseconds.
    pub retry_base_delay_ms: u64,
    /// Backoff multiplier for retries.
    pub retry_backoff_multiplier: f64,
    /// Maximum retry delay in milliseconds.
    pub retry_max_delay_ms: u64,
    /// Whether the agent auto-recovers from failures.
    pub auto_recover: bool,
    /// Maximum number of recovery attempts.
    pub max_recovery_attempts: u32,
    /// Custom properties.
    pub properties: HashMap<String, String>,
}

impl Default for AgentConfiguration {
    fn default() -> Self {
        Self {
            name: String::new(),
            agent_type: AgentType::Autonomous,
            role: AgentRole::SystemAgent,
            metadata: AgentMetadata::default(),
            max_memory_bytes: 256 * 1024 * 1024,
            heartbeat_interval_secs: 10,
            task_timeout_ms: 30_000,
            max_concurrent_tasks: 4,
            max_retries: 3,
            retry_base_delay_ms: 1_000,
            retry_backoff_multiplier: 2.0,
            retry_max_delay_ms: 30_000,
            auto_recover: true,
            max_recovery_attempts: 5,
            properties: HashMap::new(),
        }
    }
}

impl AgentConfiguration {
    /// Create a new minimal configuration with the given name.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }

    /// Set the agent type.
    #[must_use]
    pub fn with_type(mut self, agent_type: AgentType) -> Self {
        self.agent_type = agent_type;
        self
    }

    /// Set the agent role.
    #[must_use]
    pub fn with_role(mut self, role: AgentRole) -> Self {
        self.role = role;
        self
    }

    /// Set the description in metadata.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.metadata.description = description.into();
        self
    }

    /// Set the maximum concurrent tasks.
    #[must_use]
    pub fn with_max_concurrent_tasks(mut self, max: usize) -> Self {
        self.max_concurrent_tasks = max;
        self
    }

    /// Set the task timeout in milliseconds.
    #[must_use]
    pub fn with_task_timeout_ms(mut self, timeout: u64) -> Self {
        self.task_timeout_ms = timeout;
        self
    }

    /// Set the maximum retry attempts.
    #[must_use]
    pub fn with_max_retries(mut self, max: u32) -> Self {
        self.max_retries = max;
        self
    }

    /// Set the heartbeat interval in seconds.
    #[must_use]
    pub fn with_heartbeat_interval(mut self, secs: u64) -> Self {
        self.heartbeat_interval_secs = secs;
        self
    }

    /// Set the maximum memory in bytes.
    #[must_use]
    pub fn with_max_memory(mut self, bytes: u64) -> Self {
        self.max_memory_bytes = bytes;
        self
    }

    /// Set auto-recovery behavior.
    #[must_use]
    pub fn with_auto_recover(mut self, auto_recover: bool) -> Self {
        self.auto_recover = auto_recover;
        self
    }

    /// Add a custom property.
    #[must_use]
    pub fn with_property(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.properties.insert(key.into(), value.into());
        self
    }

    /// Add a tag to the metadata.
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.metadata.tags.push(tag.into());
        self
    }
}

// ---------------------------------------------------------------------------
// Valid state transitions
// ---------------------------------------------------------------------------

/// Returns whether a transition from `from` to `to` is valid.
#[must_use]
pub fn is_valid_transition(from: AgentStatus, to: AgentStatus) -> bool {
    matches!(
        (from, to),
        // From Created
        (AgentStatus::Created, AgentStatus::Initializing)
            // From Initializing
            | (AgentStatus::Initializing, AgentStatus::Ready)
            | (AgentStatus::Initializing, AgentStatus::Failed)
            // From Ready
            | (AgentStatus::Ready, AgentStatus::Running)
            | (AgentStatus::Ready, AgentStatus::Paused)
            | (AgentStatus::Ready, AgentStatus::Suspended)
            | (AgentStatus::Ready, AgentStatus::Stopping)
            | (AgentStatus::Ready, AgentStatus::Restarting)
            // From Running
            | (AgentStatus::Running, AgentStatus::Waiting)
            | (AgentStatus::Running, AgentStatus::Paused)
            | (AgentStatus::Running, AgentStatus::Suspended)
            | (AgentStatus::Running, AgentStatus::Stopping)
            | (AgentStatus::Running, AgentStatus::Failed)
            | (AgentStatus::Running, AgentStatus::Restarting)
            // From Waiting
            | (AgentStatus::Waiting, AgentStatus::Running)
            | (AgentStatus::Waiting, AgentStatus::Paused)
            | (AgentStatus::Waiting, AgentStatus::Stopping)
            | (AgentStatus::Waiting, AgentStatus::Failed)
            // From Suspended
            | (AgentStatus::Suspended, AgentStatus::Running)
            | (AgentStatus::Suspended, AgentStatus::Stopping)
            | (AgentStatus::Suspended, AgentStatus::Failed)
            // From Paused
            | (AgentStatus::Paused, AgentStatus::Running)
            | (AgentStatus::Paused, AgentStatus::Stopping)
            | (AgentStatus::Paused, AgentStatus::Failed)
            // From Stopping
            | (AgentStatus::Stopping, AgentStatus::Stopped)
            | (AgentStatus::Stopping, AgentStatus::Failed)
            // From Stopped
            | (AgentStatus::Stopped, AgentStatus::Restarting)
            | (AgentStatus::Stopped, AgentStatus::Terminated)
            // From Restarting
            | (AgentStatus::Restarting, AgentStatus::Initializing)
            | (AgentStatus::Restarting, AgentStatus::Failed)
            // From Recovering
            | (AgentStatus::Recovering, AgentStatus::Ready)
            | (AgentStatus::Recovering, AgentStatus::Running)
            | (AgentStatus::Recovering, AgentStatus::Failed)
            // From Failed
            | (AgentStatus::Failed, AgentStatus::Recovering)
            | (AgentStatus::Failed, AgentStatus::Stopped)
            | (AgentStatus::Failed, AgentStatus::Terminated)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_type_display_roundtrip() {
        let types = vec![
            AgentType::Autonomous,
            AgentType::Reactive,
            AgentType::Deliberative,
            AgentType::Hybrid,
            AgentType::Bdi,
            AgentType::Learning,
            AgentType::Coordinator,
            AgentType::Meta,
            AgentType::Custom("test".to_string()),
        ];
        for at in types {
            let s = at.to_string();
            let parsed: AgentType = s.parse().expect("failed to parse agent type");
            assert_eq!(at, parsed);
        }
    }

    #[test]
    fn test_agent_role_display_roundtrip() {
        let roles = vec![
            AgentRole::Planner,
            AgentRole::Researcher,
            AgentRole::Reasoner,
            AgentRole::Executor,
            AgentRole::Coder,
            AgentRole::Analyst,
            AgentRole::Reviewer,
            AgentRole::MemoryManager,
            AgentRole::KnowledgeManager,
            AgentRole::WorkflowManager,
            AgentRole::CapabilityManager,
            AgentRole::SystemAgent,
            AgentRole::Supervisor,
            AgentRole::Coordinator,
            AgentRole::Custom("test".to_string()),
        ];
        for role in roles {
            let s = role.to_string();
            let parsed: AgentRole = s.parse().expect("failed to parse agent role");
            assert_eq!(role, parsed);
        }
    }

    #[test]
    fn test_agent_status_properties() {
        assert!(AgentStatus::Ready.can_accept_tasks());
        assert!(AgentStatus::Running.can_accept_tasks());
        assert!(AgentStatus::Waiting.can_accept_tasks());
        assert!(!AgentStatus::Failed.can_accept_tasks());
        assert!(!AgentStatus::Terminated.can_accept_tasks());

        assert!(AgentStatus::Terminated.is_terminal());
        assert!(AgentStatus::Failed.is_terminal());
        assert!(!AgentStatus::Running.is_terminal());

        assert!(AgentStatus::Running.is_active());
        assert!(!AgentStatus::Stopped.is_active());
        assert!(!AgentStatus::Terminated.is_active());
    }

    #[test]
    fn test_valid_transitions() {
        assert!(is_valid_transition(
            AgentStatus::Created,
            AgentStatus::Initializing
        ));
        assert!(is_valid_transition(
            AgentStatus::Initializing,
            AgentStatus::Ready
        ));
        assert!(is_valid_transition(
            AgentStatus::Ready,
            AgentStatus::Running
        ));
        assert!(is_valid_transition(
            AgentStatus::Running,
            AgentStatus::Failed
        ));
        assert!(is_valid_transition(
            AgentStatus::Failed,
            AgentStatus::Recovering
        ));
        assert!(is_valid_transition(
            AgentStatus::Recovering,
            AgentStatus::Ready
        ));
        assert!(!is_valid_transition(
            AgentStatus::Created,
            AgentStatus::Running
        ));
        assert!(!is_valid_transition(
            AgentStatus::Terminated,
            AgentStatus::Running
        ));
    }

    #[test]
    fn test_agent_version() {
        let v = AgentVersion::new(1, 2, 3);
        assert_eq!(v.to_string(), "1.2.3");
        let v = AgentVersion::default();
        assert_eq!(v.to_string(), "0.1.0");
    }

    #[test]
    fn test_agent_configuration_builder() {
        let config = AgentConfiguration::new("test-agent")
            .with_type(AgentType::Deliberative)
            .with_role(AgentRole::Planner)
            .with_description("A test agent")
            .with_max_concurrent_tasks(8)
            .with_task_timeout_ms(60_000)
            .with_max_retries(5)
            .with_heartbeat_interval(30)
            .with_max_memory(1024 * 1024 * 512)
            .with_auto_recover(true)
            .with_property("key", "value")
            .with_tag("production");

        assert_eq!(config.name, "test-agent");
        assert_eq!(config.agent_type, AgentType::Deliberative);
        assert_eq!(config.role, AgentRole::Planner);
        assert_eq!(config.max_concurrent_tasks, 8);
        assert_eq!(config.task_timeout_ms, 60_000);
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.heartbeat_interval_secs, 30);
        assert_eq!(config.max_memory_bytes, 1024 * 1024 * 512);
        assert!(config.auto_recover);
        assert_eq!(config.properties.get("key").unwrap(), "value");
        assert_eq!(config.metadata.tags[0], "production");
    }

    #[test]
    fn test_conversation() {
        let id1 = AgentId::new();
        let id2 = AgentId::new();
        let conv = Conversation::new(vec![id1, id2], "test subject".to_string());
        assert!(conv.is_active);
        assert_eq!(conv.participants.len(), 2);
        assert_eq!(conv.subject, "test subject");
    }

    #[test]
    fn test_message_priority_ordering() {
        assert!(MessagePriority::Background < MessagePriority::Low);
        assert!(MessagePriority::Low < MessagePriority::Normal);
        assert!(MessagePriority::Normal < MessagePriority::High);
        assert!(MessagePriority::High < MessagePriority::Critical);
    }

    #[test]
    fn test_task_priority_ordering() {
        assert!(TaskPriority::Background < TaskPriority::Low);
        assert!(TaskPriority::Low < TaskPriority::Normal);
        assert!(TaskPriority::Normal < TaskPriority::High);
        assert!(TaskPriority::High < TaskPriority::Critical);
    }
}
