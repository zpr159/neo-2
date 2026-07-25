use crate::error::InferenceResult;
use crate::memory::{MemoryOptimizer, KvCacheEntry, CacheStrategy};

pub struct MemoryEngine {
    optimizer: MemoryOptimizer,
}

impl MemoryEngine {
    pub fn new(kv_cache_size: u64) -> Self {
        Self {
            optimizer: MemoryOptimizer::new(kv_cache_size),
        }
    }

    pub fn allocate_buffer(&mut self, size: usize) -> Vec<u8> {
        self.optimizer.allocate_buffer(size)
    }

    pub fn release_buffer(&mut self, buffer: Vec<u8>) {
        self.optimizer.release_buffer(buffer)
    }

    pub fn store_kv_cache(&self, key: String, entries: Vec<KvCacheEntry>) {
        self.optimizer.cache_kv(key, entries);
    }

    pub fn get_kv_cache(&self, key: &str) -> Option<Vec<KvCacheEntry>> {
        self.optimizer.get_kv(key)
    }

    pub fn evict_cache(&self) {
        self.optimizer.evict_cache()
    }

    #[must_use]
    pub fn stats(&self) -> &crate::memory::MemoryPoolStats {
        self.optimizer.stats()
    }
}

impl Default for MemoryEngine {
    fn default() -> Self {
        Self::new(2 * 1024 * 1024 * 1024)
    }
}
