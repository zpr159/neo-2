pub mod error;
pub mod model;
pub mod backends;
pub mod tokenizer;
pub mod generation;
pub mod context;
pub mod scheduler;
pub mod memory;
pub mod quantization;
pub mod multi_gpu;
pub mod distributed;
pub mod embedding;
pub mod api;
pub mod telemetry;
pub mod engine;

pub use error::{InferenceError, InferenceResult, InferenceErrorCode};
pub use model::{
    ModelId, ModelMetadata, ModelVersion, ModelFormat, ModelArchitecture,
    QuantizationType, ModelSlot,
};
pub use model::manager::ModelManager;
pub use model::repository::{ModelRepository, RepositoryConfig};
pub use backends::{
    InferenceBackend, BackendType, BackendInfo, BackendConfig,
    InferenceInput, InferenceOutput,
};
pub use tokenizer::{
    Tokenizer, Token, Encoding, Vocabulary, TokenizerConfig, TokenizerType,
};
pub use tokenizer::bpe::BpeTokenizer;
pub use tokenizer::wordpiece::WordPieceTokenizer;
pub use tokenizer::sentencepiece::SentencePieceTokenizer;
pub use tokenizer::character::CharacterTokenizer;
pub use generation::{
    GenerationParams, GenerationResult, FinishReason, TokenUsage, StreamChunk,
    SamplingStrategy,
};
pub use generation::engine::GenerationEngine;
pub use context::{
    ContextId, ConversationContext, Message, MessageRole, ContextConfig,
};
pub use context::engine::ContextEngine;
pub use scheduler::{
    InferenceScheduler, SchedulerConfig, SchedulerStatistics,
    InferencePriority, ScheduledRequest,
};
pub use memory::{MemoryOptimizer, KvCacheManager, KvCacheEntry, CacheStrategy, MemoryPoolStats};
pub use memory::engine::MemoryEngine;
pub use quantization::{
    QuantizationConfig, QuantizedWeight, QuantizationResult, QuantizationMetrics,
};
pub use quantization::engine::QuantizationEngine;
pub use multi_gpu::{
    MultiGpuManager, MultiGpuPlan, ParallelismStrategy,
    TensorParallelConfig, PipelineParallelConfig, DeviceAssignment,
};
pub use distributed::{
    DistributedInferenceManager, DistributedConfig, RemoteWorker, WorkerState,
    LoadBalanceStrategy, ClusterNode, NodeRole,
};
pub use embedding::{
    EmbeddingVector, EmbeddingType, EmbeddingRequest, EmbeddingResponse, EmbeddingUsage,
};
pub use embedding::engine::EmbeddingEngine;
pub use api::{
    ApiConfig, RestConfig, GrpcConfig, HealthResponse,
    InferRequest, InferResponse, InferChoice, InferUsage,
    StreamResponse, StreamChoice, StreamDelta,
    EmbedRequest as ApiEmbedRequest, EmbedResponse as ApiEmbedResponse,
    ErrorResponse,
};
pub use telemetry::{
    TelemetryConfig, TelemetrySnapshot, LatencyMetrics,
    ThroughputMetrics, GpuMetrics, BackendStatistics,
};
pub use engine::{InferenceEngine, EngineConfig};
