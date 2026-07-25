use neo_inference::memory::{
    CacheStrategy, KvCacheEntry, KvCacheManager, MemoryOptimizer,
};

fn make_entries(count: usize, size_each: u64) -> Vec<KvCacheEntry> {
    (0..count)
        .map(|i| KvCacheEntry {
            layer: i as u32,
            head: 0,
            sequence_length: 128,
            key_data: vec![0u8; size_each as usize / 2],
            value_data: vec![0u8; size_each as usize / 2],
            dtype: "f32".to_string(),
            size_bytes: size_each,
            access_count: 0,
        })
        .collect()
}

#[test]
fn test_allocate_buffer() {
    let mut optimizer = MemoryOptimizer::new(1024 * 1024);
    let buf = optimizer.allocate_buffer(1024);
    assert_eq!(buf.len(), 1024);
    assert_eq!(optimizer.stats().allocation_count, 1);
}

#[test]
fn test_release_buffer() {
    let mut optimizer = MemoryOptimizer::new(1024 * 1024);
    let buf = optimizer.allocate_buffer(1024);
    optimizer.release_buffer(buf);
    assert!(optimizer.stats().total_free > 0);
}

#[test]
fn test_buffer_reuse() {
    let mut optimizer = MemoryOptimizer::new(1024 * 1024);
    let buf1 = optimizer.allocate_buffer(1024);
    optimizer.release_buffer(buf1);
    let buf2 = optimizer.allocate_buffer(512);
    assert!(!buf2.is_empty());
}

#[test]
fn test_kv_cache_insert_and_get() {
    let mut optimizer = MemoryOptimizer::new(1024 * 1024);
    let entries = make_entries(2, 128);
    optimizer.cache_kv("layer-0".to_string(), entries);

    let got = optimizer.get_kv("layer-0");
    assert!(got.is_some());
    assert_eq!(got.unwrap().len(), 2);
}

#[test]
fn test_kv_cache_miss() {
    let mut optimizer = MemoryOptimizer::new(1024 * 1024);
    let got = optimizer.get_kv("nonexistent");
    assert!(got.is_none());
}

#[test]
fn test_cache_eviction() {
    let mut optimizer = MemoryOptimizer::new(1024);
    optimizer.cache_kv("a".to_string(), make_entries(1, 256));
    optimizer.cache_kv("b".to_string(), make_entries(1, 256));
    optimizer.cache_kv("c".to_string(), make_entries(1, 256));
    optimizer.cache_kv("d".to_string(), make_entries(1, 256));
    optimizer.evict_cache();
    assert!(optimizer.get_kv("a").is_none());
}

#[test]
fn test_usage_fraction_tracking() {
    let mut kv = KvCacheManager::new(CacheStrategy::Lru, 1000);
    assert_eq!(kv.usage_fraction(), 0.0);

    kv.insert("a".to_string(), make_entries(1, 200));
    let frac = kv.usage_fraction();
    assert!(frac > 0.0 && frac <= 1.0);
}

#[test]
fn test_kv_cache_manager_eviction() {
    let mut kv = KvCacheManager::new(CacheStrategy::Lru, 300);
    kv.insert("a".to_string(), make_entries(1, 100));
    kv.insert("b".to_string(), make_entries(1, 100));
    kv.insert("c".to_string(), make_entries(1, 100));
    kv.insert("d".to_string(), make_entries(1, 100));

    assert!(kv.len() <= 3);
}

#[test]
fn test_kv_cache_remove() {
    let mut kv = KvCacheManager::new(CacheStrategy::Lru, 1000);
    kv.insert("a".to_string(), make_entries(1, 100));
    let removed = kv.remove("a");
    assert!(removed.is_some());
    assert!(kv.is_empty());
}

#[test]
fn test_kv_cache_clear() {
    let mut kv = KvCacheManager::new(CacheStrategy::Lru, 1000);
    kv.insert("a".to_string(), make_entries(1, 100));
    kv.insert("b".to_string(), make_entries(1, 100));
    kv.clear();
    assert!(kv.is_empty());
    assert_eq!(kv.current_size(), 0);
}

#[test]
fn test_peak_usage_tracking() {
    let mut optimizer = MemoryOptimizer::new(1024 * 1024);
    let buf = optimizer.allocate_buffer(2048);
    assert!(optimizer.stats().peak_usage >= 2048);
    optimizer.release_buffer(buf);
    assert!(optimizer.stats().peak_usage >= 2048);
}

#[test]
fn test_stats_initial() {
    let optimizer = MemoryOptimizer::new(1024 * 1024);
    let stats = optimizer.stats();
    assert_eq!(stats.total_allocated, 0);
    assert_eq!(stats.allocation_count, 0);
}

#[test]
fn test_multiple_allocations() {
    let mut optimizer = MemoryOptimizer::new(1024 * 1024);
    let _b1 = optimizer.allocate_buffer(256);
    let _b2 = optimizer.allocate_buffer(512);
    let _b3 = optimizer.allocate_buffer(1024);
    assert_eq!(optimizer.stats().allocation_count, 3);
}
