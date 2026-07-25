use neo_inference::embedding::engine::EmbeddingEngine;
use neo_inference::embedding::{
    compute_similarity_matrix, find_most_similar, EmbeddingRequest, EmbeddingType, EmbeddingVector,
};

fn make_embedding(values: Vec<f32>) -> EmbeddingVector {
    EmbeddingVector::new(values, EmbeddingType::Text)
}

#[test]
fn test_basic_embedding_dimensions() {
    let engine = EmbeddingEngine::new();
    let request = EmbeddingRequest {
        input: vec!["hello world".to_string()],
        model: "neo-default".to_string(),
        embedding_type: EmbeddingType::Text,
        normalize: true,
        dimensions: None,
    };
    let response = engine.embed(&request).unwrap();
    assert_eq!(response.embeddings.len(), 1);
    assert_eq!(response.embeddings[0].dimensions, 768);
    assert_eq!(response.embeddings[0].values.len(), 768);
}

#[test]
fn test_normalize_produces_unit_vector() {
    let engine = EmbeddingEngine::new();
    let request = EmbeddingRequest {
        input: vec!["test text".to_string()],
        model: "neo-default".to_string(),
        embedding_type: EmbeddingType::Text,
        normalize: true,
        dimensions: None,
    };
    let response = engine.embed(&request).unwrap();
    let emb = &response.embeddings[0];
    let norm: f64 = emb.values.iter().map(|v| (*v as f64) * (*v as f64)).sum::<f64>().sqrt();
    assert!((norm - 1.0).abs() < 0.01, "Expected unit vector, got norm {}", norm);
}

#[test]
fn test_batch_embedding() {
    let engine = EmbeddingEngine::new();
    let texts = vec!["hello".to_string(), "world".to_string(), "test".to_string()];
    let request = EmbeddingRequest {
        input: texts,
        model: "neo-default".to_string(),
        embedding_type: EmbeddingType::Text,
        normalize: true,
        dimensions: None,
    };
    let response = engine.embed(&request).unwrap();
    assert_eq!(response.embeddings.len(), 3);
    for emb in &response.embeddings {
        assert_eq!(emb.dimensions, 768);
    }
}

#[test]
fn test_cosine_similarity_same_text() {
    let engine = EmbeddingEngine::new();
    let score = engine.cosine_similarity("hello", "hello", "neo-default").unwrap();
    assert!((score - 1.0).abs() < 0.001, "Same text should have similarity ~1.0, got {}", score);
}

#[test]
fn test_cosine_similarity_different_texts() {
    let engine = EmbeddingEngine::new();
    let score = engine.cosine_similarity("hello", "goodbye", "neo-default").unwrap();
    assert!(score >= -1.0 && score <= 1.0, "Similarity should be in [-1, 1], got {}", score);
}

#[test]
fn test_most_similar() {
    let engine = EmbeddingEngine::new();
    let query_emb = engine.embed(&EmbeddingRequest {
        input: vec!["hello".to_string()],
        model: "neo-default".to_string(),
        embedding_type: EmbeddingType::Text,
        normalize: true,
        dimensions: None,
    }).unwrap().embeddings.into_iter().next().unwrap();

    let candidates = engine.embed_batch(
        &["hello".to_string(), "goodbye".to_string(), "hello world".to_string()],
        "neo-default",
    ).unwrap();

    let results = find_most_similar(&query_emb, &candidates, 2);
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].0, 0);
    assert!(results[0].1 >= results[1].1);
}

#[test]
fn test_list_models() {
    let engine = EmbeddingEngine::new();
    let models = engine.list_models();
    assert!(models.contains(&"neo-default".to_string()));
    assert!(models.contains(&"text-embedding-ada-002".to_string()));
    assert!(models.contains(&"text-embedding-3-small".to_string()));
    assert!(models.contains(&"text-embedding-3-large".to_string()));
}

#[test]
fn test_model_dimensions() {
    let engine = EmbeddingEngine::new();
    assert_eq!(engine.model_dimensions("neo-default"), Some(768));
    assert_eq!(engine.model_dimensions("text-embedding-3-large"), Some(3072));
    assert_eq!(engine.model_dimensions("nonexistent"), None);
}

#[test]
fn test_invalid_model_error() {
    let engine = EmbeddingEngine::new();
    let request = EmbeddingRequest {
        input: vec!["test".to_string()],
        model: "nonexistent-model".to_string(),
        embedding_type: EmbeddingType::Text,
        normalize: true,
        dimensions: None,
    };
    let result = engine.embed(&request);
    assert!(result.is_err());
}

#[test]
fn test_similarity_search() {
    let engine = EmbeddingEngine::new();
    let results = engine.similarity_search(
        "hello",
        &["hello world".to_string(), "goodbye".to_string()],
        2,
        "neo-default",
    ).unwrap();
    assert_eq!(results.len(), 2);
    assert!(results[0].1 >= results[1].1, "Results should be sorted by score");
}

#[test]
fn test_embedding_vector_operations() {
    let a = make_embedding(vec![1.0, 0.0, 0.0]);
    let b = make_embedding(vec![0.0, 1.0, 0.0]);
    let c = make_embedding(vec![1.0, 0.0, 0.0]);

    assert!((a.cosine_similarity(&b)).abs() < 0.001);
    assert!((a.cosine_similarity(&c) - 1.0).abs() < 0.001);
}

#[test]
fn test_embedding_vector_l2_distance() {
    let a = make_embedding(vec![0.0, 0.0]);
    let b = make_embedding(vec![3.0, 4.0]);
    let dist = a.l2_distance(&b);
    assert!((dist - 5.0).abs() < 0.001);
}

#[test]
fn test_similarity_matrix() {
    let embs = vec![
        make_embedding(vec![1.0, 0.0]),
        make_embedding(vec![0.0, 1.0]),
        make_embedding(vec![1.0, 0.0]),
    ];
    let matrix = compute_similarity_matrix(&embs);
    assert_eq!(matrix.len(), 3);
    assert!((matrix[0][0] - 1.0).abs() < 0.001);
    assert!(matrix[0][1].abs() < 0.001);
    assert!((matrix[0][2] - 1.0).abs() < 0.001);
}

#[test]
fn test_embedding_batch_simple() {
    let engine = EmbeddingEngine::new();
    let vecs = engine.embed_batch(
        &["alpha".to_string(), "beta".to_string()],
        "neo-default",
    ).unwrap();
    assert_eq!(vecs.len(), 2);
    assert_eq!(vecs[0].dimensions, vecs[1].dimensions);
}

#[test]
fn test_embedding_type_display() {
    assert_eq!(format!("{}", EmbeddingType::Text), "text");
    assert_eq!(format!("{}", EmbeddingType::Image), "image");
}

#[test]
fn test_embedding_vector_len() {
    let emb = make_embedding(vec![1.0, 2.0, 3.0]);
    assert_eq!(emb.len(), 3);
    assert!(!emb.is_empty());
}

#[test]
fn test_embedding_vector_empty() {
    let emb = make_embedding(vec![]);
    assert!(emb.is_empty());
}
