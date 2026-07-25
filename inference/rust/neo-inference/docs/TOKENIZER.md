# Tokenizer Architecture

## Overview

The `neo-inference` crate provides a universal tokenizer interface with four built-in implementations: BPE, WordPiece, SentencePiece, and Character. All tokenizers implement the `Tokenizer` trait, making them interchangeable and composable. The tokenizer layer handles text-to-token conversion, vocabulary management, and encoding/decoding for model inference.

## Universal Tokenizer Interface

The `Tokenizer` trait defines the contract for all tokenizer implementations:

```rust
#[async_trait]
pub trait Tokenizer: Send + Sync + fmt::Debug + fmt::Display {
    /// Split text into tokens with IDs and offsets.
    fn tokenize(&self, text: &str) -> InferenceResult<Vec<Token>>;

    /// Encode text into an Encoding (IDs, attention mask, type IDs).
    fn encode(&self, text: &str) -> InferenceResult<Encoding> {
        let tokens = self.tokenize(text)?;
        let ids: Vec<u32> = tokens.iter().map(|t| t.id).collect();
        let attention_mask = vec![1; ids.len()];
        let token_type_ids = vec![0; ids.len()];
        Ok(Encoding { tokens, ids, attention_mask, token_type_ids })
    }

    /// Decode token IDs back into text.
    fn decode(&self, ids: &[u32]) -> InferenceResult<String>;

    /// Encode multiple texts in batch.
    fn encode_batch(&self, texts: &[&str]) -> InferenceResult<Vec<Encoding>>;

    /// Decode multiple ID sequences in batch.
    fn decode_batch(&self, batch_ids: &[Vec<u32>]) -> InferenceResult<Vec<String>>;

    /// Access the vocabulary.
    fn vocabulary(&self) -> &Vocabulary;

    /// Access the tokenizer configuration.
    fn config(&self) -> &TokenizerConfig;

    /// Get the vocabulary size.
    fn vocab_size(&self) -> usize;

    /// Count tokens in a text without producing a full encoding.
    fn token_length(&self, text: &str) -> usize;

    /// Truncate an encoding to a maximum length.
    fn truncate(&self, encoding: &mut Encoding, max_length: usize);

    /// Pad an encoding to a target length.
    fn pad(&self, encoding: &mut Encoding, target_length: usize, pad_token_id: u32);
}
```

### Core Types

```rust
pub struct Token {
    pub id: u32,           // Token ID in the vocabulary
    pub text: String,      // The token's text representation
    pub start: usize,      // Character offset start
    pub end: usize,        // Character offset end
    pub score: Option<f32>, // Optional token probability
}

pub struct Encoding {
    pub tokens: Vec<Token>,          // Full token information
    pub ids: Vec<u32>,               // Just the token IDs
    pub attention_mask: Vec<u32>,    // 1 for real tokens, 0 for padding
    pub token_type_ids: Vec<u32>,    // Segment IDs (for multi-segment tasks)
}

pub struct Vocabulary {
    pub token_to_id: HashMap<String, u32>,
    pub id_to_token: HashMap<u32, String>,
    pub special_tokens: HashMap<String, u32>,
    pub added_tokens: Vec<AddedToken>,
}
```

### TokenizerConfig

```rust
pub struct TokenizerConfig {
    pub tokenizer_type: TokenizerType,   // Bpe, WordPiece, SentencePiece, Character
    pub vocab_size: usize,               // Total vocabulary size
    pub max_length: usize,               // Maximum sequence length
    pub pad_token_id: Option<u32>,       // Padding token ID
    pub unk_token_id: Option<u32>,       // Unknown token ID
    pub bos_token_id: Option<u32>,       // Beginning-of-sequence token ID
    pub eos_token_id: Option<u32>,       // End-of-sequence token ID
    pub add_prefix_space: bool,          // Add space before first token
    pub lowercase: bool,                 // Lowercase input text
    pub strip_accents: bool,             // Remove accent marks
    pub word_tokens_prefix: String,      // Subword prefix (e.g., "##" for WordPiece)
    pub continuations: HashMap<String, u32>, // Continuation tokens
}
```

