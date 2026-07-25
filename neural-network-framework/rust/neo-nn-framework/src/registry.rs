use std::collections::HashMap;
use uuid::Uuid;

use neo_core::error::{NeoError, NeoResult};

use crate::model::ModelMetadata;

/// A registry for tracking known models and their metadata.
#[derive(Debug)]
pub struct ModelRegistry {
    models: HashMap<Uuid, ModelMetadata>,
}

impl ModelRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
        }
    }

    /// Registers a model and returns its assigned UUID.
    pub fn register(&mut self, metadata: ModelMetadata) -> NeoResult<Uuid> {
        let id = Uuid::new_v4();
        self.models.insert(id, metadata);
        Ok(id)
    }

    /// Retrieves metadata for a model by ID.
    pub fn get(&self, id: Uuid) -> Option<&ModelMetadata> {
        self.models.get(&id)
    }

    /// Lists all registered models.
    pub fn list(&self) -> Vec<&ModelMetadata> {
        self.models.values().collect()
    }

    /// Removes a model from the registry by ID.
    pub fn remove(&mut self, id: Uuid) -> NeoResult<()> {
        self.models
            .remove(&id)
            .ok_or_else(|| NeoError::NotFound(format!("Model {} not found in registry", id)))?;
        Ok(())
    }

    /// Searches models by name/description query (stub — returns empty).
    pub fn search(&self, _query: &str) -> Vec<&ModelMetadata> {
        // Stub: a real implementation would perform text search or fuzzy matching.
        Vec::new()
    }

    /// Returns the number of registered models.
    pub fn count(&self) -> usize {
        self.models.len()
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}
