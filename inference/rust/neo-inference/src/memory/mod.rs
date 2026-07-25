use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CacheStrategy {
    Lru,
    Lfu,
    Fifo,
    Random,
    PriorityBased,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvCacheEntry {
    pub layer: u32,
    pub head: u32,
    pub sequence_length: usize,
    pub key_data: Vec<u8>,
    pub value_data: Vec<u8>,
    pub dtype: String,
    pub size_bytes: u64,
    pub access_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPoolStats {
    pub total_allocated: u64,
    pub total_used: u64,
    pub total_free: u64,
    pub fragmentation: f64,
    pub allocation_count: usize,
    pub peak_usage: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinnedMemoryBlock {
    pub ptr: u64,
    pub size: u64,
    pub device_id: u32,
    pub is_pinned: bool,
}

pub struct KvCacheManager {
    cache: HashMap<String, Vec<KvCacheEntry>>,
    strategy: CacheStrategy,
    max_size_bytes: u64,
    current_size: AtomicU64,
    access_counter: AtomicU64,
}

impl std::fmt::Debug for KvCacheManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KvCacheManager")
            .field("strategy", &self.strategy)
            .field("max_size_bytes", &self.max_size_bytes)
            .field("current_size", &self.current_size.load(Ordering::Relaxed))
            .finish()
    }
}

impl KvCacheManager {
    pub fn new(strategy: CacheStrategy, max_size_bytes: u64) -> Self {
        Self {
            cache: HashMap::new(),
            strategy,
            max_size_bytes,
            current_size: AtomicU64::new(0),
            access_counter: AtomicU64::new(0),
        }
    }

    pub fn insert(&mut self, key: String, entries: Vec<KvCacheEntry>) {
        let entry_size: u64 = entries.iter().map(|e| e.size_bytes).sum();
        while self.current_size.load(Ordering::SeqCst) + entry_size > self.max_size_bytes
            && !self.cache.is_empty()
        {
            self.evict_one();
        }
        self.current_size.fetch_add(entry_size, Ordering::SeqCst);
        self.cache.insert(key, entries);
    }

    pub fn get(&mut self, key: &str) -> Option<&Vec<KvCacheEntry>> {
        if self.cache.contains_key(key) {
            self.access_counter.fetch_add(1, Ordering::SeqCst);
            Some(self.cache.get(key)?)
        } else {
            None
        }
    }

    pub fn remove(&mut self, key: &str) -> Option<Vec<KvCacheEntry>> {
        let entries = self.cache.remove(key)?;
        let entry_size: u64 = entries.iter().map(|e| e.size_bytes).sum();
        self.current_size.fetch_sub(entry_size, Ordering::SeqCst);
        Some(entries)
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.current_size.store(0, Ordering::SeqCst);
    }

    fn evict_one(&mut self) {
        match self.strategy {
            CacheStrategy::Lru | CacheStrategy::Lfu | CacheStrategy::Fifo | CacheStrategy::Random | CacheStrategy::PriorityBased => {
                if let Some(key) = self.cache.keys().next().cloned() {
                    if let Some(entries) = self.cache.remove(&key) {
                        let entry_size: u64 = entries.iter().map(|e| e.size_bytes).sum();
                        self.current_size.fetch_sub(entry_size, Ordering::SeqCst);
                    }
                }
            }
        }
    }

    #[must_use]
    pub fn current_size(&self) -> u64 {
        self.current_size.load(Ordering::SeqCst)
    }

    #[must_use]
    pub fn max_size(&self) -> u64 {
        self.max_size_bytes
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    #[must_use]
    pub fn usage_fraction(&self) -> f64 {
        if self.max_size_bytes == 0 {
            return 0.0;
        }
        self.current_size.load(Ordering::SeqCst) as f64 / self.max_size_bytes as f64
    }
}

pub struct MemoryOptimizer {
    kv_cache: parking_lot::RwLock<KvCacheManager>,
    pinned_blocks: parking_lot::RwLock<Vec<PinnedMemoryBlock>>,
    reuse_pool: parking_lot::RwLock<Vec<Vec<u8>>>,
    stats: MemoryPoolStats,
}

impl std::fmt::Debug for MemoryOptimizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryOptimizer")
            .field("stats", &self.stats)
            .finish()
    }
}

impl MemoryOptimizer {
    pub fn new(kv_max_bytes: u64) -> Self {
        Self {
            kv_cache: parking_lot::RwLock::new(KvCacheManager::new(CacheStrategy::Lru, kv_max_bytes)),
            pinned_blocks: parking_lot::RwLock::new(Vec::new()),
            reuse_pool: parking_lot::RwLock::new(Vec::new()),
            stats: MemoryPoolStats {
                total_allocated: 0,
                total_used: 0,
                total_free: 0,
                fragmentation: 0.0,
                allocation_count: 0,
                peak_usage: 0,
            },
        }
    }

    pub fn allocate_buffer(&mut self, size: usize) -> Vec<u8> {
        {
            let mut pool = self.reuse_pool.write();
            if let Some(pos) = pool.iter().position(|b| b.len() >= size) {
                let buf = pool.swap_remove(pos);
                self.stats.total_used += buf.len() as u64;
                return buf;
            }
        }
        self.stats.total_allocated += size as u64;
        self.stats.total_used += size as u64;
        self.stats.allocation_count += 1;
        if self.stats.total_used > self.stats.peak_usage {
            self.stats.peak_usage = self.stats.total_used;
        }
        vec![0u8; size]
    }

    pub fn release_buffer(&mut self, buffer: Vec<u8>) {
        self.stats.total_used = self.stats.total_used.saturating_sub(buffer.len() as u64);
        self.stats.total_free += buffer.len() as u64;
        self.reuse_pool.write().push(buffer);
    }

    pub fn cache_kv(&self, key: String, entries: Vec<KvCacheEntry>) {
        self.kv_cache.write().insert(key, entries);
    }

    pub fn get_kv(&self, key: &str) -> Option<Vec<KvCacheEntry>> {
        let mut cache = self.kv_cache.write();
        cache.get(key).cloned()
    }

    #[must_use]
    pub fn stats(&self) -> &MemoryPoolStats {
        &self.stats
    }

    pub fn evict_cache(&self) {
        self.kv_cache.write().clear();
    }
}

pub mod engine;
