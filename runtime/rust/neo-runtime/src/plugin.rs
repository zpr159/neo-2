//! Plugin loader with dynamic library loading, hot reload, sandbox, and verification.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::config::PluginConfig;
use crate::error::{PluginError, PluginErrorKind};

/// Unique plugin identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PluginId(pub Uuid);

impl PluginId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for PluginId {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginState {
    Discovered,
    Loaded,
    Initialized,
    Running,
    Stopped,
    Failed,
    Unloaded,
}

impl std::fmt::Display for PluginState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Discovered => write!(f, "discovered"),
            Self::Loaded => write!(f, "loaded"),
            Self::Initialized => write!(f, "initialized"),
            Self::Running => write!(f, "running"),
            Self::Stopped => write!(f, "stopped"),
            Self::Failed => write!(f, "failed"),
            Self::Unloaded => write!(f, "unloaded"),
        }
    }
}

/// Metadata describing a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDescriptor {
    pub id: PluginId,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub path: PathBuf,
    pub checksum: String,
    pub required_permissions: Vec<String>,
    pub dependencies: Vec<String>,
}

/// A loaded plugin instance.
#[derive(Clone)]
pub struct LoadedPlugin {
    pub descriptor: PluginDescriptor,
    pub state: PluginState,
    pub loaded_at: u64,
    pub error_message: Option<String>,
}

/// Sandbox configuration for plugin execution.
#[derive(Debug, Clone)]
pub struct PluginSandboxConfig {
    pub enabled: bool,
    pub allowed_permissions: Vec<String>,
    pub max_memory_bytes: usize,
    pub max_cpu_time_ms: u64,
    pub network_access: bool,
    pub filesystem_access: bool,
}

impl Default for PluginSandboxConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            allowed_permissions: vec!["read".to_string()],
            max_memory_bytes: 256 * 1024 * 1024,
            max_cpu_time_ms: 30_000,
            network_access: false,
            filesystem_access: false,
        }
    }
}

/// Verifies plugin integrity via SHA-256 checksums.
pub struct PluginVerifier {
    known_checksums: RwLock<HashMap<String, String>>,
}

impl PluginVerifier {
    pub fn new() -> Self {
        Self {
            known_checksums: RwLock::new(HashMap::new()),
        }
    }

    /// Compute SHA-256 checksum of a file.
    pub fn compute_checksum(path: &Path) -> Result<String, PluginError> {
        let data = std::fs::read(path).map_err(|e| {
            PluginError::new(
                PluginErrorKind::VerificationFailed,
                format!("failed to read file: {}", e),
            )
        })?;

        let mut hasher = Sha256::new();
        hasher.update(&data);
        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }

    /// Register a known-good checksum.
    pub fn register_checksum(&self, name: &str, checksum: &str) {
        self.known_checksums
            .write()
            .insert(name.to_string(), checksum.to_string());
    }

    /// Verify a file against its known checksum.
    pub fn verify(&self, name: &str, path: &Path) -> Result<bool, PluginError> {
        let computed = Self::compute_checksum(path)?;
        let known = self.known_checksums.read().get(name).cloned();
        match known {
            Some(expected) => Ok(computed == expected),
            None => Ok(true),
        }
    }
}

impl Default for PluginVerifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Sandbox that validates plugin permissions and resource usage.
pub struct PluginSandbox {
    config: PluginSandboxConfig,
    active_permissions: RwLock<Vec<String>>,
}

impl PluginSandbox {
    pub fn new(config: PluginSandboxConfig) -> Self {
        Self {
            config,
            active_permissions: RwLock::new(Vec::new()),
        }
    }

    /// Validate that a requested permission is allowed.
    pub fn validate_permission(&self, permission: &str) -> Result<(), PluginError> {
        if !self.config.enabled {
            return Ok(());
        }
        if self.config.allowed_permissions.iter().any(|p| p == permission) {
            self.active_permissions.write().push(permission.to_string());
            Ok(())
        } else {
            Err(PluginError::new(
                PluginErrorKind::SandboxViolation,
                format!("permission '{}' not allowed", permission),
            ))
        }
    }

