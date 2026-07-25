use crate::error::InferenceResult;
use super::{Tokenizer, Token, Vocabulary, TokenizerConfig, TokenizerType};

#[derive(Debug)]
pub struct CharacterTokenizer {
    vocab: Vocabulary,
    config: TokenizerConfig,
}

impl CharacterTokenizer {
    pub fn new() -> Self {
        let mut vocab = Vocabulary::new();
        for i in 0..256u32 {
            let ch = char::from_u32(i).unwrap_or(' ');
            vocab.insert(ch.to_string(), i);
        }
        vocab.insert_special("unk".to_string(), "<UNK>".to_string(), 256);
        vocab.insert_special("pad".to_string(), "<PAD>".to_string(), 257);
        vocab.insert_special("bos".to_string(), "<BOS>".to_string(), 258);
        vocab.insert_special("eos".to_string(), "<EOS>".to_string(), 259);
        Self {
            vocab,
            config: TokenizerConfig {
                tokenizer_type: TokenizerType::Character,
                vocab_size: 260,
                ..Default::default()
            },
        }
    }

    pub fn from_string_vocab(strings: &[&str]) -> Self {
        let mut vocab = Vocabulary::new();
        for (i, s) in strings.iter().enumerate() {
            vocab.insert(s.to_string(), i as u32);
        }
        vocab.insert_special("unk".to_string(), "<UNK>".to_string(), strings.len() as u32);
        Self {
            vocab,
            config: TokenizerConfig {
                tokenizer_type: TokenizerType::Character,
                vocab_size: strings.len() + 1,
                ..Default::default()
            },
        }
    }
}

impl Default for CharacterTokenizer {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for CharacterTokenizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CharacterTokenizer(vocab={})", self.vocab.size())
    }
}

impl Tokenizer for CharacterTokenizer {
    fn tokenize(&self, text: &str) -> InferenceResult<Vec<Token>> {
        let unk_id = self.vocab.get_id("<UNK>").unwrap_or(0);
        let mut tokens = Vec::new();
        for (i, ch) in text.char_indices() {
            let id = self.vocab.get_id(&ch.to_string()).unwrap_or(unk_id);
            tokens.push(Token {
                id,
                text: ch.to_string(),
                start: i,
                end: i + ch.len_utf8(),
                score: None,
            });
        }
        Ok(tokens)
    }

    fn decode(&self, ids: &[u32]) -> InferenceResult<String> {
        let mut result = String::new();
        for &id in ids {
            if let Some(token) = self.vocab.get_token(id) {
                if token == "<UNK>" || token == "<PAD>" || token == "<BOS>" || token == "<EOS>" {
                    continue;
                }
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
