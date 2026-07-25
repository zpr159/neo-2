use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use super::config::{LanguageEngineConfig, ProviderConfig, ProviderType};
use super::engine::LanguageEngine;
use super::error::{LanguageError, LanguageResult};
use super::types::{ProviderHealth, ProviderMetrics};

/// Descriptor for a registered provider type.
#[derive(Debug, Clone)]
pub struct ProviderDescriptor {
    pub provider_type: ProviderType,
    pub display_name: String,
    pub description: String,
    pub default_endpoint: String,
    pub requires_api_key: bool,
}

/// Factory function type for creating provider instances.
pub type ProviderFactoryFn =
    Box<dyn Fn(&ProviderConfig) -> Arc<dyn LanguageEngine> + Send + Sync>;

/// Registry for provider types and instances.
pub struct ProviderRegistry {
    descriptors: RwLock<HashMap<ProviderType, ProviderDescriptor>>,
    factories: RwLock<HashMap<ProviderType, ProviderFactoryFn>>,
    instances: RwLock<HashMap<String, Arc<dyn LanguageEngine>>>,
    health_cache: RwLock<HashMap<String, ProviderHealth>>,
    metrics_cache: RwLock<HashMap<String, ProviderMetrics>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            descriptors: RwLock::new(HashMap::new()),
            factories: RwLock::new(HashMap::new()),
            instances: RwLock::new(HashMap::new()),
            health_cache: RwLock::new(HashMap::new()),
            metrics_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Register a provider type with its descriptor and factory.
    pub async fn register<F>(
        &self,
        descriptor: ProviderDescriptor,
        factory: F,
    ) where
        F: Fn(&ProviderConfig) -> Arc<dyn LanguageEngine> + Send + Sync + 'static,
    {
        let provider_type = descriptor.provider_type.clone();
        let mut descriptors = self.descriptors.write().await;
        let mut factories = self.factories.write().await;
        descriptors.insert(provider_type.clone(), descriptor);
        factories.insert(provider_type, Box::new(factory));
    }

    /// Create a provider instance from configuration.
    pub async fn create_provider(
        &self,
        config: &ProviderConfig,
    ) -> LanguageResult<Arc<dyn LanguageEngine>> {
        let factories = self.factories.read().await;
        let factory = factories
            .get(&config.provider_type)
            .ok_or_else(|| {
                LanguageError::ProviderNotFound(format!(
                    "no factory registered for {:?}",
                    config.provider_type
                ))
            })?;

        let instance = factory(config);
        let mut instances = self.instances.write().await;
        instances.insert(config.name.clone(), instance.clone());
        Ok(instance)
    }

    /// Get a provider instance by name.
    pub async fn get_provider(&self, name: &str) -> LanguageResult<Arc<dyn LanguageEngine>> {
        let instances = self.instances.read().await;
        instances
            .get(name)
            .cloned()
            .ok_or_else(|| LanguageError::ProviderNotFound(name.to_string()))
    }

    /// List all registered provider descriptors.
    pub async fn list_descriptors(&self) -> Vec<ProviderDescriptor> {
        let descriptors = self.descriptors.read().await;
        descriptors.values().cloned().collect()
    }

    /// Check if a provider type is registered.
    pub async fn is_registered(&self, provider_type: &ProviderType) -> bool {
        let descriptors = self.descriptors.read().await;
        descriptors.contains_key(provider_type)
    }

    /// Update health status for a provider.
    pub async fn update_health(&self, name: &str, health: ProviderHealth) {
        let mut cache = self.health_cache.write().await;
        cache.insert(name.to_string(), health);
    }

    /// Get cached health status for a provider.
    pub async fn get_health(&self, name: &str) -> Option<ProviderHealth> {
        let cache = self.health_cache.read().await;
        cache.get(name).cloned()
    }

    /// Update metrics for a provider.
    pub async fn update_metrics(&self, name: &str, metrics: ProviderMetrics) {
        let mut cache = self.metrics_cache.write().await;
        cache.insert(name.to_string(), metrics);
    }

    /// Get cached metrics for a provider.
    pub async fn get_metrics(&self, name: &str) -> Option<ProviderMetrics> {
        let cache = self.metrics_cache.read().await;
        cache.get(name).cloned()
    }

    /// Remove a provider instance.
    pub async fn remove_provider(&self, name: &str) {
        let mut instances = self.instances.write().await;
        instances.remove(name);
        let mut health = self.health_cache.write().await;
        health.remove(name);
        let mut metrics = self.metrics_cache.write().await;
        metrics.remove(name);
    }

    /// Initialize providers from configuration.
    pub async fn initialize_from_config(
        &self,
        config: &LanguageEngineConfig,
    ) -> LanguageResult<Vec<Arc<dyn LanguageEngine>>> {
        let mut providers = Vec::new();
        for provider_config in &config.providers {
            if provider_config.enabled {
                let provider = self.create_provider(provider_config).await?;
                providers.push(provider);
            }
        }
        Ok(providers)
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
