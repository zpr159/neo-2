use std::collections::HashMap;
use crate::error::{InferenceError, InferenceResult};
use super::{Tokenizer, Token, Vocabulary, TokenizerConfig, TokenizerType, Encoding};

#[derive(Debug)]
pub struct BpeTokenizer {
    vocab: Vocabulary,
    config: TokenizerConfig,
    merges: Vec<(String, String)>,
}

impl BpeTokenizer {
    pub fn new() -> Self {
        let mut vocab = Vocabulary::new();
        for i in 0..256 {
            let ch = char::from_u32(i).unwrap_or(' ');
            vocab.insert(ch.to_string(), i);
        }
        vocab.insert_special("unk".to_string(), "[UNK]".to_string(), 0);
        vocab.insert_special("pad".to_string(), "[PAD]".to_string(), 1);
        vocab.insert_special("bos".to_string(), "[BOS]".to_string(), 2);
        vocab.insert_special("eos".to_string(), "[EOS]".to_string(), 3);
        Self {
            vocab,
            config: TokenizerConfig {
                tokenizer_type: TokenizerType::Bpe,
                vocab_size: 256,
                ..Default::default()
            },
            merges: Vec::new(),
        }
    }

    pub fn from_vocab_and_merges(vocab: Vocabulary, merges: Vec<(String, String)>) -> Self {
        Self {
            config: TokenizerConfig {
                tokenizer_type: TokenizerType::Bpe,
                vocab_size: vocab.size(),
                ..Default::default()
            },
            vocab,
            merges,
        }
    }

    fn byte_pair_encode(&self, word: &str) -> Vec<u32> {
        let mut tokens: Vec<String> = word.chars().map(|c| c.to_string()).collect();
        loop {
            if tokens.len() == 1 {
                break;
            }
            let mut min_rank = f64::MAX;
            let mut min_idx = 0;
            for i in 0..tokens.len().saturating_sub(1) {
                let pair = (&tokens[i], &tokens[i + 1]);
                let merged = format!("{}{}", pair.0, pair.1);
                if let Some(rank) = self.merges.iter().position(|(a, b)| a == pair.0 && b == pair.1) {
                    if (rank as f64) < min_rank {
                        min_rank = rank as f64;
                        min_idx = i;
                    }
                }
                let _ = merged;
            }
            if min_rank == f64::MAX {
                break;
            }
            let merged = format!("{}{}", tokens[min_idx], tokens[min_idx + 1]);
            tokens.remove(min_idx + 1);
            tokens[min_idx] = merged;
        }
        tokens.iter().map(|t| {
            self.vocab.get_id(t).unwrap_or_else(|| {
                self.vocab.get_id("[UNK]").unwrap_or(0)
            })
        }).collect()
    }
}

impl Default for BpeTokenizer {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for BpeTokenizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BPETokenizer(vocab={})", self.vocab.size())
    }
}

impl Tokenizer for BpeTokenizer {
    fn tokenize(&self, text: &str) -> InferenceResult<Vec<Token>> {
        let mut tokens = Vec::new();
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut offset = 0;
        for word in words {
            let ids = self.byte_pair_encode(word);
            for &id in &ids {
                let token_text = self.vocab.get_token(id).unwrap_or("<unk>").to_string();
                tokens.push(Token {
                    id,
                    text: token_text,
                    start: offset,
                    end: offset + word.len(),
                    score: None,
                });
            }
            offset += word.len() + 1;
        }
        Ok(tokens)
    }

    fn decode(&self, ids: &[u32]) -> InferenceResult<String> {
        let mut result = String::new();
        for &id in ids {
            if let Some(token) = self.vocab.get_token(id) {
                result.push_str(token);
            }
        }
        Ok(result)
    }

    fn vocabulary(&self) -> &Vocabulary {
        &self.vocab
    }

    fn config(&self) -> &TokenizerConfig {
        &self.config
    }
}
