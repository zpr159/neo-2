# Model Lifecycle

## Overview

Models in `neo-inference` progress through a well-defined lifecycle: **Registration** → **Loading** → **Inference** → **Unloading**. The system supports reference-counted lifetime management, hot swapping, versioning, and an optional model repository for local/remote model storage with integrity verification.

## Lifecycle Diagram

```
                ┌─────────────┐
                │  Repository  │  (discovery, download, caching)
                └──────┬──────┘
                       │
                ┌──────▼──────┐
  ┌─────────────│ Registration │─────────────┐
  │             └──────┬──────┘             │
  │                    │                     │
  │             ┌──────▼──────┐             │
  │             │   Loading    │             │
  │             │ (backend     │             │
  │             │  selection,  │             │
  │             │  memory      │             │
  │             │  alloc)      │             │
  │             └──────┬──────┘             │
  │                    │                     │
  │             ┌──────▼──────┐             │
  │             │   Active     │◄────────────│ (reference count > 0)
  │             │   (inference) │             │
  │             └──────┬──────┘             │
  │                    │                     │
  │             ┌──────▼──────┐             │
  │             │  Unloading   │             │
  │             │ (ref count   │             │
  │             │  → 0, free   │             │
  │             │  memory)     │             │
  │             └──────┬──────┘             │
  │                    │                     │
  │             ┌──────▼──────┐             │
  └────────────►│  Eviction /  │◄────────────┘
                │  Cleanup     │
                └─────────────┘
```

## Model Registration

Registration associates model metadata with the system. It does **not** load weights — it only records the model's identity, format, and location.

### Model Metadata

```rust
pub struct ModelMetadata {
    pub id: ModelId,                       // UUID identifier
    pub name: String,                      // Human-readable name (e.g. "llama-3-8b")
    pub version: ModelVersion,             // Semantic version (major.minor.patch)
    pub architecture: ModelArchitecture,   // Model type (Llama, Mistral, GPT, etc.)
    pub format: ModelFormat,               // SafeTensors, GGUF, ONNX, etc.
    pub quantization: QuantizationType,    // Fp16, Int8, Int4, GGUF Q4_K, etc.
    pub path: String,                      // Filesystem or URI path to weights
    pub sha256: Option<String>,            // Optional integrity hash
    pub file_size: u64,                    // Size in bytes
    pub parameter_count: u64,              // Total parameter count
    pub num_layers: u32,
    pub hidden_size: u32,
    pub num_attention_heads: u32,
    pub num_kv_heads: Option<u32>,
    pub intermediate_size: Option<u32>,
    pub vocab_size: u32,
    pub max_position_embeddings: u32,
    pub context_length: u32,
    pub rope_theta: Option<f64>,
    pub eos_token_id: Option<u32>,
    pub bos_token_id: Option<u32>,
    pub pad_token_id: Option<u32>,
    pub aliases: Vec<String>,              // Alternative names
    pub tags: Vec<String>,                 // Categorization tags
    pub dependencies: Vec<String>,         // Required models or resources
    pub metadata: HashMap<String, Value>,  // Arbitrary key-value metadata
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### Registering a Model

```rust
// Via ModelManager
let manager = ModelManager::new(ModelManagerConfig::default());
manager.register(metadata)?;

// Via InferenceEngine (also loads the model)
let engine = InferenceEngine::new(EngineConfig::default());
engine.initialize().await?;
let model_id = engine.load_model(metadata).await?;
```

### ModelId

Every model receives a `ModelId` (UUID v4) upon creation. Models can also be resolved by name via aliases:

```rust
// Register an alias
manager.add_alias("llama-chat".to_string(), model_id);

// Resolve by alias
let id = manager.get_by_alias("llama-chat");
```

## Model Loading

Loading transitions a model from metadata-only to weights-in-memory. The engine performs:

1. **Backend selection** — Finds the best backend based on format compatibility and priority
2. **Memory check** — Verifies the system has enough memory (via `ModelManager::can_load_more`)
3. **Weight loading** — The selected backend loads weights into device memory (CPU RAM, GPU VRAM, etc.)
4. **Registration** — The model ID is mapped to the backend type and metadata is stored

```rust
// Engine-level loading
let model_id = engine.load_model(metadata).await?;

// Memory estimation
let estimated_bytes = metadata.estimated_memory_bytes();
// Uses: parameter_count * (quantization.bits_per_weight / 8)
```

### Backend Selection

The engine's `select_backend` method:

1. Iterates all initialized backends
2. Filters to those that are available and support the model's format
3. Selects the highest-priority match
4. Falls back to the highest-priority available backend if no format match exists

### Memory Allocation

Each backend manages its own memory. The `ModelSlot` tracks per-model memory:

```rust
pub struct ModelSlot {
    pub metadata: ModelMetadata,
    ref_count: AtomicU64,           // Active inference references
    pub loaded_at: Option<DateTime<Utc>>,
    memory_allocated: AtomicU64,    // Bytes allocated for this model
    pub is_hot_swapped: bool,
}
```

The `ModelManager` maintains aggregate totals:

```rust
// Check capacity
if manager.can_load_more() {
    manager.register(metadata)?;
}

// Current usage
let total_memory = manager.total_memory_used();
let loaded_count = manager.loaded_count();
```

## Model Inference

### Request Lifecycle

```
Client Request
    │
    ▼
┌─────────────────┐
│ Validate Request │  (model exists, params valid)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Tokenize Input   │  (text → token IDs)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Schedule Request │  (priority queue, concurrency check)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Backend.inference│  (forward pass on device)
│  or stream       │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Decode Tokens    │  (token IDs → text)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Return Response  │  (with usage stats, latency)
└─────────────────┘
```

### Synchronous Inference

```rust
let result = engine.inference(
    model_id,
    input_ids,
    attention_mask,
    GenerationParams {
        max_tokens: 256,
        temperature: 0.7,
        top_p: Some(0.9),
        ..Default::default()
    },
).await?;

