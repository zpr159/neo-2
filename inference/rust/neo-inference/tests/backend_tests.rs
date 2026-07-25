use neo_inference::backends::cpu::CpuBackend;
use neo_inference::backends::neo_native::NeoNativeBackend;
use neo_inference::backends::llama_cpp::LlamaCppBackend;
use neo_inference::backends::{
    BackendConfig, BackendType, InferenceBackend, InferenceInput, probe_available_backends,
};
use neo_inference::model::{ModelArchitecture, ModelFormat, ModelId, ModelMetadata, ModelVersion, QuantizationType};
use std::collections::HashMap;

fn test_metadata(id: ModelId) -> ModelMetadata {
    ModelMetadata {
        id,
        name: "test-model".to_string(),
        version: ModelVersion::new(1, 0, 0),
        architecture: ModelArchitecture::TransformerDecoder,
        format: ModelFormat::SafeTensors,
        quantization: QuantizationType::Fp32,
        path: "/tmp/test-model".to_string(),
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

#[tokio::test]
async fn test_cpu_backend_initialize() {
    let mut backend = CpuBackend::new();
    let config = BackendConfig::default();
    assert!(backend.initialize(&config).await.is_ok());
    assert!(backend.is_available());
}

#[tokio::test]
async fn test_cpu_backend_info() {
    let backend = CpuBackend::new();
    let info = backend.info();
    assert_eq!(info.backend_type, BackendType::Cpu);
    assert!(info.is_available);
    assert!(!info.supported_formats.is_empty());
}

#[tokio::test]
async fn test_cpu_backend_load_and_unload_model() {
    let mut backend = CpuBackend::new();
    let config = BackendConfig::default();
    backend.initialize(&config).await.unwrap();

    let id = ModelId::new();
    let metadata = test_metadata(id);
    let loaded_id = backend.load_model(&metadata).await.unwrap();
    assert_eq!(loaded_id, id);
    assert_eq!(backend.loaded_models().len(), 1);
    assert!(backend.loaded_models().contains(&id));

    backend.unload_model(id).await.unwrap();
    assert_eq!(backend.loaded_models().len(), 0);
}

#[tokio::test]
async fn test_cpu_backend_duplicate_load_error() {
    let mut backend = CpuBackend::new();
    let config = BackendConfig::default();
    backend.initialize(&config).await.unwrap();

    let id = ModelId::new();
    let metadata = test_metadata(id);
    backend.load_model(&metadata).await.unwrap();
    let result = backend.load_model(&metadata).await;
    assert!(result.is_err());
    backend.unload_model(id).await.unwrap();
}

#[tokio::test]
async fn test_cpu_backend_inference() {
    let mut backend = CpuBackend::new();
    let config = BackendConfig::default();
    backend.initialize(&config).await.unwrap();

    let id = ModelId::new();
    let metadata = test_metadata(id);
    backend.load_model(&metadata).await.unwrap();

    let input = InferenceInput {
        input_ids: vec![1, 2, 3],
        attention_mask: vec![1, 1, 1],
        position_ids: None,
        past_key_values: None,
        parameters: HashMap::new(),
    };
    let output = backend.inference(id, input).await.unwrap();
    assert_eq!(output.logits_shape, vec![3, 32000]);
    assert_eq!(output.logits.len(), 3 * 32000);
    assert!(output.past_key_values.is_some());

    backend.unload_model(id).await.unwrap();
}

#[tokio::test]
async fn test_cpu_backend_inference_stream() {
    let mut backend = CpuBackend::new();
    let config = BackendConfig::default();
    backend.initialize(&config).await.unwrap();

    let id = ModelId::new();
    let metadata = test_metadata(id);
    backend.load_model(&metadata).await.unwrap();

    let input = InferenceInput {
        input_ids: vec![1, 2, 3],
        attention_mask: vec![1, 1, 1],
        position_ids: None,
        past_key_values: None,
        parameters: HashMap::new(),
    };
    let mut rx = backend.inference_stream(id, input).await.unwrap();
    let mut received = 0;
    while let Some(chunk) = rx.recv().await {
        let chunk = chunk.unwrap();
        received += 1;
        if chunk.finish_reason.is_some() {
            break;
        }
    }
    assert!(received >= 2);

    backend.unload_model(id).await.unwrap();
}

#[tokio::test]
async fn test_cpu_backend_model_memory_usage() {
    let mut backend = CpuBackend::new();
    let config = BackendConfig::default();
    backend.initialize(&config).await.unwrap();

    let id = ModelId::new();
    let metadata = test_metadata(id);
    backend.load_model(&metadata).await.unwrap();

    let usage = backend.model_memory_usage(id);
    assert!(usage.is_some());
    assert!(usage.unwrap() > 0);

    backend.unload_model(id).await.unwrap();
}

#[tokio::test]
async fn test_cpu_backend_supported_formats() {
    let backend = CpuBackend::new();
    let formats = backend.supported_formats();
    assert!(formats.contains(&ModelFormat::SafeTensors));
    assert!(formats.contains(&ModelFormat::Gguf));
}

// --- NeoNative Backend ---

#[tokio::test]
async fn test_neo_native_backend_initialize() {
    let mut backend = NeoNativeBackend::new();
    let config = BackendConfig::default();
    assert!(backend.initialize(&config).await.is_ok());
}

#[tokio::test]
async fn test_neo_native_backend_info() {
    let backend = NeoNativeBackend::new();
    let info = backend.info();
    assert_eq!(info.backend_type, BackendType::NeoNative);
    assert!(info.is_available);
    assert!(info.capabilities.contains(&"streaming".to_string()));
}

#[tokio::test]
async fn test_neo_native_backend_load_and_unload() {
    let mut backend = NeoNativeBackend::new();
    let config = BackendConfig::default();
    backend.initialize(&config).await.unwrap();

    let id = ModelId::new();
    let metadata = test_metadata(id);
    backend.load_model(&metadata).await.unwrap();
    assert_eq!(backend.loaded_models().len(), 1);

    backend.unload_model(id).await.unwrap();
    assert_eq!(backend.loaded_models().len(), 0);
}

#[tokio::test]
async fn test_neo_native_backend_inference() {
    let mut backend = NeoNativeBackend::new();
    let config = BackendConfig::default();
    backend.initialize(&config).await.unwrap();

    let id = ModelId::new();
    let metadata = test_metadata(id);
    backend.load_model(&metadata).await.unwrap();

    let input = InferenceInput {
        input_ids: vec![5, 10, 15],
        attention_mask: vec![1, 1, 1],
        position_ids: None,
        past_key_values: None,
        parameters: HashMap::new(),
    };
    let output = backend.inference(id, input).await.unwrap();
    assert_eq!(output.logits_shape, vec![3, 32000]);
    assert_eq!(output.logits.len(), 3 * 32000);

    backend.unload_model(id).await.unwrap();
}

#[tokio::test]
async fn test_neo_native_backend_stream() {
    let mut backend = NeoNativeBackend::new();
    let config = BackendConfig::default();
    backend.initialize(&config).await.unwrap();

    let id = ModelId::new();
    let metadata = test_metadata(id);
    backend.load_model(&metadata).await.unwrap();

    let input = InferenceInput {
        input_ids: vec![1, 2, 3],
        attention_mask: vec![1, 1, 1],
        position_ids: None,
        past_key_values: None,
        parameters: HashMap::new(),
    };
    let mut rx = backend.inference_stream(id, input).await.unwrap();
    let mut count = 0;
    while let Some(chunk) = rx.recv().await {
        chunk.unwrap();
        count += 1;
        if count > 10 {
            break;
        }
    }
    assert!(count >= 2);

    backend.unload_model(id).await.unwrap();
}

#[tokio::test]
async fn test_neo_native_memory_usage() {
    let mut backend = NeoNativeBackend::new();
    let config = BackendConfig::default();
    backend.initialize(&config).await.unwrap();

    let id = ModelId::new();
    let metadata = test_metadata(id);
    backend.load_model(&metadata).await.unwrap();

    assert!(backend.model_memory_usage(id).is_some());
    assert!(backend.model_memory_usage(ModelId::new()).is_none());

    backend.unload_model(id).await.unwrap();
}

// --- LlamaCpp Backend ---

#[tokio::test]
async fn test_llamacpp_backend_initialize() {
    let mut backend = LlamaCppBackend::new();
    let config = BackendConfig::default();
    assert!(backend.initialize(&config).await.is_ok());
}

#[tokio::test]
async fn test_llamacpp_backend_info() {
    let backend = LlamaCppBackend::new();
    let info = backend.info();
    assert_eq!(info.backend_type, BackendType::LlamaCpp);
    assert_eq!(info.priority, 180);
}

#[tokio::test]
async fn test_llamacpp_load_unload() {
    let mut backend = LlamaCppBackend::new();
    let config = BackendConfig::default();
    backend.initialize(&config).await.unwrap();

    let mut meta = test_metadata(ModelId::new());
    meta.format = ModelFormat::Gguf;
    let id = meta.id;
    backend.load_model(&meta).await.unwrap();
    assert!(backend.loaded_models().contains(&id));

    backend.unload_model(id).await.unwrap();
    assert!(backend.loaded_models().is_empty());
}

#[tokio::test]
async fn test_llamacpp_inference() {
    let mut backend = LlamaCppBackend::new();
    let config = BackendConfig::default();
    backend.initialize(&config).await.unwrap();

    let mut meta = test_metadata(ModelId::new());
    meta.format = ModelFormat::Gguf;
    let id = meta.id;
    backend.load_model(&meta).await.unwrap();

    let input = InferenceInput {
        input_ids: vec![1, 2],
        attention_mask: vec![1, 1],
        position_ids: None,
        past_key_values: None,
        parameters: HashMap::new(),
    };
    let output = backend.inference(id, input).await.unwrap();
    assert_eq!(output.logits_shape, vec![2, 32000]);

    backend.unload_model(id).await.unwrap();
}

#[tokio::test]
async fn test_llamacpp_stream() {
    let mut backend = LlamaCppBackend::new();
    let config = BackendConfig::default();
    backend.initialize(&config).await.unwrap();

    let mut meta = test_metadata(ModelId::new());
    meta.format = ModelFormat::Gguf;
    let id = meta.id;
    backend.load_model(&meta).await.unwrap();

    let input = InferenceInput {
        input_ids: vec![1, 2, 3],
        attention_mask: vec![1, 1, 1],
        position_ids: None,
        past_key_values: None,
        parameters: HashMap::new(),
    };
    let mut rx = backend.inference_stream(id, input).await.unwrap();
    let mut got_tokens = false;
    while let Some(chunk) = rx.recv().await {
        let chunk = chunk.unwrap();
        if chunk.finish_reason.is_some() {
            got_tokens = true;
            break;
        }
    }
    assert!(got_tokens);

    backend.unload_model(id).await.unwrap();
}

#[tokio::test]
async fn test_probe_available_backends() {
    let backends = probe_available_backends();
    assert!(!backends.is_empty());
    let cpu = backends.iter().find(|b| b.backend_type == BackendType::Cpu);
    assert!(cpu.is_some());
}

#[tokio::test]
async fn test_inference_not_found_error() {
    let backend = CpuBackend::new();
    let input = InferenceInput {
        input_ids: vec![1],
        attention_mask: vec![1],
        position_ids: None,
        past_key_values: None,
        parameters: HashMap::new(),
    };
    let result = backend.inference(ModelId::new(), input).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_unload_nonexistent_error() {
    let mut backend = CpuBackend::new();
    let result = backend.unload_model(ModelId::new()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_shutdown_clears_models() {
    let mut backend = CpuBackend::new();
    let config = BackendConfig::default();
    backend.initialize(&config).await.unwrap();

    let id = ModelId::new();
    let metadata = test_metadata(id);
    backend.load_model(&metadata).await.unwrap();
    assert_eq!(backend.loaded_models().len(), 1);

    backend.shutdown().await.unwrap();
    assert_eq!(backend.loaded_models().len(), 0);
}
