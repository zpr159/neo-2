use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

use neo_memory::api::*;
use neo_memory::manager::CognitiveMemoryManager;
use neo_memory::manager::UnifiedMemoryConfig;
use neo_memory::persistence::*;
use neo_memory::procedural::*;
use neo_memory::semantic::*;
use neo_memory::types::*;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn test_config() -> (tempfile::TempDir, UnifiedMemoryConfig) {
    let dir = tempfile::tempdir().unwrap();
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = dir
        .path()
        .join(format!("sled-{id}"))
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

fn store_simple(
    manager: &CognitiveMemoryManager,
    tier: MemoryTier,
    content: &str,
    tags: Vec<&str>,
    importance: f32,
) -> StoreResponse {
    manager
        .store(StoreRequest {
            tier,
            content: serde_json::json!(content),
            tags: tags.into_iter().map(String::from).collect(),
            importance: Some(importance),
            priority: None,
            namespace: None,
            ttl_secs: None,
            source: None,
        })
        .unwrap()
}

#[test]
fn full_lifecycle() {
    let (_dir, config) = test_config();
    let manager = CognitiveMemoryManager::new(config).unwrap();

    let r_working = store_simple(
        &manager,
        MemoryTier::Working,
        "working memory item",
        vec!["scratch"],
        0.3,
    );
    let _r_episodic = store_simple(
        &manager,
        MemoryTier::Episodic,
        "a meeting happened",
        vec!["event"],
        0.6,
    );
    let _r_semantic = store_simple(
        &manager,
        MemoryTier::Semantic,
        "water boils at 100C",
        vec!["fact"],
        0.7,
    );
    let _r_procedural = store_simple(
        &manager,
        MemoryTier::Procedural,
        "step 1: mix, step 2: bake",
        vec!["recipe"],
        0.5,
    );
    let r_longterm = store_simple(
        &manager,
        MemoryTier::LongTerm,
        "core principle: always be curious",
        vec!["principle"],
        0.9,
    );

    let id_working = MemoryId::from(uuid::Uuid::parse_str(&r_working.id).unwrap());
    let id_longterm = MemoryId::from(uuid::Uuid::parse_str(&r_longterm.id).unwrap());

    assert!(manager.recall(id_working).is_some(), "working recall");
    assert!(manager.recall(id_longterm).is_some(), "long-term recall");

    let resp = manager
        .search(SearchRequest {
            query: Some("meeting".to_string()),
            limit: Some(50),
            ..SearchRequest::default()
        })
        .unwrap();
    assert!(resp.total > 0, "search should find entries");

    manager
        .update(
            id_longterm,
            UpdateRequest {
                content: Some(serde_json::json!("updated principle")),
                tags: None,
                importance: None,
                priority: None,
            },
        )
        .unwrap();
    let recalled = manager.recall(id_longterm).unwrap();
    assert_eq!(recalled.content, serde_json::json!("updated principle"));

    let deleted = manager.delete(id_longterm).unwrap();
    assert!(deleted);
    assert!(manager.recall(id_longterm).is_none());

    let health = manager.health();
    assert_eq!(health.status, "healthy");
    assert!(health.total_memories > 0);

    let exported = manager
        .export(ExportRequest {
            tiers: None,
            namespace: None,
            format: ExportFormat::Json,
        })
        .unwrap();
    assert!(exported.count > 0);

    let imported = manager
        .import(ImportRequest {
            data: exported.data,
            format: ExportFormat::Json,
            namespace: None,
        })
        .unwrap();
    assert!(imported.count > 0);
}

#[test]
fn cross_tier_search() {
    let (_dir, config) = test_config();
    let manager = CognitiveMemoryManager::new(config).unwrap();

    store_simple(
        &manager,
        MemoryTier::Working,
        "quantum computing basics",
        vec!["physics"],
        0.5,
    );
    store_simple(
        &manager,
        MemoryTier::LongTerm,
        "quantum entanglement explained",
        vec!["physics"],
        0.8,
    );
    store_simple(
        &manager,
        MemoryTier::LongTerm,
        "classical mechanics overview",
        vec!["physics"],
        0.5,
    );

    let resp = manager
        .search(SearchRequest {
            query: Some("quantum".to_string()),
            limit: Some(50),
            ..SearchRequest::default()
        })
        .unwrap();

    assert!(
        resp.total >= 2,
        "search for 'quantum' should find >= 2 results, got {}",
        resp.total
    );

    // At least the top results should contain "quantum".
    let quantum_count = resp
        .results
        .iter()
        .filter(|r| r.content_preview.to_lowercase().contains("quantum"))
        .count();
    assert!(
        quantum_count >= 2,
        "at least 2 results should contain 'quantum', got {quantum_count}"
    );

    // Top result should be a quantum entry (highest keyword match).
    assert!(
        resp.results[0]
            .content_preview
            .to_lowercase()
            .contains("quantum"),
        "top result should contain 'quantum': {}",
        resp.results[0].content_preview
    );

    // Search for something that doesn't exist - with tag filter to narrow results.
    let resp_none = manager
        .search(SearchRequest {
            query: Some("nonexistent_xyz_abc".to_string()),
            tags: Some(vec!["nonexistent_tag_xyz".to_string()]),
            limit: Some(10),
            ..SearchRequest::default()
        })
        .unwrap();
    assert_eq!(
        resp_none.total, 0,
        "non-matching query with tag filter should return 0 results"
    );
}

#[test]
fn consolidation_pipeline() {
    let (_dir, config) = test_config();
    let manager = CognitiveMemoryManager::new(config).unwrap();

    for i in 0..20 {
        store_simple(
            &manager,
            MemoryTier::LongTerm,
            &format!("entry number {i}"),
            vec!["batch"],
            0.5,
        );
    }

    for _ in 0..5 {
        store_simple(
            &manager,
            MemoryTier::LongTerm,
            "duplicate content",
            vec!["dup"],
            0.3,
        );
    }

    // Consolidation ran without error.
    manager.consolidate().unwrap();

    let mut ids = Vec::new();
    for i in 0..5 {
        let r = store_simple(
            &manager,
            MemoryTier::LongTerm,
            &format!("summary source {i}"),
            vec!["summary"],
            0.6,
        );
        ids.push(r.id);
    }

    let summary = manager
        .summarize(SummarizeRequest {
            ids,
            max_length: Some(512),
        })
        .unwrap();
    assert_eq!(summary.entries_summarized, 5);
    assert!(!summary.summary.is_empty());
}

#[test]
fn decay_and_retrieval() {
    let (_dir, config) = test_config();
    let manager = CognitiveMemoryManager::new(config).unwrap();

    let r_high = store_simple(
        &manager,
        MemoryTier::LongTerm,
        "very important memory",
        vec!["critical"],
        0.95,
    );

    store_simple(
        &manager,
        MemoryTier::LongTerm,
        "unimportant note",
        vec!["trivial"],
        0.05,
    );

    let decay_record = manager.apply_decay().unwrap();
    assert!(
        decay_record.timestamp <= chrono::Utc::now(),
        "decay timestamp should be valid"
    );

    let id_high = MemoryId::from(uuid::Uuid::parse_str(&r_high.id).unwrap());
    assert!(
        manager.recall(id_high).is_some(),
        "high-importance entry should survive decay"
    );

    let analytics = manager.analytics();
    assert!(analytics.total_memories > 0);
    assert!(analytics.recall_rate >= 0.0);
}

#[test]
fn persistence_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let sled_path = dir
        .path()
        .join(format!("sled-{id}"))
        .to_str()
        .unwrap()
        .to_string();

    // Create a persistence layer and store entries directly.
    let persistence1 = MemoryPersistence::new(PersistenceConfig {
        backend: StorageBackend::Sled,
        path: sled_path.clone(),
        ..PersistenceConfig::default()
    })
    .unwrap();

    let mut tags1 = HashSet::new();
    tags1.insert("persist".to_string());
    let entry1 = MemoryEntry::new(
        MemoryTier::LongTerm,
        serde_json::json!("persisted memory 1"),
        tags1,
    )
    .with_importance(0.7);
    let id1 = entry1.id;
    persistence1.persist(&entry1).unwrap();

    let mut tags2 = HashSet::new();
    tags2.insert("persist".to_string());
    let entry2 = MemoryEntry::new(
        MemoryTier::LongTerm,
        serde_json::json!("persisted memory 2"),
        tags2,
    )
    .with_importance(0.4);
    let id2 = entry2.id;
    persistence1.persist(&entry2).unwrap();

    // Take backup to JSON for cross-backend verification.
    let backup_path = dir.path().join("backup.json");
    let backup_record = persistence1.backup(backup_path.to_str().unwrap()).unwrap();
    assert_eq!(backup_record.entry_count, 2);

    // Verify count.
    assert_eq!(persistence1.count().unwrap(), 2);

    // Drop everything.
    drop(persistence1);

    // Re-open from the same sled path.
    let persistence2 = MemoryPersistence::new(PersistenceConfig {
        backend: StorageBackend::Sled,
        path: sled_path.clone(),
        ..PersistenceConfig::default()
    })
    .unwrap();

    // Verify entries survived the round-trip.
    let loaded1 = persistence2.load(id1).unwrap();
    let loaded2 = persistence2.load(id2).unwrap();

    assert!(loaded1.is_some(), "entry 1 should be persisted in sled");
    assert!(loaded2.is_some(), "entry 2 should be persisted in sled");

    let e1 = loaded1.unwrap();
    let e2 = loaded2.unwrap();
    assert_eq!(e1.tier, MemoryTier::LongTerm);
    assert_eq!(e2.tier, MemoryTier::LongTerm);

    // Load all and verify count.
    let all = persistence2.load_all().unwrap();
    assert_eq!(all.len(), 2, "should have 2 entries in sled");

    // Restore from backup into a fresh sled path (different from persistence2's path).
    let restore_path = dir
        .path()
        .join(format!("sled-restore-{id}"))
        .to_str()
        .unwrap()
        .to_string();
    let persistence3 = MemoryPersistence::new(PersistenceConfig {
        backend: StorageBackend::Sled,
        path: restore_path,
        ..PersistenceConfig::default()
    })
    .unwrap();

    let restored_count = persistence3.restore(backup_path.to_str().unwrap()).unwrap();
    assert_eq!(restored_count, 2);
    assert_eq!(persistence3.count().unwrap(), 2);

    // Drop persistence2 so we can open the original path with the manager.
    drop(persistence2);

    // Also verify restore into a CognitiveMemoryManager works.
    let manager = CognitiveMemoryManager::new(UnifiedMemoryConfig {
        persistence: PersistenceConfig {
            path: sled_path,
            ..PersistenceConfig::default()
        },
        ..UnifiedMemoryConfig::default()
    })
    .unwrap();
    let health = manager.health();
    assert_eq!(health.status, "healthy");
}

