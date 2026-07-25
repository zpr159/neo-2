use std::fmt;

/// Integer error codes for each variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum ErrorCode {
    Config = 100,
    Io = 200,
    Serialization = 300,
    Timeout = 400,
    NotFound = 500,
    PermissionDenied = 600,
    InvalidInput = 700,
    AlreadyExists = 800,
    Internal = 900,
    ResourceExhausted = 1000,
    NotImplemented = 1100,
    Cancelled = 1200,
    Unknown = 9999,
}

/// Unified error type for the Neo AGI Operating System.
#[derive(Debug)]
pub enum NeoError {
    Config(String),
    Io(std::io::Error),
    Serialization(serde_json::Error),
    Timeout(String),
    NotFound(String),
    PermissionDenied(String),
    InvalidInput(String),
    AlreadyExists(String),
    Internal(String),
    ResourceExhausted(String),
    NotImplemented(String),
    Cancelled(String),
    Unknown(String),
}

impl NeoError {
    pub fn code(&self) -> ErrorCode {
        match self {
            NeoError::Config(_) => ErrorCode::Config,
            NeoError::Io(_) => ErrorCode::Io,
            NeoError::Serialization(_) => ErrorCode::Serialization,
            NeoError::Timeout(_) => ErrorCode::Timeout,
            NeoError::NotFound(_) => ErrorCode::NotFound,
            NeoError::PermissionDenied(_) => ErrorCode::PermissionDenied,
            NeoError::InvalidInput(_) => ErrorCode::InvalidInput,
            NeoError::AlreadyExists(_) => ErrorCode::AlreadyExists,
            NeoError::Internal(_) => ErrorCode::Internal,
            NeoError::ResourceExhausted(_) => ErrorCode::ResourceExhausted,
            NeoError::NotImplemented(_) => ErrorCode::NotImplemented,
            NeoError::Cancelled(_) => ErrorCode::Cancelled,
            NeoError::Unknown(_) => ErrorCode::Unknown,
        }
    }
}

impl fmt::Display for NeoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NeoError::Config(msg) => write!(f, "[config] {}", msg),
            NeoError::Io(err) => write!(f, "[io] {}", err),
            NeoError::Serialization(err) => write!(f, "[serialization] {}", err),
            NeoError::Timeout(msg) => write!(f, "[timeout] {}", msg),
            NeoError::NotFound(msg) => write!(f, "[not found] {}", msg),
            NeoError::PermissionDenied(msg) => write!(f, "[permission denied] {}", msg),
            NeoError::InvalidInput(msg) => write!(f, "[invalid input] {}", msg),
            NeoError::AlreadyExists(msg) => write!(f, "[already exists] {}", msg),
            NeoError::Internal(msg) => write!(f, "[internal] {}", msg),
            NeoError::ResourceExhausted(msg) => write!(f, "[resource exhausted] {}", msg),
            NeoError::NotImplemented(msg) => write!(f, "[not implemented] {}", msg),
            NeoError::Cancelled(msg) => write!(f, "[cancelled] {}", msg),
            NeoError::Unknown(msg) => write!(f, "[unknown] {}", msg),
        }
    }
}

impl std::error::Error for NeoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            NeoError::Io(err) => Some(err),
            NeoError::Serialization(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for NeoError {
    fn from(err: std::io::Error) -> Self {
        NeoError::Io(err)
    }
}

impl From<serde_json::Error> for NeoError {
    fn from(err: serde_json::Error) -> Self {
        NeoError::Serialization(err)
    }
}

impl From<config::ConfigError> for NeoError {
    fn from(err: config::ConfigError) -> Self {
        NeoError::Config(err.to_string())
    }
}

/// Convenience result alias for Neo operations.
pub type NeoResult<T> = Result<T, NeoError>;