assert_eq!(result.finish_reason, FinishReason::StopToken);
println!("Generated: {}", result.text);
println!("Tokens: {}", result.usage.completion_tokens);
```

### Streaming Inference

```rust
let mut receiver = engine.inference_stream(
    model_id,
    input_ids,
    attention_mask,
    GenerationParams::default(),
).await?;

while let Some(chunk) = receiver.recv().await {
    let chunk = chunk?;
    print!("{}", chunk.token_text);
    if let Some(reason) = &chunk.finish_reason {
        println!("\n[Finished: {}]", reason);
        break;
    }
}
```

## Model Unloading

Unloading decrements the reference count. A model is only fully removed when its reference count reaches zero.

```rust
// Decrement reference
manager.unload(model_id)?;

// Check reference count
let slot = manager.get_metadata(model_id);
// slot.ref_count == 0 means eligible for eviction
```

### Reference Counting

The `ModelSlot` uses atomic reference counting:

- **`increment_ref()`** — Called when an inference request begins using the model
- **`decrement_ref()`** — Called when the request completes
- **`ref_count()`** — Returns the current number of active references

A model with `ref_count > 0` cannot be:
- Unregistered from the manager
- Hot-swapped
- Evicted during cache cleanup

### Eviction Candidates

The manager identifies models eligible for eviction:

```rust
let candidates: Vec<ModelId> = manager.eviction_candidates();
// Returns all models with ref_count == 0
```

### Full Unload (Engine Level)

```rust
engine.unload_model(model_id).await?;
// Removes from loaded_models map
// Removes from model_backend_map
// Calls backend.unload_model() to free device memory
```

## Hot Swapping

Hot swapping replaces a running model with a new version without downtime. The old model must have zero active references.

```rust
// Engine-level hot swap
let new_id = engine.hot_swap(old_model_id, new_metadata).await?;

// Manager-level hot swap
let new_id = manager.hot_swap(old_model_id, new_metadata)?;
```

**Constraints:**
- Hot swap must be enabled in `ModelManagerConfig`
- The old model must have `ref_count == 0`
- The new model's metadata is registered before the old one is removed

**Process:**
1. Load the new model into a backend
2. Remove the old model from the manager
3. Register the new model with the same name/alias
4. Return the new `ModelId`

## Versioning and Aliases

### Semantic Versioning

Every model has a `ModelVersion` (major.minor.patch):

```rust
let v1 = ModelVersion::new(1, 0, 0);
let v2 = ModelVersion::new(1, 1, 0);

// Find a specific version
let metadata = manager.find_by_version("llama-3-8b", &v2);
```

The manager tracks all versions per model name:

```rust
// Internal: HashMap<String, Vec<ModelVersion>>
// "llama-3-8b" → [1.0.0, 1.0.1, 1.1.0]
```

### Aliases

Aliases provide human-readable names that map to model IDs:

```rust
manager.add_alias("chat-model".to_string(), model_id);
manager.add_alias("primary-llm".to_string(), model_id);

// Resolution
let id = manager.get_by_alias("chat-model");

// Removal
manager.remove_alias("primary-llm");
```

## Memory Tracking Per Model

Each `ModelSlot` tracks memory allocated for its model:

```rust
// Update memory tracking
manager.set_memory_allocated(model_id, bytes_allocated);

// Query per-model memory
if let Some(slot) = manager.models.read().get(&model_id) {
    println!("Model {} using {} bytes", model_id, slot.memory_allocated());
}

// Aggregate memory
let total = manager.total_memory_used();
```

The `ModelMetadata::estimated_memory_bytes()` method provides a theoretical estimate:

```rust
// estimated_memory_bytes = parameter_count * (bits_per_weight / 8)
// Example: 8B params at Int4 = 8_000_000_000 * 0.5 = 4 GB
```

## Model Repository

The `ModelRepository` provides persistent model storage with caching and integrity verification.

### Repository Configuration

```rust
let config = RepositoryConfig {
    local_path: PathBuf::from("./models"),
    cache_path: PathBuf::from("./models/.cache"),
    remote_endpoints: vec!["https://models.example.com".to_string()],
    auto_update: false,
    verify_integrity: true,
    max_cache_size: 50 * 1024 * 1024 * 1024, // 50 GB
    enable_rollback: true,
};

let repo = ModelRepository::new(config);
```

### Registering Local Models

```rust
repo.register_local(metadata, PathBuf::from("./models/llama-3-8b.safetensors"))?;
```

### Integrity Verification

```rust
// Verify a model's SHA-256 hash
let check = repo.verify_integrity(model_id)?;
assert!(check.verified);
println!("Expected: {}", check.expected_sha256);
println!("Actual:   {}", check.actual_sha256);

// Compute hash for any file
let hash = repo.compute_sha256(Path::new("./model.bin"))?;
```

### Rollback

If a model update fails, rollback to the previous version:

```rust
repo.rollback(model_id)?;
// Restores the previous file path from entry.previous_versions
```

### Cache Management

```rust
// Check if eviction is needed
if repo.needs_eviction() {
    repo.evict_cache()?;
}

// Current cache size
let size = repo.cache_size();
```

### Model Entry

Each registered model in the repository has:

```rust
pub struct ModelEntry {
    pub metadata: ModelMetadata,
    pub local_path: PathBuf,
    pub cached: bool,
    pub verified: bool,
    pub last_verified: Option<DateTime<Utc>>,
    pub download_count: u64,
    pub previous_versions: Vec<PathBuf>,
}
```
