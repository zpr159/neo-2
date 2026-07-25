//! Distributed runtime error types with context, recovery support, and
//! comprehensive error classification for cluster operations.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Suggested recovery action for a distributed error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RecoveryAction {
    /// Retry the operation with exponential backoff.
    Retry,
    /// Failover to another node.
    Failover,
    /// Restart the local service.
    RestartService,
    /// Migrate the workload to another node.
    Migrate,
    /// Rollback to a previous checkpoint.
    Rollback,
    /// Ignore the error and continue.
    Ignore,
    /// Abort the operation.
    Abort,
}

impl fmt::Display for RecoveryAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Retry => write!(f, "retry"),
            Self::Failover => write!(f, "failover"),
            Self::RestartService => write!(f, "restart_service"),
            Self::Migrate => write!(f, "migrate"),
            Self::Rollback => write!(f, "rollback"),
            Self::Ignore => write!(f, "ignore"),
            Self::Abort => write!(f, "abort"),
        }
    }
}

/// High-level error category for distributed operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorCategory {
    /// Cluster management errors.
    Cluster,
    /// Node lifecycle errors.
    Node,
    /// Service discovery errors.
    Discovery,
    /// Heartbeat and health errors.
    Heartbeat,
    /// Failure detection errors.
    Failure,
    /// Scheduler errors.
    Scheduler,
    /// Execution errors.
    Execution,
    /// Networking errors.
    Network,
    /// Security errors.
    Security,
    /// Consensus errors.
    Consensus,
    /// Memory replication errors.
    Memory,
    /// Knowledge graph errors.
    Knowledge,
    /// Event bus errors.
    Event,
    /// Storage errors.
    Storage,
    /// Configuration errors.
    Config,
    /// Serialization errors.
    Serialization,
    /// Timeout errors.
    Timeout,
    /// Generic internal error.
    Internal,
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cluster => write!(f, "cluster"),
            Self::Node => write!(f, "node"),
            Self::Discovery => write!(f, "discovery"),
            Self::Heartbeat => write!(f, "heartbeat"),
            Self::Failure => write!(f, "failure"),
            Self::Scheduler => write!(f, "scheduler"),
            Self::Execution => write!(f, "execution"),
            Self::Network => write!(f, "network"),
            Self::Security => write!(f, "security"),
            Self::Consensus => write!(f, "consensus"),
            Self::Memory => write!(f, "memory"),
            Self::Knowledge => write!(f, "knowledge"),
            Self::Event => write!(f, "event"),
            Self::Storage => write!(f, "storage"),
            Self::Config => write!(f, "config"),
            Self::Serialization => write!(f, "serialization"),
            Self::Timeout => write!(f, "timeout"),
            Self::Internal => write!(f, "internal"),
        }
    }
}

/// Unified error type for all distributed runtime operations.
///
/// Carries a category, error code, human-readable message, optional source
/// error, context chain, and a suggested recovery action.
#[derive(Debug, Serialize, Deserialize)]
pub struct DistributedError {
    category: ErrorCategory,
    code: u16,
    message: String,
    context: Vec<String>,
    recovery: RecoveryAction,
}

impl DistributedError {
    /// Create a new distributed error.
    pub fn new(
        category: ErrorCategory,
        code: u16,
        message: impl Into<String>,
    ) -> Self {
        Self {
            category,
            code,
            message: message.into(),
            context: Vec::new(),
            recovery: RecoveryAction::Abort,
        }
    }

    /// Set the suggested recovery action.
    #[must_use]
    pub fn with_recovery(mut self, recovery: RecoveryAction) -> Self {
        self.recovery = recovery;
        self
    }

    /// Add a context string to the error chain.
    #[must_use]
    pub fn with_context(mut self, ctx: impl Into<String>) -> Self {
        self.context.push(ctx.into());
        self
    }

    /// Get the error category.
    pub fn category(&self) -> ErrorCategory {
        self.category
    }

    /// Get the error code.
    pub fn code(&self) -> u16 {
        self.code
    }

