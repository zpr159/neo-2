use neo_inference::tokenizer::{Encoding, Tokenizer, TokenizerConfig, TokenizerType};
use neo_inference::tokenizer::bpe::BpeTokenizer;
use neo_inference::tokenizer::wordpiece::WordPieceTokenizer;
use neo_inference::tokenizer::sentencepiece::SentencePieceTokenizer;
use neo_inference::tokenizer::character::CharacterTokenizer;

// --- BpeTokenizer ---

#[test]
fn test_bpe_tokenize() {
    let tok = BpeTokenizer::new();
    let tokens = tok.tokenize("hello world").unwrap();
    assert!(!tokens.is_empty());
    for t in &tokens {
        assert!(!t.text.is_empty());
    }
}

#[test]
fn test_bpe_decode() {
    let tok = BpeTokenizer::new();
    let encoding = tok.encode("hello").unwrap();
    let decoded = tok.decode(&encoding.ids).unwrap();
    assert!(!decoded.is_empty());
}

#[test]
fn test_bpe_encode_decode_roundtrip() {
    let tok = BpeTokenizer::new();
    let original = "abc";
    let encoding = tok.encode(original).unwrap();
    let decoded = tok.decode(&encoding.ids).unwrap();
    let chars: String = original.chars().filter(|c| *c != ' ').collect();
    assert_eq!(decoded, chars);
}

#[test]
fn test_bpe_vocab_size() {
    let tok = BpeTokenizer::new();
    assert!(tok.vocab_size() >= 256);
}

#[test]
fn test_bpe_config() {
    let tok = BpeTokenizer::new();
    assert_eq!(tok.config().tokenizer_type, TokenizerType::Bpe);
}

// --- WordPieceTokenizer ---

#[test]
fn test_wordpiece_tokenize() {
    let tok = WordPieceTokenizer::new();
    let tokens = tok.tokenize("test word").unwrap();
    assert!(!tokens.is_empty());
}

#[test]
fn test_wordpiece_decode() {
    let tok = WordPieceTokenizer::new();
    let encoding = tok.encode("hello").unwrap();
    let decoded = tok.decode(&encoding.ids).unwrap();
    assert!(!decoded.is_empty());
}

#[test]
fn test_wordpiece_config() {
    let tok = WordPieceTokenizer::new();
    assert_eq!(tok.config().tokenizer_type, TokenizerType::WordPiece);
    assert_eq!(tok.config().word_tokens_prefix, "##");
}

// --- SentencePieceTokenizer ---

#[test]
fn test_sentencepiece_tokenize() {
    let tok = SentencePieceTokenizer::new();
    let tokens = tok.tokenize("hello world").unwrap();
    assert!(!tokens.is_empty());
}

#[test]
fn test_sentencepiece_decode() {
    let tok = SentencePieceTokenizer::new();
    let encoding = tok.encode("hello").unwrap();
    let decoded = tok.decode(&encoding.ids).unwrap();
    assert!(!decoded.is_empty());
}

#[test]
fn test_sentencepiece_config() {
    let tok = SentencePieceTokenizer::new();
    assert_eq!(tok.config().tokenizer_type, TokenizerType::SentencePiece);
    assert!(tok.vocab_size() >= 32000);
}

// --- CharacterTokenizer ---

#[test]
fn test_character_tokenize() {
    let tok = CharacterTokenizer::new();
    let tokens = tok.tokenize("abc").unwrap();
    assert_eq!(tokens.len(), 3);
    assert_eq!(tokens[0].text, "a");
    assert_eq!(tokens[1].text, "b");
    assert_eq!(tokens[2].text, "c");
}

#[test]
fn test_character_decode() {
    let tok = CharacterTokenizer::new();
    let encoding = tok.encode("xyz").unwrap();
    let decoded = tok.decode(&encoding.ids).unwrap();
    assert_eq!(decoded, "xyz");
}

#[test]
fn test_character_config() {
    let tok = CharacterTokenizer::new();
    assert_eq!(tok.config().tokenizer_type, TokenizerType::Character);
    assert!(tok.vocab_size() >= 256);
}

// --- Encoding pad and truncate ---

#[test]
fn test_encoding_pad() {
    let tok = BpeTokenizer::new();
    let mut enc = tok.encode("hi").unwrap();
    let original_len = enc.len();
    assert!(original_len < 10);
    tok.pad(&mut enc, 10, 1);
    assert_eq!(enc.ids.len(), 10);
    assert_eq!(enc.attention_mask.len(), 10);
    assert_eq!(enc.token_type_ids.len(), 10);
    assert_eq!(enc.tokens.len(), 10);
    for i in original_len..10 {
        assert_eq!(enc.ids[i], 1);
        assert_eq!(enc.attention_mask[i], 0);
    }
}

#[test]
fn test_encoding_truncate() {
    let tok = BpeTokenizer::new();
    let mut enc = tok.encode("hello world").unwrap();
    assert!(enc.len() > 2);
    tok.truncate(&mut enc, 2);
    assert_eq!(enc.ids.len(), 2);
    assert_eq!(enc.attention_mask.len(), 2);
    assert_eq!(enc.tokens.len(), 2);
}

#[test]
fn test_encoding_truncate_noop_when_smaller() {
    let tok = BpeTokenizer::new();
    let mut enc = tok.encode("hi").unwrap();
    let original_len = enc.len();
    tok.truncate(&mut enc, 100);
    assert_eq!(enc.len(), original_len);
}

// --- Batch encode/decode ---

#[tokio::test]
async fn test_batch_encode() {
    let tok = BpeTokenizer::new();
    let texts = vec!["hello", "world", "test"];
    let batch = tok.encode_batch(&texts).unwrap();
    assert_eq!(batch.len(), 3);
    for enc in &batch {
        assert!(!enc.ids.is_empty());
    }
}

#[tokio::test]
async fn test_batch_decode() {
    let tok = BpeTokenizer::new();
    let ids_batch: Vec<Vec<u32>> = vec![vec![72, 101, 108], vec![119, 111, 114]];
    let decoded = tok.decode_batch(&ids_batch).unwrap();
    assert_eq!(decoded.len(), 2);
}

#[test]
fn test_encoding_len_and_is_empty() {
    let tok = BpeTokenizer::new();
    let enc = tok.encode("test").unwrap();
    assert!(!enc.is_empty());
    assert_eq!(enc.len(), enc.ids.len());
}

#[test]
fn test_token_length() {
    let tok = BpeTokenizer::new();
    let len = tok.token_length("hello");
    assert!(len > 0);
}
