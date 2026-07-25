use std::collections::HashMap;
use async_trait::async_trait;
use crate::error::{InferenceError, InferenceResult};
use crate::model::{ModelId, ModelMetadata, ModelFormat};
use crate::generation::StreamChunk;
use super::{InferenceBackend, BackendInfo, BackendConfig, InferenceInput, InferenceOutput, BackendType};

pub struct PluginBackend {
    info: BackendInfo,
    inner: Option<Box<dyn InferenceBackend>>,
    plugin_name: String,
    plugin_version: String,
}

impl PluginBackend {
    pub fn new() -> Self {
        Self {
            info: BackendInfo {
                backend_type: BackendType::Plugin,
                name: "Plugin Backend".to_string(),
                version: "1.0.0".to_string(),
                is_available: true,
                priority: 10,
                supported_formats: vec![],
                capabilities: vec!["inference".to_string()],
                max_model_size: None,
                metadata: HashMap::new(),
            },
            inner: None,
            plugin_name: String::new(),
            plugin_version: String::new(),
        }
    }

    pub fn with_inner(backend: Box<dyn InferenceBackend>, name: &str, version: &str) -> Self {
        let inner_info = backend.info();
        Self {
            info: BackendInfo {
                backend_type: BackendType::Plugin,
                name: format!("Plugin: {}", name),
                version: "1.0.0".to_string(),
                is_available: true,
                priority: 10,
                supported_formats: inner_info.supported_formats.clone(),
                capabilities: inner_info.capabilities.clone(),
                max_model_size: inner_info.max_model_size,
                metadata: {
                    let mut m = HashMap::new();
                    m.insert("plugin_name".to_string(), serde_json::Value::String(name.to_string()));
                    m.insert("plugin_version".to_string(), serde_json::Value::String(version.to_string()));
                    m.insert("inner_backend".to_string(), serde_json::Value::String(inner_info.backend_type.to_string()));
                    m
                },
            },
            inner: Some(backend),
            plugin_name: name.to_string(),
            plugin_version: version.to_string(),
        }
    }

    fn ensure_inner(&self) -> InferenceResult<&dyn InferenceBackend> {
        self.inner.as_ref().map(|b| b.as_ref()).ok_or_else(|| InferenceError::BackendNotAvailable {
            backend: format!("plugin:{}", self.plugin_name),
        })
    }

    fn ensure_inner_mut(&mut self) -> InferenceResult<&mut Box<dyn InferenceBackend>> {
        self.inner.as_mut().ok_or_else(|| InferenceError::BackendNotAvailable {
            backend: format!("plugin:{}", self.plugin_name),
        })
    }
}

impl std::fmt::Debug for PluginBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginBackend")
            .field("plugin_name", &self.plugin_name)
            .field("plugin_version", &self.plugin_version)
            .field("has_inner", &self.inner.is_some())
            .field("info", &self.info)
            .finish()
    }
}

impl Default for PluginBackend {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl InferenceBackend for PluginBackend {
    fn info(&self) -> BackendInfo { self.info.clone() }
    fn is_available(&self) -> bool { self.info.is_available && self.inner.is_some() }

    async fn initialize(&mut self, config: &BackendConfig) -> InferenceResult<()> {
        if let Some(inner) = self.inner.as_mut() {
            inner.initialize(config).await?;
        }
        tracing::info!(plugin = %self.plugin_name, version = %self.plugin_version, "Plugin backend initialized");
        Ok(())
    }

    async fn shutdown(&mut self) -> InferenceResult<()> {
        if let Some(inner) = self.inner.as_mut() {
            inner.shutdown().await?;
        }
        tracing::info!(plugin = %self.plugin_name, "Plugin backend shutdown");
        Ok(())
    }

    async fn load_model(&mut self, metadata: &ModelMetadata) -> InferenceResult<ModelId> {
        let inner = self.ensure_inner_mut()?;
        inner.load_model(metadata).await
    }

    async fn unload_model(&mut self, model_id: ModelId) -> InferenceResult<()> {
        let inner = self.ensure_inner_mut()?;
        inner.unload_model(model_id).await
    }

    async fn inference(&self, model_id: ModelId, input: InferenceInput) -> InferenceResult<InferenceOutput> {
        let inner = self.ensure_inner()?;
        inner.inference(model_id, input).await
    }

    async fn inference_stream(
        &self, model_id: ModelId, input: InferenceInput,
    ) -> InferenceResult<tokio::sync::mpsc::Receiver<InferenceResult<StreamChunk>>> {
        let inner = self.ensure_inner()?;
        inner.inference_stream(model_id, input).await
    }

    fn loaded_models(&self) -> Vec<ModelId> {
        match self.inner.as_ref() {
            Some(inner) => inner.loaded_models(),
            None => vec![],
        }
    }

    fn model_memory_usage(&self, model_id: ModelId) -> Option<u64> {
        self.inner.as_ref().and_then(|inner| inner.model_memory_usage(model_id))
    }

    fn supported_formats(&self) -> Vec<ModelFormat> {
        match self.inner.as_ref() {
            Some(inner) => inner.supported_formats(),
            None => vec![],
        }
    }
}
