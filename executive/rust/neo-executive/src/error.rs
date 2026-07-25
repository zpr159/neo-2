use std::fmt;

use serde::{Deserialize, Serialize};

/// Error codes specific to the executive system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u16)]
pub enum ExecutiveErrorCode {
    GoalNotFound = 100,
    GoalAlreadyCompleted = 101,
    GoalDependencyCycle = 102,
    GoalDecompositionFailed = 103,
    TaskNotFound = 200,
    TaskAlreadyCompleted = 201,
    TaskQueueFull = 202,
    TaskDependencyNotMet = 203,
    TaskDeadlineExceeded = 204,
    TaskOwnershipConflict = 205,
    SchedulerFull = 300,
    SchedulerShutdown = 301,
    PreemptionDenied = 302,
    AttentionBudgetExceeded = 400,
    ContextSwitchFailed = 401,
    ResourceAllocationFailed = 500,
    ResourceExhausted = 501,
    ModelAllocationFailed = 502,
    InferenceBudgetExceeded = 503,
    PolicyViolation = 600,
    SafeModeViolation = 601,
    RecoveryFailed = 700,
    CheckpointCorrupted = 701,
    FallbackExhausted = 702,
    SessionNotFound = 800,
    SessionExpired = 801,
    SerializationFailed = 900,
    InternalError = 999,
}

impl fmt::Display for ExecutiveErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GoalNotFound => write!(f, "goal not found"),
            Self::GoalAlreadyCompleted => write!(f, "goal already completed"),
            Self::GoalDependencyCycle => write!(f, "goal dependency cycle detected"),
            Self::GoalDecompositionFailed => write!(f, "goal decomposition failed"),
            Self::TaskNotFound => write!(f, "task not found"),
            Self::TaskAlreadyCompleted => write!(f, "task already completed"),
            Self::TaskQueueFull => write!(f, "task queue full"),
            Self::TaskDependencyNotMet => write!(f, "task dependency not met"),
            Self::TaskDeadlineExceeded => write!(f, "task deadline exceeded"),
            Self::TaskOwnershipConflict => write!(f, "task ownership conflict"),
            Self::SchedulerFull => write!(f, "scheduler full"),
            Self::SchedulerShutdown => write!(f, "scheduler shutdown"),
            Self::PreemptionDenied => write!(f, "preemption denied"),
            Self::AttentionBudgetExceeded => write!(f, "attention budget exceeded"),
            Self::ContextSwitchFailed => write!(f, "context switch failed"),
            Self::ResourceAllocationFailed => write!(f, "resource allocation failed"),
            Self::ResourceExhausted => write!(f, "resource exhausted"),
            Self::ModelAllocationFailed => write!(f, "model allocation failed"),
            Self::InferenceBudgetExceeded => write!(f, "inference budget exceeded"),
            Self::PolicyViolation => write!(f, "policy violation"),
            Self::SafeModeViolation => write!(f, "safe mode violation"),
            Self::RecoveryFailed => write!(f, "recovery failed"),
            Self::CheckpointCorrupted => write!(f, "checkpoint corrupted"),
            Self::FallbackExhausted => write!(f, "fallback exhausted"),
            Self::SessionNotFound => write!(f, "session not found"),
            Self::SessionExpired => write!(f, "session expired"),
            Self::SerializationFailed => write!(f, "serialization failed"),
            Self::InternalError => write!(f, "internal error"),
        }
    }
}

/// The primary error type for the executive system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutiveError {
    code: ExecutiveErrorCode,
    message: String,
    context: Vec<String>,
}

impl ExecutiveError {
    /// Create a new executive error.
    pub fn new(code: ExecutiveErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            context: Vec::new(),
        }
    }

    /// Add context to the error.
    #[must_use]
    pub fn with_context(mut self, ctx: impl Into<String>) -> Self {
        self.context.push(ctx.into());
        self
    }

    /// Get the error code.
    pub fn code(&self) -> ExecutiveErrorCode {
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

    /// Create a goal-not-found error.
    pub fn goal_not_found(id: &str) -> Self {
        Self::new(
            ExecutiveErrorCode::GoalNotFound,
            format!("goal '{}' not found", id),
        )
    }

    /// Create a task-not-found error.
    pub fn task_not_found(id: &str) -> Self {
        Self::new(
            ExecutiveErrorCode::TaskNotFound,
            format!("task '{}' not found", id),
        )
    }

    /// Create a session-not-found error.
    pub fn session_not_found(id: &str) -> Self {
        Self::new(
            ExecutiveErrorCode::SessionNotFound,
            format!("session '{}' not found", id),
        )
    }

    /// Create a policy violation error.
    pub fn policy_violation(msg: impl Into<String>) -> Self {
        Self::new(ExecutiveErrorCode::PolicyViolation, msg)
    }

    /// Create an internal error.
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::new(ExecutiveErrorCode::InternalError, msg)
    }
}

impl fmt::Display for ExecutiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)?;
        for ctx in &self.context {
            write!(f, "\n  -> {}", ctx)?;
        }
        Ok(())
    }
}

impl std::error::Error for ExecutiveError {}

impl From<ExecutiveError> for neo_core::error::NeoError {
    fn from(e: ExecutiveError) -> Self {
        neo_core::error::NeoError::Internal(e.to_string())
    }
}

/// Convenience result alias for executive operations.
pub type ExecutiveResult<T> = Result<T, ExecutiveError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_creation() {
        let err = ExecutiveError::new(ExecutiveErrorCode::GoalNotFound, "missing goal");
        assert_eq!(err.code(), ExecutiveErrorCode::GoalNotFound);
        assert_eq!(err.message(), "missing goal");
    }

    #[test]
    fn error_with_context() {
        let err = ExecutiveError::new(ExecutiveErrorCode::TaskNotFound, "missing")
            .with_context("in session abc");
        assert_eq!(err.context().len(), 1);
        assert_eq!(err.context()[0], "in session abc");
    }

    #[test]
    fn error_display() {
        let err = ExecutiveError::new(ExecutiveErrorCode::InternalError, "boom");
        let display = format!("{}", err);
        assert!(display.contains("boom"));
        assert!(display.contains("internal error"));
    }

    #[test]
    fn error_into_neo_error() {
        let err = ExecutiveError::new(ExecutiveErrorCode::GoalNotFound, "gone");
        let neo_err: neo_core::error::NeoError = err.into();
        assert!(format!("{}", neo_err).contains("gone"));
    }

    #[test]
    fn helper_constructors() {
        let _ = ExecutiveError::goal_not_found("g1");
        let _ = ExecutiveError::task_not_found("t1");
        let _ = ExecutiveError::session_not_found("s1");
        let _ = ExecutiveError::policy_violation("not allowed");
        let _ = ExecutiveError::internal("oops");
    }
}
