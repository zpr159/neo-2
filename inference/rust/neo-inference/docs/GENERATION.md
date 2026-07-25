# Generation Pipeline

## Overview

The generation pipeline transforms raw model logits into coherent text. It encompasses decoding strategies (greedy, beam search, sampling), probability manipulation (temperature, penalties), stopping conditions, and streaming output. The `GenerationEngine` provides stateless utility methods, while the `InferenceEngine` orchestrates the full autoregressive generation loop.

## Generation Parameters

```rust
pub struct GenerationParams {
    pub max_tokens: usize,              // Maximum tokens to generate
    pub temperature: f64,               // Sampling temperature (0 = greedy)
    pub top_k: Option<usize>,           // Keep only top-k tokens
    pub top_p: Option<f64>,             // Nucleus sampling threshold
    pub typical_p: Option<f64>,         // Typical sampling threshold
    pub repetition_penalty: f64,        // Penalize repeated tokens (1.0 = disabled)
    pub presence_penalty: f64,          // Penalty for tokens that appeared at all
    pub frequency_penalty: f64,         // Penalty proportional to token frequency
    pub beam_count: usize,              // Number of beams for beam search
    pub stop_sequences: Vec<String>,    // Text sequences that stop generation
    pub stop_token_ids: Vec<u32>,       // Token IDs that stop generation
    pub seed: Option<u64>,              // Random seed for reproducibility
    pub logprobs: bool,                 // Return log probabilities
    pub top_logprobs: Option<usize>,    // Number of top log probabilities per token
    pub echo: bool,                     // Include prompt in the output
}
```

## Greedy Decoding

Always selects the token with the highest logit value. Deterministic and fast, but can produce repetitive text.

```rust
let logits = vec![0.1, 0.5, 0.3, 0.05, 0.05];
let token_id = GenerationEngine::greedy_decode(&logits, 5);
// → 1 (highest logit)
```

### Algorithm

```
for each position:
    token_id = argmax(logits)
    append token to sequence
```

## Beam Search

Maintains `beam_count` candidate sequences and expands each by one token at every step. Keeps the top `beam_count` candidates by cumulative score.

```rust
let mut beams: Vec<(Vec<u32>, f32)> = Vec::new();

// Step 1: Initialize beams
let logits = vec![0.1, 0.5, 0.3, 0.05, 0.05];
GenerationEngine::beam_search_decode(&logits, 5, 3, &mut beams);
// beams = [([0], 0.1), ([1], 0.5), ([2], 0.3)]

// Step 2: Expand beams
let logits2 = vec![0.2, 0.3, 0.1, 0.25, 0.15];
GenerationEngine::beam_search_decode(&logits2, 5, 3, &mut beams);
// beams = [([1,1], 0.8), ([1,0], 0.8), ([0,1], 0.6)]
```

### Algorithm

```
for each position:
    candidates = []
    for each beam in beams:
        for each token_id in vocab:
            new_score = beam.score + logits[token_id]
            candidates.append((beam.tokens + [token_id], new_score))
    beams = top_k(candidates, beam_count)
```

## Top-K Sampling

Retains the `k` most probable tokens and samples from their renormalized distribution.

```rust
let logits = vec![0.1, 0.5, 0.3, 0.05, 0.05];
let token = GenerationEngine::top_k_sample(&mut logits.clone(), 3);
// Keeps tokens 0, 1, 2 (top 3 by logit)
// Samples proportionally to their softmax values
```

### Algorithm

```
1. Sort tokens by logit value (descending)
2. Keep top-k tokens
3. Compute softmax over remaining tokens
4. Sample from the resulting distribution
```

## Top-P (Nucleus) Sampling

Retains the smallest set of tokens whose cumulative probability ≥ `p`. Adapts the number of tokens considered based on the distribution's confidence.

```rust
let logits = vec![0.1, 0.5, 0.3, 0.05, 0.05];
let token = GenerationEngine::top_p_sample(&mut logits.clone(), 0.9);
// Keeps tokens until cumulative probability ≥ 0.9
```

### Algorithm

```
1. Sort tokens by logit value (descending)
2. Compute softmax probabilities
3. Accumulate probabilities until cumulative ≥ p
4. Retain only accumulated tokens
5. Renormalize and sample
```

### Comparison: Top-K vs Top-P

| Strategy | Fixed/Adaptive | Behavior |
|----------|----------------|----------|
| Top-K | Fixed (k tokens) | Always considers exactly k tokens regardless of distribution shape |
| Top-P | Adaptive | Considers fewer tokens when confident, more when uncertain |

## Typical Sampling

Selects tokens whose information content is close to the expected (entropy-weighted) value. Reduces degenerate outputs by filtering out both too-probable and too-improbable tokens.

```rust
// Not directly exposed as a standalone method, but configurable via:
params.typical_p = Some(0.95);
```

## Temperature Scaling

Divides logits by a temperature value before softmax. Controls the "randomness" of the output.

```rust
let mut logits = vec![1.0, 2.0, 3.0, 0.5];
GenerationEngine::apply_temperature(&mut logits, 0.5);
// Logits are amplified: differences become more pronounced
// → More deterministic output
```

### Temperature Effects

| Temperature | Effect |
|-------------|--------|
| 0.0 | Greedy (argmax) — most deterministic |
| 0.1–0.5 | Very focused — low randomness |
| 0.7–1.0 | Balanced — good for chat and creative text |
| 1.0 | Standard softmax — no modification |
| 1.5–2.0 | Very diverse — high randomness, may be incoherent |

```
logits_scaled[i] = logits[i] / temperature
probs = softmax(logits_scaled)
```

## Repetition Penalty

