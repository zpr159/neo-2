#!\[forbid(unsafe_code)\]
#![deny(
    missing_docs,
    warnings,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_extern_crates
)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Errors specific to the multimodal intelligence system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MultimodalError {
    UnsupportedModality(Modality),
    ProcessingError(String),
    FileNotFound(String),
    FormatError(String),
    SecurityError(String),
    NetworkError(String),
    ResourceError(String),
    ConfigurationError(String),
    ValidationError(String),
    ModelLoadError(String),
    InferenceError(String),
    CacheError(String),
    PersistenceError(String),
    TimeoutError(String),
    MemoryError(String),
    EncodingError(String),
    DecodingError(String),
    TooManyRequests(String),
    NotAuthenticated(String),
    NotAuthorized(String),
    RateLimitExceeded(String),
    ServiceUnavailable(String),
    InternalError(String),
}

impl std::fmt::Display for MultimodalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MultimodalError::UnsupportedModality(m) => write!(f, "Unsupported modality: {:?}", m),
            MultimodalError::ProcessingError(msg) => write!(f, "Processing error: {}", msg),
            MultimodalError::FileNotFound(path) => write!(f, "File not found: {}", path),
            MultimodalError::FormatError(msg) => write!(f, "Format error: {}", msg),
            MultimodalError::SecurityError(msg) => write!(f, "Security error: {}", msg),
            MultimodalError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            MultimodalError::ResourceError(msg) => write!(f, "Resource error: {}", msg),
            MultimodalError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            MultimodalError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            MultimodalError::ModelLoadError(msg) => write!(f, "Model load error: {}", msg),
            MultimodalError::InferenceError(msg) => write!(f, "Inference error: {}", msg),
            MultimodalError::CacheError(msg) => write!(f, "Cache error: {}", msg),
            MultimodalError::PersistenceError(msg) => write!(f, "Persistence error: {}", msg),
            MultimodalError::TimeoutError(msg) => write!(f, "Timeout error: {}", msg),
            MultimodalError::MemoryError(msg) => write!(f, "Memory error: {}", msg),
            MultimodalError::EncodingError(msg) => write!(f, "Encoding error: {}", msg),
            MultimodalError::DecodingError(msg) => write!(f, "Decoding error: {}", msg),
            MultimodalError::TooManyRequests(msg) => write!(f, "Too many requests: {}", msg),
            MultimodalError::NotAuthenticated(msg) => write!(f, "Not authenticated: {}", msg),
            MultimodalError::NotAuthorized(msg) => write!(f, "Not authorized: {}", msg),
            MultimodalError::RateLimitExceeded(msg) => write!(f, "Rate limit exceeded: {}", msg),
            MultimodalError::ServiceUnavailable(msg) => write!(f, "Service unavailable: {}", msg),
            MultimodalError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for MultimodalError {}

pub type MultimodalResult<T> = std::result::Result<T, MultimodalError>;

pub type MediaAssetId = crate::id::MediaAssetId;
pub type MediaCollectionId = crate::id::MediaCollectionId;
pub type ProcessingStepId = crate::id::ProcessingStepId;
pub type MediaPipelineId = crate::id::MediaPipelineId;
pub type MediaSessionId = crate::id::MediaSessionId;
