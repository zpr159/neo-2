use std::fmt;

use crate::error::NeoError;

/// Research-specific error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum ResearchErrorCode {
    SearchFailed = 5000,
    FetchFailed = 5001,
    ExtractionFailed = 5002,
    ValidationFailed = 5003,
    SynthesisFailed = 5004,
    ProviderUnavailable = 5005,
    ProviderTimeout = 5006,
    ProviderRateLimited = 5007,
    UnsupportedContentType = 5008,
    InvalidSource = 5009,
    DeduplicationFailed = 5010,
    CitationInvalid = 5011,
    KnowledgeUpdateFailed = 5012,
    WorldUpdateFailed = 5013,
    MemoryUpdateFailed = 5014,
    GovernanceRejected = 5015,
    TaskNotFound = 5016,
    TaskCancelled = 5017,
    TaskTimeout = 5018,
    PipelineError = 5019,
    ConfigurationInvalid = 5020,
    SerializationFailed = 5021,
    InternalError = 5022,
    NotImplemented = 5023,
}

/// Unified error type for research subsystem operations.
#[derive(Debug, Clone)]
pub enum ResearchError {
    SearchFailed(String),
    FetchFailed(String),
    ExtractionFailed(String),
    ValidationFailed(String),
    SynthesisFailed(String),
    ProviderUnavailable(String),
    ProviderTimeout(String),
    ProviderRateLimited { retry_after_secs: Option<u64> },
    UnsupportedContentType(String),
    InvalidSource(String),
    DeduplicationFailed(String),
    CitationInvalid(String),
    KnowledgeUpdateFailed(String),
    WorldUpdateFailed(String),
    MemoryUpdateFailed(String),
    GovernanceRejected(String),
    TaskNotFound(String),
    TaskCancelled(String),
    TaskTimeout(String),
    PipelineError(String),
    ConfigurationInvalid(String),
    SerializationFailed(String),
    InternalError(String),
    NotImplemented(String),
}

impl ResearchError {
    pub fn code(&self) -> ResearchErrorCode {
        match self {
            ResearchError::SearchFailed(_) => ResearchErrorCode::SearchFailed,
            ResearchError::FetchFailed(_) => ResearchErrorCode::FetchFailed,
            ResearchError::ExtractionFailed(_) => ResearchErrorCode::ExtractionFailed,
            ResearchError::ValidationFailed(_) => ResearchErrorCode::ValidationFailed,
            ResearchError::SynthesisFailed(_) => ResearchErrorCode::SynthesisFailed,
            ResearchError::ProviderUnavailable(_) => ResearchErrorCode::ProviderUnavailable,
            ResearchError::ProviderTimeout(_) => ResearchErrorCode::ProviderTimeout,
            ResearchError::ProviderRateLimited { .. } => ResearchErrorCode::ProviderRateLimited,
            ResearchError::UnsupportedContentType(_) => ResearchErrorCode::UnsupportedContentType,
            ResearchError::InvalidSource(_) => ResearchErrorCode::InvalidSource,
            ResearchError::DeduplicationFailed(_) => ResearchErrorCode::DeduplicationFailed,
            ResearchError::CitationInvalid(_) => ResearchErrorCode::CitationInvalid,
            ResearchError::KnowledgeUpdateFailed(_) => ResearchErrorCode::KnowledgeUpdateFailed,
            ResearchError::WorldUpdateFailed(_) => ResearchErrorCode::WorldUpdateFailed,
            ResearchError::MemoryUpdateFailed(_) => ResearchErrorCode::MemoryUpdateFailed,
            ResearchError::GovernanceRejected(_) => ResearchErrorCode::GovernanceRejected,
            ResearchError::TaskNotFound(_) => ResearchErrorCode::TaskNotFound,
            ResearchError::TaskCancelled(_) => ResearchErrorCode::TaskCancelled,
            ResearchError::TaskTimeout(_) => ResearchErrorCode::TaskTimeout,
            ResearchError::PipelineError(_) => ResearchErrorCode::PipelineError,
            ResearchError::ConfigurationInvalid(_) => ResearchErrorCode::ConfigurationInvalid,
            ResearchError::SerializationFailed(_) => ResearchErrorCode::SerializationFailed,
            ResearchError::InternalError(_) => ResearchErrorCode::InternalError,
            ResearchError::NotImplemented(_) => ResearchErrorCode::NotImplemented,
        }
    }

