use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::core::{
    Capability, CapabilityId, CapabilityMetadata, CapabilityNamespace, CapabilityState,
    CapabilityVersion,
};
use crate::error::{CapabilityError, CapabilityResult};

/// Strategy for discovering capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscoveryStrategy {
    /// Scan filesystem directories for capability manifests.
    Filesystem,
    /// Discover capabilities registered via the plugin system.
    Plugin,
    /// Discover capabilities from a remote registry.
    Remote { registry_url: String },
    /// Discover built-in capabilities compiled into the binary.
    BuiltIn,
    /// Use all available strategies.
    All,
}

/// A discovered capability source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitySource {
    /// Where this capability was discovered from.
    pub source_type: DiscoveryStrategy,
    /// Path or URL to the source.
    pub location: String,
    /// When it was discovered.
    pub discovered_at: DateTime<Utc>,
    /// Checksum of the capability manifest.
    pub checksum: String,
}

/// Manifest describing a capability on disk or in a registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityManifest {
    /// Capability name.
    pub name: String,
    /// Version string.
    pub version: String,
    /// Description.
    pub description: String,
    /// Category.
    pub category: String,
    /// Namespace.
    pub namespace: String,
    /// Author.
    pub author: String,
    /// License.
    pub license: String,
    /// Tags.
    pub tags: Vec<String>,
    /// Aliases.
    pub aliases: Vec<String>,
    /// Dependencies (name + version constraint).
    pub dependencies: Vec<ManifestDependency>,
    /// Required permissions.
    pub permissions: Vec<String>,
    /// Entry point path (relative to manifest).
    pub entry_point: Option<String>,
    /// Minimum Neo version required.
    pub min_neo_version: String,
    /// Custom metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

/// A dependency declared in a manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestDependency {
    /// Dependency name.
    pub name: String,
    /// Version constraint (e.g., ">=1.0.0", "^1.0.0").
    pub version_constraint: String,
    /// Whether the dependency is optional.
    pub optional: bool,
}

/// Conflict type detected during discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictType {
    /// Two capabilities with the same name and version.
    Duplicate { existing_id: CapabilityId, new_source: CapabilitySource },
    /// Version conflict between capability versions.
    VersionConflict {
        name: String,
        existing: String,
        incoming: String,
    },
    /// Namespace conflict.
    NamespaceConflict {
        name: String,
        namespace: CapabilityNamespace,
        conflicting_id: CapabilityId,
    },
}

/// Hot reload event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HotReloadEvent {
    /// A new capability was discovered and loaded.
    CapabilityAdded {
        id: CapabilityId,
        name: String,
        version: CapabilityVersion,
    },
    /// A capability was updated.
    CapabilityUpdated {
        id: CapabilityId,
        name: String,
        old_version: CapabilityVersion,
        new_version: CapabilityVersion,
    },
    /// A capability was removed.
    CapabilityRemoved {
        id: CapabilityId,
        name: String,
    },
    /// Discovery scan completed.
    ScanCompleted {
        discovered: usize,
        added: usize,
        updated: usize,
        removed: usize,
    },
    /// An error occurred during hot reload.
    Error { message: String },
}

/// Discovery engine that finds and registers capabilities from various sources.
pub struct DiscoveryEngine {
    /// Known capability sources.
    sources: RwLock<HashMap<CapabilityId, CapabilitySource>>,
    /// Filesystem paths to scan.
    scan_paths: RwLock<Vec<PathBuf>>,
    /// Strategies to use for discovery.
    strategies: RwLock<Vec<DiscoveryStrategy>>,
    /// Hot reload event log.
    events: RwLock<Vec<HotReloadEvent>>,
    /// Checksums of previously seen manifests.
    known_checksums: RwLock<HashMap<String, String>>,
    /// Callbacks for hot reload events.
    listeners: RwLock<Vec<Arc<dyn Fn(HotReloadEvent) + Send + Sync>>>,
    /// Statistics.
    stats: RwLock<DiscoveryStats>,
}

/// Discovery engine statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiscoveryStats {
    pub total_discoveries: u64,
    pub total_additions: u64,
    pub total_updates: u64,
    pub total_removals: u64,
    pub total_errors: u64,
    pub total_scans: u64,
}

impl DiscoveryEngine {
    /// Create a new discovery engine.
    pub fn new() -> Self {
        Self {
            sources: RwLock::new(HashMap::new()),
            scan_paths: RwLock::new(Vec::new()),
            strategies: RwLock::new(vec![DiscoveryStrategy::Filesystem]),
            events: RwLock::new(Vec::new()),
            known_checksums: RwLock::new(HashMap::new()),
            listeners: RwLock::new(Vec::new()),
            stats: RwLock::new(DiscoveryStats::default()),
        }
    }