#[test]
fn concurrent_access() {
    let dir = tempfile::tempdir().unwrap();
    let base_id = COUNTER.fetch_add(1, Ordering::SeqCst);

    let mut handles = Vec::new();

    // Each thread gets its own MemoryPersistence with a unique sled path.
    // sled is Send+Sync, so this is safe.
    for t in 0..4u32 {
        let path = dir
            .path()
            .join(format!("concurrent-sled-{base_id}-{t}"))
            .to_str()
            .unwrap()
            .to_string();
        handles.push(thread::spawn(move || {
            let persistence = MemoryPersistence::new(PersistenceConfig {
                backend: StorageBackend::Sled,
                path,
                ..PersistenceConfig::default()
            })
            .unwrap();

            // Each thread writes 10 entries.
            for i in 0..10 {
                let mut tags = HashSet::new();
                tags.insert(format!("thread{t}"));
                let entry = MemoryEntry::new(
                    MemoryTier::LongTerm,
                    serde_json::json!(format!("thread {t} item {i}")),
                    tags,
                );
                persistence.persist(&entry).unwrap();
            }

            // Each thread reads back its entries.
            let all = persistence.load_all().unwrap();
            assert_eq!(all.len(), 10, "thread {t} should see 10 entries");
        }));
    }

    for h in handles {
        h.join().expect("thread should not panic");
    }

    // Verify per-thread persistence is intact by reading each sled db.
    for t in 0..4u32 {
        let path = dir
            .path()
            .join(format!("concurrent-sled-{base_id}-{t}"))
            .to_str()
            .unwrap()
            .to_string();
        let persistence = MemoryPersistence::new(PersistenceConfig {
            backend: StorageBackend::Sled,
            path,
            ..PersistenceConfig::default()
        })
        .unwrap();
        let all = persistence.load_all().unwrap();
        assert_eq!(
            all.len(),
            10,
            "thread {t} sled should still have 10 entries"
        );
        for entry in &all {
            assert!(
                entry.content.to_string().contains(&format!("thread {t}")),
                "entry should belong to thread {t}"
            );
        }
    }
}

