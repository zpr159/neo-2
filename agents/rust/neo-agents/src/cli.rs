use crate::error::AgentResult;
use crate::manager::AgentManager;
use crate::types::{AgentId, AgentStatus};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// CliCommand
// ---------------------------------------------------------------------------

/// CLI commands for the agent framework.
///
/// Provides the command-line interface operations that map to `neo agent` and
/// `neo task` subcommands.
pub struct AgentCli {
    /// Reference to the agent manager.
    manager: Arc<AgentManager>,
}

impl AgentCli {
    /// Create a new CLI handler.
    #[must_use]
    pub fn new(manager: Arc<AgentManager>) -> Self {
        Self { manager }
    }

    /// `neo agent create <name> [--role <role>] [--type <type>]`
    pub async fn create_agent(
        &self,
        name: &str,
        role: Option<&str>,
        agent_type: Option<&str>,
    ) -> AgentResult<String> {
        let mut config = crate::types::AgentConfiguration::new(name);

        if let Some(role_str) = role {
            let role: crate::types::AgentRole = role_str
                .parse()
                .map_err(|e: String| crate::error::AgentError::InvalidConfiguration(e))?;
            config = config.with_role(role);
        }

        if let Some(type_str) = agent_type {
            let agent_type: crate::types::AgentType = type_str
                .parse()
                .map_err(|e: String| crate::error::AgentError::InvalidConfiguration(e))?;
            config = config.with_type(agent_type);
        }

        let id = self.manager.create_agent(config).await?;
        Ok(format!("Created agent {id} ({name})"))
    }

    /// `neo agent start <id>`
    pub async fn start_agent(&self, id: &str) -> AgentResult<String> {
        let agent_id: AgentId = id.parse().map_err(|_| {
            crate::error::AgentError::InvalidConfiguration(format!("invalid agent ID: {id}"))
        })?;
        self.manager.start_agent(agent_id).await?;
        Ok(format!("Agent {agent_id} started"))
    }

    /// `neo agent stop <id>`
    pub async fn stop_agent(&self, id: &str) -> AgentResult<String> {
        let agent_id: AgentId = id.parse().map_err(|_| {
            crate::error::AgentError::InvalidConfiguration(format!("invalid agent ID: {id}"))
        })?;
        self.manager.stop_agent(agent_id).await?;
        Ok(format!("Agent {agent_id} stopped"))
    }

    /// `neo agent restart <id>`
    pub async fn restart_agent(&self, id: &str) -> AgentResult<String> {
        let agent_id: AgentId = id.parse().map_err(|_| {
            crate::error::AgentError::InvalidConfiguration(format!("invalid agent ID: {id}"))
        })?;
        self.manager.restart_agent(agent_id).await?;
        Ok(format!("Agent {agent_id} restarted"))
    }

    /// `neo agent pause <id>`
    pub async fn pause_agent(&self, id: &str) -> AgentResult<String> {
        let agent_id: AgentId = id.parse().map_err(|_| {
            crate::error::AgentError::InvalidConfiguration(format!("invalid agent ID: {id}"))
        })?;
        self.manager.pause_agent(agent_id).await?;
        Ok(format!("Agent {agent_id} paused"))
    }

    /// `neo agent resume <id>`
    pub async fn resume_agent(&self, id: &str) -> AgentResult<String> {
        let agent_id: AgentId = id.parse().map_err(|_| {
            crate::error::AgentError::InvalidConfiguration(format!("invalid agent ID: {id}"))
        })?;
        self.manager.resume_agent(agent_id).await?;
        Ok(format!("Agent {agent_id} resumed"))
    }

    /// `neo agent list [--status <status>]`
    #[must_use]
    pub fn list_agents(&self, status_filter: Option<&str>) -> String {
        let filter = status_filter.and_then(|s| s.parse::<AgentStatus>().ok());
        let ids = self.manager.list_agents(filter);

        if ids.is_empty() {
            return "No agents found.".to_string();
        }

        let mut output = format!("{:<40} {:<20} {:<15}\n", "ID", "NAME", "STATUS");
        output.push_str(&"-".repeat(75));
        output.push('\n');

        for id in &ids {
            if let Some(snap) =
                futures::executor::block_on(async { self.manager.inspect_agent(*id).await.ok() })
            {
                output.push_str(&format!(
                    "{:<40} {:<20} {:<15}\n",
                    snap.id.to_string(),
                    snap.name,
                    snap.status.to_string()
                ));
            }
        }

        output
    }

