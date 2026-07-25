# Embedding Pipeline

## Overview

The `neo-inference` embedding pipeline converts text, images, audio, or multimodal inputs into dense vector representations. These embeddings power similarity search, retrieval-augmented generation (RAG), clustering, classification, and anomaly detection. The system provides a unified `EmbeddingEngine` with built-in similarity metrics and search utilities.

## Text, Image, and Audio Embedding Interfaces

### Embedding Types

```rust
pub enum EmbeddingType {
    Text,       // Text embeddings (e.g., BERT, Ada, Cohere)
    Image,      // Image embeddings (e.g., CLIP vision encoder)
    Audio,      // Audio embeddings (e.g., Whisper encoder)
    Multimodal, // Combined embeddings (e.g., CLIP joint space)
}
```

### EmbeddingVector

The core vector type that carries embedding data:

```rust
pub struct EmbeddingVector {
    pub values: Vec<f32>,        // The embedding values
    pub dimensions: usize,       // Number of dimensions
    pub embedding_type: EmbeddingType,
}

// Create an embedding
let embedding = EmbeddingVector::new(
    vec![0.1, 0.2, 0.3, ..., 0.9],  // 768-dimensional vector
    EmbeddingType::Text,
);

// Access dimensions
assert_eq!(embedding.len(), 768);
assert!(!embedding.is_empty());
```

### Embedding Request/Response

```rust
pub struct EmbeddingRequest {
    pub input: Vec<String>,           // Texts to embed
    pub model: String,                // Model name
    pub embedding_type: EmbeddingType,
    pub normalize: bool,              // L2-normalize output vectors
    pub dimensions: Option<usize>,    // Optional dimension reduction
}

pub struct EmbeddingResponse {
    pub embeddings: Vec<EmbeddingVector>,
    pub usage: EmbeddingUsage,
}

pub struct EmbeddingUsage {
    pub prompt_tokens: u64,
    pub total_tokens: u64,
}
```

## EmbeddingEngine

The `EmbeddingEngine` manages embedding models and provides high-level operations:

```rust
let engine = EmbeddingEngine::new();

// List available models
let models = engine.list_models();
// → ["text-embedding-ada-002", "text-embedding-3-small",
//    "text-embedding-3-large", "neo-default"]

// Check model dimensions
let dims = engine.model_dimensions("text-embedding-3-large");
// → Some(3072)
```

### Built-in Models

| Model | Dimensions | Type | Vocab Size |
|-------|-----------|------|------------|
| `text-embedding-ada-002` | 1536 | Text | 50,257 |
| `text-embedding-3-small` | 1536 | Text | 50,257 |
| `text-embedding-3-large` | 3072 | Text | 50,257 |
| `neo-default` | 768 | Text | 32,000 |

### Single Embedding

```rust
let request = EmbeddingRequest {
    input: vec!["The quick brown fox jumps over the lazy dog".to_string()],
    model: "text-embedding-ada-002".to_string(),
    embedding_type: EmbeddingType::Text,
    normalize: true,
    dimensions: None,
};

let response = engine.embed(&request)?;
let embedding = &response.embeddings[0];
assert_eq!(embedding.dimensions, 1536);
```

### Batch Embedding

```rust
let texts = vec![
    "Machine learning is a subset of AI".to_string(),
    "Deep learning uses neural networks".to_string(),
    "Natural language processing handles text".to_string(),
];

let embeddings = engine.embed_batch(&texts, "text-embedding-ada-002")?;
assert_eq!(embeddings.len(), 3);
```

## Similarity Search Hooks

### Find Most Similar

```rust
let candidates = vec![
    "The cat sat on the mat".to_string(),
    "Dogs are loyal companions".to_string(),
    "Python is a programming language".to_string(),
    "Cats are independent pets".to_string(),
];

let results = engine.similarity_search(
    "Tell me about cats",
    &candidates,
    2,  // top_k
    "text-embedding-ada-002",
)?;

for (idx, score, text) in &results {
    println!("#{} (score={:.4}): {}", idx, score, text);
}
// → #0 (score=0.9234): The cat sat on the mat
//   #3 (score=0.8891): Cats are independent pets
```