    /// Add a filesystem path to scan.
    pub fn add_scan_path(&self, path: PathBuf) {
        self.scan_paths.write().push(path);
    }

    /// Add a discovery strategy.
    pub fn add_strategy(&self, strategy: DiscoveryStrategy) {
        self.strategies.write().push(strategy);
    }

    /// Register a hot reload listener.
    pub fn on_event(&self, listener: Arc<dyn Fn(HotReloadEvent) + Send + Sync>) {
        self.listeners.write().push(listener);
    }

    /// Emit a hot reload event to all listeners.
    fn emit_event(&self, event: HotReloadEvent) {
        self.events.write().push(event.clone());
        for listener in self.listeners.read().iter() {
            listener(event.clone());
        }
    }

    /// Register a discovered capability source.
    pub fn register_source(
        &self,
        id: CapabilityId,
        source: CapabilitySource,
    ) -> CapabilityResult<()> {
        self.sources.write().insert(id, source);
        self.stats.write().total_discoveries += 1;
        self.emit_event(HotReloadEvent::CapabilityAdded {
            id,
            name: id.to_string(),
            version: CapabilityVersion::initial(),
        });
        Ok(())
    }

    /// Get the source for a capability.
    pub fn get_source(&self, id: &CapabilityId) -> Option<CapabilitySource> {
        self.sources.read().get(id).cloned()
    }

    /// Remove a source.
    pub fn remove_source(&self, id: &CapabilityId) -> Option<CapabilitySource> {
        self.sources.write().remove(id)
    }

    /// Validate a manifest for dependency correctness.
    pub fn validate_manifest(
        &self,
        manifest: &CapabilityManifest,
        known_capabilities: &HashMap<String, CapabilityVersion>,
    ) -> CapabilityResult<()> {
        for dep in &manifest.dependencies {
            if !dep.optional {
                if let Some(version_str) = known_capabilities.get(&dep.name) {
                    let required = parse_version_constraint(&dep.version_constraint)?;
                    if !version_str.is_compatible_with(&required) {
                        return Err(CapabilityError::dependency_missing(format!(
                            "dependency '{}' requires version {} but found {}",
                            dep.name, dep.version_constraint, version_str
                        )));
                    }
                } else if !dep.optional {
                    return Err(CapabilityError::dependency_missing(format!(
                        "required dependency '{}' not found",
                        dep.name
                    )));
                }
            }
        }
        Ok(())
    }

    /// Detect conflicts between a new capability and existing ones.
    pub fn detect_conflicts(
        &self,
        name: &str,
        version: &CapabilityVersion,
        existing: &HashMap<String, (CapabilityId, CapabilityVersion)>,
    ) -> Option<ConflictType> {
        if let Some((existing_id, existing_version)) = existing.get(name) {
            if existing_version == version {
                return Some(ConflictType::Duplicate {
                    existing_id: *existing_id,
                    new_source: CapabilitySource {
                        source_type: DiscoveryStrategy::BuiltIn,
                        location: String::new(),
                        discovered_at: Utc::now(),
                        checksum: String::new(),
                    },
                });
            }
            return Some(ConflictType::VersionConflict {
                name: name.to_string(),
                existing: existing_version.to_string(),
                incoming: version.to_string(),
            });
        }
        None
    }