## BPE Tokenizer (Byte-Pair Encoding)

**File:** `tokenizer/bpe.rs`

BPE iteratively merges the most frequent byte pairs in the training data. It is the most widely used tokenizer for large language models (GPT, LLaMA, Mistral, Qwen, etc.).

### How It Works

1. Start with individual characters (or bytes) as tokens
2. Find the most frequent adjacent pair
3. Merge that pair into a new token
4. Repeat until vocabulary is full

```rust
// Create a BPE tokenizer
let tokenizer = BpeTokenizer::new();

// Or load from vocabulary and merge rules
let tokenizer = BpeTokenizer::from_vocab_and_merges(vocab, merges);

// Tokenize
let tokens = tokenizer.tokenize("Hello, world!")?;
// → [Token { id: 72, text: "H" }, Token { id: 100, text: "ello" }, ...]

// Encode
let encoding = tokenizer.encode("Hello, world!")?;
// encoding.ids = [72, 100, 11, 390, 995, 0]

// Decode
let text = tokenizer.decode(&[72, 100, 11, 390, 995])?;
// → "Hello, world!"
```

### Merge Algorithm

```
Input: "unbelievable"
Characters: ['u', 'n', 'b', 'e', 'l', 'i', 'e', 'v', 'a', 'b', 'l', 'e']

Step 1: Merge most frequent pair (e.g., "ab" → "ab")
  ['u', 'n', 'b', 'e', 'l', 'i', 'e', 'v', 'able']

Step 2: Merge next most frequent (e.g., "el" → "el")
  ['u', 'n', 'b', 'el', 'i', 'e', 'v', 'able']

Step 3: Continue until no more merges apply
  ['un', 'believ', 'able']

Lookup IDs: [un_id, believ_id, able_id]
```

## WordPiece Tokenizer

**File:** `tokenizer/wordpiece.rs`

WordPiece uses a greedy longest-match-first algorithm with a `##` prefix for subword continuations. It is commonly used by BERT and related models.

### How It Works

1. For each word, try to find the longest matching subword in the vocabulary
2. If a subword is not at the start of the word, prefix it with `##`
3. If no subword matches, output `[UNK]`

```rust
let tokenizer = WordPieceTokenizer::new();
// Or: WordPieceTokenizer::from_vocab(vocab);

let tokens = tokenizer.tokenize("unbelievable")?;
// → [Token { text: "un" }, Token { text: "##beli" }, Token { text: "##evable" }]

// Decode handles ## prefix merging
let text = tokenizer.decode(&[un_id, beli_id, evable_id])?;
// → "unbelievable"
```

### Special Tokens

| Token | ID | Purpose |
|-------|----|---------|
| `[UNK]` | 0 | Unknown token |
| `[PAD]` | 1 | Padding |
| `[CLS]` | 2 | Classification token (start of sequence) |
| `[SEP]` | 3 | Separator token |
| `[MASK]` | 4 | Masked language model token |

## SentencePiece Tokenizer

**File:** `tokenizer/sentencepiece.rs`

SentencePiece treats text as a raw stream of Unicode characters (no pre-tokenization by whitespace). It supports both Unigram and BPE model types. Used by LLaMA, T5, and many multilingual models.

### How It Works

**Unigram mode** (default):
1. Start with a large vocabulary
2. For each position, find the longest matching subword
3. Greedily select the best match

**BPE mode:**
1. Pre-tokenize by splitting on whitespace
2. Apply BPE merges with the `▁` (lower-one-eighth-block) marker for spaces

```rust
let tokenizer = SentencePieceTokenizer::new();
// Or: SentencePieceTokenizer::from_vocab(vocab, "unigram");

let tokens = tokenizer.tokenize("Hello world")?;
// Unigram: → [Token { text: "▁Hello" }, Token { text: "▁world" }]
// BPE:     → [Token { text: "▁Hello" }, Token { text: "▁world" }]

// Decode replaces ▁ with spaces
let text = tokenizer.decode(&[hello_id, world_id])?;
// → "Hello world"
```

