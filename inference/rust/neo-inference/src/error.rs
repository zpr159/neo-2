use std::fmt;
use neo_core::error::NeoError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum InferenceErrorCode {
    ModelNotFound = 3000,
    ModelLoadFailed = 3001,
    ModelUnloadFailed = 3002,
    ModelAlreadyLoaded = 3003,
    BackendNotAvailable = 3004,
    BackendInitFailed = 3005,
    TokenizerError = 3006,
    GenerationFailed = 3007,
    RequestCancelled = 3008,
    RequestTimeout = 3009,
    QueueFull = 3010,
    BatchFailed = 3011,
    MemoryExhausted = 3012,
    DeviceError = 3013,
    QuantizationFailed = 3014,
    EmbeddingFailed = 3015,
    ContextError = 3016,
    SchedulerError = 3017,
    SerializationFailed = 3018,
    IntegrityFailed = 3019,
    DistributedError = 3020,
    HotSwapFailed = 3021,
    VersionConflict = 3022,
    DependencyMissing = 3023,
    InvalidConfig = 3024,
    CacheEvictionFailed = 3025,
    TensorParallelFailed = 3026,
    PipelineParallelFailed = 3027,
}

#[derive(Debug)]
pub enum InferenceError {
    ModelNotFound { model_id: String },
    ModelLoadFailed { model_id: String, reason: String },
    ModelUnloadFailed { model_id: String, reason: String },
    ModelAlreadyLoaded { model_id: String },
    BackendNotAvailable { backend: String },
    BackendInitFailed { backend: String, reason: String },
    TokenizerError { reason: String },
    GenerationFailed { reason: String },
    RequestCancelled { request_id: String },
    RequestTimeout { request_id: String, timeout_ms: u64 },
    QueueFull { queue: String, capacity: usize },
    BatchFailed { reason: String },
    MemoryExhausted { requested: u64, available: u64 },
    DeviceError { device: String, reason: String },
    QuantizationFailed { reason: String },
    EmbeddingFailed { reason: String },
    ContextError { reason: String },
    SchedulerError { reason: String },
    SerializationFailed { reason: String },
    IntegrityFailed { path: String, expected: String, actual: String },
    DistributedError { reason: String },
    HotSwapFailed { model_id: String, reason: String },
    VersionConflict { model_id: String, expected: String, actual: String },
    DependencyMissing { model_id: String, dependency: String },
    InvalidConfig { reason: String },
    CacheEvictionFailed { reason: String },
    TensorParallelFailed { reason: String },
    PipelineParallelFailed { reason: String },
}

impl InferenceError {
    #[must_use]
    pub fn code(&self) -> InferenceErrorCode {
        match self {
            Self::ModelNotFound { .. } => InferenceErrorCode::ModelNotFound,
            Self::ModelLoadFailed { .. } => InferenceErrorCode::ModelLoadFailed,
            Self::ModelUnloadFailed { .. } => InferenceErrorCode::ModelUnloadFailed,
            Self::ModelAlreadyLoaded { .. } => InferenceErrorCode::ModelAlreadyLoaded,
            Self::BackendNotAvailable { .. } => InferenceErrorCode::BackendNotAvailable,
            Self::BackendInitFailed { .. } => InferenceErrorCode::BackendInitFailed,
            Self::TokenizerError { .. } => InferenceErrorCode::TokenizerError,
            Self::GenerationFailed { .. } => InferenceErrorCode::GenerationFailed,
            Self::RequestCancelled { .. } => InferenceErrorCode::RequestCancelled,
            Self::RequestTimeout { .. } => InferenceErrorCode::RequestTimeout,
            Self::QueueFull { .. } => InferenceErrorCode::QueueFull,
            Self::BatchFailed { .. } => InferenceErrorCode::BatchFailed,
            Self::MemoryExhausted { .. } => InferenceErrorCode::MemoryExhausted,
            Self::DeviceError { .. } => InferenceErrorCode::DeviceError,
            Self::QuantizationFailed { .. } => InferenceErrorCode::QuantizationFailed,
            Self::EmbeddingFailed { .. } => InferenceErrorCode::EmbeddingFailed,
            Self::ContextError { .. } => InferenceErrorCode::ContextError,
            Self::SchedulerError { .. } => InferenceErrorCode::SchedulerError,
            Self::SerializationFailed { .. } => InferenceErrorCode::SerializationFailed,
            Self::IntegrityFailed { .. } => InferenceErrorCode::IntegrityFailed,
            Self::DistributedError { .. } => InferenceErrorCode::DistributedError,
            Self::HotSwapFailed { .. } => InferenceErrorCode::HotSwapFailed,
            Self::VersionConflict { .. } => InferenceErrorCode::VersionConflict,
            Self::DependencyMissing { .. } => InferenceErrorCode::DependencyMissing,
            Self::InvalidConfig { .. } => InferenceErrorCode::InvalidConfig,
            Self::CacheEvictionFailed { .. } => InferenceErrorCode::CacheEvictionFailed,
            Self::TensorParallelFailed { .. } => InferenceErrorCode::TensorParallelFailed,
            Self::PipelineParallelFailed { .. } => InferenceErrorCode::PipelineParallelFailed,
        }
    }
}