    /// Scan filesystem paths for capability manifests.
    pub fn scan_filesystem(&self) -> CapabilityResult<Vec<CapabilityManifest>> {
        let paths = self.scan_paths.read().clone();
        let mut manifests = Vec::new();

        for path in paths {
            if !path.exists() {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(&path) {
                for entry in entries.flatten() {
                    let manifest_path = entry.path().join("capability.toml");
                    if manifest_path.exists() {
                        match self.load_manifest_from_file(&manifest_path) {
                            Ok(manifest) => manifests.push(manifest),
                            Err(e) => {
                                self.emit_event(HotReloadEvent::Error {
                                    message: format!(
                                        "failed to load manifest from {}: {}",
                                        manifest_path.display(),
                                        e
                                    ),
                                });
                                self.stats.write().total_errors += 1;
                            }
                        }
                    }
                    let manifest_json = entry.path().join("capability.json");
                    if manifest_json.exists() {
                        match self.load_manifest_from_json(&manifest_json) {
                            Ok(manifest) => manifests.push(manifest),
                            Err(e) => {
                                self.emit_event(HotReloadEvent::Error {
                                    message: format!(
                                        "failed to load manifest from {}: {}",
                                        manifest_json.display(),
                                        e
                                    ),
                                });
                                self.stats.write().total_errors += 1;
                            }
                        }
                    }
                }
            }
        }

        self.stats.write().total_scans += 1;
        Ok(manifests)
    }

    /// Load a manifest from a TOML file.
    fn load_manifest_from_file(&self, path: &Path) -> CapabilityResult<CapabilityManifest> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            CapabilityError::discovery_failed(format!("failed to read {}: {}", path.display(), e))
        })?;
        let manifest: CapabilityManifest = toml::from_str(&content).map_err(|e| {
            CapabilityError::discovery_failed(format!(
                "failed to parse manifest {}: {}",
                path.display(),
                e
            ))
        })?;
        Ok(manifest)
    }

    /// Load a manifest from a JSON file.
    fn load_manifest_from_json(&self, path: &Path) -> CapabilityResult<CapabilityManifest> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            CapabilityError::discovery_failed(format!("failed to read {}: {}", path.display(), e))
        })?;
        let manifest: CapabilityManifest = serde_json::from_str(&content).map_err(|e| {
            CapabilityError::discovery_failed(format!(
                "failed to parse manifest {}: {}",
                path.display(),
                e
            ))
        })?;
        Ok(manifest)
    }

    /// Compute checksum of a file.
    pub fn compute_checksum(path: &Path) -> CapabilityResult<String> {
        let data = std::fs::read(path).map_err(|e| {
            CapabilityError::discovery_failed(format!("failed to read file: {}", e))
        })?;
        let mut hasher = Sha256::new();
        hasher.update(&data);
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Check if a manifest has changed since last seen.
    pub fn has_changed(&self, name: &str, checksum: &str) -> bool {
        let known = self.known_checksums.read();
        match known.get(name) {
            Some(old) => old != checksum,
            None => true,
        }
    }

    /// Record a manifest checksum.
    pub fn record_checksum(&self, name: &str, checksum: &str) {
        self.known_checksums
            .write()
            .insert(name.to_string(), checksum.to_string());
    }

    /// Perform a hot reload scan and return events.
    pub fn hot_reload_scan(&self) -> Vec<HotReloadEvent> {
        let mut events = Vec::new();
        match self.scan_filesystem() {
            Ok(manifests) => {
                let mut added = 0;
                let mut updated = 0;
                for manifest in &manifests {
                    let checksum = format!("{:x}", {
                        let mut hasher = Sha256::new();
                        hasher.update(manifest.name.as_bytes());
                        hasher.update(manifest.version.as_bytes());
                        hasher.finalize()
                    });

                    if self.has_changed(&manifest.name, &checksum) {
                        if self.known_checksums.read().contains_key(&manifest.name) {
                            events.push(HotReloadEvent::Error {
                                message: format!("updated capability: {}", manifest.name),
                            });
                            updated += 1;
                        } else {
                            events.push(HotReloadEvent::Error {
                                message: format!("new capability: {}", manifest.name),
                            });
                            added += 1;
                        }
                        self.record_checksum(&manifest.name, &checksum);
                    }
                }

                events.push(HotReloadEvent::ScanCompleted {
                    discovered: manifests.len(),
                    added,
                    updated,
                    removed: 0,
                });
            }
            Err(e) => {
                events.push(HotReloadEvent::Error {
                    message: format!("hot reload scan failed: {}", e),
                });
                self.stats.write().total_errors += 1;
            }
        }

        for event in &events {
            self.emit_event(event.clone());
        }

        events
    }

    /// Get all known sources.
    pub fn all_sources(&self) -> Vec<(CapabilityId, CapabilitySource)> {
        self.sources.read().iter().map(|(k, v)| (*k, v.clone())).collect()
    }

    /// Get discovery statistics.
    pub fn statistics(&self) -> DiscoveryStats {
        self.stats.read().clone()
    }

    /// Get recent events.
    pub fn recent_events(&self, limit: usize) -> Vec<HotReloadEvent> {
        let events = self.events.read();
        events.iter().rev().take(limit).cloned().collect()
    }
}

