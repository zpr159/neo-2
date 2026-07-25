use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use super::engine::LanguageEngine;
use super::error::LanguageResult;
use super::types::ModelInfo;

/// Lifecycle state of a model.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ModelState {
    Unloaded,
    Loading,
    Loaded,
    Warm,
    Sleeping,
    Error,
}

impl std::fmt::Display for ModelState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelState::Unloaded => write!(f, "unloaded"),
            ModelState::Loading => write!(f, "loading"),
            ModelState::Loaded => write!(f, "loaded"),
            ModelState::Warm => write!(f, "warm"),
            ModelState::Sleeping => write!(f, "sleeping"),
            ModelState::Error => write!(f, "error"),
        }
    }
}

/// Tracks the state and metadata of a model.
#[derive(Debug, Clone)]
pub struct ModelEntry {
    pub info: ModelInfo,
    pub state: ModelState,
    pub provider_name: String,
    pub loaded_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
    pub reference_count: u64,
}

/// Manages model discovery, lifecycle, and state tracking.
pub struct ModelManager {
    models: RwLock<HashMap<String, ModelEntry>>,
    provider: Arc<dyn LanguageEngine>,
}

impl ModelManager {
    pub fn new(provider: Arc<dyn LanguageEngine>) -> Self {
        Self {
            models: RwLock::new(HashMap::new()),
            provider,
        }
    }

    /// Discover available models from the provider.
    pub async fn discover_models(&self) -> LanguageResult<Vec<ModelInfo>> {
        let models = self.provider.list_models().await?;
        let mut entries = self.models.write().await;
        for model in &models {
            let name = model.name.clone();
            entries
                .entry(name)
                .or_insert_with(|| ModelEntry {
                    info: model.clone(),
                    state: ModelState::Unloaded,
                    provider_name: self.provider.name().to_string(),
                    loaded_at: None,
                    last_used_at: None,
                    reference_count: 0,
                });
        }
        Ok(models)
    }

    /// Load a model.
    pub async fn load_model(&self, model_name: &str) -> LanguageResult<()> {
        {
            let mut models = self.models.write().await;
            if let Some(entry) = models.get_mut(model_name) {
                if entry.state == ModelState::Loaded || entry.state == ModelState::Warm {
                    return Ok(());
                }
                entry.state = ModelState::Loading;
            }
        }

        match self.provider.load_model(model_name).await {
            Ok(()) => {
                let mut models = self.models.write().await;
                if let Some(entry) = models.get_mut(model_name) {
                    entry.state = ModelState::Loaded;
                    entry.loaded_at = Some(chrono::Utc::now());
                }
                Ok(())
            }
            Err(e) => {
                let mut models = self.models.write().await;
                if let Some(entry) = models.get_mut(model_name) {
                    entry.state = ModelState::Error;
                }
                Err(e)
            }
        }
    }

    /// Unload a model.
    pub async fn unload_model(&self, model_name: &str) -> LanguageResult<()> {
        self.provider.unload_model(model_name).await?;
        let mut models = self.models.write().await;
        if let Some(entry) = models.get_mut(model_name) {
            entry.state = ModelState::Unloaded;
            entry.loaded_at = None;
        }
        Ok(())
    }

    /// Warm a loaded model (pre-compute KV cache, etc.).
    pub async fn warm_model(&self, model_name: &str) -> LanguageResult<()> {
        let mut models = self.models.write().await;
        if let Some(entry) = models.get_mut(model_name) {
            if entry.state == ModelState::Loaded {
                entry.state = ModelState::Warm;
            }
        }
        Ok(())
    }

    /// Put a model to sleep (release GPU memory, keep metadata).
    pub async fn sleep_model(&self, model_name: &str) -> LanguageResult<()> {
        self.provider.unload_model(model_name).await?;
        let mut models = self.models.write().await;
        if let Some(entry) = models.get_mut(model_name) {
            entry.state = ModelState::Sleeping;
        }
        Ok(())
    }

    /// Wake a sleeping model.
    pub async fn wake_model(&self, model_name: &str) -> LanguageResult<()> {
        self.load_model(model_name).await
    }

    /// Reload a model (unload then load).
    pub async fn reload_model(&self, model_name: &str) -> LanguageResult<()> {
        self.unload_model(model_name).await.ok();
        self.load_model(model_name).await
    }

    /// Get the state of a model.
    pub async fn model_state(&self, model_name: &str) -> ModelState {
        let models = self.models.read().await;
        models
            .get(model_name)
            .map(|e| e.state.clone())
            .unwrap_or(ModelState::Unloaded)
    }

    /// Get all discovered models and their states.
    pub async fn list_models(&self) -> Vec<ModelEntry> {
        let models = self.models.read().await;
        models.values().cloned().collect()
    }

    /// Check if a model is ready for inference.
    pub async fn is_ready(&self, model_name: &str) -> bool {
        let state = self.model_state(model_name).await;
        state == ModelState::Loaded || state == ModelState::Warm
    }

    /// Mark a model as recently used.
    pub async fn touch_model(&self, model_name: &str) {
        let mut models = self.models.write().await;
        if let Some(entry) = models.get_mut(model_name) {
            entry.last_used_at = Some(chrono::Utc::now());
            entry.reference_count += 1;
        }
    }

    /// Find the least recently used model for cleanup.
    pub async fn find_lru_model(&self) -> Option<String> {
        let models = self.models.read().await;
        models
            .iter()
            .filter(|(_, e)| e.state == ModelState::Loaded || e.state == ModelState::Warm)
            .min_by_key(|(_, e)| e.last_used_at)
            .map(|(name, _)| name.clone())
    }

    /// Cleanup memory by unloading least recently used models.
    pub async fn cleanup_memory(&self, keep_count: usize) -> LanguageResult<Vec<String>> {
        let loaded: Vec<String> = {
            let models = self.models.read().await;
            models
                .iter()
                .filter(|(_, e)| e.state == ModelState::Loaded || e.state == ModelState::Warm)
                .map(|(name, _)| name.clone())
                .collect()
        };

        let mut unloaded = Vec::new();
        if loaded.len() > keep_count {
            let to_unload = loaded.len() - keep_count;
            for model_name in loaded.iter().take(to_unload) {
                if self.unload_model(model_name).await.is_ok() {
                    unloaded.push(model_name.clone());
                }
            }
        }
        Ok(unloaded)
    }
}
