//! Sandboxed execution environments for tools.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::{ToolError, ToolResult};
use crate::types::SandboxConfig;

// ---------------------------------------------------------------------------
// ResourceLimits
// ---------------------------------------------------------------------------

/// Resource consumption limits for a sandbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_cpu_pct: Option<f64>,
    pub max_memory_bytes: Option<u64>,
    pub max_disk_bytes: Option<u64>,
    pub max_processes: Option<u32>,
    pub max_execution_ms: Option<u64>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_cpu_pct: Some(100.0),
            max_memory_bytes: Some(256 * 1024 * 1024),
            max_disk_bytes: Some(1024 * 1024 * 1024),
            max_processes: Some(64),
            max_execution_ms: Some(30_000),
        }
    }
}

impl ResourceLimits {
    pub fn strict() -> Self {
        Self {
            max_cpu_pct: Some(25.0),
            max_memory_bytes: Some(64 * 1024 * 1024),
            max_disk_bytes: Some(128 * 1024 * 1024),
            max_processes: Some(8),
            max_execution_ms: Some(10_000),
        }
    }

    pub fn relaxed() -> Self {
        Self {
            max_cpu_pct: Some(100.0),
            max_memory_bytes: Some(1024 * 1024 * 1024),
            max_disk_bytes: Some(10 * 1024 * 1024 * 1024),
            max_processes: Some(256),
            max_execution_ms: Some(300_000),
        }
    }

    pub fn unrestricted() -> Self {
        Self {
            max_cpu_pct: None,
            max_memory_bytes: None,
            max_disk_bytes: None,
            max_processes: None,
            max_execution_ms: None,
        }
    }
}

// ---------------------------------------------------------------------------
// FilesystemSandbox
// ---------------------------------------------------------------------------

/// Sandbox that restricts filesystem access.
#[derive(Debug, Clone)]
pub struct FilesystemSandbox {
    pub allowed_paths: HashSet<PathBuf>,
    pub denied_paths: HashSet<PathBuf>,
    pub temp_dir: Option<PathBuf>,
    pub read_only: bool,
}

impl FilesystemSandbox {
    pub fn new() -> Self {
        Self {
            allowed_paths: HashSet::new(),
            denied_paths: HashSet::new(),
            temp_dir: None,
            read_only: false,
        }
    }

    pub fn allow_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.allowed_paths.insert(path.into());
        self
    }

    pub fn deny_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.denied_paths.insert(path.into());
        self
    }

    pub fn with_temp_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.temp_dir = Some(dir.into());
        self
    }

    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }

    /// Check if a path operation is allowed.
    pub fn check_path(&self, path: &Path, is_write: bool) -> ToolResult<()> {
        if is_write && self.read_only {
            return Err(ToolError::sandbox_violation(
                "filesystem sandbox is read-only",
            ));
        }

        for denied in &self.denied_paths {
            if path.starts_with(denied) {
                return Err(ToolError::sandbox_violation(format!(
                    "access denied: {} is in denied path {}",
                    path.display(),
                    denied.display()
                )));
            }
        }

        if !self.allowed_paths.is_empty() {
            let allowed = self.allowed_paths.iter().any(|a| path.starts_with(a));
            if !allowed {
                return Err(ToolError::sandbox_violation(format!(
                    "access denied: {} is not in any allowed path",
                    path.display()
                )));
            }
        }

        Ok(())
    }
}

impl Default for FilesystemSandbox {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// NetworkSandbox
// ---------------------------------------------------------------------------

/// Sandbox that restricts network access.
#[derive(Debug, Clone)]
pub struct NetworkSandbox {
    pub allowed_hosts: HashSet<String>,
    pub denied_hosts: HashSet<String>,
    pub allowed_ports: HashSet<u16>,
    pub denied_ports: HashSet<u16>,
    pub allow_outbound: bool,
    pub allow_inbound: bool,
    pub max_connections: Option<u32>,
}

impl NetworkSandbox {
    pub fn new() -> Self {
        Self {
            allowed_hosts: HashSet::new(),
            denied_hosts: HashSet::new(),
            allowed_ports: HashSet::new(),
            denied_ports: HashSet::new(),
            allow_outbound: true,
            allow_inbound: false,
            max_connections: Some(100),
        }
    }