#[test]
fn context_building() {
    let (_dir, config) = test_config();
    let manager = CognitiveMemoryManager::new(config).unwrap();

    let mut entry = MemoryEntry::new(
        MemoryTier::LongTerm,
        serde_json::json!("high priority context"),
        HashSet::from(["high".to_string()]),
    );
    entry.importance = 0.95;
    entry.estimated_tokens = 50;
    manager.long_term_memory().store(entry).unwrap();

    let mut entry2 = MemoryEntry::new(
        MemoryTier::LongTerm,
        serde_json::json!("medium priority context"),
        HashSet::from(["med".to_string()]),
    );
    entry2.importance = 0.5;
    entry2.estimated_tokens = 100;
    manager.long_term_memory().store(entry2).unwrap();

    let mut entry3 = MemoryEntry::new(
        MemoryTier::LongTerm,
        serde_json::json!("low priority context"),
        HashSet::from(["low".to_string()]),
    );
    entry3.importance = 0.1;
    entry3.estimated_tokens = 200;
    manager.long_term_memory().store(entry3).unwrap();

    let ctx = manager.build_context(None, 120);

    assert!(
        ctx.total_tokens <= 120,
        "total tokens {} should be <= 120",
        ctx.total_tokens
    );
    assert!(!ctx.items.is_empty(), "context should have items");

    let first_score = ctx.items[0].score;
    for item in &ctx.items {
        assert!(
            item.score <= first_score + 0.001,
            "items should be sorted by score descending"
        );
    }

    assert!(
        ctx.dropped_count > 0 || ctx.total_tokens <= 120,
        "context should respect token budget"
    );
}

