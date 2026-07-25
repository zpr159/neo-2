use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use tokio::sync::RwLock;

use super::config::{LoadBalancingPolicy, ProviderConfig};
use super::engine::LanguageEngine;
use super::error::{LanguageError, LanguageResult};

/// Entry for load balancing.
struct ProviderEntry {
    name: String,
    config: ProviderConfig,
    engine: Arc<dyn LanguageEngine>,
    active_requests: AtomicUsize,
    total_requests: AtomicUsize,
    total_latency_ms: AtomicUsize,
}

/// Load balancer for distributing requests across providers.
pub struct LoadBalancer {
    providers: Vec<ProviderEntry>,
    policy: LoadBalancingPolicy,
    round_robin_index: AtomicUsize,
    sticky_sessions: RwLock<HashMap<String, usize>>,
}

impl LoadBalancer {
    pub fn new(
        providers: Vec<(ProviderConfig, Arc<dyn LanguageEngine>)>,
        policy: LoadBalancingPolicy,
    ) -> Self {
        let entries: Vec<ProviderEntry> = providers
            .into_iter()
            .map(|(config, engine)| ProviderEntry {
                name: config.name.clone(),
                config,
                engine,
                active_requests: AtomicUsize::new(0),
                total_requests: AtomicUsize::new(0),
                total_latency_ms: AtomicUsize::new(0),
            })
            .collect();

        Self {
            providers: entries,
            policy,
            round_robin_index: AtomicUsize::new(0),
            sticky_sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Select a provider based on the configured policy.
    pub async fn select(&self, session_id: Option<&str>) -> LanguageResult<Arc<dyn LanguageEngine>> {
        if self.providers.is_empty() {
            return Err(LanguageError::LoadBalancerExhausted(
                "no providers configured".to_string(),
            ));
        }

        match &self.policy {
            LoadBalancingPolicy::RoundRobin => self.select_round_robin(),
            LoadBalancingPolicy::LeastLoaded => self.select_least_loaded(),
            LoadBalancingPolicy::LatencyOptimized => self.select_latency_optimized(),
            LoadBalancingPolicy::Priority => self.select_priority(),
            LoadBalancingPolicy::Weighted => self.select_weighted(),
            LoadBalancingPolicy::StickySession => {
                self.select_sticky_session(session_id.unwrap_or("default"))
                    .await
            }
        }
    }

    fn select_round_robin(&self) -> LanguageResult<Arc<dyn LanguageEngine>> {
        let index = self.round_robin_index.fetch_add(1, Ordering::Relaxed) % self.providers.len();
        Ok(self.providers[index].engine.clone())
    }

    fn select_least_loaded(&self) -> LanguageResult<Arc<dyn LanguageEngine>> {
        self.providers
            .iter()
            .min_by_key(|p| p.active_requests.load(Ordering::Relaxed))
            .map(|p| p.engine.clone())
            .ok_or_else(|| {
                LanguageError::LoadBalancerExhausted("no providers available".to_string())
            })
    }

    fn select_latency_optimized(&self) -> LanguageResult<Arc<dyn LanguageEngine>> {
        self.providers
            .iter()
            .min_by_key(|p| {
                let total = p.total_requests.load(Ordering::Relaxed);
                if total == 0 {
                    0
                } else {
                    p.total_latency_ms.load(Ordering::Relaxed) / total
                }
            })
            .map(|p| p.engine.clone())
            .ok_or_else(|| {
                LanguageError::LoadBalancerExhausted("no providers available".to_string())
            })
    }

    fn select_priority(&self) -> LanguageResult<Arc<dyn LanguageEngine>> {
        self.providers
            .iter()
            .max_by_key(|p| p.config.priority)
            .map(|p| p.engine.clone())
            .ok_or_else(|| {
                LanguageError::LoadBalancerExhausted("no providers available".to_string())
            })
    }

    fn select_weighted(&self) -> LanguageResult<Arc<dyn LanguageEngine>> {
        let total_weight: f64 = self.providers.iter().map(|p| p.config.weight).sum();
        if total_weight <= 0.0 {
            return self.select_round_robin();
        }

        let mut rng_state: u64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        rng_state ^= rng_state << 13;
        rng_state ^= rng_state >> 7;
        rng_state ^= rng_state << 17;
        let random = (rng_state as f64 / u64::MAX as f64) * total_weight;

        let mut cumulative = 0.0;
        for provider in &self.providers {
            cumulative += provider.config.weight;
            if random <= cumulative {
                return Ok(provider.engine.clone());
            }
        }

        Ok(self.providers.last().unwrap().engine.clone())
    }

    async fn select_sticky_session(&self, session_id: &str) -> LanguageResult<Arc<dyn LanguageEngine>> {
        let mut sessions = self.sticky_sessions.write().await;

        if let Some(&index) = sessions.get(session_id) {
            if index < self.providers.len() {
                return Ok(self.providers[index].engine.clone());
            }
        }

        let index = self.round_robin_index.fetch_add(1, Ordering::Relaxed) % self.providers.len();
        sessions.insert(session_id.to_string(), index);
        Ok(self.providers[index].engine.clone())
    }

    /// Record request start for a provider.
    pub fn record_start(&self, provider_name: &str) {
        if let Some(p) = self.providers.iter().find(|p| p.name == provider_name) {
            p.active_requests.fetch_add(1, Ordering::Relaxed);
            p.total_requests.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record request completion for a provider.
    pub fn record_completion(&self, provider_name: &str, latency_ms: u64) {
        if let Some(p) = self.providers.iter().find(|p| p.name == provider_name) {
            p.active_requests.fetch_sub(1, Ordering::Relaxed);
            p.total_latency_ms
                .fetch_add(latency_ms as usize, Ordering::Relaxed);
        }
    }

    /// Get active request count for a provider.
    pub fn active_requests(&self, provider_name: &str) -> usize {
        self.providers
            .iter()
            .find(|p| p.name == provider_name)
            .map(|p| p.active_requests.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Get provider by name.
    pub fn get_provider(&self, name: &str) -> Option<Arc<dyn LanguageEngine>> {
        self.providers
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.engine.clone())
    }

    /// List all provider names.
    pub fn provider_names(&self) -> Vec<String> {
        self.providers.iter().map(|p| p.name.clone()).collect()
    }

    /// Remove a sticky session.
    pub async fn remove_session(&self, session_id: &str) {
        let mut sessions = self.sticky_sessions.write().await;
        sessions.remove(session_id);
    }
}
