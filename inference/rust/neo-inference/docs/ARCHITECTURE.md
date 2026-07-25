# Backend Architecture

## Overview

The Universal Inference Layer (`neo-inference`) provides a unified abstraction over diverse hardware backends and model formats. It enables running any modern AI model — transformer, diffusion, state-space, or custom — through a single, coherent API. The layer sits between application code (REST/gRPC endpoints, CLI tools, agents) and the raw compute infrastructure (CPU, GPU, remote workers).

## Architecture Diagram

```
┌──────────────────────────────────────────────────────────────────────┐
│                         Application Layer                            │
│              REST API · gRPC API · CLI · Agent Framework             │
└───────────────────────────────┬──────────────────────────────────────┘
                                │
┌───────────────────────────────▼──────────────────────────────────────┐
│                          InferenceEngine                             │
│  ┌─────────────┐ ┌────────────────┐ ┌───────────────┐ ┌──────────┐  │
│  │   Scheduler  │ │   Telemetry    │ │ MemoryOptimizer│ │ Context  │  │
│  └─────────────┘ └────────────────┘ └───────────────┘ └──────────┘  │
│  ┌──────────────────────────────────────────────────────────────────┐│
│  │               Backend Selection (priority + format match)        ││
│  └──────────────────────────────────────────────────────────────────┘│
└───────────────────────────────┬──────────────────────────────────────┘
                                │
┌───────────────────────────────▼──────────────────────────────────────┐
│                     InferenceBackend Trait Objects                    │
│  ┌──────┐ ┌──────┐ ┌──────────┐ ┌──────┐ ┌───────┐ ┌──────────┐   │
│  │ CPU  │ │ CUDA │ │  Metal   │ │ ROCm │ │ Llama │ │  ONNX    │   │
│  │      │ │      │ │          │ │      │ │ Cpp   │ │ Runtime  │   │
│  └──────┘ └──────┘ └──────────┘ └──────┘ └───────┘ └──────────┘   │
│  ┌──────────┐ ┌──────────┐ ┌──────┐ ┌──────────┐ ┌──────────────┐ │
│  │TensorRT  │ │ OpenVINO │ │ MLX  │ │ CoreML   │ │ NeoNative    │ │
│  └──────────┘ └──────────┘ └──────┘ └──────────┘ └──────────────┘ │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐                            │
│  │RemoteHTTP│ │RemotegRPC│ │  Plugin  │                            │
│  └──────────┘ └──────────┘ └──────────┘                            │
└─────────────────────────────────────────────────────────────────────┘
        │                │                │
┌───────▼────────┐ ┌─────▼──────┐ ┌──────▼──────┐
│ Runtime Manager│ │  Neural    │ │ NN Framework│
│  (neo-core)    │ │  Engine    │ │(neo-nn-fwk) │
│                │ │(neo-neural)│ │             │
└────────────────┘ └────────────┘ └─────────────┘
```

## The `InferenceBackend` Trait

Every backend implements the `InferenceBackend` trait, which defines the contract for model loading, inference, and resource management.

```rust
#[async_trait]
pub trait InferenceBackend: Send + Sync + fmt::Debug {
    /// Returns metadata about this backend (name, version, supported formats).
    fn info(&self) -> BackendInfo;

    /// Whether this backend is currently available on this system.
    fn is_available(&self) -> bool;

    /// Initialize the backend with the given configuration.
    async fn initialize(&mut self, config: &BackendConfig) -> InferenceResult<()>;

    /// Gracefully shut down the backend, releasing all resources.
    async fn shutdown(&mut self) -> InferenceResult<()>;

    /// Load a model into the backend's memory. Returns the assigned ModelId.
    async fn load_model(&mut self, metadata: &ModelMetadata) -> InferenceResult<ModelId>;

    /// Unload a previously loaded model, freeing memory.
    async fn unload_model(&mut self, model_id: ModelId) -> InferenceResult<()>;

    /// Run synchronous (non-streaming) inference on a loaded model.
    async fn inference(
        &self,
        model_id: ModelId,
        input: InferenceInput,
    ) -> InferenceResult<InferenceOutput>;

    /// Run streaming inference, returning a channel that yields token chunks.
    async fn inference_stream(
        &self,
        model_id: ModelId,
        input: InferenceInput,
    ) -> InferenceResult<tokio::sync::mpsc::Receiver<InferenceResult<StreamChunk>>>;

    /// List all model IDs currently loaded in this backend.
    fn loaded_models(&self) -> Vec<ModelId>;

    /// Return memory usage in bytes for a specific loaded model.
    fn model_memory_usage(&self, model_id: ModelId) -> Option<u64>;

    /// List model formats this backend supports.
    fn supported_formats(&self) -> Vec<ModelFormat>;
}
```