    /// `neo agent inspect <id>`
    pub async fn inspect_agent(&self, id: &str) -> AgentResult<String> {
        let agent_id: AgentId = id.parse().map_err(|_| {
            crate::error::AgentError::InvalidConfiguration(format!("invalid agent ID: {id}"))
        })?;
        let snapshot = self.manager.inspect_agent(agent_id).await?;

        let output = serde_json::to_string_pretty(&snapshot)
            .map_err(crate::error::AgentError::Serialization)?;
        Ok(output)
    }

    /// `neo agent logs <id>`
    pub async fn agent_logs(&self, id: &str) -> AgentResult<String> {
        let agent_id: AgentId = id.parse().map_err(|_| {
            crate::error::AgentError::InvalidConfiguration(format!("invalid agent ID: {id}"))
        })?;
        let snapshot = self.manager.inspect_agent(agent_id).await?;

        let mut output = String::new();
        output.push_str(&format!(
            "Logs for agent: {} ({})\n",
            snapshot.name, snapshot.id
        ));
        output.push_str(&format!("Status: {}\n", snapshot.status));
        output.push_str(&format!("Health: {}\n", snapshot.health));
        if let Some(ref error) = snapshot.error {
            output.push_str(&format!("Error: {error}\n"));
        }
        output.push_str(&format!("Uptime: {}s\n", snapshot.metrics.uptime_secs));
        output.push_str(&format!(
            "Tasks: {} completed, {} failed, {} active\n",
            snapshot.metrics.tasks_completed,
            snapshot.metrics.tasks_failed,
            snapshot.metrics.tasks_active
        ));
        output.push_str(&format!(
            "Messages: {} sent, {} received\n",
            snapshot.metrics.messages_sent, snapshot.metrics.messages_received
        ));

        Ok(output)
    }

    /// `neo agent metrics <id>`
    pub async fn agent_metrics(&self, id: &str) -> AgentResult<String> {
        let agent_id: AgentId = id.parse().map_err(|_| {
            crate::error::AgentError::InvalidConfiguration(format!("invalid agent ID: {id}"))
        })?;
        let snapshot = self.manager.inspect_agent(agent_id).await?;
        let output = serde_json::to_string_pretty(&snapshot.metrics)
            .map_err(crate::error::AgentError::Serialization)?;
        Ok(output)
    }

    /// `neo task create <name> [--description <desc>] [--priority <priority>]`
    pub async fn create_task(
        &self,
        name: &str,
        description: Option<&str>,
        priority: Option<&str>,
    ) -> AgentResult<String> {
        let desc = description.unwrap_or("");
        let mut task = crate::task::Task::new(name, desc, serde_json::json!(null));

        if let Some(priority_str) = priority {
            let priority: crate::types::TaskPriority = priority_str
                .parse()
                .map_err(|e: String| crate::error::AgentError::InvalidConfiguration(e))?;
            task = task.with_priority(priority);
        }

        let task_id = crate::task::TaskId::new();
        task.queue()
            .map_err(|e| crate::error::AgentError::Internal(e.to_string()))?;
        Ok(format!("Created task {task_id} ({name})"))
    }

    /// `neo task list`
    pub fn list_tasks(&self) -> String {
        "Task listing requires scheduler integration.".to_string()
    }

    /// `neo task inspect <id>`
    pub async fn inspect_task(&self, id: &str) -> AgentResult<String> {
        let _task_id: crate::task::TaskId = id.parse().map_err(|_| {
            crate::error::AgentError::InvalidConfiguration(format!("invalid task ID: {id}"))
        })?;
        Ok(format!(
            "Task inspection for {id} requires scheduler integration."
        ))
    }
}
