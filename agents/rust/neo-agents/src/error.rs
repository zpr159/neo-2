use std::fmt;

/// Error codes specific to agent operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum AgentErrorCode {
    NotFound = 2000,
    InvalidState = 2001,
    NotSupported = 2002,
    QuotaExceeded = 2003,
    MaxRetriesExceeded = 2004,
    DependencyUnavailable = 2005,
    ExecutionTimeout = 2006,
    MessageDeliveryFailed = 2007,
    AlreadyRegistered = 2008,
    Terminated = 2009,
    CircularDependency = 2010,
    DeadlockDetected = 2011,
    Unauthorized = 2012,
    InvalidConfiguration = 2013,
    HealthCheckFailed = 2014,
    ContextConflict = 2015,
    ResourceReservationFailed = 2016,
    MigrationFailed = 2017,
    SchedulingConflict = 2018,
    Internal = 2999,
}

/// Unified error type for all agent framework operations.
#[derive(Debug)]
pub enum AgentError {
    NotFound(String),
    InvalidState(String),
    NotSupported(String),
    QuotaExceeded(String),
    MaxRetriesExceeded(String),
    DependencyUnavailable(String),
    ExecutionTimeout(String),
    MessageDeliveryFailed(String),
    AlreadyRegistered(String),
    Terminated(String),
    CircularDependency(String),
    DeadlockDetected(String),
    Unauthorized(String),
    InvalidConfiguration(String),
    HealthCheckFailed(String),
    ContextConflict(String),
    ResourceReservationFailed(String),
    MigrationFailed(String),
    SchedulingConflict(String),
    Internal(String),
    Io(std::io::Error),
    Serialization(serde_json::Error),
}

impl AgentError {
    pub fn code(&self) -> AgentErrorCode {
        match self {
            Self::NotFound(_) => AgentErrorCode::NotFound,
            Self::InvalidState(_) => AgentErrorCode::InvalidState,
            Self::NotSupported(_) => AgentErrorCode::NotSupported,
            Self::QuotaExceeded(_) => AgentErrorCode::QuotaExceeded,
            Self::MaxRetriesExceeded(_) => AgentErrorCode::MaxRetriesExceeded,
            Self::DependencyUnavailable(_) => AgentErrorCode::DependencyUnavailable,
            Self::ExecutionTimeout(_) => AgentErrorCode::ExecutionTimeout,
            Self::MessageDeliveryFailed(_) => AgentErrorCode::MessageDeliveryFailed,
            Self::AlreadyRegistered(_) => AgentErrorCode::AlreadyRegistered,
            Self::Terminated(_) => AgentErrorCode::Terminated,
            Self::CircularDependency(_) => AgentErrorCode::CircularDependency,
            Self::DeadlockDetected(_) => AgentErrorCode::DeadlockDetected,
            Self::Unauthorized(_) => AgentErrorCode::Unauthorized,
            Self::InvalidConfiguration(_) => AgentErrorCode::InvalidConfiguration,
            Self::HealthCheckFailed(_) => AgentErrorCode::HealthCheckFailed,
            Self::ContextConflict(_) => AgentErrorCode::ContextConflict,
            Self::ResourceReservationFailed(_) => AgentErrorCode::ResourceReservationFailed,
            Self::MigrationFailed(_) => AgentErrorCode::MigrationFailed,
            Self::SchedulingConflict(_) => AgentErrorCode::SchedulingConflict,
            Self::Internal(_) => AgentErrorCode::Internal,
            Self::Io(_) => AgentErrorCode::Internal,
            Self::Serialization(_) => AgentErrorCode::Internal,
        }
    }
}

impl fmt::Display for AgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(s) => write!(f, "NotFound: {}", s),
            Self::InvalidState(s) => write!(f, "InvalidState: {}", s),
            Self::NotSupported(s) => write!(f, "NotSupported: {}", s),
            Self::QuotaExceeded(s) => write!(f, "QuotaExceeded: {}", s),
            Self::MaxRetriesExceeded(s) => write!(f, "MaxRetriesExceeded: {}", s),
            Self::DependencyUnavailable(s) => write!(f, "DependencyUnavailable: {}", s),
            Self::ExecutionTimeout(s) => write!(f, "ExecutionTimeout: {}", s),
            Self::MessageDeliveryFailed(s) => write!(f, "MessageDeliveryFailed: {}", s),
            Self::AlreadyRegistered(s) => write!(f, "AlreadyRegistered: {}", s),
            Self::Terminated(s) => write!(f, "Terminated: {}", s),
            Self::CircularDependency(s) => write!(f, "CircularDependency: {}", s),
            Self::DeadlockDetected(s) => write!(f, "DeadlockDetected: {}", s),
            Self::Unauthorized(s) => write!(f, "Unauthorized: {}", s),
            Self::InvalidConfiguration(s) => write!(f, "InvalidConfiguration: {}", s),
            Self::HealthCheckFailed(s) => write!(f, "HealthCheckFailed: {}", s),
            Self::ContextConflict(s) => write!(f, "ContextConflict: {}", s),
            Self::ResourceReservationFailed(s) => write!(f, "ResourceReservationFailed: {}", s),
            Self::MigrationFailed(s) => write!(f, "MigrationFailed: {}", s),
            Self::SchedulingConflict(s) => write!(f, "SchedulingConflict: {}", s),
            Self::Internal(s) => write!(f, "Internal: {}", s),
            Self::Io(e) => write!(f, "Io: {}", e),
            Self::Serialization(e) => write!(f, "Serialization: {}", e),
        }
    }
}

impl std::error::Error for AgentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Serialization(e) => Some(e),
            _ => None,
        }
    }
}

impl From<neo_core::NeoError> for AgentError {
    fn from(err: neo_core::NeoError) -> Self {
        match err {
            neo_core::NeoError::NotFound(msg) => Self::NotFound(msg),
            neo_core::NeoError::InvalidInput(msg) => Self::InvalidConfiguration(msg),
            neo_core::NeoError::AlreadyExists(msg) => Self::AlreadyRegistered(msg),
            neo_core::NeoError::ResourceExhausted(msg) => Self::QuotaExceeded(msg),
            neo_core::NeoError::Timeout(msg) => Self::ExecutionTimeout(msg),
            neo_core::NeoError::PermissionDenied(msg) => Self::Unauthorized(msg),
            neo_core::NeoError::Cancelled(msg) => Self::Internal(format!("cancelled: {msg}")),
            neo_core::NeoError::Internal(msg) => Self::Internal(msg),
            neo_core::NeoError::Config(msg) => Self::InvalidConfiguration(msg),
            neo_core::NeoError::Network(msg) => Self::DependencyUnavailable(msg),
            neo_core::NeoError::Storage(msg) => Self::Internal(msg),
            neo_core::NeoError::Crypto(msg) => Self::Internal(msg),
            neo_core::NeoError::Serialization(msg) => Self::Serialization(serde_json::Error::io(std::io::Error::other(msg))),
            neo_core::NeoError::Io(msg) => Self::Io(std::io::Error::other(msg)),
            neo_core::NeoError::Cancelled(msg) => Self::Internal(format!("cancelled: {msg}")),
        }
    }
}

impl From<std::io::Error> for AgentError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for AgentError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(err)
    }
}

/// Convenience result alias for agent operations.
pub type AgentResult<T> = Result<T, AgentError>;
