use std::fmt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EmbeddingType {
    Text,
    Image,
    Audio,
    Multimodal,
}

impl fmt::Display for EmbeddingType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text => write!(f, "text"),
            Self::Image => write!(f, "image"),
            Self::Audio => write!(f, "audio"),
            Self::Multimodal => write!(f, "multimodal"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingVector {
    pub values: Vec<f32>,
    pub dimensions: usize,
    pub embedding_type: EmbeddingType,
}

impl EmbeddingVector {
    pub fn new(values: Vec<f32>, embedding_type: EmbeddingType) -> Self {
        let dimensions = values.len();
        Self {
            values,
            dimensions,
            embedding_type,
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.dimensions
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn normalize(&mut self) {
        let norm: f32 = self.values.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut self.values {
                *v /= norm;
            }
        }
    }

    #[must_use]
    pub fn dot_product(&self, other: &Self) -> f64 {
        self.values
            .iter()
            .zip(other.values.iter())
            .map(|(a, b)| (*a as f64) * (*b as f64))
            .sum()
    }

    #[must_use]
    pub fn cosine_similarity(&self, other: &Self) -> f64 {
        let dot = self.dot_product(other);
        let norm_a: f64 = self.values.iter().map(|v| (*v as f64) * (*v as f64)).sum::<f64>().sqrt();
        let norm_b: f64 = other.values.iter().map(|v| (*v as f64) * (*v as f64)).sum::<f64>().sqrt();
        if norm_a > 0.0 && norm_b > 0.0 {
            dot / (norm_a * norm_b)
        } else {
            0.0
        }
    }

    #[must_use]
    pub fn l2_distance(&self, other: &Self) -> f64 {
        self.values
            .iter()
            .zip(other.values.iter())
            .map(|(a, b)| {
                let d = *a as f64 - *b as f64;
                d * d
            })
            .sum::<f64>()
            .sqrt()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    pub input: Vec<String>,
    pub model: String,
    pub embedding_type: EmbeddingType,
    pub normalize: bool,
    pub dimensions: Option<usize>,
}

impl Default for EmbeddingRequest {
    fn default() -> Self {
        Self {
            input: Vec::new(),
            model: "default".to_string(),
            embedding_type: EmbeddingType::Text,
            normalize: true,
            dimensions: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    pub embeddings: Vec<EmbeddingVector>,
    pub usage: EmbeddingUsage,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct EmbeddingUsage {
    pub prompt_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityResult {
    pub index_a: usize,
    pub index_b: usize,
    pub score: f64,
}

pub fn compute_similarity_matrix(embeddings: &[EmbeddingVector]) -> Vec<Vec<f64>> {
    let n = embeddings.len();
    let mut matrix = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        for j in i..n {
            let sim = embeddings[i].cosine_similarity(&embeddings[j]);
            matrix[i][j] = sim;
            matrix[j][i] = sim;
        }
    }
    matrix
}

pub fn find_most_similar(
    query: &EmbeddingVector,
    candidates: &[EmbeddingVector],
    top_k: usize,
) -> Vec<(usize, f64)> {
    let mut scores: Vec<(usize, f64)> = candidates
        .iter()
        .enumerate()
        .map(|(i, c)| (i, query.cosine_similarity(c)))
        .collect();
    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scores.truncate(top_k);
    scores
}

pub mod engine;
