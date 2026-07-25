use neo_inference::backends::cpu::CpuBackend;
use neo_inference::backends::{BackendConfig, InferenceBackend, InferenceInput};
use neo_inference::generation::{FinishReason, StreamChunk};
use neo_inference::model::{ModelArchitecture, ModelFormat, ModelId, ModelMetadata, ModelVersion, QuantizationType};
use std::collections::HashMap;

fn test_metadata() -> ModelMetadata {
    let id = ModelId::new();
    ModelMetadata {
        id,
        name: "stream-test-model".to_string(),
        version: ModelVersion::new(1, 0, 0),
        architecture: ModelArchitecture::TransformerDecoder,
        format: ModelFormat::SafeTensors,
        quantization: QuantizationType::Fp32,
        path: "/tmp/stream-test".to_string(),
        sha256: None,
        file_size: 1_000_000,
        parameter_count: 50_000_000,
        num_layers: 6,
        hidden_size: 256,
        num_attention_heads: 4,
        num_kv_heads: Some(4),
        intermediate_size: Some(1024),
        vocab_size: 1000,
        max_position_embeddings: 512,
        context_length: 512,
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
async fn test_stream_produces_chunks() {
    let mut backend = CpuBackend::new();
    backend.initialize(&BackendConfig::default()).await.unwrap();
    let metadata = test_metadata();
    let id = metadata.id;
    backend.load_model(&metadata).await.unwrap();

    let input = InferenceInput {
        input_ids: vec![1, 2, 3, 4],
        attention_mask: vec![1, 1, 1, 1],
        position_ids: None,
        past_key_values: None,
        parameters: HashMap::new(),
    };
    let mut rx = backend.inference_stream(id, input).await.unwrap();
    let mut chunks = Vec::new();
    while let Some(result) = rx.recv().await {
        let chunk = result.unwrap();
        chunks.push(chunk);
        if chunks.len() > 20 {
            break;
        }
    }
    assert!(chunks.len() >= 2, "Expected at least 2 chunks, got {}", chunks.len());

    backend.unload_model(id).await.unwrap();
}

#[tokio::test]
async fn test_stream_chunks_have_valid_token_ids() {
    let mut backend = CpuBackend::new();
    backend.initialize(&BackendConfig::default()).await.unwrap();
    let metadata = test_metadata();
    let id = metadata.id;
    backend.load_model(&metadata).await.unwrap();

    let input = InferenceInput {
        input_ids: vec![10, 20, 30],
        attention_mask: vec![1, 1, 1],
        position_ids: None,
        past_key_values: None,
        parameters: HashMap::new(),
    };
    let mut rx = backend.inference_stream(id, input).await.unwrap();
    while let Some(result) = rx.recv().await {
        let chunk = result.unwrap();
        if chunk.finish_reason.is_some() {
            break;
        }
        assert!(
            !chunk.token_text.is_empty() || chunk.finish_reason.is_some(),
            "Non-terminal chunk should have token_text"
        );
    }

    backend.unload_model(id).await.unwrap();
}

#[tokio::test]
async fn test_stream_finish_reason() {
    let mut backend = CpuBackend::new();
    backend.initialize(&BackendConfig::default()).await.unwrap();
    let metadata = test_metadata();
    let id = metadata.id;
    backend.load_model(&metadata).await.unwrap();

    let input = InferenceInput {
        input_ids: vec![1, 2],
        attention_mask: vec![1, 1],
        position_ids: None,
        past_key_values: None,
        parameters: HashMap::new(),
    };
    let mut rx = backend.inference_stream(id, input).await.unwrap();
    let mut got_finish = false;
    while let Some(result) = rx.recv().await {
        let chunk = result.unwrap();
        if let Some(reason) = chunk.finish_reason {
            assert!(matches!(
                reason,
                FinishReason::StopToken | FinishReason::MaxTokens
            ));
            got_finish = true;
            break;
        }
    }
    assert!(got_finish, "Stream should produce a finish_reason");

    backend.unload_model(id).await.unwrap();
}

#[tokio::test]
async fn test_stream_multiple_tokens() {
    let mut backend = CpuBackend::new();
    backend.initialize(&BackendConfig::default()).await.unwrap();
    let metadata = test_metadata();
    let id = metadata.id;
    backend.load_model(&metadata).await.unwrap();

    let input = InferenceInput {
        input_ids: vec![1, 2, 3, 4, 5],
        attention_mask: vec![1, 1, 1, 1, 1],
        position_ids: None,
        past_key_values: None,
        parameters: HashMap::new(),
    };
    let mut rx = backend.inference_stream(id, input).await.unwrap();
    let mut token_count = 0;
    while let Some(result) = rx.recv().await {
        let chunk = result.unwrap();
        if chunk.finish_reason.is_some() {
            break;
        }
        token_count += 1;
    }
    assert!(token_count >= 1);

    backend.unload_model(id).await.unwrap();
}
