use std::fmt;

/// Errors that can occur within the evolution subsystem.
#[derive(Debug)]
pub enum EvolutionError {
    /// An analysis failed for the given reason.
    AnalysisFailed(String),
    /// The requested entity was not found.
    NotFound(String),
    /// A configuration error occurred.
    ConfigError(String),
    /// Invalid configuration parameters.
    InvalidConfiguration(String),
    /// Serialization or deserialization failed.
    SerializationError(String),
    /// A state transition was invalid.
    InvalidStateTransition(String),
    /// An internal invariant was violated.
    InternalError(String),
    /// A resource limit was exceeded.
    ResourceExhausted(String),
    /// A timeout occurred while waiting for a operation.
    Timeout(String),
    /// The requested operation is not supported.
    UnsupportedOperation(String),
    /// A governance or authorization violation.
    GovernanceViolation(String),
    /// An experiment failed.
    ExperimentFailed(String),
    /// A sandbox error occurred.
    SandboxError(String),
    /// A strategy error occurred.
    StrategyError(String),
    /// An optimization failed.
    OptimizationFailed(String),
    /// A benchmark failed.
    BenchmarkFailed(String),
    /// A metrics error occurred.
    MetricsError(String),
    /// A rollback failed.
    RollbackFailed(String),
    /// A policy violation occurred.
    PolicyViolation(String),
    /// Not authorized for the requested operation.
    NotAuthorized(String),
    /// The system is already running the requested operation.
    AlreadyRunning(String),
    /// The system is not running.
    NotRunning(String),
    /// An IO error occurred.
    Io(std::io::Error),
    /// A serialization error occurred.
    Serialization(serde_json::Error),
}

impl EvolutionError {
    /// Returns a human-readable description of this error.
    #[must_use]
    pub fn description(&self) -> &str {
        match self {
            Self::AnalysisFailed(_) => "analysis failed",
            Self::NotFound(_) => "not found",
            Self::ConfigError(_) => "configuration error",
            Self::InvalidConfiguration(_) => "invalid configuration",
            Self::SerializationError(_) => "serialization error",
            Self::InvalidStateTransition(_) => "invalid state transition",
            Self::InternalError(_) => "internal error",
            Self::ResourceExhausted(_) => "resource exhausted",
            Self::Timeout(_) => "timeout",
            Self::UnsupportedOperation(_) => "unsupported operation",
            Self::GovernanceViolation(_) => "governance violation",
            Self::ExperimentFailed(_) => "experiment failed",
            Self::SandboxError(_) => "sandbox error",
            Self::StrategyError(_) => "strategy error",
            Self::OptimizationFailed(_) => "optimization failed",
            Self::BenchmarkFailed(_) => "benchmark failed",
            Self::MetricsError(_) => "metrics error",
            Self::RollbackFailed(_) => "rollback failed",
            Self::PolicyViolation(_) => "policy violation",
            Self::NotAuthorized(_) => "not authorized",
            Self::AlreadyRunning(_) => "already running",
            Self::NotRunning(_) => "not running",
            Self::Io(_) => "io error",
            Self::Serialization(_) => "serialization error",
        }
    }
}

impl fmt::Display for EvolutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AnalysisFailed(msg) => write!(f, "analysis failed: {msg}"),
            Self::NotFound(msg) => write!(f, "not found: {msg}"),
            Self::ConfigError(msg) => write!(f, "configuration error: {msg}"),
            Self::InvalidConfiguration(msg) => write!(f, "invalid configuration: {msg}"),
            Self::SerializationError(msg) => write!(f, "serialization error: {msg}"),
            Self::InvalidStateTransition(msg) => write!(f, "invalid state transition: {msg}"),
            Self::InternalError(msg) => write!(f, "internal error: {msg}"),
            Self::ResourceExhausted(msg) => write!(f, "resource exhausted: {msg}"),
            Self::Timeout(msg) => write!(f, "timeout: {msg}"),
            Self::UnsupportedOperation(msg) => write!(f, "unsupported operation: {msg}"),
            Self::GovernanceViolation(msg) => write!(f, "governance violation: {msg}"),
            Self::ExperimentFailed(msg) => write!(f, "experiment failed: {msg}"),
            Self::SandboxError(msg) => write!(f, "sandbox error: {msg}"),
            Self::StrategyError(msg) => write!(f, "strategy error: {msg}"),
            Self::OptimizationFailed(msg) => write!(f, "optimization failed: {msg}"),
            Self::BenchmarkFailed(msg) => write!(f, "benchmark failed: {msg}"),
            Self::MetricsError(msg) => write!(f, "metrics error: {msg}"),
            Self::RollbackFailed(msg) => write!(f, "rollback failed: {msg}"),
            Self::PolicyViolation(msg) => write!(f, "policy violation: {msg}"),
            Self::NotAuthorized(msg) => write!(f, "not authorized: {msg}"),
            Self::AlreadyRunning(msg) => write!(f, "already running: {msg}"),
            Self::NotRunning(msg) => write!(f, "not running: {msg}"),
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::Serialization(err) => write!(f, "serialization error: {err}"),
        }
    }
}

impl std::error::Error for EvolutionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Serialization(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for EvolutionError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for EvolutionError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(err)
    }
}

/// Convenience alias for `Result<T, EvolutionError>`.
pub type EvolutionResult<T> = Result<T, EvolutionError>;
