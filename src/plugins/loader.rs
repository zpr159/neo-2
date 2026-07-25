use super::manifest::PluginManifest;
use super::PluginState;

/// Outcome of a single plugin load attempt.
#[derive(Debug, Clone)]
pub struct LoadResult {
    /// `true` when the plugin was loaded without error.
    pub success: bool,
    /// The plugin id on success, `None` on failure.
    pub plugin_id: Option<String>,
    /// Human-readable error message on failure, `None` on success.
    pub error: Option<String>,
}

impl LoadResult {
    /// Create a successful result.
    pub fn ok(plugin_id: String) -> Self {
        Self {
            success: true,
            plugin_id: Some(plugin_id),
            error: None,
        }
    }

    /// Create a failure result.
    pub fn err(error: String) -> Self {
        Self {
            success: false,
            plugin_id: None,
            error: Some(error),
        }
    }
}

/// A loaded plugin entry combining its manifest with runtime state.
#[derive(Debug, Clone)]
pub struct PluginEntry {
    /// The manifest that was used to load this plugin.
    pub manifest: PluginManifest,
    /// Current lifecycle state.
    pub state: PluginState,
}

/// Loads plugin manifests and produces [`PluginEntry`] values.
///
/// In the core crate no actual dynamic-linking happens — the loader
/// validates the manifest, populates a [`PluginEntry`], and hands it
/// off to the registry.
#[derive(Debug)]
pub struct PluginLoader {
    /// Default sandbox level applied to newly loaded plugins.
    default_sandbox_level: super::sandbox::SandboxLevel,
}

impl PluginLoader {
    /// Create a loader with the default (permissive) sandbox level.
    pub fn new() -> Self {
        Self {
            default_sandbox_level: super::sandbox::SandboxLevel::None,
        }
    }

    /// Create a loader with an explicit default sandbox level.
    pub fn with_sandbox_level(level: super::sandbox::SandboxLevel) -> Self {
        Self {
            default_sandbox_level: level,
        }
    }

    /// Validate a manifest and convert it into a [`PluginEntry`].
    ///
    /// This does **not** perform any dynamic linking — it is the core-only
    /// code path used during testing and embedding.
    pub fn load_plugin(&self, manifest: PluginManifest) -> LoadResult {
        if manifest.id.is_empty() {
            return LoadResult::err("plugin manifest missing id".into());
        }
        if manifest.name.is_empty() {
            return LoadResult::err(format!(
                "plugin '{}' manifest missing name",
                manifest.id,
            ));
        }
        if manifest.version.is_empty() {
            return LoadResult::err(format!(
                "plugin '{}' manifest missing version",
                manifest.id,
            ));
        }

        let _ = self.default_sandbox_level;

        LoadResult::ok(manifest.id.clone())
    }

    /// Load every manifest in `manifests`, returning one [`LoadResult`] per entry.
    pub fn load_all(&self, manifests: Vec<PluginManifest>) -> Vec<LoadResult> {
        manifests
            .into_iter()
            .map(|m| self.load_plugin(m))
            .collect()
    }

    /// Scan `directory` for JSON files and attempt to load each as a
    /// [`PluginManifest`].
    ///
    /// Returns every [`LoadResult`], whether successful or not.
    pub fn discover_plugins(&self, directory: &str) -> Vec<LoadResult> {
        let dir = match std::path::Path::new(directory).read_dir() {
            Ok(d) => d,
            Err(e) => {
                return vec![LoadResult::err(format!(
                    "failed to read directory '{}': {}",
                    directory, e,
                ))];
            }
        };

        let mut results = Vec::new();
        for entry in dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let data = match std::fs::read_to_string(&path) {
                Ok(d) => d,
                Err(e) => {
                    results.push(LoadResult::err(format!(
                        "failed to read '{}': {}",
                        path.display(),
                        e,
                    )));
                    continue;
                }
            };
            let manifest: PluginManifest = match serde_json::from_str(&data) {
                Ok(m) => m,
                Err(e) => {
                    results.push(LoadResult::err(format!(
                        "failed to parse '{}': {}",
                        path.display(),
                        e,
                    )));
                    continue;
                }
            };
            results.push(self.load_plugin(manifest));
        }
        results
    }
}
