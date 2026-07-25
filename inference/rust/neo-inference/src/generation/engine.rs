use crate::error::InferenceResult;
use crate::generation::{GenerationParams, GenerationResult, FinishReason, TokenUsage, StreamChunk};

pub struct GenerationEngine;

impl GenerationEngine {
    pub fn greedy_decode(logits: &[f32], vocab_size: usize) -> u32 {
        if logits.len() < vocab_size {
            return 0;
        }
        let mut best_id = 0u32;
        let mut best_val = f32::NEG_INFINITY;
        for (i, &val) in logits.iter().take(vocab_size).enumerate() {
            if val > best_val {
                best_val = val;
                best_id = i as u32;
            }
        }
        best_id
    }

    pub fn top_k_sample(logits: &mut [f32], k: usize) -> u32 {
        let vocab_size = logits.len();
        let k = k.min(vocab_size);
        let mut indexed: Vec<(u32, f32)> = logits.iter().enumerate().map(|(i, &v)| (i as u32, v)).collect();
        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        indexed.truncate(k);
        let max_val = indexed[0].1;
        let probs: Vec<(u32, f32)> = indexed.iter().map(|&(id, v)| (id, (v - max_val).exp())).collect();
        let sum: f32 = probs.iter().map(|(_, p)| p).sum();
        let r = {
            let mut state: u64 = 0x1234_5678_9ABC_DEF0;
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((state >> 33) as f32 / (1u32 << 31) as f32) * sum
        };
        let mut cumulative = 0.0;
        for &(id, p) in &probs {
            cumulative += p;
            if r <= cumulative {
                return id;
            }
        }
        probs.last().map(|(id, _)| *id).unwrap_or(0)
    }

    pub fn top_p_sample(logits: &mut [f32], p: f64) -> u32 {
        let vocab_size = logits.len();
        let mut indexed: Vec<(u32, f32)> = logits.iter().enumerate().map(|(i, &v)| (i as u32, v)).collect();
        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let max_val = indexed[0].1;
        let total: f32 = indexed.iter().map(|(_, v)| (*v - max_val).exp()).sum();
        let mut cumulative = 0.0;
        let threshold = p as f32;
        for (id, v) in &indexed {
            cumulative += (*v - max_val).exp() / total;
            if cumulative >= threshold {
                return *id;
            }
        }
        indexed.last().map(|(id, _)| *id).unwrap_or(0)
    }

    pub fn beam_search_decode(
        logits: &[f32],
        vocab_size: usize,
        beam_width: usize,
        beams: &mut Vec<(Vec<u32>, f32)>,
    ) {
        if beams.is_empty() {
            for i in 0..beam_width.min(vocab_size) {
                let score = logits.get(i).copied().unwrap_or(0.0);
                beams.push((vec![i as u32], score));
            }
            return;
        }
        let mut candidates = Vec::new();
        for (tokens, score) in beams.iter() {
            for (i, &logit) in logits.iter().take(vocab_size).enumerate() {
                let mut new_tokens = tokens.clone();
                new_tokens.push(i as u32);
                candidates.push((new_tokens, score + logit));
            }
        }
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        candidates.truncate(beam_width);
        *beams = candidates;
    }

    pub fn apply_temperature(logits: &mut [f32], temperature: f64) {
        if temperature > 0.0 && temperature != 1.0 {
            for l in logits.iter_mut() {
                *l /= temperature as f32;
            }
        }
    }

    pub fn apply_repetition_penalty(logits: &mut [f32], generated: &[u32], penalty: f64) {
        if penalty == 1.0 {
            return;
        }
        for &token_id in generated {
            let idx = token_id as usize;
            if idx < logits.len() {
                if logits[idx] > 0.0 {
                    logits[idx] /= penalty as f32;
                } else {
                    logits[idx] *= penalty as f32;
                }
            }
        }
    }

    pub fn apply_presence_penalty(logits: &mut [f32], generated: &[u32], penalty: f64) {
        let mut seen = std::collections::HashSet::new();
        for &token_id in generated {
            if seen.insert(token_id) {
                let idx = token_id as usize;
                if idx < logits.len() {
                    logits[idx] -= penalty as f32;
                }
            }
        }
    }

    pub fn apply_frequency_penalty(logits: &mut [f32], generated: &[u32], penalty: f64) {
        let mut counts = std::collections::HashMap::new();
        for &token_id in generated {
            *counts.entry(token_id).or_insert(0u32) += 1;
        }
        for (&token_id, &count) in &counts {
            let idx = token_id as usize;
            if idx < logits.len() {
                logits[idx] -= penalty as f32 * count as f32;
            }
        }
    }

    pub fn check_stop_sequences(text: &str, stop_sequences: &[String]) -> bool {
        for seq in stop_sequences {
            if text.contains(seq.as_str()) {
                return true;
            }
        }
        false
    }

    pub fn check_stop_token(token_id: u32, stop_token_ids: &[u32]) -> bool {
        stop_token_ids.contains(&token_id)
    }
}