### Key Types

| Type | Description |
|------|-------------|
| `BackendInfo` | Descriptive metadata: name, version, priority, supported formats, capabilities |
| `BackendConfig` | Runtime configuration: device ID, thread count, memory limits, custom key-value config |
| `InferenceInput` | Tokenized input: `input_ids`, `attention_mask`, optional `position_ids` and KV cache |
| `InferenceOutput` | Model output: `logits`, shape metadata, optional KV cache, hidden states, attention weights |
| `StreamChunk` | A single token from a streaming response, with optional log-probability |

## All 14 Backends

| # | Backend | Type | Priority | Supported Formats | Capabilities | Platform |
|---|---------|------|----------|-------------------|--------------|----------|
| 1 | **NeoNative** | Local | 200 | Bincode, SafeTensors, GGUF | inference, streaming, batching, quantization, KV cache | All |
| 2 | **CUDA** | GPU | 300 | SafeTensors, GGUF, TensorRT | inference, streaming, batching, quantization, KV cache, multi-GPU | Linux/Windows |
| 3 | **TensorRT** | GPU | 310 | TensorRT | inference, optimization | Linux/Windows |
| 4 | **ROCm** | GPU | 290 | SafeTensors, GGUF | inference, batching | Linux |
| 5 | **Metal** | GPU | 280 | MLX, SafeTensors, GGUF | inference, streaming, batching | macOS |
| 6 | **MLX** | GPU | 270 | MLX, SafeTensors | inference, streaming | macOS |
| 7 | **CoreML** | NPU/GPU | 250 | CoreML | inference | macOS |
| 8 | **Llama.cpp** | CPU/GPU | 180 | GGUF | inference, streaming, batching, quantization, KV cache | All |
| 9 | **ONNX Runtime** | Multi | 160 | ONNX | inference, batching | All |
| 10 | **OpenVINO** | CPU/iGPU | 200 | OpenVINO | inference | Linux/Windows |
| 11 | **CPU** | CPU | 100 | Bincode, JSON, SafeTensors | inference, quantization | All |
| 12 | **Remote HTTP** | Network | 50 | _(none — passthrough)_ | inference, streaming | All |
| 13 | **Remote gRPC** | Network | 60 | _(none — passthrough)_ | inference, streaming | All |
| 14 | **Plugin** | Dynamic | 10 | _(dynamic)_ | inference | All |

### Priority and Format Matching

The `InferenceEngine::select_backend` method uses a two-pass algorithm:

1. **First pass — format match**: Iterate all available backends. For each backend that supports the model's `ModelFormat`, select the one with the highest `priority` value (higher = preferred).
2. **Second pass — fallback**: If no backend matches the format, select the highest-priority available backend regardless of format support.

```
For a SafeTensors model:
  CUDA (300) > NeoNative (200) > Llama.cpp (180) > CPU (100)

For a GGUF model:
  CUDA (300) > ROCm (290) > Metal (280) > NeoNative (200) > Llama.cpp (180) > CPU (100)

For an ONNX model:
  ONNX Runtime (160) > CPU (100)
```

## Backend Selection Flow

```
load_model(metadata)
    │
    ▼
┌─────────────────────────┐
│ Already loaded?          │──Yes──► Return existing ModelId
└─────────┬───────────────┘
          │ No
          ▼
┌─────────────────────────┐
│ select_backend(metadata) │
│  1. Filter available     │
│  2. Match format         │
│  3. Pick highest priority│
└─────────┬───────────────┘
          │
          ▼
┌─────────────────────────┐
│ backend.load_model()     │
│  → Allocates memory      │
│  → Loads weights         │
│  → Registers in map      │
└─────────────────────────┘
```

