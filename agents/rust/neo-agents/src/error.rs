use std::fmt;

/// Error codes specific to agent operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum AgentErrorCode {
    /// Agent was not found in the registry.
    NotFound = 2000,
    /// Agent is in a state that does not support the requested operation.
    InvalidState = 2001,
    /// The requested operation is not supported.
    NotSupported = 2002,
    /// Agent has exceeded its resource quota.
    QuotaExceeded = 2003,
    /// Agent has exceeded the maximum retry count.
    MaxRetriesExceeded = 2004,
    /// A dependency required by the agent is unavailable.
    DependencyUnavailable = 2005,
    /// The agent encountered a timeout during execution.
    ExecutionTimeout = 2006,
    /// A message failed to deliver.
    MessageDeliveryFailed = 2007,
    /// The agent is already registered.
    AlreadyRegistered = 2008,
    /// The agent has been terminated and cannot be restarted.
    Terminated = 2009,
    /// A circular dependency was detected.
    CircularDependency = 2010,
    /// The supervisor detected a deadlock.
    DeadlockDetected = 2011,
    /// The agent exceeded its permitted authority.
    Unauthorized = 2012,
    /// Agent configuration is invalid.
    InvalidConfiguration = 2013,
    /// Agent health check failed.
    HealthCheckFailed = 2014,
    /// Shared context conflict detected.
    ContextConflict = 2015,
    /// Resource reservation failed.
    ResourceReservationFailed = 2016,
    /// Agent migration failed.
    MigrationFailed = 2017,
    /// Task scheduling conflict.
    SchedulingConflict = 2018,
    /// Internal agent framework error.
    Internal = 2999,
}

/// Unified error type for all agent framework operations.
#[derive(Debug)]
pub enum AgentError {
    /// Agent was not found.
    NotFound(String),
    /// Invalid state transition or operation on agent.
    InvalidState(String),
    /// Operation not supported.
    NotSupported(String),
    /// Resource quota exceeded.
    QuotaExceeded(String),
    /// Maximum retries exceeded.
    MaxRetriesExceeded(String),
    /// Required dependency unavailable.
    DependencyUnavailable(String),
    /// Execution timed out.
    ExecutionTimeout(String),
    /// Message delivery failed.
    MessageDeliveryFailed(String),
    /// Agent already registered.
    AlreadyRegistered(String),
    /// Agent has been terminated.
    Terminated(String),
    /// Circular dependency detected.
    CircularDependency(String),
    /// Deadlock detected by supervisor.
    DeadlockDetected(String),
    /// Unauthorized operation.
    Unauthorized(String),
    /// Invalid configuration.
    InvalidConfiguration(String),
    /// Health check failed.
    HealthCheckFailed(String),
    /// Shared context conflict.
    ContextConflict(String),
    /// Resource reservation failed.
    ResourceReservationFailed(String),
    /// Agent migration failed.
    MigrationFailed(String),
    /// Task scheduling conflict.
    SchedulingConflict(String),
    /// Internal framework error.
    Internal(String),
    /// IO error.
    Io(std::io::Error),
    /// Serialization error.
    Serialization(serde_json::Error),
}

impl AgentError {
    /// Returns the error code for this error variant.
    #[must_use]
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

    /// Returns the human-readable description of this error.
    #[must_use]
    pub fn description(&self) -> &str {
        match self {
            Self::NotFound(_) => "agent not found",
            Self::InvalidState(_) => "invalid agent state",
            Self::NotSupported(_) => "operation not supported",
            Self::QuotaExceeded(_) => "resource quota exceeded",
            Self::MaxRetriesExceeded(_) => "max retries exceeded",
            Self::DependencyUnavailable(_) => "dependency unavailable",
            Self::ExecutionTimeout(_) => "execution timeout",
            Self::MessageDeliveryFailed(_) => "message delivery failed",
            Self::AlreadyRegistered(_) => "agent already registered",
            Self::Terminated(_) => "agent terminated",
            Self::CircularDependency(_) => "circular dependency",
            Self::DeadlockDetected(_) => "deadlock detected",
            Self::Unauthorized(_) => "unauthorized",
            Self::InvalidConfiguration(_) => "invalid configuration",
            Self::HealthCheckFailed(_) => "health check failed",
            Self::ContextConflict(_) => "context conflict",
            Self::ResourceReservationFailed(_) => "resource reservation failed",
            Self::MigrationFailed(_) => "migration failed",
            Self::SchedulingConflict(_) => "scheduling conflict",
            Self::Internal(_) => "internal error",
            Self::Io(_) => "io error",
            Self::Serialization(_) => "serialization error",
        }
    }
}

impl fmt::Display for AgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(msg) => write!(f, "[agent not found] {msg}"),
            Self::InvalidState(msg) => write!(f, "[invalid state] {msg}"),
            Self::NotSupported(msg) => write!(f, "[not supported] {msg}"),
            Self::QuotaExceeded(msg) => write!(f, "[quota exceeded] {msg}"),
            Self::MaxRetriesExceeded(msg) => write!(f, "[max retries] {msg}"),
            Self::DependencyUnavailable(msg) => write!(f, "[dependency unavailable] {msg}"),
            Self::ExecutionTimeout(msg) => write!(f, "[timeout] {msg}"),
            Self::MessageDeliveryFailed(msg) => write!(f, "[delivery failed] {msg}"),
            Self::AlreadyRegistered(msg) => write!(f, "[already registered] {msg}"),
            Self::Terminated(msg) => write!(f, "[terminated] {msg}"),
            Self::CircularDependency(msg) => write!(f, "[circular dependency] {msg}"),
            Self::DeadlockDetected(msg) => write!(f, "[deadlock] {msg}"),
            Self::Unauthorized(msg) => write!(f, "[unauthorized] {msg}"),
            Self::InvalidConfiguration(msg) => write!(f, "[invalid config] {msg}"),
            Self::HealthCheckFailed(msg) => write!(f, "[health check failed] {msg}"),
            Self::ContextConflict(msg) => write!(f, "[context conflict] {msg}"),
            Self::ResourceReservationFailed(msg) => write!(f, "[resource reservation] {msg}"),
            Self::MigrationFailed(msg) => write!(f, "[migration failed] {msg}"),
            Self::SchedulingConflict(msg) => write!(f, "[scheduling conflict] {msg}"),
            Self::Internal(msg) => write!(f, "[internal] {msg}"),
            Self::Io(err) => write!(f, "[io] {err}"),
            Self::Serialization(err) => write!(f, "[serialization] {err}"),
        }
    }
}

impl std::error::Error for AgentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Serialization(err) => Some(err),
            _ => None,
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
            neo_core::NeoError::Io(err) => Self::Io(err),
            neo_core::NeoError::Serialization(err) => Self::Serialization(err),
            other => Self::Internal(other.to_string()),
        }
    }
}

/// Convenience result alias for agent operations.
pub type AgentResult<T> = Result<T, AgentError>;