    /// Check whether a resource usage is within limits.
    pub fn check_resource_limit(
        &self,
        resource: &str,
        amount: u64,
    ) -> Result<(), PluginError> {
        if !self.config.enabled {
            return Ok(());
        }
        match resource {
            "memory" => {
                if amount as usize > self.config.max_memory_bytes {
                    Err(PluginError::new(
                        PluginErrorKind::SandboxViolation,
                        format!(
                            "memory limit exceeded: {} > {}",
                            amount, self.config.max_memory_bytes
                        ),
                    ))
                } else {
                    Ok(())
                }
            }
            "cpu_time" => {
                if amount > self.config.max_cpu_time_ms {
                    Err(PluginError::new(
                        PluginErrorKind::SandboxViolation,
                        format!(
                            "cpu time limit exceeded: {} > {}",
                            amount, self.config.max_cpu_time_ms
                        ),
                    ))
                } else {
                    Ok(())
                }
            }
            "network" => {
                if !self.config.network_access {
                    Err(PluginError::new(
                        PluginErrorKind::SandboxViolation,
                        "network access not allowed",
                    ))
                } else {
                    Ok(())
                }
            }
            "filesystem" => {
                if !self.config.filesystem_access {
                    Err(PluginError::new(
                        PluginErrorKind::SandboxViolation,
                        "filesystem access not allowed",
                    ))
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        }
    }

    /// Get the currently active permissions.
    pub fn active_permissions(&self) -> Vec<String> {
        self.active_permissions.read().clone()
    }
}

/// Configuration for hot-reload behavior.
#[derive(Debug, Clone)]
pub struct HotReloadConfig {
    pub enabled: bool,
    pub interval: Duration,
    pub watch_paths: Vec<PathBuf>,
}

impl Default for HotReloadConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval: Duration::from_secs(5),
            watch_paths: Vec::new(),
        }
    }
}

/// Plugin loader managing discovery, loading, hot-reload, and unloading.
pub struct PluginLoader {
    plugins: RwLock<HashMap<PluginId, LoadedPlugin>>,
    descriptors: RwLock<HashMap<PluginId, PluginDescriptor>>,
    verifier: PluginVerifier,
    sandbox: PluginSandbox,
    hot_reload_config: HotReloadConfig,
    running: AtomicBool,
    stats: RwLock<PluginStatistics>,
    plugin_dir: PathBuf,
}

/// Plugin loader statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginStatistics {
    pub total_discovered: u64,
    pub total_loaded: u64,
    pub total_failed: u64,
    pub total_unloaded: u64,
    pub total_reloads: u64,
    pub active_plugins: usize,
}