impl Default for DiscoveryEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a version constraint string like ">=1.0.0" or "^1.0.0".
fn parse_version_constraint(constraint: &str) -> CapabilityResult<CapabilityVersion> {
    let trimmed = constraint
        .trim_start_matches(">=")
        .trim_start_matches("^")
        .trim_start_matches("~")
        .trim_start_matches("=");
    trimmed.parse()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::CapabilityCategory;

    #[test]
    fn discovery_engine_creation() {
        let engine = DiscoveryEngine::new();
        assert!(engine.all_sources().is_empty());
        assert_eq!(engine.statistics().total_discoveries, 0);
    }

    #[test]
    fn manifest_validation_success() {
        let engine = DiscoveryEngine::new();
        let manifest = CapabilityManifest {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            description: "test".to_string(),
            category: "system".to_string(),
            namespace: "neo.core".to_string(),
            author: "test".to_string(),
            license: "MIT".to_string(),
            tags: Vec::new(),
            aliases: Vec::new(),
            dependencies: Vec::new(),
            permissions: Vec::new(),
            entry_point: None,
            min_neo_version: "0.1.0".to_string(),
            metadata: HashMap::new(),
        };

        let mut known = HashMap::new();
        known.insert(
            "some-dep".to_string(),
            CapabilityVersion::new(1, 0, 0),
        );

        assert!(engine.validate_manifest(&manifest, &known).is_ok());
    }

    #[test]
    fn manifest_validation_missing_dep() {
        let engine = DiscoveryEngine::new();
        let manifest = CapabilityManifest {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            description: "test".to_string(),
            category: "system".to_string(),
            namespace: "neo.core".to_string(),
            author: "test".to_string(),
            license: "MIT".to_string(),
            tags: Vec::new(),
            aliases: Vec::new(),
            dependencies: vec![ManifestDependency {
                name: "missing-dep".to_string(),
                version_constraint: ">=1.0.0".to_string(),
                optional: false,
            }],
            permissions: Vec::new(),
            entry_point: None,
            min_neo_version: "0.1.0".to_string(),
            metadata: HashMap::new(),
        };

        let known = HashMap::new();
        assert!(engine.validate_manifest(&manifest, &known).is_err());
    }

    #[test]
    fn conflict_detection() {
        let engine = DiscoveryEngine::new();
        let mut existing = HashMap::new();
        existing.insert(
            "my-cap".to_string(),
            (CapabilityId::new(), CapabilityVersion::new(1, 0, 0)),
        );

        let conflict = engine.detect_conflicts(
            "my-cap",
            &CapabilityVersion::new(1, 0, 0),
            &existing,
        );
        assert!(matches!(conflict, Some(ConflictType::Duplicate { .. })));

        let conflict = engine.detect_conflicts(
            "my-cap",
            &CapabilityVersion::new(2, 0, 0),
            &existing,
        );
        assert!(matches!(conflict, Some(ConflictType::VersionConflict { .. })));

        let no_conflict = engine.detect_conflicts(
            "other-cap",
            &CapabilityVersion::new(1, 0, 0),
            &existing,
        );
        assert!(no_conflict.is_none());
    }

    #[test]
    fn version_constraint_parsing() {
        let v = parse_version_constraint(">=1.0.0").unwrap();
        assert_eq!(v, CapabilityVersion::new(1, 0, 0));

        let v = parse_version_constraint("^2.1.3").unwrap();
        assert_eq!(v, CapabilityVersion::new(2, 1, 3));

        let v = parse_version_constraint("1.0.0").unwrap();
        assert_eq!(v, CapabilityVersion::new(1, 0, 0));
    }

    #[test]
    fn source_registration() {
        let engine = DiscoveryEngine::new();
        let id = CapabilityId::new();
        let source = CapabilitySource {
            source_type: DiscoveryStrategy::BuiltIn,
            location: "builtin".to_string(),
            discovered_at: Utc::now(),
            checksum: "abc".to_string(),
        };

        engine.register_source(id, source.clone()).unwrap();
        assert_eq!(engine.all_sources().len(), 1);
        assert_eq!(engine.statistics().total_discoveries, 1);

        let retrieved = engine.get_source(&id).unwrap();
        assert_eq!(retrieved.location, "builtin");

        engine.remove_source(&id);
        assert!(engine.get_source(&id).is_none());
    }

    #[test]
    fn hot_reload_events() {
        let engine = DiscoveryEngine::new();
        let events = engine.hot_reload_scan();
        assert!(!events.is_empty());
    }

    #[test]
    fn checksum_tracking() {
        let engine = DiscoveryEngine::new();
        assert!(engine.has_changed("test", "abc"));
        engine.record_checksum("test", "abc");
        assert!(!engine.has_changed("test", "abc"));
        assert!(engine.has_changed("test", "def"));
    }

    #[test]
    fn event_listeners() {
        let engine = DiscoveryEngine::new();
        let received = Arc::new(RwLock::new(Vec::new()));
        let received_clone = received.clone();

        engine.on_event(Arc::new(move |event| {
            received_clone.write().push(event);
        }));

        let id = CapabilityId::new();
        let source = CapabilitySource {
            source_type: DiscoveryStrategy::BuiltIn,
            location: "test".to_string(),
            discovered_at: Utc::now(),
            checksum: "x".to_string(),
        };
        engine.register_source(id, source).unwrap();

        let recent = engine.recent_events(10);
        assert!(recent.len() >= 1);
    }
}
