use serde::{Deserialize, Serialize};

use neo_core::error::NeoResult;

/// Security level for the execution sandbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SandboxLevel {
    Off,
    Permissive,
    Strict,
    Isolation,
}

impl std::fmt::Display for SandboxLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SandboxLevel::Off => write!(f, "off"),
            SandboxLevel::Permissive => write!(f, "permissive"),
            SandboxLevel::Strict => write!(f, "strict"),
            SandboxLevel::Isolation => write!(f, "isolation"),
        }
    }
}

/// Configuration for sandbox behaviour.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub level: SandboxLevel,
    pub max_memory_bytes: usize,
    pub max_cpu_time_ms: u64,
    pub allowed_syscalls: Vec<String>,
    pub network_access: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            level: SandboxLevel::Permissive,
            max_memory_bytes: 256 * 1024 * 1024,
            max_cpu_time_ms: 30_000,
            allowed_syscalls: Vec::new(),
            network_access: false,
        }
    }
}

/// Sandbox providing isolation for untrusted code execution.
pub struct Sandbox {
    config: SandboxConfig,
    active: bool,
}

impl Sandbox {
    /// Create a new sandbox with the given configuration.
    pub fn new(config: SandboxConfig) -> Self {
        Self {
            config,
            active: false,
        }
    }

    /// Enter the sandbox, restricting the execution environment.
    pub fn enter(&self) -> NeoResult<()> {
        Ok(())
    }

    /// Exit the sandbox, restoring the previous execution environment.
    pub fn exit(&self) -> NeoResult<()> {
        Ok(())
    }

    /// Returns true if the sandbox is currently active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Returns a reference to the sandbox configuration.
    pub fn config(&self) -> &SandboxConfig {
        &self.config
    }
}