Divides the logit of each previously generated token by the penalty factor. Prevents the model from repeating the same tokens.

```rust
let logits = vec![0.5, 1.0, 0.8, 0.3];
let generated = vec![0, 2]; // Already generated tokens 0 and 2

GenerationEngine::apply_repetition_penalty(&mut logits, &generated, 1.3);
// logits[0] /= 1.3 → 0.385
// logits[2] /= 1.3 → 0.615
```

### Behavior

- **penalty = 1.0**: No effect (disabled)
- **penalty > 1.0**: Reduces probability of repeated tokens
- For positive logits: `logit /= penalty`
- For negative logits: `logit *= penalty`

## Presence Penalty

Subtracts a fixed penalty for each token that has appeared at least once in the generated sequence, regardless of how many times it appeared.

```rust
let logits = vec![0.5, 1.0, 0.8, 0.3];
let generated = vec![0, 0, 0, 2]; // Token 0 appeared 3x, token 2 appeared 1x

GenerationEngine::apply_presence_penalty(&mut logits, &generated, 0.5);
// logits[0] -= 0.5 → 0.0  (penalized because it appeared)
// logits[2] -= 0.5 → 0.3  (penalized because it appeared)
// logits[1] unchanged (never appeared)
```

## Frequency Penalty

Subtracts a penalty proportional to how many times each token has appeared. More frequent tokens receive larger penalties.

```rust
let logits = vec![0.5, 1.0, 0.8, 0.3];
let generated = vec![0, 0, 0, 2]; // Token 0 appeared 3x, token 2 appeared 1x

GenerationEngine::apply_frequency_penalty(&mut logits, &generated, 0.3);
// logits[0] -= 0.3 * 3 = 0.5 - 0.9 = -0.4
// logits[2] -= 0.3 * 1 = 0.8 - 0.3 = 0.5
// logits[1] unchanged
```

## Grammar Constraints

Grammar constraints can be enforced through the stop sequences and stop token IDs mechanism:

```rust
let params = GenerationParams {
    stop_sequences: vec![
        "```".to_string(),           // Stop at code block end
        "\n\n\n".to_string(),        // Stop at triple newline
        "User:".to_string(),         // Stop at next user turn
    ],
    stop_token_ids: vec![2],         // Stop at EOS token
    ..Default::default()
};
```

## Stop Sequences

Generation stops when any of the following conditions are met:

1. **EOS token** is generated (token ID in `stop_token_ids`)
2. **Stop sequence** appears in the generated text
3. **Max tokens** limit is reached
4. **Request is cancelled**

```rust
pub enum FinishReason {
    StopToken,      // EOS or stop token ID generated
    StopSequence,   // Stop text sequence found
    MaxTokens,      // Reached max_tokens limit
    Cancelled,      // Client cancelled the request
    Error,          // An error occurred during generation
}
```

### Checking Stop Conditions

```rust
// Check if any stop sequence appears in the text
let stopped = GenerationEngine::check_stop_sequences(
    "The model generated some text. User:",
    &["User:".to_string(), "Assistant:".to_string()],
);
// → true

// Check if a token is a stop token
let stopped = GenerationEngine::check_stop_token(2, &[2, 32000]);
// → true (token 2 is EOS)
```

## Streaming Generation

The `StreamChunk` type represents a single token in a streaming response:

```rust
pub struct StreamChunk {
    pub token_id: u32,                     // The generated token ID
    pub token_text: String,                // The decoded text
    pub logprob: Option<f64>,              // Optional log probability
    pub finish_reason: Option<FinishReason>, // Set on final chunk
}
```

### Streaming Flow

```
Token 1 → StreamChunk { token_text: "Hello", finish_reason: None }
Token 2 → StreamChunk { token_text: " world", finish_reason: None }
Token 3 → StreamChunk { token_text: "!", finish_reason: None }
Token 4 → StreamChunk { token_text: "", finish_reason: Some(StopToken) }
```

### Engine-Level Streaming

```rust
let mut receiver = engine.inference_stream(
    model_id,
    input_ids,
    attention_mask,
    GenerationParams {
        max_tokens: 256,
        temperature: 0.7,
        ..Default::default()
    },
).await?;

while let Some(chunk) = receiver.recv().await {
    let chunk = chunk?;
    print!("{}", chunk.token_text);
    if let Some(reason) = &chunk.finish_reason {
        eprintln!("\n[Finished: {}]", reason);
        break;
    }
}
```

## Autoregressive Generation Loop

The `InferenceEngine::inference` method implements the full generation loop:

```
1. Tokenize input → input_ids, attention_mask
2. Send input_ids to backend → get logits
3. Apply temperature scaling
4. Apply repetition/presence/frequency penalties
5. Convert logits → probabilities (softmax)
6. Apply top-k filtering
7. Apply top-p (nucleus) filtering
8. Sample or greedy-select next token
9. Check stop conditions
10. If not stopped: set input_ids = [next_token], goto step 2
11. Return GenerationResult with text, tokens, usage, finish_reason
```

## GenerationResult

```rust
pub struct GenerationResult {
    pub text: String,                    // Concatenated output text
    pub tokens: Vec<u32>,               // Generated token IDs
    pub token_texts: Vec<String>,        // Individual token texts
    pub logprobs: Option<Vec<f64>>,      // Log probabilities per token
    pub finish_reason: FinishReason,     // Why generation stopped
    pub usage: TokenUsage,               // Token counts
}

pub struct TokenUsage {
    pub prompt_tokens: u64,       // Tokens in the input
    pub completion_tokens: u64,   // Tokens generated
    pub total_tokens: u64,        // prompt + completion
}
```
