use std::collections::HashSet;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use neo_memory::api::{SearchRequest, StoreRequest};
use neo_memory::context_builder::ContextBuilderConfig;
use neo_memory::manager::{CognitiveMemoryManager, UnifiedMemoryConfig};
use neo_memory::persistence::PersistenceConfig;
use neo_memory::types::{MemoryId, MemoryPriority, MemoryTier};

fn bench_config() -> (tempfile::TempDir, UnifiedMemoryConfig) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir
        .path()
        .join(format!("sled-bench-{}", uuid::Uuid::new_v4()))
        .to_str()
        .unwrap()
        .to_string();
    let config = UnifiedMemoryConfig {
        persistence: PersistenceConfig {
            path,
            ..PersistenceConfig::default()
        },
        ..UnifiedMemoryConfig::default()
    };
    (dir, config)
}

fn bench_store_1000(c: &mut Criterion) {
    c.bench_function("store_1000_entries", |b| {
        b.iter_batched(
            || {
                let (_dir, config) = bench_config();
                let manager = CognitiveMemoryManager::new(config).unwrap();
                (manager, 0u32)
            },
            |(manager, _)| {
                for i in 0..1000 {
                    manager
                        .store(StoreRequest {
                            tier: MemoryTier::LongTerm,
                            content: serde_json::json!({
                                "id": i,
                                "text": format!("Memory entry number {i} with some content for testing"),
                            }),
                            tags: vec!["benchmark".to_string(), "store".to_string()],
                            importance: Some((i as f32 % 100.0) / 100.0),
                            priority: None,
                            namespace: None,
                            ttl_secs: None,
                            source: Some("benchmark".to_string()),
                        })
                        .unwrap();
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_search_1000(c: &mut Criterion) {
    c.bench_function("search_1000_entries", |b| {
        b.iter_batched(
            || {
                let (_dir, config) = bench_config();
                let manager = CognitiveMemoryManager::new(config).unwrap();
                for i in 0..1000 {
                    manager
                        .store(StoreRequest {
                            tier: MemoryTier::LongTerm,
                            content: serde_json::json!({
                                "id": i,
                                "text": format!("Searchable memory about topic {i} with unique keywords"),
                            }),
                            tags: vec![format!("tag_{}", i % 10)],
                            importance: Some(0.5),
                            priority: None,
                            namespace: None,
                            ttl_secs: None,
                            source: None,
                        })
                        .unwrap();
                }
                manager
            },
            |manager| {
                let _results = manager
                    .search(black_box(SearchRequest {
                        query: Some("topic".to_string()),
                        limit: Some(10),
                        ..SearchRequest::default()
                    }))
                    .unwrap();
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_recall_100(c: &mut Criterion) {
    c.bench_function("recall_100_by_id", |b| {
        b.iter_batched(
            || {
                let (_dir, config) = bench_config();
                let manager = CognitiveMemoryManager::new(config).unwrap();
                let mut ids = Vec::new();
                for i in 0..1000 {
                    let resp = manager
                        .store(StoreRequest {
                            tier: MemoryTier::LongTerm,
                            content: serde_json::json!({"data": i}),
                            tags: vec![],
                            importance: Some(0.5),
                            priority: None,
                            namespace: None,
                            ttl_secs: None,
                            source: None,
                        })
                        .unwrap();
                    ids.push(MemoryId::from(uuid::Uuid::parse_str(&resp.id).unwrap()));
                }
                (manager, ids)
            },
            |(manager, ids)| {
                for id in &ids[0..100] {
                    black_box(manager.recall(*id));
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_working_memory_throughput(c: &mut Criterion) {
    c.bench_function("working_memory_push_pop_10000", |b| {
        b.iter(|| {
            let config = UnifiedMemoryConfig {
                working: neo_memory::working::WorkingMemoryConfig {
                    max_capacity: 10000,
                    attention_slots: 5,
                    default_expiration_secs: 1800,
                    priority_eviction: true,
                    max_tokens: 81920,
                },
                ..Default::default()
            };
            let manager = CognitiveMemoryManager::new(config).unwrap();
            let wm = manager.working_memory();
            for i in 0..10000 {
                let mut entry = neo_memory::types::MemoryEntry::new(
                    MemoryTier::Working,
                    serde_json::json!({"i": i}),
                    HashSet::new(),
                );
                entry.estimated_tokens = 10;
                black_box(wm.push(entry));
            }
            for _ in 0..10000 {
                black_box(wm.pop());
            }
        });
    });
}

fn bench_episodic_store_1000(c: &mut Criterion) {
    c.bench_function("episodic_store_1000", |b| {
        b.iter_batched(
            || {
                let (_dir, config) = bench_config();
                let manager = CognitiveMemoryManager::new(config).unwrap();
                (manager, 0u32)
            },
            |(manager, _)| {
                for i in 0..1000 {
                    let entry = neo_memory::types::MemoryEntry::new(
                        MemoryTier::Episodic,
                        serde_json::json!({"event": i, "description": format!("Episode {i}")}),
                        HashSet::from(["benchmark".to_string()]),
                    );
                    let episode = neo_memory::episodic::Episode::new(
                        entry.id,
                        format!("Benchmark episode {i}"),
                    );
                    manager.store_episode(
                        serde_json::json!({"event": i}),
                        episode,
                        HashSet::from(["bench".to_string()]),
                    ).unwrap();
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_context_building_500(c: &mut Criterion) {
    c.bench_function("context_building_500_entries", |b| {
        b.iter_batched(
            || {
                let (_dir, config) = bench_config();
                let manager = CognitiveMemoryManager::new(config).unwrap();
                for i in 0..500 {
                    manager
                        .store(StoreRequest {
                            tier: MemoryTier::LongTerm,
                            content: serde_json::json!({
                                "id": i,
                                "topic": "machine learning",
                                "detail": format!("Neural network training step {i} with gradient descent"),
                            }),
                            tags: vec!["ml".to_string()],
                            importance: Some((i as f32 % 100.0) / 100.0),
                            priority: None,
                            namespace: None,
                            ttl_secs: None,
                            source: None,
                        })
                        .unwrap();
                }
                manager
            },
            |manager| {
                black_box(manager.build_context(None, 4096));
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_consolidation_200(c: &mut Criterion) {
    c.bench_function("consolidation_200_entries", |b| {
        b.iter_batched(
            || {
                let (_dir, config) = bench_config();
                let manager = CognitiveMemoryManager::new(config).unwrap();
                for i in 0..200 {
                    manager
                        .store(StoreRequest {
                            tier: MemoryTier::LongTerm,
                            content: serde_json::json!({
                                "id": i,
                                "content": format!("Consolidation test entry {i} with similar content for dedup testing"),
                            }),
                            tags: vec![format!("group_{}", i % 5)],
                            importance: Some((i as f32 % 100.0) / 100.0),
                            priority: None,
                            namespace: None,
                            ttl_secs: None,
                            source: None,
                        })
                        .unwrap();
                }
                manager
            },
            |manager| {
                black_box(manager.consolidate().unwrap());
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_decay_1000(c: &mut Criterion) {
    c.bench_function("decay_1000_entries", |b| {
        b.iter_batched(
            || {
                let (_dir, config) = bench_config();
                let manager = CognitiveMemoryManager::new(config).unwrap();
                for i in 0..1000 {
                    manager
                        .store(StoreRequest {
                            tier: MemoryTier::LongTerm,
                            content: serde_json::json!({
                                "id": i,
                                "data": format!("Decay test entry {i}"),
                            }),
                            tags: vec![],
                            importance: Some((i as f32 % 100.0) / 100.0),
                            priority: None,
                            namespace: None,
                            ttl_secs: None,
                            source: None,
                        })
                        .unwrap();
                }
                manager
            },
            |manager| {
                black_box(manager.apply_decay().unwrap());
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

criterion_group!(
    benches,
    bench_store_1000,
    bench_search_1000,
    bench_recall_100,
    bench_working_memory_throughput,
    bench_episodic_store_1000,
    bench_context_building_500,
    bench_consolidation_200,
    bench_decay_1000,
);
criterion_main!(benches);