#[test]
fn security_and_namespaces() {
    let dir = tempfile::tempdir().unwrap();
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let sled_path = dir
        .path()
        .join(format!("sled-{id}"))
        .to_str()
        .unwrap()
        .to_string();

    let config = UnifiedMemoryConfig {
        persistence: PersistenceConfig {
            path: sled_path,
            ..PersistenceConfig::default()
        },
        security: SecurityConfig {
            enabled: true,
            encryption_key: Some("test-secret-key-12345".to_string()),
            ..SecurityConfig::default()
        },
        ..UnifiedMemoryConfig::default()
    };
    let manager = CognitiveMemoryManager::new(config).unwrap();

    manager
        .store(StoreRequest {
            tier: MemoryTier::LongTerm,
            content: serde_json::json!("project A secret data"),
            tags: vec!["secret".to_string()],
            importance: Some(0.9),
            priority: None,
            namespace: Some("project_a".to_string()),
            ttl_secs: None,
            source: None,
        })
        .unwrap();

    manager
        .store(StoreRequest {
            tier: MemoryTier::LongTerm,
            content: serde_json::json!("project B public data"),
            tags: vec!["public".to_string()],
            importance: Some(0.5),
            priority: None,
            namespace: Some("project_b".to_string()),
            ttl_secs: None,
            source: None,
        })
        .unwrap();

    let resp_a = manager
        .search(SearchRequest {
            query: None,
            namespace: Some("project_a".to_string()),
            limit: Some(50),
            ..SearchRequest::default()
        })
        .unwrap();

    for r in &resp_a.results {
        assert!(
            r.content_preview.contains("project A"),
            "namespace filter should isolate: {}",
            r.content_preview
        );
    }

    let resp_b = manager
        .search(SearchRequest {
            query: None,
            namespace: Some("project_b".to_string()),
            limit: Some(50),
            ..SearchRequest::default()
        })
        .unwrap();

    for r in &resp_b.results {
        assert!(
            r.content_preview.contains("project B"),
            "namespace filter should isolate: {}",
            r.content_preview
        );
    }

    let resp_all = manager
        .search(SearchRequest {
            query: None,
            limit: Some(50),
            ..SearchRequest::default()
        })
        .unwrap();
    assert!(
        resp_all.total >= 2,
        "unfiltered search should find both namespaces, got {}",
        resp_all.total
    );

    let security = manager.security();
    let ns_a = MemoryNamespace::new("project_a");

    assert!(
        !security.check_permission("unknown_user", &ns_a, MemoryPermission::Read),
        "unknown user should not have access"
    );

    security.grant_permission("alice", &ns_a, MemoryPermission::Write, None);
    assert!(security.check_permission("alice", &ns_a, MemoryPermission::Read));
    assert!(security.check_permission("alice", &ns_a, MemoryPermission::Write));
    assert!(!security.check_permission("alice", &ns_a, MemoryPermission::Admin));

    security.revoke_permission("alice", &ns_a);
    assert!(!security.check_permission("alice", &ns_a, MemoryPermission::Read));

    security.grant_global_permission("admin", MemoryPermission::Admin);
    let ns_b = MemoryNamespace::new("project_b");
    assert!(security.check_permission("admin", &ns_b, MemoryPermission::Admin));

    let plaintext = b"sensitive data";
    let encrypted = security.encrypt(plaintext);
    assert_ne!(encrypted, plaintext.to_vec(), "encrypted should differ");
    let decrypted = security.decrypt(&encrypted);
    assert_eq!(decrypted, plaintext.to_vec(), "decrypted should match original");
}

