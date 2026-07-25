use std::collections::HashMap;
use crate::error::{InferenceError, InferenceResult};
use crate::embedding::{EmbeddingRequest, EmbeddingResponse, EmbeddingVector, EmbeddingType, EmbeddingUsage, find_most_similar, SimilarityResult};

pub struct EmbeddingEngine {
    default_dimensions: usize,
    models: HashMap<String, EmbeddingModel>,
}

#[derive(Debug, Clone)]
struct EmbeddingModel {
    dimensions: usize,
    embedding_type: EmbeddingType,
    vocab_size: usize,
    hidden_size: usize,
}

impl EmbeddingEngine {
    pub fn new() -> Self {
        let mut models = HashMap::new();
        models.insert("text-embedding-ada-002".to_string(), EmbeddingModel {
            dimensions: 1536,
            embedding_type: EmbeddingType::Text,
            vocab_size: 50257,
            hidden_size: 1536,
        });
        models.insert("text-embedding-3-small".to_string(), EmbeddingModel {
            dimensions: 1536,
            embedding_type: EmbeddingType::Text,
            vocab_size: 50257,
            hidden_size: 1536,
        });
        models.insert("text-embedding-3-large".to_string(), EmbeddingModel {
            dimensions: 3072,
            embedding_type: EmbeddingType::Text,
            vocab_size: 50257,
            hidden_size: 3072,
        });
        models.insert("neo-default".to_string(), EmbeddingModel {
            dimensions: 768,
            embedding_type: EmbeddingType::Text,
            vocab_size: 32000,
            hidden_size: 768,
        });
        Self {
            default_dimensions: 768,
            models,
        }
    }

    pub fn embed(&self, request: &EmbeddingRequest) -> InferenceResult<EmbeddingResponse> {
        let model_info = self.models.get(&request.model)
            .ok_or_else(|| InferenceError::EmbeddingFailed {
                reason: format!("model '{}' not found", request.model),
            })?;
        let dimensions = request.dimensions.unwrap_or(model_info.dimensions);
        let mut embeddings = Vec::with_capacity(request.input.len());
        for (idx, text) in request.input.iter().enumerate() {
            let embedding = self.compute_embedding(text, dimensions, model_info)?;
            embeddings.push(embedding);
        }
        let usage = EmbeddingUsage {
            prompt_tokens: request.input.iter().map(|t| t.len() as u64 / 4).sum(),
            total_tokens: embeddings.len() as u64,
        };
        Ok(EmbeddingResponse { embeddings, usage })
    }

    fn compute_embedding(&self, text: &str, dimensions: usize, model: &EmbeddingModel) -> InferenceResult<EmbeddingVector> {
        let mut values = Vec::with_capacity(dimensions);
        let bytes = text.as_bytes();
        let mut hash_state: u64 = 0xcbf29ce484222325;
        for &byte in bytes {
            hash_state ^= byte as u64;
            hash_state = hash_state.wrapping_mul(0x100000001b3);
        }
        let mut rng_state = hash_state;
        for i in 0..dimensions {
            rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(i as u64 + 1);
            let raw = ((rng_state >> 11) as f64 / (1u64 << 53) as f64) * 2.0 - 1.0;
            let positional = ((i as f64 / dimensions as f64) * std::f64::consts::PI * 2.0).sin() * 0.1;
            values.push((raw * 0.9 + positional * 0.1) as f32);
        }
        let mut embedding = EmbeddingVector::new(values, request_embedding_type());
        if true {
            embedding.normalize();
        }
        Ok(embedding)
    }

    pub fn embed_batch(&self, texts: &[String], model: &str) -> InferenceResult<Vec<EmbeddingVector>> {
        let request = EmbeddingRequest {
            input: texts.to_vec(),
            model: model.to_string(),
            embedding_type: EmbeddingType::Text,
            normalize: true,
            dimensions: None,
        };
        let response = self.embed(&request)?;
        Ok(response.embeddings)
    }

    pub fn similarity_search(
        &self,
        query: &str,
        candidates: &[String],
        top_k: usize,
        model: &str,
    ) -> InferenceResult<Vec<(usize, f64, String)>> {
        let query_embedding = self.embed(&EmbeddingRequest {
            input: vec![query.to_string()],
            model: model.to_string(),
            embedding_type: EmbeddingType::Text,
            normalize: true,
            dimensions: None,
        })?;
        if query_embedding.embeddings.is_empty() {
            return Err(InferenceError::EmbeddingFailed { reason: "empty query embedding".to_string() });
        }
        let query_vec = &query_embedding.embeddings[0];
        let candidate_embeddings = self.embed_batch(candidates, model)?;
        let results = find_most_similar(query_vec, &candidate_embeddings, top_k);
        Ok(results.into_iter().map(|(idx, score)| (idx, score, candidates[idx].clone())).collect())
    }

    pub fn cosine_similarity(&self, text_a: &str, text_b: &str, model: &str) -> InferenceResult<f64> {
        let response = self.embed(&EmbeddingRequest {
            input: vec![text_a.to_string(), text_b.to_string()],
            model: model.to_string(),
            embedding_type: EmbeddingType::Text,
            normalize: true,
            dimensions: None,
        })?;
        if response.embeddings.len() < 2 {
            return Err(InferenceError::EmbeddingFailed { reason: "insufficient embeddings".to_string() });
        }
        Ok(response.embeddings[0].cosine_similarity(&response.embeddings[1]))
    }

    pub fn list_models(&self) -> Vec<String> {
        self.models.keys().cloned().collect()
    }

    pub fn model_dimensions(&self, model: &str) -> Option<usize> {
        self.models.get(model).map(|m| m.dimensions)
    }
}

fn request_embedding_type() -> EmbeddingType {
    EmbeddingType::Text
}

impl Default for EmbeddingEngine {
    fn default() -> Self {
        Self::new()
    }
}
