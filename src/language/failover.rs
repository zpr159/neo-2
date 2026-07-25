use std::sync::Arc;

use tokio::sync::RwLock;

use super::config::LanguageEngineConfig;
use super::engine::LanguageEngine;
use super::error::{LanguageError, LanguageResult};
use super::types::{ProviderHealth, ProviderMetrics};

/// Tracks failure state for a provider.
#[derive(Debug, Clone)]
struct ProviderFailureState {
    consecutive_failures: u32,
    last_failure_at: Option<chrono::DateTime<chrono::Utc>>,
    total_failures: u64,
    is_healthy: bool,
}

impl Default for ProviderFailureState {
    fn default() -> Self {
        Self {
            consecutive_failures: 0,
            last_failure_at: None,
            total_failures: 0,
            is_healthy: true,
        }
    }
}

/// Manages provider failover with automatic health tracking.
pub struct FailoverManager {
    providers: Vec<(String, Arc<dyn LanguageEngine>)>,
    failure_states: RwLock<std::collections::HashMap<String, ProviderFailureState>>,
    config: LanguageEngineConfig,
    active_index: RwLock<usize>,
}

impl FailoverManager {
    pub fn new(
        providers: Vec<(String, Arc<dyn LanguageEngine>)>,
        config: LanguageEngineConfig,
    ) -> Self {
        Self {
            providers,
            failure_states: RwLock::new(std::collections::HashMap::new()),
            config,
            active_index: RwLock::new(0),
        }
    }

    /// Get the currently active provider.
    pub async fn active_provider(&self) -> LanguageResult<Arc<dyn LanguageEngine>> {
        let index = *self.active_index.read().await;
        if index < self.providers.len() {
            Ok(self.providers[index].1.clone())
        } else {
            Err(LanguageError::ProviderUnavailable(
                "no providers available".to_string(),
            ))
        }
    }

    /// Get the name of the currently active provider.
    pub async fn active_provider_name(&self) -> LanguageResult<String> {
        let index = *self.active_index.read().await;
        if index < self.providers.len() {
            Ok(self.providers[index].0.clone())
        } else {
            Err(LanguageError::ProviderUnavailable(
                "no providers available".to_string(),
            ))
        }
    }

    /// Record a successful request to the current provider.
    pub async fn record_success(&self) {
        let index = *self.active_index.read().await;
        if index < self.providers.len() {
            let name = &self.providers[index].0;
            let mut states = self.failure_states.write().await;
            let state = states
                .entry(name.clone())
                .or_insert_with(ProviderFailureState::default);
            state.consecutive_failures = 0;
            state.is_healthy = true;
        }
    }

    /// Record a failure and potentially trigger failover.
    pub async fn record_failure(&self, error: &LanguageError) -> LanguageResult<()> {
        if !self.config.enable_failover {
            return Err(error.clone());
        }

        let index = *self.active_index.read().await;
        if index >= self.providers.len() {
            return Err(LanguageError::ProviderUnavailable(
                "no providers available".to_string(),
            ));
        }

        let name = self.providers[index].0.clone();
        let mut states = self.failure_states.write().await;
        let state = states
            .entry(name.clone())
            .or_insert_with(ProviderFailureState::default);
        state.consecutive_failures += 1;
        state.total_failures += 1;
        state.last_failure_at = Some(chrono::Utc::now());

        if error.is_fatal() || state.consecutive_failures >= self.config.retry_count {
            state.is_healthy = false;
            drop(states);
            self.failover().await?;
        }

        Ok(())
    }

    /// Fail over to the next available provider.
    pub async fn failover(&self) -> LanguageResult<()> {
        let mut active = self.active_index.write().await;
        let start_index = *active;
        let mut states = self.failure_states.write().await;

        loop {
            *active = (*active + 1) % self.providers.len();
            let name = &self.providers[*active].0;

            let state = states
                .entry(name.clone())
                .or_insert_with(ProviderFailureState::default);

            if state.is_healthy || state.consecutive_failures < self.config.retry_count {
                tracing::warn!(
                    "failing over from provider {} to {}",
                    self.providers[start_index].0,
                    name
                );
                return Ok(());
            }

            if *active == start_index {
                return Err(LanguageError::ProviderUnavailable(
                    "all providers exhausted".to_string(),
                ));
            }
        }
    }

    /// Check health of all providers and update failover state.
    pub async fn check_health(&self) -> LanguageResult<Vec<(String, ProviderHealth)>> {
        let mut results = Vec::new();
        let mut states = self.failure_states.write().await;

        for (name, provider) in &self.providers {
            let health = match provider.health_check().await {
                Ok(h) => {
                    let state = states
                        .entry(name.clone())
                        .or_insert_with(ProviderFailureState::default);
                    state.is_healthy = h.healthy;
                    if h.healthy {
                        state.consecutive_failures = 0;
                    }
                    h
                }
                Err(e) => {
                    let state = states
                        .entry(name.clone())
                        .or_insert_with(ProviderFailureState::default);
                    state.is_healthy = false;
                    state.consecutive_failures += 1;
                    ProviderHealth::unhealthy(e.to_string())
                }
            };
            results.push((name.clone(), health));
        }

        Ok(results)
    }

    /// Get metrics from all providers.
    pub async fn collect_metrics(&self) -> LanguageResult<Vec<(String, ProviderMetrics)>> {
        let mut results = Vec::new();
        for (name, provider) in &self.providers {
            if let Ok(metrics) = provider.metrics().await {
                results.push((name.clone(), metrics));
            }
        }
        Ok(results)
    }

    /// Get the number of healthy providers.
    pub async fn healthy_count(&self) -> usize {
        let states = self.failure_states.read().await;
        states.values().filter(|s| s.is_healthy).count()
    }

    /// Check if failover is needed.
    pub async fn needs_failover(&self) -> bool {
        let index = *self.active_index.read().await;
        if index >= self.providers.len() {
            return true;
        }
        let name = &self.providers[index].0;
        let states = self.failure_states.read().await;
        states
            .get(name)
            .map(|s| !s.is_healthy)
            .unwrap_or(false)
    }

    /// Force failover to a specific provider by name.
    pub async fn force_provider(&self, name: &str) -> LanguageResult<()> {
        let index = self
            .providers
            .iter()
            .position(|(n, _)| n == name)
            .ok_or_else(|| LanguageError::ProviderNotFound(name.to_string()))?;

        let mut active = self.active_index.write().await;
        *active = index;

        let mut states = self.failure_states.write().await;
        if let Some(state) = states.get_mut(name) {
            state.is_healthy = true;
            state.consecutive_failures = 0;
        }

        Ok(())
    }
}
