use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use neo_core::error::{NeoError, NeoResult};

/// Unique process identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcessId(pub uuid::Uuid);

impl ProcessId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for ProcessId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ProcessId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for ProcessId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

/// Lifecycle state of a process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProcessState {
    Idle,
    Running,
    Blocked,
    Suspended,
    Completed,
    Failed,
}

impl std::fmt::Display for ProcessState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessState::Idle => write!(f, "idle"),
            ProcessState::Running => write!(f, "running"),
            ProcessState::Blocked => write!(f, "blocked"),
            ProcessState::Suspended => write!(f, "suspended"),
            ProcessState::Completed => write!(f, "completed"),
            ProcessState::Failed => write!(f, "failed"),
        }
    }
}

/// A managed execution process within Neo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Process {
    id: ProcessId,
    name: String,
    state: ProcessState,
    created_at: DateTime<Utc>,
    metadata: HashMap<String, String>,
}

impl Process {
    pub fn new(name: String) -> Self {
        Self {
            id: ProcessId::new(),
            name,
            state: ProcessState::Idle,
            created_at: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn state(&self) -> ProcessState {
        self.state
    }

    pub fn pid(&self) -> ProcessId {
        self.id
    }

    pub fn terminate(&mut self) {
        self.state = ProcessState::Completed;
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }

    pub fn metadata_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.metadata
    }
}

/// Configuration for spawning a new process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessConfig {
    pub name: String,
    pub priority: u32,
    pub memory_limit: usize,
}

/// Manages the lifecycle of all processes in the runtime.
pub struct ProcessManager {
    processes: HashMap<ProcessId, Process>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            processes: HashMap::new(),
        }
    }

    /// Spawn a new process from the given configuration.
    pub async fn spawn(&mut self, config: ProcessConfig) -> NeoResult<ProcessId> {
        let mut proc = Process::new(config.name);
        proc.metadata
            .insert("priority".to_string(), config.priority.to_string());
        proc.metadata
            .insert("memory_limit".to_string(), config.memory_limit.to_string());
        proc.state = ProcessState::Running;

        let id = proc.pid();
        self.processes.insert(id, proc);
        Ok(id)
    }

    /// Kill a process by its identifier.
    pub async fn kill(&mut self, id: ProcessId) -> NeoResult<()> {
        let proc = self
            .processes
            .get_mut(&id)
            .ok_or_else(|| NeoError::NotFound(format!("process {} not found", id)))?;
        proc.state = ProcessState::Failed;
        Ok(())
    }

    /// List all process identifiers managed by this manager.
    pub fn list(&self) -> Vec<ProcessId> {
        self.processes.keys().copied().collect()
    }

    /// Get a reference to a process by id.
    pub fn get(&self, id: &ProcessId) -> NeoResult<&Process> {
        self.processes
            .get(id)
            .ok_or_else(|| NeoError::NotFound(format!("process {} not found", id)))
    }
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}