### Pairwise Cosine Similarity

```rust
let similarity = engine.cosine_similarity(
    "The weather is nice today",
    "It's a beautiful sunny day",
    "text-embedding-ada-002",
)?;
println!("Similarity: {:.4}", similarity);
// → Similarity: 0.8567
```

### Similarity Matrix

For comparing all embeddings against each other:

```rust
use neo_inference::embedding::compute_similarity_matrix;

let matrix = compute_similarity_matrix(&embeddings);
// matrix[i][j] = cosine_similarity(embeddings[i], embeddings[j])
// Diagonal is always 1.0
// Matrix is symmetric
```

## Vector Normalization

L2-normalization scales vectors to unit length, which is essential for cosine similarity to work correctly:

```rust
let mut embedding = EmbeddingVector::new(values, EmbeddingType::Text);
embedding.normalize();

// After normalization: ||v|| = 1.0
let norm: f32 = embedding.values.iter().map(|v| v * v).sum::<f32>().sqrt();
assert!((norm - 1.0).abs() < 1e-6);
```

Normalization is applied automatically when `EmbeddingRequest.normalize = true`.

## Cosine Similarity

Measures the angle between two vectors (range: -1 to 1, where 1 means identical direction):

```rust
let a = EmbeddingVector::new(vec![1.0, 0.0, 0.0], EmbeddingType::Text);
let b = EmbeddingVector::new(vec![0.0, 1.0, 0.0], EmbeddingType::Text);
let c = EmbeddingVector::new(vec![1.0, 0.0, 0.0], EmbeddingType::Text);

assert!((a.cosine_similarity(&b) - 0.0).abs() < 1e-6);  // Orthogonal
assert!((a.cosine_similarity(&c) - 1.0).abs() < 1e-6);  // Identical
```

### Implementation

```rust
pub fn cosine_similarity(&self, other: &Self) -> f64 {
    let dot = self.dot_product(other);
    let norm_a = self.values.iter().map(|v| (*v as f64) * (*v as f64)).sum::<f64>().sqrt();
    let norm_b = other.values.iter().map(|v| (*v as f64) * (*v as f64)).sum::<f64>().sqrt();
    if norm_a > 0.0 && norm_b > 0.0 {
        dot / (norm_a * norm_b)
    } else {
        0.0
    }
}
```

## L2 Distance (Euclidean Distance)

Measures the straight-line distance between two vectors in the embedding space:

```rust
let a = EmbeddingVector::new(vec![0.0, 0.0], EmbeddingType::Text);
let b = EmbeddingVector::new(vec![3.0, 4.0], EmbeddingType::Text);

assert!((a.l2_distance(&b) - 5.0).abs() < 1e-6);  // 3-4-5 triangle
```

### When to Use Which

| Metric | Range | Best For |
|--------|-------|----------|
| Cosine Similarity | [-1, 1] | Text similarity, normalized embeddings, direction matters |
| L2 Distance | [0, ∞) | Spatial clustering, when magnitude matters |
| Dot Product | (-∞, ∞) | When embeddings are already normalized (equivalent to cosine) |

## SimilarityResult

For programmatic similarity queries:

```rust
pub struct SimilarityResult {
    pub index_a: usize,  // Index in the first embedding list
    pub index_b: usize,  // Index in the second embedding list
    pub score: f64,      // Similarity score
}
```

## Integration with Inference Pipeline

The embedding engine integrates with the broader inference system for RAG and retrieval:

```
User Query
    │
    ▼
┌─────────────────┐
│ Tokenizer        │  (text → tokens)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ EmbeddingEngine  │  (text → vector)
│ .embed()         │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Similarity Search│  (query vector vs. document vectors)
│ .similarity_     │
│  search()        │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Top-K Documents  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Context Engine   │  (inject documents into prompt)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Inference Engine │  (generate response)
└─────────────────┘
```