    pub fn allow_host(mut self, host: impl Into<String>) -> Self {
        self.allowed_hosts.insert(host.into());
        self
    }

    pub fn deny_host(mut self, host: impl Into<String>) -> Self {
        self.denied_hosts.insert(host.into());
        self
    }

    pub fn allow_port(mut self, port: u16) -> Self {
        self.allowed_ports.insert(port);
        self
    }

    pub fn deny_port(mut self, port: u16) -> Self {
        self.denied_ports.insert(port);
        self
    }

    pub fn outbound_only(mut self) -> Self {
        self.allow_outbound = true;
        self.allow_inbound = false;
        self
    }

    pub fn no_network(mut self) -> Self {
        self.allow_outbound = false;
        self.allow_inbound = false;
        self
    }

    /// Check if a network connection is allowed.
    pub fn check_connection(&self, host: &str, port: u16) -> ToolResult<()> {
        if !self.allow_outbound {
            return Err(ToolError::sandbox_violation("outbound network is disabled"));
        }

        if self.denied_hosts.contains(host) {
            return Err(ToolError::sandbox_violation(format!(
                "host '{host}' is denied"
            )));
        }

        if self.denied_ports.contains(&port) {
            return Err(ToolError::sandbox_violation(format!(
                "port {port} is denied"
            )));
        }

        if !self.allowed_hosts.is_empty() && !self.allowed_hosts.contains(host) {
            return Err(ToolError::sandbox_violation(format!(
                "host '{host}' is not in allow list"
            )));
        }

        if !self.allowed_ports.is_empty() && !self.allowed_ports.contains(&port) {
            return Err(ToolError::sandbox_violation(format!(
                "port {port} is not in allow list"
            )));
        }

        Ok(())
    }
}

impl Default for NetworkSandbox {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Sandbox — combined execution sandbox
// ---------------------------------------------------------------------------

/// Combined execution sandbox with filesystem, network, and resource limits.
#[derive(Debug, Clone)]
pub struct Sandbox {
    pub name: String,
    pub filesystem: FilesystemSandbox,
    pub network: NetworkSandbox,
    pub resources: ResourceLimits,
    pub active: bool,
}

impl Sandbox {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            filesystem: FilesystemSandbox::new(),
            network: NetworkSandbox::new(),
            resources: ResourceLimits::default(),
            active: true,
        }
    }

    pub fn strict(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            filesystem: FilesystemSandbox::new().read_only(),
            network: NetworkSandbox::new().no_network(),
            resources: ResourceLimits::strict(),
            active: true,
        }
    }

    pub fn permissive(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            filesystem: FilesystemSandbox::new(),
            network: NetworkSandbox::new(),
            resources: ResourceLimits::relaxed(),
            active: true,
        }
    }

    pub fn with_filesystem(mut self, fs: FilesystemSandbox) -> Self {
        self.filesystem = fs;
        self
    }

    pub fn with_network(mut self, net: NetworkSandbox) -> Self {
        self.network = net;
        self
    }

    pub fn with_resources(mut self, res: ResourceLimits) -> Self {
        self.resources = res;
        self
    }

    pub fn from_config(name: &str, config: &SandboxConfig) -> Self {
        let mut fs = FilesystemSandbox::new();
        for p in &config.allowed_paths {
            fs = fs.allow_path(p);
        }
        for p in &config.denied_paths {
            fs = fs.deny_path(p);
        }
        if let Some(ref td) = config.temp_dir {
            fs = fs.with_temp_dir(td);
        }

        let mut net = NetworkSandbox::new();
        if !config.network_allowed {
            net = net.no_network();
        }

        Self {
            name: name.to_string(),
            filesystem: fs,
            network: net,
            resources: ResourceLimits::default(),
            active: true,
        }
    }

    /// Validate a filesystem path operation.
    pub fn check_filesystem(&self, path: &Path, is_write: bool) -> ToolResult<()> {
        if !self.active {
            return Ok(());
        }
        self.filesystem.check_path(path, is_write)
    }

    /// Validate a network connection.
    pub fn check_network(&self, host: &str, port: u16) -> ToolResult<()> {
        if !self.active {
            return Ok(());
        }
        self.network.check_connection(host, port)
    }

    pub fn deactivate(&mut self) {
        self.active = false;
    }

    pub fn activate(&mut self) {
        self.active = true;
    }
}