impl fmt::Display for InferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ModelNotFound { model_id } => write!(f, "model not found: {}", model_id),
            Self::ModelLoadFailed { model_id, reason } => write!(f, "model load failed for {}: {}", model_id, reason),
            Self::ModelUnloadFailed { model_id, reason } => write!(f, "model unload failed for {}: {}", model_id, reason),
            Self::ModelAlreadyLoaded { model_id } => write!(f, "model already loaded: {}", model_id),
            Self::BackendNotAvailable { backend } => write!(f, "backend not available: {}", backend),
            Self::BackendInitFailed { backend, reason } => write!(f, "backend init failed for {}: {}", backend, reason),
            Self::TokenizerError { reason } => write!(f, "tokenizer error: {}", reason),
            Self::GenerationFailed { reason } => write!(f, "generation failed: {}", reason),
            Self::RequestCancelled { request_id } => write!(f, "request cancelled: {}", request_id),
            Self::RequestTimeout { request_id, timeout_ms } => write!(f, "request {} timed out after {}ms", request_id, timeout_ms),
            Self::QueueFull { queue, capacity } => write!(f, "queue '{}' full at capacity {}", queue, capacity),
            Self::BatchFailed { reason } => write!(f, "batch failed: {}", reason),
            Self::MemoryExhausted { requested, available } => write!(f, "memory exhausted: requested {} bytes, available {} bytes", requested, available),
            Self::DeviceError { device, reason } => write!(f, "device error on {}: {}", device, reason),
            Self::QuantizationFailed { reason } => write!(f, "quantization failed: {}", reason),
            Self::EmbeddingFailed { reason } => write!(f, "embedding failed: {}", reason),
            Self::ContextError { reason } => write!(f, "context error: {}", reason),
            Self::SchedulerError { reason } => write!(f, "scheduler error: {}", reason),
            Self::SerializationFailed { reason } => write!(f, "serialization failed: {}", reason),
            Self::IntegrityFailed { path, expected, actual } => write!(f, "integrity check failed for {}: expected {}, got {}", path, expected, actual),
            Self::DistributedError { reason } => write!(f, "distributed error: {}", reason),
            Self::HotSwapFailed { model_id, reason } => write!(f, "hot swap failed for {}: {}", model_id, reason),
            Self::VersionConflict { model_id, expected, actual } => write!(f, "version conflict for {}: expected {}, got {}", model_id, expected, actual),
            Self::DependencyMissing { model_id, dependency } => write!(f, "dependency '{}' missing for model {}", dependency, model_id),
            Self::InvalidConfig { reason } => write!(f, "invalid config: {}", reason),
            Self::CacheEvictionFailed { reason } => write!(f, "cache eviction failed: {}", reason),
            Self::TensorParallelFailed { reason } => write!(f, "tensor parallel failed: {}", reason),
            Self::PipelineParallelFailed { reason } => write!(f, "pipeline parallel failed: {}", reason),
        }
    }
}

impl std::error::Error for InferenceError {}

impl From<InferenceError> for NeoError {
    fn from(e: InferenceError) -> Self {
        NeoError::Internal(e.to_string())
    }
}

impl From<NeoError> for InferenceError {
    fn from(e: NeoError) -> Self {
        Self::GenerationFailed { reason: e.to_string() }
    }
}

impl From<std::io::Error> for InferenceError {
    fn from(e: std::io::Error) -> Self {
        Self::GenerationFailed { reason: format!("io error: {}", e) }
    }
}

pub type InferenceResult<T> = Result<T, InferenceError>;
