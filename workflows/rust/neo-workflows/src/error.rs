use std::fmt;

use crate::core::WorkflowState;

/// Workflow-specific error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u16)]
pub enum WorkflowErrorCode {
    /// The workflow definition is invalid.
    InvalidDefinition = 100,
    /// The workflow was not found.
    NotFound = 200,
    /// The workflow is in an invalid state for the requested operation.
    InvalidState = 300,
    /// A cycle was detected in the DAG.
    CycleDetected = 400,
    /// A node was not found in the workflow.
    NodeNotFound = 500,
    /// An edge references a non-existent node.
    InvalidEdge = 600,
    /// Execution timed out.
    Timeout = 700,
    /// The workflow was cancelled.
    Cancelled = 800,
    /// A required dependency is missing.
    DependencyMissing = 900,
    /// Checkpoint creation or recovery failed.
    CheckpointError = 1000,
    /// Rollback or compensation failed.
    RollbackFailed = 1100,
    /// Variable resolution failed.
    VariableError = 1200,
    /// The workflow version is incompatible.
    VersionConflict = 1300,
    /// Scheduling error.
    ScheduleError = 1400,
    /// Persistence error.
    PersistenceError = 1500,
    /// Serialization error.
    SerializationError = 1600,
    /// The operation is not implemented.
    NotImplemented = 1700,
    /// An internal error occurred.
    Internal = 9000,
}

impl fmt::Display for WorkflowErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDefinition => write!(f, "InvalidDefinition"),
            Self::NotFound => write!(f, "NotFound"),
            Self::InvalidState => write!(f, "InvalidState"),
            Self::CycleDetected => write!(f, "CycleDetected"),
            Self::NodeNotFound => write!(f, "NodeNotFound"),
            Self::InvalidEdge => write!(f, "InvalidEdge"),
            Self::Timeout => write!(f, "Timeout"),
            Self::Cancelled => write!(f, "Cancelled"),
            Self::DependencyMissing => write!(f, "DependencyMissing"),
            Self::CheckpointError => write!(f, "CheckpointError"),
            Self::RollbackFailed => write!(f, "RollbackFailed"),
            Self::VariableError => write!(f, "VariableError"),
            Self::VersionConflict => write!(f, "VersionConflict"),
            Self::ScheduleError => write!(f, "ScheduleError"),
            Self::PersistenceError => write!(f, "PersistenceError"),
            Self::SerializationError => write!(f, "SerializationError"),
            Self::NotImplemented => write!(f, "NotImplemented"),
            Self::Internal => write!(f, "Internal"),
        }
    }
}

/// Error type for workflow operations.
#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    /// The workflow definition is invalid.
    #[error("invalid workflow definition: {message}")]
    InvalidDefinition {
        /// Description of the validation failure.
        message: String,
    },

    /// The workflow was not found.
    #[error("workflow not found: {0}")]
    NotFound(String),

    /// The workflow is in an invalid state for the requested operation.
    #[error("invalid state: workflow is {current}, but {operation} requires {required}")]
    InvalidState {
        /// The current state of the workflow.
        current: WorkflowState,
        /// The operation that was attempted.
        operation: String,
        /// The state(s) required for the operation.
        required: String,
    },

    /// A cycle was detected in the workflow DAG.
    #[error("cycle detected in workflow graph: {0}")]
    CycleDetected(String),

    /// A node was not found in the workflow.
    #[error("node not found: {0}")]
    NodeNotFound(String),

    /// An edge references a non-existent node.
    #[error("invalid edge: {message}")]
    InvalidEdge {
        /// Description of the edge problem.
        message: String,
    },

    /// Execution timed out.
    #[error("workflow execution timed out after {timeout_ms}ms")]
    Timeout {
        /// The timeout in milliseconds.
        timeout_ms: u64,
    },

    /// The workflow was cancelled.
    #[error("workflow cancelled: {0}")]
    Cancelled(String),

    /// A required dependency is missing.
    #[error("dependency missing: {0}")]
    DependencyMissing(String),

    /// Checkpoint creation or recovery failed.
    #[error("checkpoint error: {0}")]
    CheckpointError(String),

    /// Rollback or compensation failed.
    #[error("rollback failed at node {node_id}: {message}")]
    RollbackFailed {
        /// The node where rollback failed.
        node_id: String,
        /// Description of the failure.
        message: String,
    },

    /// Variable resolution failed.
    #[error("variable error: {0}")]
    VariableError(String),

    /// The workflow version is incompatible.
    #[error("version conflict: {0}")]
    VersionConflict(String),

    /// Scheduling error.
    #[error("schedule error: {0}")]
    ScheduleError(String),

    /// Persistence error.
    #[error("persistence error: {0}")]
    PersistenceError(String),

    /// Serialization error.
    #[error("serialization error: {0}")]
    SerializationError(String),

    /// The operation is not implemented.
    #[error("not implemented: {0}")]
    NotImplemented(String),

    /// An internal error occurred.
    #[error("internal error: {0}")]
    Internal(String),
}

