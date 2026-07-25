//! Persistence for tool manifests, configurations, and execution logs.

use std::path::{Path, PathBuf};

use crate::analytics::ExecutionRecord;
use crate::error::ToolResult;
use crate::types::{ToolConfiguration, ToolManifest, ToolMetrics};

// ---------------------------------------------------------------------------
// PersistenceConfig
// ---------------------------------------------------------------------------

/// Configuration for tool persistence.
#[derive(Debug, Clone)]
pub struct PersistenceConfig {
    pub data_dir: PathBuf,
    pub manifests_dir: PathBuf,
    pub configs_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub metrics_dir: PathBuf,
}

impl PersistenceConfig {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        let base = data_dir.into();
        Self {
            manifests_dir: base.join("manifests"),
            configs_dir: base.join("configs"),
            logs_dir: base.join("logs"),
            metrics_dir: base.join("metrics"),
            data_dir: base,
        }
    }

    pub fn ensure_dirs(&self) -> ToolResult<()> {
        for dir in [
            &self.manifests_dir,
            &self.configs_dir,
            &self.logs_dir,
            &self.metrics_dir,
        ] {
            std::fs::create_dir_all(dir)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// ToolPersistence
// ---------------------------------------------------------------------------

/// Handles persistence of tool data to disk.
pub struct ToolPersistence {
    pub config: PersistenceConfig,
}

impl std::fmt::Debug for ToolPersistence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolPersistence")
            .field("data_dir", &self.config.data_dir)
            .finish()
    }
}

impl ToolPersistence {
    pub fn new(config: PersistenceConfig) -> Self {
        Self { config }
    }

    /// Save a tool manifest.
    pub fn save_manifest(&self, name: &str, manifest: &ToolManifest) -> ToolResult<()> {
        let path = self.config.manifests_dir.join(format!("{name}.json"));
        let json = serde_json::to_string_pretty(manifest)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    /// Load a tool manifest.
    pub fn load_manifest(&self, name: &str) -> ToolResult<ToolManifest> {
        let path = self.config.manifests_dir.join(format!("{name}.json"));
        let json = std::fs::read_to_string(&path)?;
        let manifest: ToolManifest = serde_json::from_str(&json)?;
        Ok(manifest)
    }

    /// List all saved manifests.
    pub fn list_manifests(&self) -> ToolResult<Vec<String>> {
        let mut names = Vec::new();
        if self.config.manifests_dir.exists() {
            for entry in std::fs::read_dir(&self.config.manifests_dir)? {
                let entry = entry?;
                if entry.path().extension().is_some_and(|e| e == "json") {
                    if let Some(name) = entry.path().file_stem().and_then(|s| s.to_str()) {
                        names.push(name.to_string());
                    }
                }
            }
        }
        Ok(names)
    }

    /// Delete a manifest.
    pub fn delete_manifest(&self, name: &str) -> ToolResult<()> {
        let path = self.config.manifests_dir.join(format!("{name}.json"));
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// Save tool configuration.
    pub fn save_config(&self, name: &str, config: &ToolConfiguration) -> ToolResult<()> {
        let path = self.config.configs_dir.join(format!("{name}.json"));
        let json = serde_json::to_string_pretty(config)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    /// Load tool configuration.
    pub fn load_config(&self, name: &str) -> ToolResult<ToolConfiguration> {
        let path = self.config.configs_dir.join(format!("{name}.json"));
        let json = std::fs::read_to_string(&path)?;
        let config: ToolConfiguration = serde_json::from_str(&json)?;
        Ok(config)
    }

    /// Save execution log record.
    pub fn append_execution_log(&self, record: &ExecutionRecord) -> ToolResult<()> {
        let path = self
            .config
            .logs_dir
            .join(format!("{}.jsonl", record.tool_name));
        let mut json = serde_json::to_string(record)?;
        json.push('\n');
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    /// Load execution logs for a tool.
    pub fn load_execution_logs(&self, name: &str) -> ToolResult<Vec<ExecutionRecord>> {
        let path = self.config.logs_dir.join(format!("{name}.jsonl"));
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = std::fs::read_to_string(&path)?;
        let mut records = Vec::new();
        for line in content.lines() {
            if !line.trim().is_empty() {
                let record: ExecutionRecord = serde_json::from_str(line)?;
                records.push(record);
            }
        }
        Ok(records)
    }

    /// Save tool metrics.
    pub fn save_metrics(&self, name: &str, metrics: &ToolMetrics) -> ToolResult<()> {
        let path = self.config.metrics_dir.join(format!("{name}.json"));
        let json = serde_json::to_string_pretty(metrics)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    /// Load tool metrics.
    pub fn load_metrics(&self, name: &str) -> ToolResult<ToolMetrics> {
        let path = self.config.metrics_dir.join(format!("{name}.json"));
        let json = std::fs::read_to_string(&path)?;
        let metrics: ToolMetrics = serde_json::from_str(&json)?;
        Ok(metrics)
    }

    pub fn data_dir(&self) -> &Path {
        &self.config.data_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ToolCategory, ToolMetadata, ToolType, ToolVersion};

    #[test]
    fn test_persistence_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let persistence = ToolPersistence::new(PersistenceConfig::new(dir.path()));
        persistence.config.ensure_dirs().unwrap();

        let manifest = ToolManifest::new(
            ToolMetadata::new(
                "test_tool",
                "A test",
                ToolType::Shell,
                ToolCategory::Execute,
                ToolVersion::new(1, 0, 0),
            ),
            ToolConfiguration::enabled(),
        );

        persistence.save_manifest("test_tool", &manifest).unwrap();
        let loaded = persistence.load_manifest("test_tool").unwrap();
        assert_eq!(loaded.metadata.name, "test_tool");

        let names = persistence.list_manifests().unwrap();
        assert_eq!(names.len(), 1);

        persistence.delete_manifest("test_tool").unwrap();
        assert!(persistence.load_manifest("test_tool").is_err());
    }
}
