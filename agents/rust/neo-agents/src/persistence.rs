use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::agent::Agent;
use crate::error::{AgentError, AgentResult};
use crate::task::Task;
use crate::types::{AgentId, AgentSnapshot};

// ---------------------------------------------------------------------------
// PersistedAgent
// ---------------------------------------------------------------------------

/// Serializable representation of an agent for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedAgent {
    /// Agent snapshot.
    pub snapshot: AgentSnapshot,
    /// Agent configuration.
    pub config: crate::types::AgentConfiguration,
}

// ---------------------------------------------------------------------------
// PersistedTask
// ---------------------------------------------------------------------------

/// Serializable representation of a task for persistence.
pub type PersistedTask = Task;

// ---------------------------------------------------------------------------
// PersistedConversation
// ---------------------------------------------------------------------------

/// Serializable representation of a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedConversation {
    /// Conversation data.
    pub conversation: crate::types::Conversation,
    /// Message history.
    pub messages: Vec<crate::communication::AgentMessage>,
}

// ---------------------------------------------------------------------------
// PersistenceConfig
// ---------------------------------------------------------------------------

/// Configuration for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceConfig {
    /// Directory for storing agent data.
    pub data_dir: PathBuf,
    /// Whether persistence is enabled.
    pub enabled: bool,
    /// Auto-save interval in seconds.
    pub auto_save_interval_secs: u64,
    /// Maximum file size per agent in bytes.
    pub max_file_size_bytes: u64,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("./data/agents"),
            enabled: true,
            auto_save_interval_secs: 60,
            max_file_size_bytes: 10 * 1024 * 1024,
        }
    }
}

// ---------------------------------------------------------------------------
// AgentPersistence
// ---------------------------------------------------------------------------

/// Handles persistence of agent state to disk.
pub struct AgentPersistence {
    /// Persistence configuration.
    config: PersistenceConfig,
}

impl AgentPersistence {
    /// Create a new persistence handler.
    #[must_use]
    pub fn new(config: PersistenceConfig) -> Self {
        Self { config }
    }

    /// Ensure the data directory exists.
    fn ensure_data_dir(&self) -> AgentResult<()> {
        if self.config.enabled && !self.config.data_dir.exists() {
            std::fs::create_dir_all(&self.config.data_dir)
                .map_err(|e| AgentError::Internal(format!("failed to create data dir: {e}")))?;
        }
        Ok(())
    }

    /// Get the file path for an agent.
    fn agent_path(&self, agent_id: &AgentId) -> PathBuf {
        self.config
            .data_dir
            .join(format!("agent_{}.json", agent_id))
    }

    /// Get the file path for a task.
    fn task_path(&self, task_id: &crate::task::TaskId) -> PathBuf {
        self.config.data_dir.join(format!("task_{}.json", task_id))
    }

    /// Save an agent's state to disk.
    pub fn save_agent(
        &self,
        agent: &Agent,
        config: &crate::types::AgentConfiguration,
    ) -> AgentResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        self.ensure_data_dir()?;

        let persisted = PersistedAgent {
            snapshot: agent.snapshot(),
            config: config.clone(),
        };

        let json = serde_json::to_string_pretty(&persisted).map_err(AgentError::Serialization)?;

        let path = self.agent_path(&agent.id());
        std::fs::write(&path, json)
            .map_err(|e| AgentError::Internal(format!("failed to write agent file: {e}")))?;

