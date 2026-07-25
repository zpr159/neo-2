use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use uuid::Uuid;

use crate::error::{InferenceError, InferenceResult};
use crate::model::{ModelId, ModelMetadata, ModelSlot, ModelVersion};

#[derive(Debug, Clone)]
pub struct ModelManagerConfig {
    pub max_loaded_models: usize,
    pub max_memory_bytes: u64,
    pub enable_reference_counting: bool,
    pub enable_hot_swap: bool,
    pub enable_versioning: bool,
}

impl Default for ModelManagerConfig {
    fn default() -> Self {
        Self {
            max_loaded_models: 20,
            max_memory_bytes: 16 * 1024 * 1024 * 1024,
            enable_reference_counting: true,
            enable_hot_swap: true,
            enable_versioning: true,
        }
    }
}

pub struct ModelManager {
    config: ModelManagerConfig,
    models: RwLock<HashMap<ModelId, ModelSlot>>,
    aliases: RwLock<HashMap<String, ModelId>>,
    versions: RwLock<HashMap<String, Vec<ModelVersion>>>,
    total_memory: AtomicU64,
    total_loaded: AtomicU64,
}

impl std::fmt::Debug for ModelManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModelManager")
            .field("model_count", &self.total_loaded.load(Ordering::Relaxed))
            .field("memory_used", &self.total_memory.load(Ordering::Relaxed))
            .finish()
    }
}

impl ModelManager {
    pub fn new(config: ModelManagerConfig) -> Self {
        Self {
            config,
            models: RwLock::new(HashMap::new()),
            aliases: RwLock::new(HashMap::new()),
            versions: RwLock::new(HashMap::new()),
            total_memory: AtomicU64::new(0),
            total_loaded: AtomicU64::new(0),
        }
    }

    pub fn register(&self, metadata: ModelMetadata) -> InferenceResult<()> {
        let mut models = self.models.write();
        if models.contains_key(&metadata.id) {
            return Err(InferenceError::ModelAlreadyLoaded {
                model_id: metadata.id.to_string(),
            });
        }
        let id = metadata.id;
        let name_key = metadata.name.clone();
        let version = metadata.version.clone();
        models.insert(id, ModelSlot::new(metadata));
        self.total_loaded.fetch_add(1, Ordering::SeqCst);
        let mut alias_map = self.aliases.write();
        alias_map.insert(name_key.clone(), id);
        let mut version_map = self.versions.write();
        version_map.entry(name_key).or_default().push(version);
        Ok(())
    }

    pub fn unregister(&self, model_id: ModelId) -> InferenceResult<()> {
        let slot = self.models.write().remove(&model_id)
            .ok_or_else(|| InferenceError::ModelNotFound { model_id: model_id.to_string() })?;
        if slot.ref_count() > 0 {
            return Err(InferenceError::ModelUnloadFailed {
                model_id: model_id.to_string(),
                reason: "model still has active references".to_string(),
            });
        }
        let mem = slot.memory_allocated();
        self.total_memory.fetch_sub(mem, Ordering::SeqCst);
        self.total_loaded.fetch_sub(1, Ordering::SeqCst);
        Ok(())
    }

    pub fn load(&self, model_id: ModelId) -> InferenceResult<()> {
        let models = self.models.read();
        let slot = models.get(&model_id)
            .ok_or_else(|| InferenceError::ModelNotFound { model_id: model_id.to_string() })?;
        slot.increment_ref();
        Ok(())
    }

    pub fn unload(&self, model_id: ModelId) -> InferenceResult<()> {
        let models = self.models.read();
        let slot = models.get(&model_id)
            .ok_or_else(|| InferenceError::ModelNotFound { model_id: model_id.to_string() })?;
        slot.decrement_ref();
        Ok(())
    }

    pub fn get_by_alias(&self, alias: &str) -> Option<ModelId> {
        self.aliases.read().get(alias).copied()
    }

    pub fn add_alias(&self, alias: String, model_id: ModelId) {
        self.aliases.write().insert(alias, model_id);
    }

    pub fn remove_alias(&self, alias: &str) -> bool {
        self.aliases.write().remove(alias).is_some()
    }

    pub fn get_metadata(&self, model_id: ModelId) -> Option<ModelMetadata> {
        self.models.read().get(&model_id).map(|s| s.metadata.clone())
    }

    pub fn list_models(&self) -> Vec<ModelMetadata> {
        self.models.read().values().map(|s| s.metadata.clone()).collect()
    }

    pub fn find_by_name(&self, name: &str) -> Vec<ModelMetadata> {
        self.models.read()
            .values()
            .filter(|s| s.metadata.name == name)
            .map(|s| s.metadata.clone())
            .collect()
    }

    pub fn find_by_version(&self, name: &str, version: &ModelVersion) -> Option<ModelMetadata> {
        self.models.read()
            .values()
            .filter(|s| s.metadata.name == name && s.metadata.version == *version)
            .map(|s| s.metadata.clone())
            .next()
    }

    pub fn hot_swap(&self, old_id: ModelId, new_metadata: ModelMetadata) -> InferenceResult<ModelId> {
        if !self.config.enable_hot_swap {
            return Err(InferenceError::HotSwapFailed {
                model_id: old_id.to_string(),
                reason: "hot swap not enabled".to_string(),
            });
        }
        let new_id = new_metadata.id;
        {
            let models = self.models.read();
            let old_slot = models.get(&old_id)
                .ok_or_else(|| InferenceError::ModelNotFound { model_id: old_id.to_string() })?;
            if old_slot.ref_count() > 0 {
                return Err(InferenceError::HotSwapFailed {
                    model_id: old_id.to_string(),
                    reason: "model has active references during hot swap".to_string(),
                });
            }
        }
        self.unregister(old_id)?;
        self.register(new_metadata)?;
        tracing::info!(old = %old_id, new = %new_id, "Model hot-swapped");
        Ok(new_id)
    }

    pub fn set_memory_allocated(&self, model_id: ModelId, bytes: u64) {
        if let Some(slot) = self.models.read().get(&model_id) {
            let old = slot.memory_allocated();
            slot.set_memory_allocated(bytes);
            self.total_memory.fetch_add(bytes, Ordering::SeqCst);
            self.total_memory.fetch_sub(old, Ordering::SeqCst);
        }
    }

    pub fn eviction_candidates(&self) -> Vec<ModelId> {
        self.models.read()
            .iter()
            .filter(|(_, slot)| slot.ref_count() == 0)
            .map(|(id, _)| *id)
            .collect()
    }

    #[must_use]
    pub fn total_memory_used(&self) -> u64 {
        self.total_memory.load(Ordering::SeqCst)
    }

    #[must_use]
    pub fn loaded_count(&self) -> usize {
        self.total_loaded.load(Ordering::SeqCst) as usize
    }

    #[must_use]
    pub fn can_load_more(&self) -> bool {
        self.loaded_count() < self.config.max_loaded_models
            && self.total_memory_used() < self.config.max_memory_bytes
    }
}
