use std::fmt;

use crate::error::NeoError;

/// Provider-specific error codes for the language engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum LanguageErrorCode {
    ConnectionFailed = 2000,
    Timeout = 2001,
    AuthenticationFailed = 2002,
    ModelNotFound = 2003,
    ModelLoadingFailed = 2004,
    GenerationFailed = 2005,
    StreamingFailed = 2006,
    ContextTooLarge = 2007,
    RateLimited = 2008,
    ProviderUnavailable = 2009,
    UnsupportedCapability = 2010,
    SerializationFailed = 2011,
    ConfigurationInvalid = 2012,
    ProviderNotFound = 2013,
    LoadBalancerExhausted = 2014,
    HealthCheckFailed = 2015,
    ModelNotLoaded = 2016,
    InsufficientResources = 2017,
    RequestCancelled = 2018,
    InternalError = 2019,
    NotImplemented = 2020,
}

/// Unified error type for language engine operations.
#[derive(Debug, Clone)]
pub enum LanguageError {
    ConnectionFailed(String),
    Timeout(String),
    AuthenticationFailed(String),
    ModelNotFound(String),
    ModelLoadingFailed(String),
    GenerationFailed(String),
    StreamingFailed(String),
    ContextTooLarge { provided: usize, maximum: usize },
    RateLimited { retry_after_secs: Option<u64> },
    ProviderUnavailable(String),
    UnsupportedCapability(String),
    SerializationFailed(String),
    ConfigurationInvalid(String),
    ProviderNotFound(String),
    LoadBalancerExhausted(String),
    HealthCheckFailed(String),
    ModelNotLoaded(String),
    InsufficientResources(String),
    RequestCancelled(String),
    InternalError(String),
    NotImplemented(String),
}

impl LanguageError {
    pub fn code(&self) -> LanguageErrorCode {
        match self {
            LanguageError::ConnectionFailed(_) => LanguageErrorCode::ConnectionFailed,
            LanguageError::Timeout(_) => LanguageErrorCode::Timeout,
            LanguageError::AuthenticationFailed(_) => LanguageErrorCode::AuthenticationFailed,
            LanguageError::ModelNotFound(_) => LanguageErrorCode::ModelNotFound,
            LanguageError::ModelLoadingFailed(_) => LanguageErrorCode::ModelLoadingFailed,
            LanguageError::GenerationFailed(_) => LanguageErrorCode::GenerationFailed,
            LanguageError::StreamingFailed(_) => LanguageErrorCode::StreamingFailed,
            LanguageError::ContextTooLarge { .. } => LanguageErrorCode::ContextTooLarge,
            LanguageError::RateLimited { .. } => LanguageErrorCode::RateLimited,
            LanguageError::ProviderUnavailable(_) => LanguageErrorCode::ProviderUnavailable,
            LanguageError::UnsupportedCapability(_) => LanguageErrorCode::UnsupportedCapability,
            LanguageError::SerializationFailed(_) => LanguageErrorCode::SerializationFailed,
            LanguageError::ConfigurationInvalid(_) => LanguageErrorCode::ConfigurationInvalid,
            LanguageError::ProviderNotFound(_) => LanguageErrorCode::ProviderNotFound,
            LanguageError::LoadBalancerExhausted(_) => LanguageErrorCode::LoadBalancerExhausted,
            LanguageError::HealthCheckFailed(_) => LanguageErrorCode::HealthCheckFailed,
            LanguageError::ModelNotLoaded(_) => LanguageErrorCode::ModelNotLoaded,
            LanguageError::InsufficientResources(_) => LanguageErrorCode::InsufficientResources,
            LanguageError::RequestCancelled(_) => LanguageErrorCode::RequestCancelled,
            LanguageError::InternalError(_) => LanguageErrorCode::InternalError,
            LanguageError::NotImplemented(_) => LanguageErrorCode::NotImplemented,
        }
    }

    pub fn is_retriable(&self) -> bool {
        matches!(
            self,
            LanguageError::Timeout(_)
                | LanguageError::RateLimited { .. }
                | LanguageError::ProviderUnavailable(_)
                | LanguageError::ConnectionFailed(_)
                | LanguageError::HealthCheckFailed(_)
        )
    }

    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            LanguageError::AuthenticationFailed(_)
                | LanguageError::ConfigurationInvalid(_)
                | LanguageError::UnsupportedCapability(_)
        )
    }
}

impl fmt::Display for LanguageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LanguageError::ConnectionFailed(msg) => write!(f, "[connection failed] {}", msg),
            LanguageError::Timeout(msg) => write!(f, "[timeout] {}", msg),
            LanguageError::AuthenticationFailed(msg) => write!(f, "[auth failed] {}", msg),
            LanguageError::ModelNotFound(msg) => write!(f, "[model not found] {}", msg),
            LanguageError::ModelLoadingFailed(msg) => write!(f, "[model loading failed] {}", msg),
            LanguageError::GenerationFailed(msg) => write!(f, "[generation failed] {}", msg),
            LanguageError::StreamingFailed(msg) => write!(f, "[streaming failed] {}", msg),
            LanguageError::ContextTooLarge { provided, maximum } => {
                write!(
                    f,
                    "[context too large] provided {} tokens, maximum {}",
                    provided, maximum
                )
            }
            LanguageError::RateLimited { retry_after_secs } => {
                write!(
                    f,
                    "[rate limited] retry after {}s",
                    retry_after_secs.map_or("unknown".to_string(), |s| format!("{}s", s))
                )
            }
            LanguageError::ProviderUnavailable(msg) => write!(f, "[provider unavailable] {}", msg),
            LanguageError::UnsupportedCapability(msg) => {
                write!(f, "[unsupported capability] {}", msg)
            }
            LanguageError::SerializationFailed(msg) => {
                write!(f, "[serialization failed] {}", msg)
            }
            LanguageError::ConfigurationInvalid(msg) => {
                write!(f, "[configuration invalid] {}", msg)
            }
            LanguageError::ProviderNotFound(msg) => write!(f, "[provider not found] {}", msg),
            LanguageError::LoadBalancerExhausted(msg) => {
                write!(f, "[load balancer exhausted] {}", msg)
            }
            LanguageError::HealthCheckFailed(msg) => write!(f, "[health check failed] {}", msg),
            LanguageError::ModelNotLoaded(msg) => write!(f, "[model not loaded] {}", msg),
            LanguageError::InsufficientResources(msg) => {
                write!(f, "[insufficient resources] {}", msg)
            }
            LanguageError::RequestCancelled(msg) => write!(f, "[request cancelled] {}", msg),
            LanguageError::InternalError(msg) => write!(f, "[internal error] {}", msg),
            LanguageError::NotImplemented(msg) => write!(f, "[not implemented] {}", msg),
        }
    }
}

impl std::error::Error for LanguageError {}

impl From<LanguageError> for NeoError {
    fn from(err: LanguageError) -> Self {
        NeoError::Internal(format!("language engine: {}", err))
    }
}

impl From<reqwest::Error> for LanguageError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            LanguageError::Timeout(err.to_string())
        } else if err.is_connect() {
            LanguageError::ConnectionFailed(err.to_string())
        } else {
            LanguageError::ConnectionFailed(err.to_string())
        }
    }
}

impl From<serde_json::Error> for LanguageError {
    fn from(err: serde_json::Error) -> Self {
        LanguageError::SerializationFailed(err.to_string())
    }
}

/// Convenience result alias for language engine operations.
pub type LanguageResult<T> = Result<T, LanguageError>;
