use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub created_at: DateTime<Utc>,
    pub parameters_count: u64,
    pub checksum: String,
    pub download_url: Option<String>,
}

#[derive(Debug)]
pub struct ModelRegistry {
    models: HashMap<String, Vec<ModelInfo>>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self { models: HashMap::new() }
    }

    pub fn register(&mut self, info: ModelInfo) {
        self.models
            .entry(info.name.clone())
            .or_insert_with(Vec::new)
            .push(info);
    }

    pub fn get(&self, name: &str, version: &str) -> Option<&ModelInfo> {
        self.models.get(name)?.iter().find(|m| m.version == version)
    }

    pub fn latest(&self, name: &str) -> Option<&ModelInfo> {
        self.models.get(name)?.last()
    }

    pub fn list(&self) -> Vec<&ModelInfo> {
        self.models.values().flat_map(|v| v.iter()).collect()
    }

    pub fn search(&self, query: &str) -> Vec<&ModelInfo> {
        self.models.values()
            .flat_map(|v| v.iter())
            .filter(|m| m.name.contains(query) || m.description.contains(query))
            .collect()
    }

    pub fn remove(&mut self, name: &str, version: &str) -> bool {
        let result = if let Some(versions) = self.models.get_mut(name) {
            let len = versions.len();
            versions.retain(|m| m.version != version);
            let changed = versions.len() < len;
            let empty = versions.is_empty();
            Some((changed, empty))
        } else {
            None
        };
        match result {
            Some((changed, true)) => {
                self.models.remove(name);
                changed
            }
            Some((changed, false)) => changed,
            None => false,
        }
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct DownloadManager {
    cache_dir: std::path::PathBuf,
}

impl DownloadManager {
    pub fn new(cache_dir: impl Into<std::path::PathBuf>) -> Self {
        Self { cache_dir: cache_dir.into() }
    }

    pub fn download(&self, url: &str) -> Result<Vec<u8>, String> {
        let filename = url.split('/').last().unwrap_or("model.bin");
        let path = self.cache_dir.join(filename);
        if path.exists() {
            std::fs::read(&path).map_err(|e| e.to_string())
        } else {
            Err("Download not implemented - provide local files".to_string())
        }
    }

    pub fn verify_integrity(&self, data: &[u8], expected_checksum: &str) -> bool {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = format!("{:x}", hasher.finalize());
        result == expected_checksum
    }
}
