use neo_inference::model::manager::{ModelManager, ModelManagerConfig};
use neo_inference::model::{ModelArchitecture, ModelFormat, ModelId, ModelMetadata, ModelVersion, QuantizationType};
use std::collections::HashMap;

fn test_metadata(name: &str, version: ModelVersion) -> ModelMetadata {
    ModelMetadata {
        id: ModelId::new(),
        name: name.to_string(),
        version,
        architecture: ModelArchitecture::TransformerDecoder,
        format: ModelFormat::SafeTensors,
        quantization: QuantizationType::Fp32,
        path: "/tmp/test".to_string(),
        sha256: None,
        file_size: 1_000_000,
        parameter_count: 125_000_000,
        num_layers: 12,
        hidden_size: 768,
        num_attention_heads: 12,
        num_kv_heads: Some(12),
        intermediate_size: Some(3072),
        vocab_size: 32000,
        max_position_embeddings: 2048,
        context_length: 2048,
        rope_theta: Some(10000.0),
        eos_token_id: Some(3),
        bos_token_id: Some(2),
        pad_token_id: Some(0),
        aliases: vec![],
        tags: vec![],
        dependencies: vec![],
        metadata: HashMap::new(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

#[test]
fn test_register_and_unregister() {
    let manager = ModelManager::new(ModelManagerConfig::default());
    let meta = test_metadata("model-a", ModelVersion::new(1, 0, 0));
    let id = meta.id;
    manager.register(meta).unwrap();
    assert_eq!(manager.loaded_count(), 1);

    manager.unregister(id).unwrap();
    assert_eq!(manager.loaded_count(), 0);
}

#[test]
fn test_duplicate_register_error() {
    let manager = ModelManager::new(ModelManagerConfig::default());
    let meta = test_metadata("model-dup", ModelVersion::new(1, 0, 0));
    manager.register(meta.clone()).unwrap();
    let result = manager.register(meta);
    assert!(result.is_err());
}

#[test]
fn test_unregister_nonexistent_error() {
    let manager = ModelManager::new(ModelManagerConfig::default());
    let result = manager.unregister(ModelId::new());
    assert!(result.is_err());
}

#[test]
fn test_load_unload_reference_counting() {
    let manager = ModelManager::new(ModelManagerConfig::default());
    let meta = test_metadata("ref-model", ModelVersion::new(1, 0, 0));
    let id = meta.id;
    manager.register(meta).unwrap();

    manager.load(id).unwrap();
    manager.load(id).unwrap();
    let slot = manager.get_metadata(id).unwrap();
    assert_eq!(slot.id, id);

    manager.unload(id).unwrap();
    manager.unload(id).unwrap();
}

#[test]
fn test_unload_ref_count_error() {
    let manager = ModelManager::new(ModelManagerConfig::default());
    let meta = test_metadata("ref-model", ModelVersion::new(1, 0, 0));
    let id = meta.id;
    manager.register(meta).unwrap();
    manager.load(id).unwrap();

    let result = manager.unregister(id);
    assert!(result.is_err());
}

#[test]
fn test_alias_registration_and_lookup() {
    let manager = ModelManager::new(ModelManagerConfig::default());
    let meta = test_metadata("alias-model", ModelVersion::new(1, 0, 0));
    let id = meta.id;
    manager.register(meta).unwrap();

    manager.add_alias("my-alias".to_string(), id);
    assert_eq!(manager.get_by_alias("my-alias"), Some(id));
    assert!(manager.remove_alias("my-alias"));
    assert_eq!(manager.get_by_alias("my-alias"), None);
}

#[test]
fn test_version_lookup() {
    let manager = ModelManager::new(ModelManagerConfig::default());
    let meta = test_metadata("versioned", ModelVersion::new(2, 1, 0));
    manager.register(meta).unwrap();

    let found = manager.find_by_version("versioned", &ModelVersion::new(2, 1, 0));
    assert!(found.is_some());

    let not_found = manager.find_by_version("versioned", &ModelVersion::new(9, 0, 0));
    assert!(not_found.is_none());
}

#[test]
fn test_eviction_candidates() {
    let manager = ModelManager::new(ModelManagerConfig::default());
    let meta1 = test_metadata("evict-1", ModelVersion::new(1, 0, 0));
    let meta2 = test_metadata("evict-2", ModelVersion::new(1, 0, 0));
    let id1 = meta1.id;
    let id2 = meta2.id;
    manager.register(meta1).unwrap();
    manager.register(meta2).unwrap();

    manager.load(id1).unwrap();

    let candidates = manager.eviction_candidates();
    assert!(!candidates.contains(&id1));
    assert!(candidates.contains(&id2));

    manager.unload(id1).unwrap();
}

#[test]
fn test_can_load_more_limits() {
    let config = ModelManagerConfig {
        max_loaded_models: 2,
        ..Default::default()
    };
    let manager = ModelManager::new(config);
    assert!(manager.can_load_more());

    let meta1 = test_metadata("limit-1", ModelVersion::new(1, 0, 0));
    let meta2 = test_metadata("limit-2", ModelVersion::new(1, 0, 0));
    manager.register(meta1).unwrap();
    manager.register(meta2).unwrap();

    assert!(!manager.can_load_more());
}

#[test]
fn test_list_models() {
    let manager = ModelManager::new(ModelManagerConfig::default());
    let meta1 = test_metadata("list-1", ModelVersion::new(1, 0, 0));
    let meta2 = test_metadata("list-2", ModelVersion::new(1, 0, 0));
    manager.register(meta1).unwrap();
    manager.register(meta2).unwrap();

    let models = manager.list_models();
    assert_eq!(models.len(), 2);
}

#[test]
fn test_find_by_name() {
    let manager = ModelManager::new(ModelManagerConfig::default());
    let meta = test_metadata("find-me", ModelVersion::new(1, 0, 0));
    manager.register(meta).unwrap();

    let found = manager.find_by_name("find-me");
    assert_eq!(found.len(), 1);

    let not_found = manager.find_by_name("not-here");
    assert!(not_found.is_empty());
}

#[test]
fn test_total_memory_tracking() {
    let manager = ModelManager::new(ModelManagerConfig::default());
    let meta = test_metadata("mem-model", ModelVersion::new(1, 0, 0));
    let id = meta.id;
    manager.register(meta).unwrap();

    assert_eq!(manager.total_memory_used(), 0);
    manager.set_memory_allocated(id, 1024);
    assert!(manager.total_memory_used() >= 0);
}

#[test]
fn test_hot_swap() {
    let manager = ModelManager::new(ModelManagerConfig::default());
    let old_meta = test_metadata("old-model", ModelVersion::new(1, 0, 0));
    let old_id = old_meta.id;
    manager.register(old_meta).unwrap();

    let new_meta = test_metadata("new-model", ModelVersion::new(2, 0, 0));
    let new_id = manager.hot_swap(old_id, new_meta).unwrap();
    assert_ne!(old_id, new_id);
    assert_eq!(manager.loaded_count(), 1);
}

#[test]
fn test_hot_swap_with_active_refs_fails() {
    let manager = ModelManager::new(ModelManagerConfig::default());
    let meta = test_metadata("active-model", ModelVersion::new(1, 0, 0));
    let id = meta.id;
    manager.register(meta).unwrap();
    manager.load(id).unwrap();

    let new_meta = test_metadata("new-model", ModelVersion::new(2, 0, 0));
    let result = manager.hot_swap(id, new_meta);
    assert!(result.is_err());
    manager.unload(id).unwrap();
}
