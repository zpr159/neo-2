use crate::agent::Agent;
use crate::manager::AgentManager;
use crate::task::Task;
use crate::types::{AgentConfiguration, AgentRole, AgentType, TaskPriority};

// ---------------------------------------------------------------------------
// AgentBuilder
// ---------------------------------------------------------------------------

/// Builder for constructing an `Agent` with a fluent API.
///
/// # Example
///
/// ```no_run
/// use neo_agents::Agent;
/// use neo_agents::types::{AgentRole, AgentType};
///
/// let agent = Agent::builder()
///     .name("Research Agent")
///     .role(AgentRole::Researcher)
///     .agent_type(AgentType::Deliberative)
///     .description("Gathers information from external sources")
///     .max_concurrent_tasks(8)
///     .task_timeout_ms(60_000)
///     .max_retries(5)
///     .heartbeat_interval(30)
///     .auto_recover(true)
///     .build();
/// ```
pub struct AgentBuilder {
    config: AgentConfiguration,
}

impl AgentBuilder {
    /// Create a new builder with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: AgentConfiguration::default(),
        }
    }

    /// Set the agent name.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.config.name = name.into();
        self
    }

    /// Set the agent role.
    #[must_use]
    pub fn role(mut self, role: AgentRole) -> Self {
        self.config.role = role;
        self
    }

    /// Set the agent type.
    #[must_use]
    pub fn agent_type(mut self, agent_type: AgentType) -> Self {
        self.config.agent_type = agent_type;
        self
    }

    /// Set the description.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.config.metadata.description = description.into();
        self
    }

    /// Set the maximum concurrent tasks.
    #[must_use]
    pub fn max_concurrent_tasks(mut self, max: usize) -> Self {
        self.config.max_concurrent_tasks = max;
        self
    }

    /// Set the task timeout in milliseconds.
    #[must_use]
    pub fn task_timeout_ms(mut self, timeout: u64) -> Self {
        self.config.task_timeout_ms = timeout;
        self
    }

    /// Set the maximum retry attempts.
    #[must_use]
    pub fn max_retries(mut self, max: u32) -> Self {
        self.config.max_retries = max;
        self
    }

    /// Set the heartbeat interval in seconds.
    #[must_use]
    pub fn heartbeat_interval(mut self, secs: u64) -> Self {
        self.config.heartbeat_interval_secs = secs;
        self
    }

    /// Set the maximum memory in bytes.
    #[must_use]
    pub fn max_memory(mut self, bytes: u64) -> Self {
        self.config.max_memory_bytes = bytes;
        self
    }

    /// Enable or disable auto-recovery.
    #[must_use]
    pub fn auto_recover(mut self, auto: bool) -> Self {
        self.config.auto_recover = auto;
        self
    }

    /// Add a tag.
    #[must_use]
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.config.metadata.tags.push(tag.into());
        self
    }

    /// Add a custom property.
    #[must_use]
    pub fn property(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.config.properties.insert(key.into(), value.into());
        self
    }

    /// Build the agent.
    #[must_use]
    pub fn build(self) -> Agent {
        Agent::new(self.config)
    }
}

impl Default for AgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// TaskBuilder
// ---------------------------------------------------------------------------

/// Builder for constructing a `Task` with a fluent API.
///
/// # Example
///
/// ```no_run
/// use neo_agents::task::Task;
/// use neo_agents::types::TaskPriority;
///
/// let task = Task::builder()
///     .name("Analyze Data")
///     .description("Perform deep analysis on the provided dataset")
///     .priority(TaskPriority::High)
///     .timeout_ms(30_000)
///     .max_retries(3)
///     .tag("analysis")
///     .tag("production")
///     .build();
/// ```
pub struct TaskBuilder {
    name: String,
    description: String,
    input: serde_json::Value,
    priority: TaskPriority,
    timeout_ms: Option<u64>,
    max_retries: u32,
    tags: Vec<String>,
    metadata: std::collections::HashMap<String, String>,
}

impl TaskBuilder {
    /// Create a new task builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            input: serde_json::json!(null),
            priority: TaskPriority::default(),
            timeout_ms: None,
            max_retries: 3,
            tags: Vec::new(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Set the task name.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set the task description.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set the input payload.
    #[must_use]
    pub fn input(mut self, input: serde_json::Value) -> Self {
        self.input = input;
        self
    }

    /// Set the priority.
    #[must_use]
    pub fn priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set the timeout in milliseconds.
    #[must_use]
    pub fn timeout_ms(mut self, timeout: u64) -> Self {
        self.timeout_ms = Some(timeout);
        self
    }

    /// Set the maximum retries.
    #[must_use]
    pub fn max_retries(mut self, max: u32) -> Self {
        self.max_retries = max;
        self
    }

    /// Add a tag.
    #[must_use]
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add custom metadata.
    #[must_use]
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Build the task.
    #[must_use]
    pub fn build(self) -> Task {
        let mut task = Task::new(self.name, self.description, self.input)
            .with_priority(self.priority)
            .with_max_retries(self.max_retries);

        for tag in self.tags {
            task = task.with_tag(tag);
        }

        for (key, value) in self.metadata {
            task = task.with_metadata(key, value);
        }

        if let Some(timeout) = self.timeout_ms {
            task = task.with_timeout_ms(timeout);
        }

        task
    }
}

impl Default for TaskBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// AgentManagerBuilder (re-export convenience)
// ---------------------------------------------------------------------------

/// Builder for constructing an `AgentManager` with a fluent API.
///
/// # Example
///
/// ```no_run
/// use neo_agents::AgentManager;
///
/// let manager = AgentManager::builder()
///     .with_max_agents(256)
///     .build();
/// ```
pub struct NeoAgentManagerBuilder {
    max_agents: usize,
}

impl NeoAgentManagerBuilder {
    /// Create a new builder.
    #[must_use]
    pub fn new() -> Self {
        Self { max_agents: 64 }
    }

    /// Set the maximum number of agents.
    #[must_use]
    pub fn with_max_agents(mut self, max: usize) -> Self {
        self.max_agents = max;
        self
    }

    /// Build the manager.
    #[must_use]
    pub fn build(self) -> AgentManager {
        AgentManager::new(self.max_agents)
    }
}

impl Default for NeoAgentManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_builder() {
        let agent = Agent::builder()
            .name("builder-test")
            .role(AgentRole::Planner)
            .agent_type(AgentType::Deliberative)
            .description("A test agent built with the SDK")
            .max_concurrent_tasks(8)
            .task_timeout_ms(60_000)
            .max_retries(5)
            .heartbeat_interval(30)
            .max_memory(1024 * 1024 * 512)
            .auto_recover(true)
            .tag("test")
            .property("env", "testing")
            .build();

        assert_eq!(agent.name(), "builder-test");
        assert_eq!(*agent.role(), AgentRole::Planner);
    }

    #[test]
    fn test_task_builder() {
        let task = Task::builder()
            .name("build-test")
            .description("A task built with the SDK")
            .input(serde_json::json!({"data": [1, 2, 3]}))
            .priority(TaskPriority::High)
            .timeout_ms(30_000)
            .max_retries(3)
            .tag("important")
            .metadata("category", "analysis")
            .build();

        assert_eq!(task.name, "build-test");
        assert_eq!(task.priority, TaskPriority::High);
        assert!(task.tags.contains(&"important".to_string()));
    }

    #[test]
    fn test_manager_builder() {
        let mgr = AgentManager::builder().with_max_agents(200).build();
        assert_eq!(mgr.agent_count(), 0);
    }
}
