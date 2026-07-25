pub mod capabilities;
pub mod loader;
pub mod manifest;
pub mod sandbox;

use dashmap::DashMap;
use std::fmt;
use std::sync::Arc;

pub use capabilities::*;
pub use loader::{LoadResult, PluginEntry, PluginLoader};
pub use manifest::{Dependency, PluginManifest, PluginType};
pub use sandbox::{PluginSandbox, SandboxConfig, SandboxLevel};

/// Lifecycle state of a registered plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginState {
    /// Plugin manifest has been accepted and stored.
    Registered,
    /// Plugin binary / entry-point is being loaded.
    Loading,
    /// Plugin is fully loaded and available for invocation.
    Active,
    /// Plugin has been deliberately disabled by the operator.
    Disabled,
    /// An error prevented the plugin from reaching Active state.
    Error,
}

impl fmt::Display for PluginState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginState::Registered => write!(f, "registered"),
            PluginState::Loading => write!(f, "loading"),
            PluginState::Active => write!(f, "active"),
            PluginState::Disabled => write!(f, "disabled"),
            PluginState::Error => write!(f, "error"),
        }
    }
}

/// Compact read-only view of a plugin's metadata, suitable for listing.
#[derive(Debug, Clone)]
pub struct PluginInfo {
    /// Unique plugin identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Semver version string.
    pub version: String,
    /// Current lifecycle state.
    pub state: PluginState,
    /// Capabilities advertised by this plugin.
    pub capabilities: PluginCapabilities,
    /// Timestamp (epoch millis) when the plugin was loaded, if applicable.
    pub loaded_at: Option<i64>,
}

/// Thread-safe registry that manages all known plugins.
///
/// Uses [`DashMap`] internally so that concurrent reads and writes
/// from async tasks do not block each other.
#[derive(Debug)]
pub struct PluginRegistry {
    /// Plugin id → full plugin info, protected by a concurrent hash-map.
    plugins: DashMap<String, Arc<PluginInfo>>,
}

impl PluginRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            plugins: DashMap::new(),
        }
    }

    /// Register a plugin. Returns `Err` if a plugin with the same id
    /// already exists.
    pub fn register(&self, info: PluginInfo) -> Result<(), String> {
        if self.plugins.contains_key(&info.id) {
            return Err(format!("plugin '{}' is already registered", info.id));
        }
        self.plugins.insert(info.id.clone(), Arc::new(info));
        Ok(())
    }

    /// Unregister a plugin by id. Returns `true` if the plugin existed
    /// and was removed.
    pub fn unregister(&self, id: &str) -> bool {
        self.plugins.remove(id).is_some()
    }

    /// Return a list of [`PluginInfo`] for every registered plugin.
    pub fn list(&self) -> Vec<PluginInfo> {
        self.plugins
            .iter()
            .map(|entry| entry.value().as_ref().clone())
            .collect()
    }

    /// Look up a single plugin by id.
    pub fn get(&self, id: &str) -> Option<PluginInfo> {
        self.plugins.get(id).map(|r| r.value().as_ref().clone())
    }

    /// Register a plugin directly from its [`PluginManifest`].
    ///
    /// The plugin starts in [`PluginState::Registered`] with empty
    /// capabilities. Use [`PluginLoader`](loader::PluginLoader) when
    /// you also need to populate capabilities from a load step.
    pub fn load_from_manifest(&self, manifest: PluginManifest) -> Result<(), String> {
        let info = PluginInfo {
            id: manifest.id.clone(),
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            state: PluginState::Registered,
            capabilities: PluginCapabilities::default(),
            loaded_at: None,
        };
        self.register(info)
    }

    /// Number of plugins currently in the registry.
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// Returns `true` when the registry contains no plugins.
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
