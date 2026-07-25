use std::collections::HashMap;
use std::time::Duration;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::chain::InternalReasoningState;
use crate::reflection::ReflectionResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub id: Uuid,
    pub query_hash: u64,
    pub result: CachedReasoningResult,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub access_count: u64,
    pub last_accessed: DateTime<Utc>,
    pub size_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedReasoningResult {
    pub conclusion: String,
    pub confidence: f32,
    pub explanation: String,
    pub strategy_used: String,
    pub step_count: usize,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub hit_count: u64,
    pub miss_count: u64,
    pub expired_count: u64,
    pub evicted_count: u64,
    pub total_size_bytes: usize,
}

pub struct ReasoningCache {
    entries: RwLock<HashMap<u64, CacheEntry>>,
    stats: RwLock<CacheStats>,
    max_entries: usize,
    ttl: Duration,
}

impl std::fmt::Debug for ReasoningCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReasoningCache")
            .field("max_entries", &self.max_entries)
            .field("ttl", &self.ttl)
            .finish()
    }
}

impl ReasoningCache {
    pub fn new(max_entries: usize, ttl_secs: u64) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            stats: RwLock::new(CacheStats::default()),
            max_entries,
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    pub fn get(&self, query: &str) -> Option<CachedReasoningResult> {
        let hash = self.hash_query(query);
        let mut entries = self.entries.write();

        if let Some(entry) = entries.get_mut(&hash) {
            if Utc::now() > entry.expires_at {
                self.stats.write().expired_count += 1;
                entries.remove(&hash);
                return None;
            }

            entry.access_count += 1;
            entry.last_accessed = Utc::now();
            self.stats.write().hit_count += 1;
            Some(entry.result.clone())
        } else {
            self.stats.write().miss_count += 1;
            None
        }
    }

    pub fn store(
        &self,
        query: &str,
        result: CachedReasoningResult,
    ) {
        let hash = self.hash_query(query);
        let now = Utc::now();
        let size_bytes = serde_json::to_string(&result).map_or(0, |s| s.len());

        let entry = CacheEntry {
            id: Uuid::new_v4(),
            query_hash: hash,
            result,
            created_at: now,
            expires_at: now + chrono::Duration::from_std(self.ttl).unwrap_or_default(),
            access_count: 0,
            last_accessed: now,
            size_bytes,
        };

        let mut entries = self.entries.write();

        if entries.len() >= self.max_entries {
            self.evict_lru(&mut entries);
        }

        entries.insert(hash, entry);
    }

    pub fn store_from_state(
        &self,
        query: &str,
        state: &InternalReasoningState,
        reflection: Option<&ReflectionResult>,
    ) {
        if let Some(best) = state.best_chain() {
            let conclusion = best
                .get_conclusion()
                .map(|s| s.content.clone())
                .unwrap_or_default();

            let confidence = reflection
                .map(|r| {
                    (best.average_confidence() + r.confidence_adjustment).clamp(0.0, 1.0)
                })
                .unwrap_or_else(|| best.average_confidence());

            let explanation = reflection
                .map(|r| r.recommendations.join("; "))
                .unwrap_or_default();

            let cached = CachedReasoningResult {
                conclusion,
                confidence,
                explanation,
                strategy_used: best.strategy.to_string(),
                step_count: best.step_count(),
                metadata: HashMap::new(),
            };

            self.store(query, cached);
        }
    }

    pub fn invalidate(&self, query: &str) -> bool {
        let hash = self.hash_query(query);
        self.entries.write().remove(&hash).is_some()
    }

    pub fn clear(&self) {
        self.entries.write().clear();
    }

    pub fn cleanup_expired(&self) -> usize {
        let now = Utc::now();
        let mut entries = self.entries.write();
        let before = entries.len();
        entries.retain(|_, entry| now <= entry.expires_at);
        before - entries.len()
    }

    pub fn stats(&self) -> CacheStats {
        let mut stats = self.stats.read().clone();
        stats.total_entries = self.entries.read().len();
        stats.total_size_bytes = self
            .entries
            .read()
            .values()
            .map(|e| e.size_bytes)
            .sum();
        stats
    }

    fn hash_query(&self, query: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        query.hash(&mut hasher);
        hasher.finish()
    }

    fn evict_lru(&self, entries: &mut HashMap<u64, CacheEntry>) {
        if let Some((&oldest_key, _)) = entries
            .iter()
            .min_by_key(|(_, e)| e.last_accessed)
        {
            entries.remove(&oldest_key);
            self.stats.write().evicted_count += 1;
        }
    }

    pub fn entries_within_ttl(&self) -> Vec<CachedReasoningResult> {
        let now = Utc::now();
        self.entries
            .read()
            .values()
            .filter(|e| now <= e.expires_at)
            .map(|e| e.result.clone())
            .collect()
    }

    pub fn compress(&self) -> usize {
        let before = self.entries.read().len();
        let mut entries = self.entries.write();
        let now = Utc::now();

        entries.retain(|_, entry| {
            if now > entry.expires_at {
                return false;
            }
            if entry.access_count == 0 && Utc::now().signed_duration_since(entry.created_at) > chrono::Duration::seconds(300) {
                return false;
            }
            true
        });

        before - entries.len()
    }
}

impl Default for ReasoningCache {
    fn default() -> Self {
        Self::new(10_000, 3600)
    }
}
