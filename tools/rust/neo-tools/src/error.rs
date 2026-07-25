//! Unified error type for the Neo Tools subsystem.

use std::fmt;

/// Numeric error codes for each tool error variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum ToolErrorCode {
    ToolNotFound = 2001,
    ToolAlreadyExists = 2002,
    ToolNotReady = 2003,
    ToolDisabled = 2004,
    ToolFailed = 2005,
    InvalidParameters = 2006,
    PermissionDenied = 2007,
    SandboxViolation = 2008,
    ExecutionTimeout = 2009,
    ExecutionCancelled = 2010,
    ExecutionFailed = 2011,
    DependencyUnmet = 2012,
    VersionConflict = 2013,
    LifecycleViolation = 2014,
    ConfigurationError = 2015,
    IoError = 2016,
    SerializationError = 2017,
    ResourceExhausted = 2018,
    RateLimited = 2019,
    ValidationError = 2020,
    UnsupportedOperation = 2021,
    InternalError = 2022,
}

/// Domain-specific error type for tool operations.
#[derive(Debug, Clone)]
pub struct ToolError {
    kind: ToolErrorKind,
    message: String,
    source: Option<Box<ToolError>>,
}

/// Categorized error kinds for pattern matching.
#[derive(Debug, Clone)]
pub enum ToolErrorKind {
    ToolNotFound,
    ToolAlreadyExists,
    ToolNotReady,
    ToolDisabled,
    ToolFailed,
    InvalidParameters,
    PermissionDenied,
    SandboxViolation,
    ExecutionTimeout,
    ExecutionCancelled,
    ExecutionFailed,
    DependencyUnmet,
    VersionConflict,
    LifecycleViolation,
    ConfigurationError,
    IoError,
    SerializationError,
    ResourceExhausted,
    RateLimited,
    ValidationError,
    UnsupportedOperation,
    InternalError,
}

