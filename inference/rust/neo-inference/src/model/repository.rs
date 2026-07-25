use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use sha2::{Sha256, Digest};
use uuid::Uuid;

use crate::error::{InferenceError, InferenceResult};
use crate::model::{ModelId, ModelMetadata, ModelFormat, ModelVersion, QuantizationType, ModelArchitecture};

#[derive(Debug, Clone)]
pub struct RepositoryConfig {
    pub local_path: PathBuf,
    pub cache_path: PathBuf,
    pub remote_endpoints: Vec<String>,
    pub auto_update: bool,
    pub verify_integrity: bool,
    pub max_cache_size: u64,
    pub enable_rollback: bool,
}

impl Default for RepositoryConfig {
    fn default() -> Self {
        Self {
            local_path: PathBuf::from("./models"),
            cache_path: PathBuf::from("./models/.cache"),
            remote_endpoints: Vec::new(),
            auto_update: false,
            verify_integrity: true,
            max_cache_size: 50 * 1024 * 1024 * 1024,
            enable_rollback: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    pub metadata: ModelMetadata,
    pub local_path: PathBuf,
    pub cached: bool,
    pub verified: bool,
    pub last_verified: Option<DateTime<Utc>>,
    pub download_count: u64,
    pub previous_versions: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityCheck {
    pub path: PathBuf,
    pub expected_sha256: String,
    pub actual_sha256: String,
    pub verified: bool,
    pub checked_at: DateTime<Utc>,
}

use serde::{Deserialize, Serialize};

pub struct ModelRepository {
    config: RepositoryConfig,
    entries: RwLock<HashMap<ModelId, ModelEntry>>,
    name_index: RwLock<HashMap<String, ModelId>>,
    total_cache_size: AtomicU64,
}

impl std::fmt::Debug for ModelRepository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModelRepository")
            .field("entry_count", &self.entries.read().len())
            .field("cache_size", &self.total_cache_size.load(Ordering::Relaxed))
            .finish()
    }
}

impl ModelRepository {
    pub fn new(config: RepositoryConfig) -> Self {
        Self {
            config,
            entries: RwLock::new(HashMap::new()),
            name_index: RwLock::new(HashMap::new()),
            total_cache_size: AtomicU64::new(0),
        }
    }

    pub fn register_local(&self, metadata: ModelMetadata, local_path: PathBuf) -> InferenceResult<()> {
        let entry = ModelEntry {
            metadata: metadata.clone(),
            local_path: local_path.clone(),
            cached: false,
            verified: false,
            last_verified: None,
            download_count: 0,
            previous_versions: Vec::new(),
        };
        self.name_index.write().insert(metadata.name.clone(), metadata.id);
        self.entries.write().insert(metadata.id, entry);
        tracing::info!(model = %metadata.name, path = %local_path.display(), "Model registered in repository");
        Ok(())
    }

    pub fn unregister(&self, model_id: ModelId) -> InferenceResult<()> {
        let entry = self.entries.write().remove(&model_id)
            .ok_or_else(|| InferenceError::ModelNotFound { model_id: model_id.to_string() })?;
        self.name_index.write().remove(&entry.metadata.name);
        Ok(())
    }

    pub fn get(&self, model_id: ModelId) -> Option<ModelEntry> {
        self.entries.read().get(&model_id).cloned()
    }

    pub fn get_by_name(&self, name: &str) -> Option<ModelEntry> {
        let id = self.name_index.read().get(name).copied()?;
        self.entries.read().get(&id).cloned()
    }

    pub fn list(&self) -> Vec<ModelEntry> {
        self.entries.read().values().cloned().collect()
    }

    pub fn verify_integrity(&self, model_id: ModelId) -> InferenceResult<IntegrityCheck> {
        let entry = self.entries.read().get(&model_id)
            .ok_or_else(|| InferenceError::ModelNotFound { model_id: model_id.to_string() })?
            .clone();
        let expected = entry.metadata.sha256.clone()
            .ok_or_else(|| InferenceError::IntegrityFailed {
                path: entry.local_path.display().to_string(),
                expected: "none".to_string(),
                actual: "no hash provided".to_string(),
            })?;
        let data = std::fs::read(&entry.local_path)
            .map_err(|e| InferenceError::IntegrityFailed {
                path: entry.local_path.display().to_string(),
                expected: expected.clone(),
                actual: format!("read error: {}", e),
            })?;
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let actual = format!("{:x}", hasher.finalize());
        let verified = expected == actual;
        if !verified && self.config.verify_integrity {
            return Err(InferenceError::IntegrityFailed {
                path: entry.local_path.display().to_string(),
                expected,
                actual,
            });
        }
        Ok(IntegrityCheck {
            path: entry.local_path.clone(),
            expected_sha256: expected,
            actual_sha256: actual,
            verified,
            checked_at: Utc::now(),
        })
    }

    pub fn compute_sha256(&self, path: &Path) -> InferenceResult<String> {
        let data = std::fs::read(path).map_err(InferenceError::from)?;
        let mut hasher = Sha256::new();
        hasher.update(&data);
        Ok(format!("{:x}", hasher.finalize()))
    }

    pub fn rollback(&self, model_id: ModelId) -> InferenceResult<()> {
        if !self.config.enable_rollback {
            return Err(InferenceError::ModelUnloadFailed {
                model_id: model_id.to_string(),
                reason: "rollback not enabled".to_string(),
            });
        }
        let mut entries = self.entries.write();
        let entry = entries.get_mut(&model_id)
            .ok_or_else(|| InferenceError::ModelNotFound { model_id: model_id.to_string() })?;
        if let Some(previous_path) = entry.previous_versions.pop() {
            tracing::info!(model = %model_id, rollback_to = %previous_path.display(), "Model rolled back");
            entry.local_path = previous_path;
            Ok(())
        } else {
            Err(InferenceError::ModelUnloadFailed {
                model_id: model_id.to_string(),
                reason: "no previous versions available for rollback".to_string(),
            })
        }
    }

    pub fn cache_size(&self) -> u64 {
        self.total_cache_size.load(Ordering::Relaxed)
    }

    pub fn needs_eviction(&self) -> bool {
        self.total_cache_size.load(Ordering::Relaxed) > self.config.max_cache_size
    }

    pub fn evict_cache(&self) -> InferenceResult<()> {
        let mut cache_size = self.total_cache_size.load(Ordering::Relaxed);
        let mut entries = self.entries.write();
        let mut to_remove = Vec::new();
        for (id, entry) in entries.iter() {
            if cache_size <= self.config.max_cache_size {
                break;
            }
            if entry.cached {
                to_remove.push(*id);
            }
        }
        for id in to_remove {
            if let Some(entry) = entries.remove(&id) {
                let size = entry.metadata.estimated_memory_bytes();
                cache_size = cache_size.saturating_sub(size);
                self.total_cache_size.store(cache_size, Ordering::SeqCst);
                tracing::info!(model = %entry.metadata.name, "Model cache evicted");
            }
        }
        Ok(())
    }
}