### Special Tokens

| Token | ID | Purpose |
|-------|----|---------|
| `<unk>` | 0 | Unknown |
| `<s>` | 1 | Beginning of sequence |
| `</s>` | 2 | End of sequence |
| `<pad>` | 3 | Padding |

## Character Tokenizer

**File:** `tokenizer/character.rs`

The simplest tokenizer: each character becomes a separate token. Useful for character-level models, spelling tasks, or as a fallback.

```rust
let tokenizer = CharacterTokenizer::new();

let tokens = tokenizer.tokenize("Hi")?;
// → [Token { id: 72, text: "H" }, Token { id: 105, text: "i" }]

// Or create from a custom string vocabulary
let tokenizer = CharacterTokenizer::from_string_vocab(&["a", "b", "c", "hello"]);
```

### Special Tokens

| Token | ID | Purpose |
|-------|----|---------|
| `<UNK>` | 256 | Unknown |
| `<PAD>` | 257 | Padding |
| `<BOS>` | 258 | Beginning of sequence |
| `<EOS>` | 259 | End of sequence |

## Vocabulary Management

The `Vocabulary` struct provides bidirectional token↔ID mapping:

```rust
let mut vocab = Vocabulary::new();

// Add tokens
vocab.insert("hello".to_string(), 0);
vocab.insert("world".to_string(), 1);

// Add special tokens
vocab.insert_special("unk".to_string(), "[UNK]".to_string(), 2);
vocab.insert_special("pad".to_string(), "[PAD]".to_string(), 3);

// Lookup
let id = vocab.get_id("hello");      // Some(0)
let token = vocab.get_token(1);       // Some("world")
let size = vocab.size();              // 4

// Special token lookup
let unk_id = vocab.special_token_id("unk");  // Some(2)
```

### Added Tokens

For tokens that need special handling (e.g., multi-word special tokens):

```rust
pub struct AddedToken {
    pub id: u32,
    pub content: String,
    pub special: bool,      // Whether this is a special token
    pub lstrip: bool,       // Strip leading whitespace before matching
    pub rstrip: bool,       // Strip trailing whitespace after matching
}
```

## Streaming Tokenization

While the core `Tokenizer` trait is synchronous, the engine supports streaming tokenization by processing chunks of text incrementally:

```rust
// Split input into chunks for streaming
let chunks: Vec<&str> = text.split_inclusive(' ').collect();

let mut all_tokens = Vec::new();
for chunk in chunks {
    let tokens = tokenizer.tokenize(chunk)?;
    all_tokens.extend(tokens);
}

// Or use encode_batch for parallel encoding
let encodings = tokenizer.encode_batch(&["text 1", "text 2", "text 3"])?;
```

## Batch Encoding/Decoding

All tokenizers support batch operations for throughput:

```rust
// Batch encode
let texts = &["Hello", "World", "Test"];
let encodings = tokenizer.encode_batch(texts)?;
// → [Encoding, Encoding, Encoding]

// Batch decode
let id_sequences = vec![vec![1, 2, 3], vec![4, 5, 6]];
let decoded = tokenizer.decode_batch(&id_sequences)?;
// → ["token1 token2 token3", "token4 token5 token6"]
```

## Truncation and Padding

```rust
let mut encoding = tokenizer.encode("This is a long text that needs truncation")?;

// Truncate to max 10 tokens
tokenizer.truncate(&mut encoding, 10);
assert!(encoding.ids.len() <= 10);

// Pad to exactly 16 tokens
tokenizer.pad(&mut encoding, 16, pad_token_id);
assert!(encoding.ids.len() == 16);
// attention_mask: [1,1,...,1,0,0,...,0] (1s for real tokens, 0s for padding)
```
