//! CLI error types.

/// Errors that can occur in the Neo CLI.
#[derive(Debug, thiserror::Error)]
pub(crate) enum CliError {
    /// Configuration loading or validation failed.
    #[error("configuration error: {0}")]
    Config(String),

    /// System bootstrap failed.
    #[error("bootstrap error: {0}")]
    Bootstrap(String),

    /// Runtime subsystem error.
    #[error("runtime error: {0}")]
    Runtime(#[from] neo_runtime::RuntimeError),

    /// Executive subsystem error.
    #[error("executive error: {0}")]
    Executive(#[from] neo_executive::ExecutiveError),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Server error.
    #[error("server error: {0}")]
    Server(String),

    /// Daemon error.
    #[error("daemon error: {0}")]
    Daemon(String),

    /// General-purpose error.
    #[error("{0}")]
    Custom(String),
}

impl CliError {
    pub(crate) fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    pub(crate) fn bootstrap(msg: impl Into<String>) -> Self {
        Self::Bootstrap(msg.into())
    }

    pub(crate) fn server(msg: impl Into<String>) -> Self {
        Self::Server(msg.into())
    }

    pub(crate) fn daemon(msg: impl Into<String>) -> Self {
        Self::Daemon(msg.into())
    }

    pub(crate) fn custom(msg: impl Into<String>) -> Self {
        Self::Custom(msg.into())
    }
}

/// Result type alias for CLI operations.
pub(crate) type CliResult<T> = Result<T, CliError>;
