use crate::error::InferenceResult;
use super::{Tokenizer, Token, Vocabulary, TokenizerConfig, TokenizerType};

#[derive(Debug)]
pub struct SentencePieceTokenizer {
    vocab: Vocabulary,
    config: TokenizerConfig,
    model_type: SpModelType,
}

#[derive(Debug, Clone, Copy)]
enum SpModelType {
    Unigram,
    Bpe,
}

impl SentencePieceTokenizer {
    pub fn new() -> Self {
        let mut vocab = Vocabulary::new();
        vocab.insert_special("unk".to_string(), "<unk>".to_string(), 0);
        vocab.insert_special("bos".to_string(), "<s>".to_string(), 1);
        vocab.insert_special("eos".to_string(), "</s>".to_string(), 2);
        vocab.insert_special("pad".to_string(), "<pad>".to_string(), 3);
        vocab.insert("<pad>".to_string(), 3);
        for i in 4..32000 {
            vocab.insert(format!("<token_{}>", i), i);
        }
        Self {
            vocab,
            config: TokenizerConfig {
                tokenizer_type: TokenizerType::SentencePiece,
                vocab_size: 32000,
                ..Default::default()
            },
            model_type: SpModelType::Unigram,
        }
    }

    pub fn from_vocab(vocab: Vocabulary, model_type: &str) -> Self {
        let sp_type = match model_type {
            "bpe" => SpModelType::Bpe,
            _ => SpModelType::Unigram,
        };
        Self {
            config: TokenizerConfig {
                tokenizer_type: TokenizerType::SentencePiece,
                vocab_size: vocab.size(),
                ..Default::default()
            },
            vocab,
            model_type: sp_type,
        }
    }

    fn unigram_tokenize(&self, text: &str) -> Vec<u32> {
        let mut tokens = Vec::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let mut best_id = self.vocab.get_id("<unk>").unwrap_or(0);
            let mut best_len = 1;
            let mut max_len = chars.len().min(i + 32);
            for end in (i + 1)..=max_len {
                let candidate: String = chars[i..end].iter().collect();
                if let Some(id) = self.vocab.get_id(&candidate) {
                    best_id = id;
                    best_len = end - i;
                }
            }
            tokens.push(best_id);
            i += best_len;
        }
        tokens
    }

    fn bpe_tokenize(&self, text: &str) -> Vec<u32> {
        let mut tokens = Vec::new();
        for word in text.split_whitespace() {
            let chars: Vec<char> = word.chars().collect();
            for (i, &ch) in chars.iter().enumerate() {
                let prefix = if i > 0 { "▁" } else { "" };
                let token = format!("{}{}", prefix, ch);
                if let Some(id) = self.vocab.get_id(&token) {
                    tokens.push(id);
                } else {
                    tokens.push(self.vocab.get_id("<unk>").unwrap_or(0));
                }
            }
        }
        tokens
    }
}

impl Default for SentencePieceTokenizer {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SentencePieceTokenizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SentencePieceTokenizer(vocab={})", self.vocab.size())
    }
}

impl Tokenizer for SentencePieceTokenizer {
    fn tokenize(&self, text: &str) -> InferenceResult<Vec<Token>> {
        let ids = match self.model_type {
            SpModelType::Unigram => self.unigram_tokenize(text),
            SpModelType::Bpe => self.bpe_tokenize(text),
        };
        let mut tokens = Vec::new();
        let mut offset = 0;
        for &id in &ids {
            let token_text = self.vocab.get_token(id).unwrap_or("<unk>").to_string();
            tokens.push(Token {
                id,
                text: token_text.clone(),
                start: offset,
                end: offset + token_text.len(),
                score: None,
            });
            offset += token_text.len();
        }
        Ok(tokens)
    }

    fn decode(&self, ids: &[u32]) -> InferenceResult<String> {
        let mut result = String::new();
        for &id in ids {
            if let Some(token) = self.vocab.get_token(id) {
                if token == "<s>" || token == "</s>" || token == "<pad>" {
                    continue;
                }
                result.push_str(token);
            }
        }
        Ok(result.replace('▁', " ").trim().to_string())
    }

    fn vocabulary(&self) -> &Vocabulary {
        &self.vocab
    }

    fn config(&self) -> &TokenizerConfig {
        &self.config
    }
}
