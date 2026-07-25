use std::collections::HashMap;
use crate::error::InferenceResult;
use super::{Tokenizer, Token, Vocabulary, TokenizerConfig, TokenizerType};

#[derive(Debug)]
pub struct WordPieceTokenizer {
    vocab: Vocabulary,
    config: TokenizerConfig,
    max_word_length: usize,
}

impl WordPieceTokenizer {
    pub fn new() -> Self {
        let mut vocab = Vocabulary::new();
        for i in 0..256 {
            let ch = char::from_u32(i).unwrap_or(' ');
            vocab.insert(ch.to_string(), i);
        }
        vocab.insert_special("unk".to_string(), "[UNK]".to_string(), 0);
        vocab.insert_special("pad".to_string(), "[PAD]".to_string(), 1);
        vocab.insert_special("cls".to_string(), "[CLS]".to_string(), 2);
        vocab.insert_special("sep".to_string(), "[SEP]".to_string(), 3);
        vocab.insert_special("mask".to_string(), "[MASK]".to_string(), 4);
        Self {
            vocab,
            config: TokenizerConfig {
                tokenizer_type: TokenizerType::WordPiece,
                vocab_size: 256,
                word_tokens_prefix: "##".to_string(),
                ..Default::default()
            },
            max_word_length: 200,
        }
    }

    pub fn from_vocab(vocab: Vocabulary) -> Self {
        Self {
            config: TokenizerConfig {
                tokenizer_type: TokenizerType::WordPiece,
                vocab_size: vocab.size(),
                word_tokens_prefix: "##".to_string(),
                ..Default::default()
            },
            vocab,
            max_word_length: 200,
        }
    }

    fn wordpiece_tokenize(&self, word: &str) -> Vec<u32> {
        if word.len() > self.max_word_length {
            return vec![self.vocab.get_id("[UNK]").unwrap_or(0)];
        }
        let chars: Vec<char> = word.chars().collect();
        let mut sub_tokens = Vec::new();
        let mut start = 0;
        while start < chars.len() {
            let mut end = chars.len();
            let mut found = false;
            while start < end {
                let substr: String = if start == 0 {
                    chars[start..end].iter().collect()
                } else {
                    format!("{}{}", self.config.word_tokens_prefix, chars[start..end].iter().collect::<String>())
                };
                if let Some(id) = self.vocab.get_id(&substr) {
                    sub_tokens.push(id);
                    found = true;
                    break;
                }
                end -= 1;
            }
            if !found {
                return vec![self.vocab.get_id("[UNK]").unwrap_or(0)];
            }
            start = end;
        }
        sub_tokens
    }
}

impl Default for WordPieceTokenizer {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for WordPieceTokenizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WordPieceTokenizer(vocab={})", self.vocab.size())
    }
}

impl Tokenizer for WordPieceTokenizer {
    fn tokenize(&self, text: &str) -> InferenceResult<Vec<Token>> {
        let mut tokens = Vec::new();
        let mut offset = 0;
        for word in text.split_whitespace() {
            let ids = self.wordpiece_tokenize(word);
            for &id in &ids {
                let token_text = self.vocab.get_token(id).unwrap_or("[UNK]").to_string();
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
                if token.starts_with("##") && !result.is_empty() {
                    result.push_str(&token[2..]);
                } else {
                    if !result.is_empty() {
                        result.push(' ');
                    }
                    result.push_str(token);
                }
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
