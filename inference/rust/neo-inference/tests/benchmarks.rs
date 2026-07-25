use neo_inference::backends::cpu::CpuBackend;
use neo_inference::backends::{BackendConfig, InferenceBackend, InferenceInput};
use neo_inference::tokenizer::bpe::BpeTokenizer;
use neo_inference::tokenizer::Tokenizer;
use neo_inference::embedding::engine::EmbeddingEngine;
use neo_inference::embedding::EmbeddingRequest;
use neo_inference::model::{
    ModelArchitecture, ModelFormat, ModelId, ModelMetadata, ModelVersion, QuantizationType,
};
use std::collections::HashMap;
use std::time::Instant;

fn test_metadata() -> ModelMetadata {
    let id = ModelId::new();
    ModelMetadata {
        id,
        name: "bench-model".to_string(),
        version: ModelVersion::new(1, 0, 0),
        architecture: ModelArchitecture::TransformerDecoder,
        format: ModelFormat::SafeTensors,
        quantization: QuantizationType::Fp32,
        path: "/tmp/bench".to_string(),
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
async fn bench_cpu_backend_inference() {
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

    let iterations = 10;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = backend.inference(id, input.clone()).await.unwrap();
    }
    let elapsed = start.elapsed();
    let avg_ms = elapsed.as_secs_f64() * 1000.0 / iterations as f64;
    eprintln!(
        "CPU backend inference: {:.2}ms avg ({} iterations)",
        avg_ms, iterations
    );

    backend.unload_model(id).await.unwrap();
}

#[test]
fn bench_tokenizer_encode() {
    let tok = BpeTokenizer::new();
    let text = "The quick brown fox jumps over the lazy dog and runs away";
    let iterations = 1000;

    let start = Instant::now();
    for _ in 0..iterations {
        let _ = tok.encode(text).unwrap();
    }
    let elapsed = start.elapsed();
    let avg_us = elapsed.as_micros() as f64 / iterations as f64;
    eprintln!("BPE encode: {:.2}us avg ({} iterations)", avg_us, iterations);
}

#[test]
fn bench_tokenizer_decode() {
    let tok = BpeTokenizer::new();
    let encoding = tok.encode("hello world test string").unwrap();
    let iterations = 1000;

    let start = Instant::now();
    for _ in 0..iterations {
        let _ = tok.decode(&encoding.ids).unwrap();
    }
    let elapsed = start.elapsed();
    let avg_us = elapsed.as_micros() as f64 / iterations as f64;
    eprintln!(
        "BPE decode: {:.2}us avg ({} iterations)",
        avg_us, iterations
    );
}

#[test]
fn bench_embedding_computation() {
    let engine = EmbeddingEngine::new();
    let request = EmbeddingRequest {
        input: vec!["benchmark text for embedding".to_string()],
        model: "neo-default".to_string(),
        ..Default::default()
    };
    let iterations = 50;

    let start = Instant::now();
    for _ in 0..iterations {
        let _ = engine.embed(&request).unwrap();
    }
    let elapsed = start.elapsed();
    let avg_ms = elapsed.as_secs_f64() * 1000.0 / iterations as f64;
    eprintln!(
        "Embedding compute: {:.2}ms avg ({} iterations)",
        avg_ms, iterations
    );
}
