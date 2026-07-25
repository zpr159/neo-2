use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::{EvolutionId, SubsystemTarget};

/// Isolation level for sandbox execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SandboxLevel {
    Full,
    Partial,
    ReadOnly,
}

impl std::fmt::Display for SandboxLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Full => write!(f, "full"),
            Self::Partial => write!(f, "partial"),
            Self::ReadOnly => write!(f, "read_only"),
        }
    }
}

/// State of a sandbox instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SandboxState {
    Created,
    Active,
    Completed,
    Failed,
    TimedOut,
}

/// Configuration for sandbox isolation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub max_memory_mb: usize,
    pub max_cpu_percent: f64,
    pub timeout_secs: u64,
    pub allowed_subsystems: Vec<SubsystemTarget>,
    pub network_isolation: bool,
    pub filesystem_isolation: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: 512,
            max_cpu_percent: 50.0,
            timeout_secs: 300,
            allowed_subsystems: SubsystemTarget::all_targets(),
            network_isolation: true,
            filesystem_isolation: true,
        }
    }
}

impl SubsystemTarget {
    fn all_targets() -> Vec<Self> {
        vec![
            Self::Runtime,
            Self::Agents,
            Self::Planning,
            Self::Memory,
            Self::KnowledgeGraph,
            Self::Reasoning,
            Self::Workflows,
            Self::Distributed,
            Self::Capabilities,
            Self::Executive,
            Self::Learning,
            Self::Tools,
            Self::Core,
        ]
    }
}

/// An isolated execution sandbox.
#[derive(Debug, Clone)]
pub struct Sandbox {
    pub id: EvolutionId,
    pub config: SandboxConfig,
    pub level: SandboxLevel,
    pub created_at: DateTime<Utc>,
    state: SandboxState,
}

impl Sandbox {
    pub fn new(config: SandboxConfig, level: SandboxLevel) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            config,
            level,
            created_at: Utc::now(),
            state: SandboxState::Created,
        }
    }

    pub fn state(&self) -> SandboxState {
        self.state
    }

    pub fn activate(&mut self) {
        self.state = SandboxState::Active;
    }

    pub fn complete(&mut self) {
        self.state = SandboxState::Completed;
    }

    pub fn fail(&mut self) {
        self.state = SandboxState::Failed;
    }

    pub fn timeout(&mut self) {
        self.state = SandboxState::TimedOut;
    }

    pub fn validate_config(&self) -> Result<(), String> {
        if self.config.max_memory_mb == 0 {
            return Err("max_memory_mb must be > 0".into());
        }
        if self.config.max_cpu_percent <= 0.0 || self.config.max_cpu_percent > 100.0 {
            return Err("max_cpu_percent must be in (0, 100]".into());
        }
        if self.config.timeout_secs == 0 {
            return Err("timeout_secs must be > 0".into());
        }
        Ok(())
    }

    pub fn is_isolated(&self) -> bool {
        self.config.network_isolation && self.config.filesystem_isolation
    }
}