// ---------------------------------------------------------------------------
// SandboxManager — manages per-execution sandboxes
// ---------------------------------------------------------------------------

/// Manages sandboxes for active executions.
pub struct SandboxManager {
    sandboxes: DashMap<String, Arc<RwLock<Sandbox>>>,
}

impl std::fmt::Debug for SandboxManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SandboxManager")
            .field("sandbox_count", &self.sandboxes.len())
            .finish()
    }
}

impl SandboxManager {
    pub fn new() -> Self {
        Self {
            sandboxes: DashMap::new(),
        }
    }

    /// Create a sandbox for an execution.
    pub fn create_sandbox(
        &self,
        execution_id: &str,
        config: Option<&SandboxConfig>,
    ) -> Arc<RwLock<Sandbox>> {
        let sandbox = match config {
            Some(cfg) => Sandbox::from_config(execution_id, cfg),
            None => Sandbox::permissive(execution_id),
        };
        let arc = Arc::new(RwLock::new(sandbox));
        self.sandboxes
            .insert(execution_id.to_string(), Arc::clone(&arc));
        arc
    }

    /// Get a sandbox by execution ID.
    pub fn get(&self, execution_id: &str) -> Option<Arc<RwLock<Sandbox>>> {
        self.sandboxes
            .get(execution_id)
            .map(|entry| Arc::clone(entry.value()))
    }

    /// Remove a sandbox after execution completes.
    pub fn remove(&self, execution_id: &str) -> Option<Arc<RwLock<Sandbox>>> {
        self.sandboxes.remove(execution_id).map(|(_, v)| v)
    }

    pub fn active_count(&self) -> usize {
        self.sandboxes.len()
    }
}

impl Default for SandboxManager {
    fn default() -> Self {
        Self::new()
    }
}

use dashmap::DashMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filesystem_sandbox_read_only() {
        let sandbox = FilesystemSandbox::new().read_only();
        assert!(sandbox
            .check_path(Path::new("/tmp/file.txt"), true)
            .is_err());
        assert!(sandbox
            .check_path(Path::new("/tmp/file.txt"), false)
            .is_ok());
    }

    #[test]
    fn test_filesystem_sandbox_deny_path() {
        let sandbox = FilesystemSandbox::new().deny_path("/etc");
        assert!(sandbox.check_path(Path::new("/etc/passwd"), false).is_err());
        assert!(sandbox
            .check_path(Path::new("/tmp/file.txt"), false)
            .is_ok());
    }

    #[test]
    fn test_network_sandbox_no_network() {
        let sandbox = NetworkSandbox::new().no_network();
        assert!(sandbox.check_connection("example.com", 443).is_err());
    }

    #[test]
    fn test_network_sandbox_allow_host() {
        let sandbox = NetworkSandbox::new().allow_host("example.com");
        assert!(sandbox.check_connection("example.com", 443).is_ok());
        assert!(sandbox.check_connection("evil.com", 443).is_err());
    }

    #[test]
    fn test_sandbox_combined() {
        let sandbox = Sandbox::strict("test");
        assert!(sandbox.check_filesystem(Path::new("/tmp"), true).is_err());
        assert!(sandbox.check_network("example.com", 443).is_err());
    }

    #[test]
    fn test_resource_limits_strict() {
        let limits = ResourceLimits::strict();
        assert_eq!(limits.max_cpu_pct, Some(25.0));
    }
}