    pub fn is_retriable(&self) -> bool {
        matches!(
            self,
            ResearchError::ProviderTimeout(_)
                | ResearchError::ProviderRateLimited { .. }
                | ResearchError::ProviderUnavailable(_)
                | ResearchError::FetchFailed(_)
                | ResearchError::SearchFailed(_)
        )
    }

    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            ResearchError::ConfigurationInvalid(_)
                | ResearchError::InvalidSource(_)
                | ResearchError::NotImplemented(_)
        )
    }
}

impl fmt::Display for ResearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResearchError::SearchFailed(msg) => write!(f, "[search failed] {}", msg),
            ResearchError::FetchFailed(msg) => write!(f, "[fetch failed] {}", msg),
            ResearchError::ExtractionFailed(msg) => write!(f, "[extraction failed] {}", msg),
            ResearchError::ValidationFailed(msg) => write!(f, "[validation failed] {}", msg),
            ResearchError::SynthesisFailed(msg) => write!(f, "[synthesis failed] {}", msg),
            ResearchError::ProviderUnavailable(msg) => write!(f, "[provider unavailable] {}", msg),
            ResearchError::ProviderTimeout(msg) => write!(f, "[provider timeout] {}", msg),
            ResearchError::ProviderRateLimited { retry_after_secs } => write!(
                f,
                "[provider rate limited] retry after {}",
                retry_after_secs
                    .map(|s| format!("{}s", s))
                    .unwrap_or_else(|| "unknown".to_string())
            ),
            ResearchError::UnsupportedContentType(msg) => {
                write!(f, "[unsupported content type] {}", msg)
            }
            ResearchError::InvalidSource(msg) => write!(f, "[invalid source] {}", msg),
            ResearchError::DeduplicationFailed(msg) => {
                write!(f, "[deduplication failed] {}", msg)
            }
            ResearchError::CitationInvalid(msg) => write!(f, "[citation invalid] {}", msg),
            ResearchError::KnowledgeUpdateFailed(msg) => {
                write!(f, "[knowledge update failed] {}", msg)
            }
            ResearchError::WorldUpdateFailed(msg) => write!(f, "[world update failed] {}", msg),
            ResearchError::MemoryUpdateFailed(msg) => write!(f, "[memory update failed] {}", msg),
            ResearchError::GovernanceRejected(msg) => {
                write!(f, "[governance rejected] {}", msg)
            }
            ResearchError::TaskNotFound(msg) => write!(f, "[task not found] {}", msg),
            ResearchError::TaskCancelled(msg) => write!(f, "[task cancelled] {}", msg),
            ResearchError::TaskTimeout(msg) => write!(f, "[task timeout] {}", msg),
            ResearchError::PipelineError(msg) => write!(f, "[pipeline error] {}", msg),
            ResearchError::ConfigurationInvalid(msg) => {
                write!(f, "[configuration invalid] {}", msg)
            }
            ResearchError::SerializationFailed(msg) => {
                write!(f, "[serialization failed] {}", msg)
            }
            ResearchError::InternalError(msg) => write!(f, "[internal error] {}", msg),
            ResearchError::NotImplemented(msg) => write!(f, "[not implemented] {}", msg),
        }
    }
}

impl std::error::Error for ResearchError {}

impl From<ResearchError> for NeoError {
    fn from(err: ResearchError) -> Self {
        NeoError::Internal(format!("research: {}", err))
    }
}

impl From<reqwest::Error> for ResearchError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            ResearchError::ProviderTimeout(err.to_string())
        } else if err.is_connect() {
            ResearchError::ProviderUnavailable(err.to_string())
        } else {
            ResearchError::FetchFailed(err.to_string())
        }
    }
}

impl From<serde_json::Error> for ResearchError {
    fn from(err: serde_json::Error) -> Self {
        ResearchError::SerializationFailed(err.to_string())
    }
}

/// Convenience result alias for research operations.
pub type ResearchResult<T> = Result<T, ResearchError>;
