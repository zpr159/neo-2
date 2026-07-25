use serde::{Deserialize, Serialize};
use std::fmt;

/// Isolation level enforced on a running plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SandboxLevel {
    /// No sandbox — full host access.
    None,
    /// Basic checks (env-var filtering, panic catching).
    Basic,
    /// Restricted file-system and network access.
    Restricted,
    /// Full WASM / process-level isolation.
    Full,
}

impl fmt::Display for SandboxLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SandboxLevel::None => write!(f, "none"),
            SandboxLevel::Basic => write!(f, "basic"),
            SandboxLevel::Restricted => write!(f, "restricted"),
            SandboxLevel::Full => write!(f, "full"),
        }
    }
}

/// Configuration knobs for a plugin sandbox instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Isolation level to enforce.
    pub level: SandboxLevel,
    /// Module / crate names the plugin is allowed to import.
    pub allowed_modules: Vec<String>,
    /// Hard cap on memory usage in bytes.
    pub max_memory_bytes: u64,
    /// Hard cap on cumulative CPU time in milliseconds.
    pub max_cpu_time_ms: u64,
    /// Whether outbound network access is permitted.
    pub network_access: bool,
    /// Whether host file-system access is permitted.
    pub filesystem_access: bool,
}

impl Default for SandboxConfig {
    /// Returns a permissive default: unrestricted access at [`SandboxLevel::None`].
    fn default() -> Self {
        Self {
            level: SandboxLevel::None,
            allowed_modules: Vec::new(),
            max_memory_bytes: u64::MAX,
            max_cpu_time_ms: u64::MAX,
            network_access: true,
            filesystem_access: true,
        }
    }
}

/// Runtime sandbox that wraps a plugin and enforces the configured limits.
#[derive(Debug)]
pub struct PluginSandbox {
    /// The sandbox configuration currently in effect.
    config: SandboxConfig,
}

impl PluginSandbox {
    /// Create a new sandbox with the given configuration.
    pub fn new(config: SandboxConfig) -> Self {
        Self { config }
    }

    /// Create a sandbox from a [`SandboxLevel`], filling in sensible defaults.
    pub fn from_level(level: SandboxLevel) -> Self {
        let mut config = SandboxConfig::default();
        config.level = level;
        match level {
            SandboxLevel::None => {
                config.network_access = true;
                config.filesystem_access = true;
            }
            SandboxLevel::Basic => {
                config.network_access = true;
                config.filesystem_access = true;
            }
            SandboxLevel::Restricted => {
                config.network_access = false;
                config.filesystem_access = false;
            }
            SandboxLevel::Full => {
                config.network_access = false;
                config.filesystem_access = false;
                config.max_memory_bytes = 128 * 1024 * 1024; // 128 MiB
                config.max_cpu_time_ms = 30_000; // 30 s
            }
        }
        Self::new(config)
    }

    /// Validate the sandbox configuration, returning an error string on failure.
    pub fn validate(&self) -> Result<(), String> {
        if self.config.max_memory_bytes == 0 {
            return Err("max_memory_bytes must be greater than zero".into());
        }
        if self.config.max_cpu_time_ms == 0 {
            return Err("max_cpu_time_ms must be greater than zero".into());
        }
        Ok(())
    }

    /// Tighten the current sandbox to a more restrictive level.
    ///
    /// You can only move *towards* greater restriction — calling with an
    /// equal or less-restrictive level is a no-op.
    pub fn restrict(&mut self, target: SandboxLevel) {
        use SandboxLevel as L;
        let rank = |l: &L| match l {
            L::None => 0,
            L::Basic => 1,
            L::Restricted => 2,
            L::Full => 3,
        };
        if rank(&target) > rank(&self.config.level) {
            self.config.level = target;
            if matches!(target, L::Restricted | L::Full) {
                self.config.network_access = false;
                self.config.filesystem_access = false;
            }
            if matches!(target, L::Full) {
                self.config.max_memory_bytes = self.config.max_memory_bytes.min(128 * 1024 * 1024);
                self.config.max_cpu_time_ms = self.config.max_cpu_time_ms.min(30_000);
            }
        }
    }

    /// Check whether the plugin is permitted to perform the named action.
    ///
    /// Well-known action names: `"network"`, `"filesystem"`, and module
    /// import names.
    pub fn check_permission(&self, action: &str) -> bool {
        match action {
            "network" => self.config.network_access,
            "filesystem" => self.config.filesystem_access,
            module if !module.contains(':') => {
                self.config.allowed_modules.is_empty()
                    || self.config.allowed_modules.iter().any(|m| m == module)
            }
            _ => false,
        }
    }

    /// Enforce resource limits, returning an error when a limit is exceeded.
    ///
    /// `memory_used` is current bytes; `cpu_used_ms` is cumulative CPU ms.
    pub fn enforce_limits(&self, memory_used: u64, cpu_used_ms: u64) -> Result<(), String> {
        if memory_used > self.config.max_memory_bytes {
            return Err(format!(
                "memory limit exceeded: {} > {} bytes",
                memory_used, self.config.max_memory_bytes,
            ));
        }
        if cpu_used_ms > self.config.max_cpu_time_ms {
            return Err(format!(
                "CPU time limit exceeded: {} > {} ms",
                cpu_used_ms, self.config.max_cpu_time_ms,
            ));
        }
        Ok(())
    }
}