        tracing::debug!("Saved agent {} to {}", agent.id(), path.display());
        Ok(())
    }

    /// Load an agent's state from disk.
    pub fn load_agent(&self, agent_id: &AgentId) -> AgentResult<PersistedAgent> {
        let path = self.agent_path(agent_id);
        if !path.exists() {
            return Err(AgentError::NotFound(format!(
                "agent file not found: {}",
                path.display()
            )));
        }

        let json = std::fs::read_to_string(&path)
            .map_err(|e| AgentError::Internal(format!("failed to read agent file: {e}")))?;

        let persisted: PersistedAgent =
            serde_json::from_str(&json).map_err(AgentError::Serialization)?;

        Ok(persisted)
    }

    /// Delete an agent's persisted state.
    pub fn delete_agent(&self, agent_id: &AgentId) -> AgentResult<()> {
        let path = self.agent_path(agent_id);
        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| AgentError::Internal(format!("failed to delete agent file: {e}")))?;
        }
        Ok(())
    }

    /// Save a task to disk.
    pub fn save_task(&self, task: &Task) -> AgentResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        self.ensure_data_dir()?;

        let json = serde_json::to_string_pretty(task).map_err(AgentError::Serialization)?;
        let path = self.task_path(&task.id);

        std::fs::write(&path, json)
            .map_err(|e| AgentError::Internal(format!("failed to write task file: {e}")))?;

        Ok(())
    }

    /// Load a task from disk.
    pub fn load_task(&self, task_id: &crate::task::TaskId) -> AgentResult<Task> {
        let path = self.task_path(task_id);
        if !path.exists() {
            return Err(AgentError::NotFound(format!(
                "task file not found: {}",
                path.display()
            )));
        }

        let json = std::fs::read_to_string(&path)
            .map_err(|e| AgentError::Internal(format!("failed to read task file: {e}")))?;

        let task: Task = serde_json::from_str(&json).map_err(AgentError::Serialization)?;
        Ok(task)
    }

    /// Delete a task's persisted state.
    pub fn delete_task(&self, task_id: &crate::task::TaskId) -> AgentResult<()> {
        let path = self.task_path(task_id);
        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| AgentError::Internal(format!("failed to delete task file: {e}")))?;
        }
        Ok(())
    }

    /// List all persisted agent IDs.
    pub fn list_agents(&self) -> AgentResult<Vec<AgentId>> {
        if !self.config.enabled || !self.config.data_dir.exists() {
            return Ok(Vec::new());
        }

        let mut agents = Vec::new();
        let entries = std::fs::read_dir(&self.config.data_dir)
            .map_err(|e| AgentError::Internal(format!("failed to read data dir: {e}")))?;

        for entry in entries {
            let entry = entry
                .map_err(|e| AgentError::Internal(format!("failed to read dir entry: {e}")))?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if let Some(id_str) = name_str
                .strip_prefix("agent_")
                .and_then(|s| s.strip_suffix(".json"))
            {
                if let Ok(uuid) = id_str.parse::<uuid::Uuid>() {
                    agents.push(AgentId(uuid));
                }
            }
        }

        Ok(agents)
    }

    /// Save conversation history.
    pub fn save_conversation(
        &self,
        conversation: &crate::types::Conversation,
        messages: &[crate::communication::AgentMessage],
    ) -> AgentResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        self.ensure_data_dir()?;

        let persisted = PersistedConversation {
            conversation: conversation.clone(),
            messages: messages.to_vec(),
        };

        let json = serde_json::to_string_pretty(&persisted).map_err(AgentError::Serialization)?;
        let path = self
            .config
            .data_dir
            .join(format!("conversation_{}.json", conversation.id));

        std::fs::write(&path, json)
            .map_err(|e| AgentError::Internal(format!("failed to write conversation file: {e}")))?;

        Ok(())
    }

    /// Load a snapshot of all persisted state.
    pub fn load_all_agents(&self) -> AgentResult<Vec<PersistedAgent>> {
        let ids = self.list_agents()?;
        let mut agents = Vec::new();
        for id in &ids {
            if let Ok(agent) = self.load_agent(id) {
                agents.push(agent);
            }
        }
        Ok(agents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AgentConfiguration;

    fn test_persistence_config() -> PersistenceConfig {
        PersistenceConfig {
            data_dir: PathBuf::from("/tmp/neo-agents-test"),
            enabled: true,
            auto_save_interval_secs: 60,
            max_file_size_bytes: 10 * 1024 * 1024,
        }
    }

    #[test]
    fn test_save_and_load_agent() {
        let persistence = AgentPersistence::new(test_persistence_config());
        let config = AgentConfiguration::new("test-persist");
        let mut agent = Agent::new(config.clone());
        agent.initialize().unwrap();

        persistence.save_agent(&agent, &config).unwrap();
        let loaded = persistence.load_agent(&agent.id()).unwrap();

        assert_eq!(loaded.snapshot.name, "test-persist");
        assert_eq!(loaded.snapshot.status, crate::types::AgentStatus::Ready);

        // Cleanup
        persistence.delete_agent(&agent.id()).unwrap();
    }

    #[test]
    fn test_save_and_load_task() {
        let persistence = AgentPersistence::new(test_persistence_config());
        let mut task = crate::task::Task::new("test-task", "desc", serde_json::json!(null));
        task.queue().unwrap();

        persistence.save_task(&task).unwrap();
        let loaded = persistence.load_task(&task.id).unwrap();
        assert_eq!(loaded.name, "test-task");

        persistence.delete_task(&task.id).unwrap();
    }

    #[test]
    fn test_list_agents() {
        let persistence = AgentPersistence::new(test_persistence_config());
        let config = AgentConfiguration::new("list-test");
        let agent = Agent::new(config.clone());

        persistence.save_agent(&agent, &config).unwrap();
        let agents = persistence.list_agents().unwrap();
        assert!(agents.contains(&agent.id()));

        persistence.delete_agent(&agent.id()).unwrap();
    }

    #[test]
    fn test_disabled_persistence() {
        let config = PersistenceConfig {
            enabled: false,
            ..test_persistence_config()
        };
        let persistence = AgentPersistence::new(config);
        let agent_config = AgentConfiguration::new("disabled-test");
        let agent = Agent::new(agent_config.clone());

        // Should succeed without writing
        persistence.save_agent(&agent, &agent_config).unwrap();
    }
}