impl WorkflowError {
    /// Returns the error code for this error.
    #[must_use]
    pub fn code(&self) -> WorkflowErrorCode {
        match self {
            Self::InvalidDefinition { .. } => WorkflowErrorCode::InvalidDefinition,
            Self::NotFound(_) => WorkflowErrorCode::NotFound,
            Self::InvalidState { .. } => WorkflowErrorCode::InvalidState,
            Self::CycleDetected(_) => WorkflowErrorCode::CycleDetected,
            Self::NodeNotFound(_) => WorkflowErrorCode::NodeNotFound,
            Self::InvalidEdge { .. } => WorkflowErrorCode::InvalidEdge,
            Self::Timeout { .. } => WorkflowErrorCode::Timeout,
            Self::Cancelled(_) => WorkflowErrorCode::Cancelled,
            Self::DependencyMissing(_) => WorkflowErrorCode::DependencyMissing,
            Self::CheckpointError(_) => WorkflowErrorCode::CheckpointError,
            Self::RollbackFailed { .. } => WorkflowErrorCode::RollbackFailed,
            Self::VariableError(_) => WorkflowErrorCode::VariableError,
            Self::VersionConflict(_) => WorkflowErrorCode::VersionConflict,
            Self::ScheduleError(_) => WorkflowErrorCode::ScheduleError,
            Self::PersistenceError(_) => WorkflowErrorCode::PersistenceError,
            Self::SerializationError(_) => WorkflowErrorCode::SerializationError,
            Self::NotImplemented(_) => WorkflowErrorCode::NotImplemented,
            Self::Internal(_) => WorkflowErrorCode::Internal,
        }
    }

    /// Creates an `InvalidDefinition` error.
    pub fn invalid_definition(message: impl Into<String>) -> Self {
        Self::InvalidDefinition {
            message: message.into(),
        }
    }

    /// Creates a `NotFound` error.
    pub fn not_found(id: impl fmt::Display) -> Self {
        Self::NotFound(id.to_string())
    }

    /// Creates an `InvalidState` error.
    pub fn invalid_state(
        current: WorkflowState,
        operation: impl Into<String>,
        required: impl Into<String>,
    ) -> Self {
        Self::InvalidState {
            current,
            operation: operation.into(),
            required: required.into(),
        }
    }

    /// Creates a `NodeNotFound` error.
    pub fn node_not_found(id: impl fmt::Display) -> Self {
        Self::NodeNotFound(id.to_string())
    }

    /// Creates a `Timeout` error.
    pub fn timeout(timeout_ms: u64) -> Self {
        Self::Timeout { timeout_ms }
    }

    /// Creates a `RollbackFailed` error.
    pub fn rollback_failed(node_id: impl fmt::Display, message: impl Into<String>) -> Self {
        Self::RollbackFailed {
            node_id: node_id.to_string(),
            message: message.into(),
        }
    }

    /// Creates an `Internal` error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }
}

/// Result type for workflow operations.
pub type WorkflowResult<T> = Result<T, WorkflowError>;

impl From<WorkflowError> for neo_core::NeoError {
    fn from(e: WorkflowError) -> Self {
        neo_core::NeoError::Internal(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_are_unique() {
        let codes = [
            WorkflowErrorCode::InvalidDefinition,
            WorkflowErrorCode::NotFound,
            WorkflowErrorCode::InvalidState,
            WorkflowErrorCode::CycleDetected,
            WorkflowErrorCode::NodeNotFound,
            WorkflowErrorCode::InvalidEdge,
            WorkflowErrorCode::Timeout,
            WorkflowErrorCode::Cancelled,
            WorkflowErrorCode::DependencyMissing,
            WorkflowErrorCode::CheckpointError,
            WorkflowErrorCode::RollbackFailed,
            WorkflowErrorCode::VariableError,
            WorkflowErrorCode::VersionConflict,
            WorkflowErrorCode::ScheduleError,
            WorkflowErrorCode::PersistenceError,
            WorkflowErrorCode::SerializationError,
            WorkflowErrorCode::NotImplemented,
            WorkflowErrorCode::Internal,
        ];
        let mut seen = std::collections::HashSet::new();
        for code in &codes {
            assert!(seen.insert(*code), "duplicate error code: {code}");
        }
    }

    #[test]
    fn error_display() {
        let err = WorkflowError::not_found("wf-123");
        assert!(err.to_string().contains("wf-123"));
    }

    #[test]
    fn error_code_mapping() {
        let err = WorkflowError::invalid_definition("bad");
        assert_eq!(err.code(), WorkflowErrorCode::InvalidDefinition);
    }

    #[test]
    fn error_convenience_constructors() {
        let err = WorkflowError::invalid_state(WorkflowState::Completed, "run", "Created, Queued");
        assert!(err.to_string().contains("Completed"));
        assert!(err.to_string().contains("run"));

        let err = WorkflowError::timeout(5000);
        assert!(err.to_string().contains("5000"));

        let err = WorkflowError::rollback_failed("node-1", "comp failed");
        assert!(err.to_string().contains("node-1"));

        let err = WorkflowError::internal("something broke");
        assert_eq!(err.code(), WorkflowErrorCode::Internal);
    }

    #[test]
    fn conversion_to_neo_error() {
        let wf_err = WorkflowError::not_found("test");
        let neo_err: neo_core::NeoError = wf_err.into();
        assert!(neo_err.to_string().contains("test"));
    }
}