#[test]
fn merge_and_split_flow() {
    let (_dir, config) = test_config();
    let manager = CognitiveMemoryManager::new(config).unwrap();

    let r1 = store_simple(
        &manager,
        MemoryTier::LongTerm,
        "first memory about rust",
        vec!["rust", "lang"],
        0.3,
    );
    let r2 = store_simple(
        &manager,
        MemoryTier::LongTerm,
        "second memory about rust",
        vec!["rust", "code"],
        0.8,
    );

    let merge_resp = manager
        .merge(MergeRequest {
            ids: vec![r1.id.clone(), r2.id.clone()],
            strategy: MergeStrategy::HighestImportance,
        })
        .unwrap();

    assert_eq!(merge_resp.entries_merged, 2);

    let merged_id = MemoryId::from(uuid::Uuid::parse_str(&merge_resp.merged_id).unwrap());
    let merged = manager.recall(merged_id);
    assert!(merged.is_some(), "merged entry should be recallable");
    assert_eq!(merged.unwrap().importance, 0.8, "should keep highest importance");

    let id1 = MemoryId::from(uuid::Uuid::parse_str(&r1.id).unwrap());
    let id2 = MemoryId::from(uuid::Uuid::parse_str(&r2.id).unwrap());
    assert!(manager.recall(id1).is_none(), "original 1 should be deleted");
    assert!(manager.recall(id2).is_none(), "original 2 should be deleted");
}

#[test]
fn procedural_memory_workflow() {
    let (_dir, config) = test_config();
    let manager = CognitiveMemoryManager::new(config).unwrap();

    let mut proc = Procedure::new("Deploy App", "Deploy the application to production");
    proc.steps.push(ProcedureStep::new(1, "Run tests"));
    proc.steps.push(ProcedureStep::new(2, "Build release"));
    proc.steps.push(ProcedureStep::new(3, "Push to registry"));
    proc.tags = vec!["deploy".to_string(), "ci".to_string()];

    let proc_id = manager.store_procedure(proc).unwrap();

    let results = manager.search_procedures("Deploy");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].steps.len(), 3);

    let exec_record = ExecutionRecord {
        id: uuid::Uuid::new_v4(),
        procedure_id: proc_id,
        started_at: chrono::Utc::now(),
        completed_at: Some(chrono::Utc::now()),
        success: true,
        duration_ms: 5000.0,
        output: None,
        error: None,
        parameters: std::collections::HashMap::new(),
        step_results: Vec::new(),
    };
    manager.record_execution(exec_record).unwrap();

    let fact = SemanticFact::new("Neo", "is_a", serde_json::json!("AGI System"))
        .with_confidence(0.95);
    manager.add_fact(fact).unwrap();

    let fact2 = SemanticFact::new("Neo", "has_feature", serde_json::json!("cognitive memory"))
        .with_confidence(0.90);
    manager.add_fact(fact2).unwrap();

    let facts = manager.query_facts("Neo");
    assert_eq!(facts.len(), 2);
    assert!(facts.iter().all(|f| f.subject == "Neo"));
}