## Adding a New Backend

To add a new backend, implement the `InferenceBackend` trait:

```rust
use crate::backends::{InferenceBackend, BackendInfo, BackendType, BackendConfig,
                      InferenceInput, InferenceOutput, ModelFormat};
use crate::model::{ModelId, ModelMetadata};
use crate::error::InferenceResult;

#[derive(Debug)]
pub struct MyCustomBackend {
    initialized: bool,
}

impl MyCustomBackend {
    pub fn new() -> Self {
        Self { initialized: false }
    }
}

#[async_trait::async_trait]
impl InferenceBackend for MyCustomBackend {
    fn info(&self) -> BackendInfo {
        BackendInfo {
            backend_type: BackendType::Plugin, // or define a new variant
            name: "My Custom Backend".to_string(),
            version: "0.1.0".to_string(),
            is_available: self.initialized,
            priority: 150,
            supported_formats: vec![ModelFormat::SafeTensors],
            capabilities: vec!["inference".to_string()],
            max_model_size: Some(4_000_000_000),
            metadata: HashMap::new(),
        }
    }

    fn is_available(&self) -> bool {
        self.initialized
    }

    async fn initialize(&mut self, config: &BackendConfig) -> InferenceResult<()> {
        // Probe hardware, load libraries, etc.
        self.initialized = true;
        Ok(())
    }

    async fn shutdown(&mut self) -> InferenceResult<()> {
        self.initialized = false;
        Ok(())
    }

    async fn load_model(&mut self, metadata: &ModelMetadata) -> InferenceResult<ModelId> {
        // Load model weights into device memory
        Ok(ModelId::new())
    }

    async fn unload_model(&mut self, model_id: ModelId) -> InferenceResult<()> {
        Ok(())
    }

    async fn inference(
        &self,
        model_id: ModelId,
        input: InferenceInput,
    ) -> InferenceResult<InferenceOutput> {
        // Run forward pass
        todo!()
    }

    async fn inference_stream(
        &self,
        model_id: ModelId,
        input: InferenceInput,
    ) -> InferenceResult<tokio::sync::mpsc::Receiver<InferenceResult<StreamChunk>>> {
        todo!()
    }

    fn loaded_models(&self) -> Vec<ModelId> {
        vec![]
    }

    fn model_memory_usage(&self, _model_id: ModelId) -> Option<u64> {
        None
    }

    fn supported_formats(&self) -> Vec<ModelFormat> {
        vec![ModelFormat::SafeTensors]
    }
}
```

Then register it in `InferenceEngine::initialize`:

```rust
let mut custom = MyCustomBackend::new();
custom.initialize(&BackendConfig::default()).await?;
backends.push(Box::new(custom));
```

## Integration with System Components

### Runtime Manager (`neo-core`)

The Runtime Manager provides device abstraction, thread pool management, and OS-level resource control. Backends query the Runtime Manager for:

- Available compute devices (CPU cores, GPU devices)
- Memory budgets per device
- Thread allocation for parallel operations

### Neural Engine (`neo-neural-engine`)

The Neural Engine provides low-level tensor operations, memory layout management, and device-specific kernels. Backends delegate:

- Tensor allocation and deallocation on specific devices
- Kernel execution for matrix multiplications, attention, activations
- Synchronization barriers between devices

### NN Framework (`neo-nn-framework`)

The NN Framework provides pre-built neural network layers and model loading utilities. Backends use it for:

- Model architecture parsing and weight mapping
- Layer-level inference primitives
- Quantization-aware weight loading

### Model Manager

The `ModelManager` sits alongside the engine and handles:

- Registration and unregistration of model metadata
- Reference counting for safe unloading
- Alias resolution (human-readable names → UUIDs)
- Version tracking per model name
- Memory accounting across all loaded models

### Scheduler

The `InferenceScheduler` queues incoming requests with priority levels and concurrency limits, ensuring the engine is not overwhelmed and high-priority requests are served first.