    /// Get the error message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Get the context chain.
    pub fn context(&self) -> &[String] {
        &self.context
    }

    /// Get the recovery action.
    pub fn recovery(&self) -> RecoveryAction {
        self.recovery
    }

    // --- Convenience constructors ---

    /// Cluster error.
    pub fn cluster(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Cluster, 1000, message)
    }

    /// Node error.
    pub fn node(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Node, 2000, message)
    }

    /// Discovery error.
    pub fn discovery(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Discovery, 3000, message)
    }

    /// Heartbeat error.
    pub fn heartbeat(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Heartbeat, 4000, message)
    }

    /// Failure detection error.
    pub fn failure(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Failure, 5000, message)
    }

    /// Scheduler error.
    pub fn scheduler(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Scheduler, 6000, message)
    }

    /// Execution error.
    pub fn execution(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Execution, 7000, message)
    }

    /// Network error.
    pub fn network(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Network, 8000, message)
    }

    /// Security error.
    pub fn security(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Security, 9000, message)
    }

    /// Consensus error.
    pub fn consensus(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Consensus, 10_000, message)
    }

    /// Memory error.
    pub fn memory(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Memory, 11_000, message)
    }

    /// Storage error.
    pub fn storage(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Storage, 12_000, message)
    }

    /// Configuration error.
    pub fn config(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Config, 13_000, message)
    }

    /// Timeout error.
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Timeout, 14_000, message)
            .with_recovery(RecoveryAction::Retry)
    }

    /// Internal error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorCategory::Internal, 15_000, message)
    }
}

impl fmt::Display for DistributedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}:{}] {}",
            self.category, self.code, self.message
        )?;
        for ctx in &self.context {
            write!(f, "\n  -> {ctx}")?;
        }
        Ok(())
    }
}

impl std::error::Error for DistributedError {}

impl From<std::io::Error> for DistributedError {
    fn from(e: std::io::Error) -> Self {
        Self::network(format!("io error: {e}"))
    }
}

impl From<serde_json::Error> for DistributedError {
    fn from(e: serde_json::Error) -> Self {
        Self::new(ErrorCategory::Serialization, 13_100, format!("json error: {e}"))
    }
}

impl From<bincode::Error> for DistributedError {
    fn from(e: bincode::Error) -> Self {
        Self::new(ErrorCategory::Serialization, 13_200, format!("bincode error: {e}"))
    }
}

/// Convenience result alias.
pub type NeoResult<T> = Result<T, DistributedError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_creation_and_display() {
        let err = DistributedError::cluster("insufficient nodes");
        assert_eq!(err.category(), ErrorCategory::Cluster);
        assert_eq!(err.message(), "insufficient nodes");
        let display = format!("{err}");
        assert!(display.contains("cluster"));
        assert!(display.contains("insufficient nodes"));
    }

    #[test]
    fn error_with_context() {
        let err = DistributedError::node("node unreachable")
            .with_context("during heartbeat check")
            .with_context("from coordinator");
        assert_eq!(err.context().len(), 2);
        assert_eq!(err.context()[0], "during heartbeat check");
    }

    #[test]
    fn error_with_recovery() {
        let err = DistributedError::timeout("operation timed out")
            .with_recovery(RecoveryAction::Failover);
        assert_eq!(err.recovery(), RecoveryAction::Failover);
    }

    #[test]
    fn io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");
        let err: DistributedError = io_err.into();
        assert_eq!(err.category(), ErrorCategory::Network);
    }

    #[test]
    fn json_error_conversion() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let err: DistributedError = json_err.into();
        assert_eq!(err.category(), ErrorCategory::Serialization);
    }

    #[test]
    fn recovery_action_display() {
        assert_eq!(RecoveryAction::Retry.to_string(), "retry");
        assert_eq!(RecoveryAction::Failover.to_string(), "failover");
        assert_eq!(RecoveryAction::Migrate.to_string(), "migrate");
    }
}
