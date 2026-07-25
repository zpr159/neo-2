use std::collections::HashMap;
use std::fmt;
use serde::{Deserialize, Serialize};

pub mod bpe;
pub mod wordpiece;
pub mod sentencepiece;
pub mod character;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TokenizerType {
    Bpe,
    WordPiece,
    SentencePiece,
    Character,
}

impl fmt::Display for TokenizerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bpe => write!(f, "bpe"),
            Self::WordPiece => write!(f, "wordpiece"),
            Self::SentencePiece => write!(f, "sentencepiece"),
            Self::Character => write!(f, "character"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub id: u32,
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Encoding {
    pub tokens: Vec<Token>,
    pub ids: Vec<u32>,
    pub attention_mask: Vec<u32>,
    pub token_type_ids: Vec<u32>,
}

impl Encoding {
    #[must_use]
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vocabulary {
    pub token_to_id: HashMap<String, u32>,
    pub id_to_token: HashMap<u32, String>,
    pub special_tokens: HashMap<String, u32>,
    pub added_tokens: Vec<AddedToken>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddedToken {
    pub id: u32,
    pub content: String,
    pub special: bool,
    pub lstrip: bool,
    pub rstrip: bool,
}

impl Vocabulary {
    pub fn new() -> Self {
        Self {
            token_to_id: HashMap::new(),
            id_to_token: HashMap::new(),
            special_tokens: HashMap::new(),
            added_tokens: Vec::new(),
        }
    }

    pub fn insert(&mut self, token: String, id: u32) {
        self.id_to_token.insert(id, token.clone());
        self.token_to_id.insert(token, id);
    }

    pub fn insert_special(&mut self, name: String, token: String, id: u32) {
        self.special_tokens.insert(name, id);
        self.insert(token, id);
    }

    #[must_use]
    pub fn get_id(&self, token: &str) -> Option<u32> {
        self.token_to_id.get(token).copied()
    }

    #[must_use]
    pub fn get_token(&self, id: u32) -> Option<&str> {
        self.id_to_token.get(&id).map(|s| s.as_str())
    }

    #[must_use]
    pub fn size(&self) -> usize {
        self.token_to_id.len()
    }

    #[must_use]
    pub fn special_token_id(&self, name: &str) -> Option<u32> {
        self.special_tokens.get(name).copied()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenizerConfig {
    pub tokenizer_type: TokenizerType,
    pub vocab_size: usize,
    pub max_length: usize,
    pub pad_token_id: Option<u32>,
    pub unk_token_id: Option<u32>,
    pub bos_token_id: Option<u32>,
    pub eos_token_id: Option<u32>,
    pub add_prefix_space: bool,
    pub lowercase: bool,
    pub strip_accents: bool,
    pub word_tokens_prefix: String,
    pub continuations: HashMap<String, u32>,
}

impl Default for TokenizerConfig {
    fn default() -> Self {
        Self {
            tokenizer_type: TokenizerType::Bpe,
            vocab_size: 32000,
            max_length: 2048,
            pad_token_id: Some(0),
            unk_token_id: Some(1),
            bos_token_id: Some(2),
            eos_token_id: Some(3),
            add_prefix_space: false,
            lowercase: false,
            strip_accents: false,
            word_tokens_prefix: String::new(),
            continuations: HashMap::new(),
        }
    }
}

#[async_trait::async_trait]
pub trait Tokenizer: Send + Sync + fmt::Debug + fmt::Display {
    fn tokenize(&self, text: &str) -> InferenceResult<Vec<Token>>;

    fn encode(&self, text: &str) -> InferenceResult<Encoding> {
        let tokens = self.tokenize(text)?;
        let ids: Vec<u32> = tokens.iter().map(|t| t.id).collect();
        let attention_mask = vec![1; ids.len()];
        let token_type_ids = vec![0; ids.len()];
        Ok(Encoding {
            tokens,
            ids,
            attention_mask,
            token_type_ids,
        })
    }

    fn decode(&self, ids: &[u32]) -> InferenceResult<String>;

    fn encode_batch(&self, texts: &[&str]) -> InferenceResult<Vec<Encoding>> {
        texts.iter().map(|t| self.encode(t)).collect()
    }

    fn decode_batch(&self, batch_ids: &[Vec<u32>]) -> InferenceResult<Vec<String>> {
        batch_ids.iter().map(|ids| self.decode(ids)).collect()
    }

    fn vocabulary(&self) -> &Vocabulary;

    fn config(&self) -> &TokenizerConfig;

    fn vocab_size(&self) -> usize {
        self.vocabulary().size()
    }

    fn token_length(&self, text: &str) -> usize {
        self.encode(text).map(|e| e.len()).unwrap_or(0)
    }

    fn truncate(&self, encoding: &mut Encoding, max_length: usize) {
        if encoding.ids.len() > max_length {
            encoding.ids.truncate(max_length);
            encoding.attention_mask.truncate(max_length);
            encoding.token_type_ids.truncate(max_length);
            encoding.tokens.truncate(max_length);
        }
    }

    fn pad(&self, encoding: &mut Encoding, target_length: usize, pad_token_id: u32) {
        while encoding.ids.len() < target_length {
            encoding.ids.push(pad_token_id);
            encoding.attention_mask.push(0);
            encoding.token_type_ids.push(0);
            encoding.tokens.push(Token {
                id: pad_token_id,
                text: "[PAD]".to_string(),
                start: 0,
                end: 0,
                score: None,
            });
        }
    }
}

use crate::error::InferenceResult;