impl ToolError {
    pub fn new(kind: ToolErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            source: None,
        }
    }

    pub fn with_source(mut self, source: ToolError) -> Self {
        self.source = Some(Box::new(source));
        self
    }

    pub fn kind(&self) -> &ToolErrorKind {
        &self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn code(&self) -> ToolErrorCode {
        match self.kind {
            ToolErrorKind::ToolNotFound => ToolErrorCode::ToolNotFound,
            ToolErrorKind::ToolAlreadyExists => ToolErrorCode::ToolAlreadyExists,
            ToolErrorKind::ToolNotReady => ToolErrorCode::ToolNotReady,
            ToolErrorKind::ToolDisabled => ToolErrorCode::ToolDisabled,
            ToolErrorKind::ToolFailed => ToolErrorCode::ToolFailed,
            ToolErrorKind::InvalidParameters => ToolErrorCode::InvalidParameters,
            ToolErrorKind::PermissionDenied => ToolErrorCode::PermissionDenied,
            ToolErrorKind::SandboxViolation => ToolErrorCode::SandboxViolation,
            ToolErrorKind::ExecutionTimeout => ToolErrorCode::ExecutionTimeout,
            ToolErrorKind::ExecutionCancelled => ToolErrorCode::ExecutionCancelled,
            ToolErrorKind::ExecutionFailed => ToolErrorCode::ExecutionFailed,
            ToolErrorKind::DependencyUnmet => ToolErrorCode::DependencyUnmet,
            ToolErrorKind::VersionConflict => ToolErrorCode::VersionConflict,
            ToolErrorKind::LifecycleViolation => ToolErrorCode::LifecycleViolation,
            ToolErrorKind::ConfigurationError => ToolErrorCode::ConfigurationError,
            ToolErrorKind::IoError => ToolErrorCode::IoError,
            ToolErrorKind::SerializationError => ToolErrorCode::SerializationError,
            ToolErrorKind::ResourceExhausted => ToolErrorCode::ResourceExhausted,
            ToolErrorKind::RateLimited => ToolErrorCode::RateLimited,
            ToolErrorKind::ValidationError => ToolErrorCode::ValidationError,
            ToolErrorKind::UnsupportedOperation => ToolErrorCode::UnsupportedOperation,
            ToolErrorKind::InternalError => ToolErrorCode::InternalError,
        }
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::new(ToolErrorKind::ToolNotFound, msg)
    }

    pub fn already_exists(msg: impl Into<String>) -> Self {
        Self::new(ToolErrorKind::ToolAlreadyExists, msg)
    }

    pub fn not_ready(msg: impl Into<String>) -> Self {
        Self::new(ToolErrorKind::ToolNotReady, msg)
    }

    pub fn disabled(msg: impl Into<String>) -> Self {
        Self::new(ToolErrorKind::ToolDisabled, msg)
    }

    pub fn failed(msg: impl Into<String>) -> Self {
        Self::new(ToolErrorKind::ToolFailed, msg)
    }

    pub fn invalid_params(msg: impl Into<String>) -> Self {
        Self::new(ToolErrorKind::InvalidParameters, msg)
    }

    pub fn permission_denied(msg: impl Into<String>) -> Self {
        Self::new(ToolErrorKind::PermissionDenied, msg)
    }

    pub fn sandbox_violation(msg: impl Into<String>) -> Self {
        Self::new(ToolErrorKind::SandboxViolation, msg)
    }

    pub fn timeout(msg: impl Into<String>) -> Self {
        Self::new(ToolErrorKind::ExecutionTimeout, msg)
    }

    pub fn cancelled(msg: impl Into<String>) -> Self {
        Self::new(ToolErrorKind::ExecutionCancelled, msg)
    }

    pub fn execution_failed(msg: impl Into<String>) -> Self {
        Self::new(ToolErrorKind::ExecutionFailed, msg)
    }

    pub fn dependency_unmet(msg: impl Into<String>) -> Self {
        Self::new(ToolErrorKind::DependencyUnmet, msg)
    }

    pub fn version_conflict(msg: impl Into<String>) -> Self {
        Self::new(ToolErrorKind::VersionConflict, msg)
    }

    pub fn lifecycle_violation(msg: impl Into<String>) -> Self {
        Self::new(ToolErrorKind::LifecycleViolation, msg)
    }

    pub fn config(msg: impl Into<String>) -> Self {
        Self::new(ToolErrorKind::ConfigurationError, msg)
    }

    pub fn io(err: impl std::error::Error + 'static) -> Self {
        Self::new(ToolErrorKind::IoError, err.to_string())
    }

    pub fn serialization(err: impl std::error::Error + 'static) -> Self {
        Self::new(ToolErrorKind::SerializationError, err.to_string())
    }

    pub fn resource_exhausted(msg: impl Into<String>) -> Self {
        Self::new(ToolErrorKind::ResourceExhausted, msg)
    }

    pub fn rate_limited(msg: impl Into<String>) -> Self {
        Self::new(ToolErrorKind::RateLimited, msg)
    }

    pub fn validation(msg: impl Into<String>) -> Self {
        Self::new(ToolErrorKind::ValidationError, msg)
    }

    pub fn unsupported(msg: impl Into<String>) -> Self {
        Self::new(ToolErrorKind::UnsupportedOperation, msg)
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::new(ToolErrorKind::InternalError, msg)
    }

    /// Whether this error is retryable.
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        matches!(
            self.kind,
            ToolErrorKind::ExecutionTimeout
                | ToolErrorKind::ResourceExhausted
                | ToolErrorKind::RateLimited
                | ToolErrorKind::IoError
        )
    }
}

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.kind, self.message)?;
        if let Some(ref source) = self.source {
            write!(f, "\n  Caused by: {}", source)?;
        }
        Ok(())
    }
}

impl fmt::Display for ToolErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::ToolNotFound => "not found",
            Self::ToolAlreadyExists => "already exists",
            Self::ToolNotReady => "not ready",
            Self::ToolDisabled => "disabled",
            Self::ToolFailed => "failed",
            Self::InvalidParameters => "invalid params",
            Self::PermissionDenied => "permission denied",
            Self::SandboxViolation => "sandbox violation",
            Self::ExecutionTimeout => "timeout",
            Self::ExecutionCancelled => "cancelled",
            Self::ExecutionFailed => "execution failed",
            Self::DependencyUnmet => "dependency unmet",
            Self::VersionConflict => "version conflict",
            Self::LifecycleViolation => "lifecycle violation",
            Self::ConfigurationError => "config error",
            Self::IoError => "io error",
            Self::SerializationError => "serialization error",
            Self::ResourceExhausted => "resource exhausted",
            Self::RateLimited => "rate limited",
            Self::ValidationError => "validation error",
            Self::UnsupportedOperation => "unsupported",
            Self::InternalError => "internal error",
        };
        write!(f, "{}", label)
    }
}

impl std::error::Error for ToolError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|e| e as &(dyn std::error::Error + 'static))
    }
}

impl From<std::io::Error> for ToolError {
    fn from(err: std::io::Error) -> Self {
        Self::io(err)
    }
}

impl From<serde_json::Error> for ToolError {
    fn from(err: serde_json::Error) -> Self {
        Self::serialization(err)
    }
}

impl From<ToolError> for neo_core::error::NeoError {
    fn from(err: ToolError) -> Self {
        neo_core::error::NeoError::Internal(err.to_string())
    }
}

/// Convenience result alias for tool operations.
pub type ToolResult<T> = Result<T, ToolError>;
