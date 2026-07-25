use serde::{Deserialize, Serialize};

/// Describes the kind of plugin being registered.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginType {
    Tool,
    Workflow,
    Provider,
    Capability,
    PromptTemplate,
    Retriever,
    Planner,
}

/// A dependency declared by a plugin manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// Crate or package name of the dependency.
    pub name: String,
    /// Semver version requirement string (e.g. `">=1.0"`).
    pub version_req: String,
    /// Whether the dependency may be omitted at load time.
    pub optional: bool,
}

/// Declarative manifest for a single Neo plugin.
///
/// The manifest is the primary metadata source consumed by the
/// [`PluginLoader`](super::loader::PluginLoader) and
/// [`PluginRegistry`](super::mod::PluginRegistry).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Unique identifier for the plugin (e.g. `"neo-plugin-search"`).
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Semver version string.
    pub version: String,
    /// Plugin author or organisation.
    pub author: String,
    /// Short description of the plugin's purpose.
    pub description: String,
    /// Minimum Neo core version required to run this plugin.
    pub neo_version_req: String,
    /// The kind of plugin.
    pub capabilities: PluginType,
    /// Relative path or symbol name used to load the plugin entry point.
    pub entry_point: String,
    /// Other plugins or crates this plugin depends on.
    pub dependencies: Vec<Dependency>,
    /// Optional JSON-Schema describing valid configuration for this plugin.
    pub config_schema: Option<serde_json::Value>,
}