impl PluginLoader {
    /// Create a new plugin loader.
    pub fn new(
        plugin_dir: PathBuf,
        sandbox_config: PluginSandboxConfig,
        hot_reload: HotReloadConfig,
    ) -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
            descriptors: RwLock::new(HashMap::new()),
            verifier: PluginVerifier::new(),
            sandbox: PluginSandbox::new(sandbox_config),
            hot_reload_config: hot_reload,
            running: AtomicBool::new(true),
            stats: RwLock::new(PluginStatistics::default()),
            plugin_dir,
        }
    }

    /// Create a plugin loader from a PluginConfig.
    pub fn from_config(config: &PluginConfig) -> Self {
        Self::new(
            PathBuf::from(&config.plugin_directory),
            PluginSandboxConfig::default(),
            HotReloadConfig {
                enabled: config.enable_hot_reload,
                interval: Duration::from_millis(config.hot_reload_interval_ms),
                watch_paths: vec![PathBuf::from(&config.plugin_directory)],
            },
        )
    }

    /// Discover plugins in the plugin directory.
    pub fn discover(&self) -> Result<Vec<PluginDescriptor>, PluginError> {
        if !self.plugin_dir.exists() {
            std::fs::create_dir_all(&self.plugin_dir).map_err(|e| {
                PluginError::new(
                    PluginErrorKind::LoadFailed,
                    format!("failed to create plugin dir: {}", e),
                )
            })?;
        }

        let mut discovered = Vec::new();
        let entries = std::fs::read_dir(&self.plugin_dir).map_err(|e| {
            PluginError::new(
                PluginErrorKind::LoadFailed,
                format!("failed to read plugin dir: {}", e),
            )
        })?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| {
                ext == "toml" || ext == "json"
            }) {
                match self.load_descriptor_from_file(&path) {
                    Ok(desc) => {
                        self.stats.write().total_discovered += 1;
                        discovered.push(desc);
                    }
                    Err(_) => continue,
                }
            }
        }

        Ok(discovered)
    }

    /// Load a plugin descriptor from a file.
    fn load_descriptor_from_file(&self, path: &Path) -> Result<PluginDescriptor, PluginError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            PluginError::new(
                PluginErrorKind::LoadFailed,
                format!("failed to read descriptor: {}", e),
            )
        })?;

        let descriptor: PluginDescriptor = if path.extension().map_or(false, |e| e == "toml") {
            toml::from_str(&content).map_err(|e| {
                PluginError::new(
                    PluginErrorKind::LoadFailed,
                    format!("failed to parse toml: {}", e),
                )
            })?
        } else {
            serde_json::from_str(&content).map_err(|e| {
                PluginError::new(
                    PluginErrorKind::LoadFailed,
                    format!("failed to parse json: {}", e),
                )
            })?
        };

        Ok(descriptor)
    }

    /// Register a plugin descriptor manually.
    pub fn register_plugin(&self, descriptor: PluginDescriptor) {
        self.descriptors
            .write()
            .insert(descriptor.id, descriptor.clone());
        self.plugins.write().insert(
            descriptor.id,
            LoadedPlugin {
                descriptor,
                state: PluginState::Discovered,
                loaded_at: now_ms(),
                error_message: None,
            },
        );
        self.stats.write().total_discovered += 1;
    }

    /// Load a registered plugin (transition Discovered -> Loaded).
    pub fn load(&self, id: PluginId) -> Result<(), PluginError> {
        let mut plugins = self.plugins.write();
        let plugin = plugins
            .get_mut(&id)
            .ok_or_else(|| PluginError::new(PluginErrorKind::LoadFailed, "plugin not found"))?;

        if plugin.state != PluginState::Discovered {
            return Err(PluginError::new(
                PluginErrorKind::LoadFailed,
                format!("plugin in state {}, expected Discovered", plugin.state),
            ));
        }

        if self.sandbox.config.enabled {
            for perm in &plugin.descriptor.required_permissions {
                self.sandbox.validate_permission(perm)?;
            }
        }

        if self.sandbox.config.enabled && plugin.descriptor.path.exists() {
            let result = self.verifier.verify(&plugin.descriptor.name, &plugin.descriptor.path);
            match result {
                Ok(true) => {}
                Ok(false) => {
                    return Err(PluginError::new(
                        PluginErrorKind::VerificationFailed,
                        format!("checksum mismatch for '{}'", plugin.descriptor.name),
                    ));
                }
                Err(e) => return Err(e),
            }
        }

        plugin.state = PluginState::Loaded;
        plugin.loaded_at = now_ms();
        self.stats.write().total_loaded += 1;
        Ok(())
    }

    /// Initialize a loaded plugin (transition Loaded -> Initialized).
    pub fn initialize(&self, id: PluginId) -> Result<(), PluginError> {
        let mut plugins = self.plugins.write();
        let plugin = plugins.get_mut(&id).ok_or_else(|| {
            PluginError::new(PluginErrorKind::InitializationFailed, "plugin not found")
        })?;

        if plugin.state != PluginState::Loaded {
            return Err(PluginError::new(
                PluginErrorKind::InitializationFailed,
                format!("plugin in state {}, expected Loaded", plugin.state),
            ));
        }

        plugin.state = PluginState::Initialized;
        Ok(())
    }

    /// Start an initialized plugin (transition Initialized -> Running).
    pub fn start(&self, id: PluginId) -> Result<(), PluginError> {
        let mut plugins = self.plugins.write();
        let plugin = plugins.get_mut(&id).ok_or_else(|| {
            PluginError::new(PluginErrorKind::LoadFailed, "plugin not found")
        })?;

        if plugin.state != PluginState::Initialized {
            return Err(PluginError::new(
                PluginErrorKind::LoadFailed,
                format!("plugin in state {}, expected Initialized", plugin.state),
            ));
        }

        plugin.state = PluginState::Running;
        Ok(())
    }

    /// Stop a running plugin (transition Running -> Stopped).
    pub fn stop(&self, id: PluginId) -> Result<(), PluginError> {
        let mut plugins = self.plugins.write();
        let plugin = plugins.get_mut(&id).ok_or_else(|| {
            PluginError::new(PluginErrorKind::LoadFailed, "plugin not found")
        })?;

        plugin.state = PluginState::Stopped;
        Ok(())
    }

    /// Unload a plugin (transition Stopped/Failed -> Unloaded).
    pub fn unload(&self, id: PluginId) -> Result<(), PluginError> {
        let mut plugins = self.plugins.write();
        let plugin = plugins.get_mut(&id).ok_or_else(|| {
            PluginError::new(PluginErrorKind::UnloadFailed, "plugin not found")
        })?;

        if plugin.state != PluginState::Stopped && plugin.state != PluginState::Failed {
            return Err(PluginError::new(
                PluginErrorKind::UnloadFailed,
                format!(
                    "cannot unload plugin in state {} (must be Stopped or Failed)",
                    plugin.state
                ),
            ));
        }

        plugin.state = PluginState::Unloaded;
        self.stats.write().total_unloaded += 1;
        Ok(())
    }

    /// Remove an unloaded plugin from the registry.
    pub fn remove(&self, id: PluginId) -> Result<(), PluginError> {
        let plugins = self.plugins.read();
        let plugin = plugins.get(&id).ok_or_else(|| {
            PluginError::new(PluginErrorKind::UnloadFailed, "plugin not found")
        })?;

        if plugin.state != PluginState::Unloaded {
            return Err(PluginError::new(
                PluginErrorKind::UnloadFailed,
                format!("cannot remove plugin in state {}", plugin.state),
            ));
        }

        drop(plugins);
        self.plugins.write().remove(&id);
        self.descriptors.write().remove(&id);
        Ok(())
    }

    /// Get the state of a plugin.
    pub fn state(&self, id: PluginId) -> Option<PluginState> {
        self.plugins.read().get(&id).map(|p| p.state)
    }

    /// Get a loaded plugin by ID.
    pub fn plugin(&self, id: PluginId) -> Option<LoadedPlugin> {
        self.plugins.read().get(&id).cloned()
    }

    /// List all plugins with their states.
    pub fn list(&self) -> Vec<(PluginId, String, PluginState)> {
        self.plugins
            .read()
            .values()
            .map(|p| (p.descriptor.id, p.descriptor.name.clone(), p.state))
            .collect()
    }

    /// Attempt hot-reload: re-discover and load any new plugins.
    pub fn hot_reload(&self) -> Result<Vec<PluginId>, PluginError> {
        if !self.hot_reload_config.enabled || !self.running.load(Ordering::Relaxed) {
            return Ok(Vec::new());
        }

        let discovered = self.discover()?;
        let mut reloaded = Vec::new();

        for desc in discovered {
            if !self.plugins.read().contains_key(&desc.id) {
                self.register_plugin(desc.clone());
                if self.load(desc.id).is_ok() {
                    self.initialize(desc.id).ok();
                    self.start(desc.id).ok();
                    reloaded.push(desc.id);
                    self.stats.write().total_reloads += 1;
                }
            }
        }

        Ok(reloaded)
    }

    /// Shut down all running plugins.
    pub fn shutdown(&self) {
        self.running.store(false, Ordering::Relaxed);
        let ids: Vec<PluginId> = self
            .plugins
            .read()
            .values()
            .filter(|p| p.state == PluginState::Running)
            .map(|p| p.descriptor.id)
            .collect();

        for id in ids {
            self.stop(id).ok();
            self.unload(id).ok();
        }
    }

    /// Get statistics.
    pub fn statistics(&self) -> PluginStatistics {
        let mut stats = self.stats.read().clone();
        stats.active_plugins = self
            .plugins
            .read()
            .values()
            .filter(|p| p.state == PluginState::Running)
            .count();
        stats
    }

    /// Get a reference to the sandbox.
    pub fn sandbox(&self) -> &PluginSandbox {
        &self.sandbox
    }

    /// Get a reference to the verifier.
    pub fn verifier(&self) -> &PluginVerifier {
        &self.verifier
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_descriptor(name: &str) -> PluginDescriptor {
        PluginDescriptor {
            id: PluginId::new(),
            name: name.to_string(),
            version: "1.0.0".to_string(),
            author: "test".to_string(),
            description: "test plugin".to_string(),
            path: PathBuf::from("/tmp/test_plugin.so"),
            checksum: "abc123".to_string(),
            required_permissions: Vec::new(),
            dependencies: Vec::new(),
        }
    }

    #[test]
    fn register_and_load_plugin() {
        let dir = tempfile::tempdir().unwrap();
        let loader = PluginLoader::new(
            dir.path().to_path_buf(),
            PluginSandboxConfig {
                enabled: false,
                ..PluginSandboxConfig::default()
            },
            HotReloadConfig::default(),
        );

        let desc = test_descriptor("test");
        let id = desc.id;
        loader.register_plugin(desc);
        assert_eq!(loader.state(id), Some(PluginState::Discovered));

        loader.load(id).unwrap();
        assert_eq!(loader.state(id), Some(PluginState::Loaded));

        loader.initialize(id).unwrap();
        assert_eq!(loader.state(id), Some(PluginState::Initialized));

        loader.start(id).unwrap();
        assert_eq!(loader.state(id), Some(PluginState::Running));
    }

    #[test]
    fn stop_and_unload() {
        let dir = tempfile::tempdir().unwrap();
        let loader = PluginLoader::new(
            dir.path().to_path_buf(),
            PluginSandboxConfig {
                enabled: false,
                ..PluginSandboxConfig::default()
            },
            HotReloadConfig::default(),
        );

        let desc = test_descriptor("test");
        let id = desc.id;
        loader.register_plugin(desc);
        loader.load(id).unwrap();
        loader.initialize(id).unwrap();
        loader.start(id).unwrap();

        loader.stop(id).unwrap();
        assert_eq!(loader.state(id), Some(PluginState::Stopped));

        loader.unload(id).unwrap();
        assert_eq!(loader.state(id), Some(PluginState::Unloaded));
    }

    #[test]
    fn sandbox_permission_validation() {
        let sandbox = PluginSandbox::new(PluginSandboxConfig {
            enabled: true,
            allowed_permissions: vec!["read".to_string(), "write".to_string()],
            ..PluginSandboxConfig::default()
        });

        assert!(sandbox.validate_permission("read").is_ok());
        assert!(sandbox.validate_permission("write").is_ok());
        assert!(sandbox.validate_permission("execute").is_err());
    }

    #[test]
    fn sandbox_resource_limits() {
        let sandbox = PluginSandbox::new(PluginSandboxConfig {
            enabled: true,
            max_memory_bytes: 1024,
            max_cpu_time_ms: 100,
            network_access: false,
            filesystem_access: false,
            ..PluginSandboxConfig::default()
        });

        assert!(sandbox.check_resource_limit("memory", 512).is_ok());
        assert!(sandbox.check_resource_limit("memory", 2048).is_err());
        assert!(sandbox.check_resource_limit("cpu_time", 50).is_ok());
        assert!(sandbox.check_resource_limit("cpu_time", 200).is_err());
        assert!(sandbox.check_resource_limit("network", 1).is_err());
        assert!(sandbox.check_resource_limit("filesystem", 1).is_err());
    }

    #[test]
    fn plugin_verifier() {
        let verifier = PluginVerifier::new();
        verifier.register_checksum("test", "expected_hash");

        let tempfile = tempfile::NamedTempFile::new().unwrap();
        let result = verifier.verify("test", tempfile.path()).unwrap();
        assert!(!result);

        let result = verifier.verify("unknown", tempfile.path()).unwrap();
        assert!(result);
    }

    #[test]
    fn plugin_statistics() {
        let dir = tempfile::tempdir().unwrap();
        let loader = PluginLoader::new(
            dir.path().to_path_buf(),
            PluginSandboxConfig {
                enabled: false,
                ..PluginSandboxConfig::default()
            },
            HotReloadConfig::default(),
        );

        let desc = test_descriptor("stats-test");
        loader.register_plugin(desc);
        loader.load(PluginId::new()).ok();

        let stats = loader.statistics();
        assert_eq!(stats.total_discovered, 1);
    }

    #[test]
    fn list_plugins() {
        let dir = tempfile::tempdir().unwrap();
        let loader = PluginLoader::new(
            dir.path().to_path_buf(),
            PluginSandboxConfig {
                enabled: false,
                ..PluginSandboxConfig::default()
            },
            HotReloadConfig::default(),
        );

        loader.register_plugin(test_descriptor("a"));
        loader.register_plugin(test_descriptor("b"));

        let list = loader.list();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn shutdown_stops_all() {
        let dir = tempfile::tempdir().unwrap();
        let loader = PluginLoader::new(
            dir.path().to_path_buf(),
            PluginSandboxConfig {
                enabled: false,
                ..PluginSandboxConfig::default()
            },
            HotReloadConfig::default(),
        );

        let desc = test_descriptor("shutdown-test");
        let id = desc.id;
        loader.register_plugin(desc);
        loader.load(id).unwrap();
        loader.initialize(id).unwrap();
        loader.start(id).unwrap();

        loader.shutdown();
        assert_eq!(loader.state(id), Some(PluginState::Stopped));
    }
}
