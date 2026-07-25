//! Error types for the Neo Planning System.

use std::fmt;

/// Error codes specific to the planning system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u16)]
pub enum PlanningErrorCode {
    GoalNotFound = 100,
    GoalAlreadyCompleted = 101,
    GoalDependencyCycle = 102,
    GoalDecompositionFailed = 103,
    GoalConstraintViolation = 104,
    PlanNotFound = 200,
    PlanAlreadyExecuting = 201,
    PlanInvalidState = 202,
    PlanValidationFailed = 203,
    PlanGraphCycleDetected = 204,
    StrategyNotFound = 300,
    StrategyEvaluationFailed = 301,
    AlgorithmNotSupported = 302,
    AlgorithmExecutionFailed = 303,
    OptimizationFailed = 400,
    OptimizationRuleViolation = 401,
    ReplanningFailed = 500,
    ReplanningConflict = 501,
    ResourceAllocationFailed = 600,
    ResourceExhausted = 601,
    ResourceConflict = 602,
    BudgetExceeded = 603,
    RiskThresholdExceeded = 700,
    RiskAssessmentFailed = 701,
    CostEstimationFailed = 800,
    CostBudgetExceeded = 801,
    WorkflowSynthesisFailed = 900,
    CapabilitySelectionFailed = 901,
    ToolSelectionFailed = 902,
    AgentAllocationFailed = 1000,
    ConsensusFailed = 1001,
    SessionNotFound = 1100,
    SessionExpired = 1101,
    SerializationFailed = 1200,
    PersistenceFailed = 1201,
    PolicyViolation = 1300,
    Unauthorized = 1301,
    InternalError = 9999,
}

impl fmt::Display for PlanningErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GoalNotFound => write!(f, "GoalNotFound"),
            Self::GoalAlreadyCompleted => write!(f, "GoalAlreadyCompleted"),
            Self::GoalDependencyCycle => write!(f, "GoalDependencyCycle"),
            Self::GoalDecompositionFailed => write!(f, "GoalDecompositionFailed"),
            Self::GoalConstraintViolation => write!(f, "GoalConstraintViolation"),
            Self::PlanNotFound => write!(f, "PlanNotFound"),
            Self::PlanAlreadyExecuting => write!(f, "PlanAlreadyExecuting"),
            Self::PlanInvalidState => write!(f, "PlanInvalidState"),
            Self::PlanValidationFailed => write!(f, "PlanValidationFailed"),
            Self::PlanGraphCycleDetected => write!(f, "PlanGraphCycleDetected"),
            Self::StrategyNotFound => write!(f, "StrategyNotFound"),
            Self::StrategyEvaluationFailed => write!(f, "StrategyEvaluationFailed"),
            Self::AlgorithmNotSupported => write!(f, "AlgorithmNotSupported"),
            Self::AlgorithmExecutionFailed => write!(f, "AlgorithmExecutionFailed"),
            Self::OptimizationFailed => write!(f, "OptimizationFailed"),
            Self::OptimizationRuleViolation => write!(f, "OptimizationRuleViolation"),
            Self::ReplanningFailed => write!(f, "ReplanningFailed"),
            Self::ReplanningConflict => write!(f, "ReplanningConflict"),
            Self::ResourceAllocationFailed => write!(f, "ResourceAllocationFailed"),
            Self::ResourceExhausted => write!(f, "ResourceExhausted"),
            Self::ResourceConflict => write!(f, "ResourceConflict"),
            Self::BudgetExceeded => write!(f, "BudgetExceeded"),
            Self::RiskThresholdExceeded => write!(f, "RiskThresholdExceeded"),
            Self::RiskAssessmentFailed => write!(f, "RiskAssessmentFailed"),
            Self::CostEstimationFailed => write!(f, "CostEstimationFailed"),
            Self::CostBudgetExceeded => write!(f, "CostBudgetExceeded"),
            Self::WorkflowSynthesisFailed => write!(f, "WorkflowSynthesisFailed"),
            Self::CapabilitySelectionFailed => write!(f, "CapabilitySelectionFailed"),
            Self::ToolSelectionFailed => write!(f, "ToolSelectionFailed"),
            Self::AgentAllocationFailed => write!(f, "AgentAllocationFailed"),
            Self::ConsensusFailed => write!(f, "ConsensusFailed"),
            Self::SessionNotFound => write!(f, "SessionNotFound"),
            Self::SessionExpired => write!(f, "SessionExpired"),
            Self::SerializationFailed => write!(f, "SerializationFailed"),
            Self::PersistenceFailed => write!(f, "PersistenceFailed"),
            Self::PolicyViolation => write!(f, "PolicyViolation"),
            Self::Unauthorized => write!(f, "Unauthorized"),
            Self::InternalError => write!(f, "InternalError"),
        }
    }
}

/// Primary error type for the planning system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningError {
    code: PlanningErrorCode,
    message: String,
    context: Vec<String>,
}

impl PlanningError {
    /// Create a new planning error.
    pub fn new(code: PlanningErrorCode, message: impl Into<String>) -> Self {
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
    pub fn code(&self) -> PlanningErrorCode {
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
            PlanningErrorCode::GoalNotFound,
            format!("goal '{}' not found", id),
        )
    }

    /// Create a plan-not-found error.
    pub fn plan_not_found(id: &str) -> Self {
        Self::new(
            PlanningErrorCode::PlanNotFound,
            format!("plan '{}' not found", id),
        )
    }

    /// Create a strategy-not-found error.
    pub fn strategy_not_found(id: &str) -> Self {
        Self::new(
            PlanningErrorCode::StrategyNotFound,
            format!("strategy '{}' not found", id),
        )
    }

    /// Create an internal error.
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::new(PlanningErrorCode::InternalError, msg)
    }

    /// Create a validation error.
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::new(PlanningErrorCode::PlanValidationFailed, msg)
    }

    /// Create a budget exceeded error.
    pub fn budget_exceeded(msg: impl Into<String>) -> Self {
        Self::new(PlanningErrorCode::CostBudgetExceeded, msg)
    }

    /// Create a risk threshold error.
    pub fn risk_exceeded(msg: impl Into<String>) -> Self {
        Self::new(PlanningErrorCode::RiskThresholdExceeded, msg)
    }

    /// Create a session-not-found error.
    pub fn session_not_found(id: &str) -> Self {
        Self::new(
            PlanningErrorCode::SessionNotFound,
            format!("session '{}' not found", id),
        )
    }

    /// Create a policy violation error.
    pub fn policy_violation(msg: impl Into<String>) -> Self {
        Self::new(PlanningErrorCode::PolicyViolation, msg)
    }

    /// Create a resource exhausted error.
    pub fn resource_exhausted(msg: impl Into<String>) -> Self {
        Self::new(PlanningErrorCode::ResourceExhausted, msg)
    }
}

impl fmt::Display for PlanningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)?;
        for ctx in &self.context {
            write!(f, "\n  -> {}", ctx)?;
        }
        Ok(())
    }
}

impl std::error::Error for PlanningError {}

impl From<PlanningError> for neo_core::error::NeoError {
    fn from(e: PlanningError) -> Self {
        neo_core::error::NeoError::Internal(e.to_string())
    }
}

/// Convenience result alias for planning operations.
pub type PlanningResult<T> = Result<T, PlanningError>;

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_creation() {
        let err = PlanningError::new(PlanningErrorCode::GoalNotFound, "missing goal");
        assert_eq!(err.code(), PlanningErrorCode::GoalNotFound);
        assert_eq!(err.message(), "missing goal");
    }

    #[test]
    fn error_with_context() {
        let err = PlanningError::new(PlanningErrorCode::PlanNotFound, "missing")
            .with_context("in session abc");
        assert_eq!(err.context().len(), 1);
        assert_eq!(err.context()[0], "in session abc");
    }

    #[test]
    fn error_display() {
        let err = PlanningError::new(PlanningErrorCode::InternalError, "boom");
        let display = format!("{}", err);
        assert!(display.contains("boom"));
        assert!(display.contains("InternalError"));
    }

    #[test]
    fn error_into_neo_error() {
        let err = PlanningError::new(PlanningErrorCode::GoalNotFound, "gone");
        let neo_err: neo_core::error::NeoError = err.into();
        assert!(format!("{}", neo_err).contains("gone"));
    }

    #[test]
    fn helper_constructors() {
        let _ = PlanningError::goal_not_found("g1");
        let _ = PlanningError::plan_not_found("p1");
        let _ = PlanningError::strategy_not_found("s1");
        let _ = PlanningError::internal("oops");
        let _ = PlanningError::validation("bad");
        let _ = PlanningError::budget_exceeded("over budget");
        let _ = PlanningError::risk_exceeded("too risky");
        let _ = PlanningError::session_not_found("s1");
        let _ = PlanningError::policy_violation("nope");
        let _ = PlanningError::resource_exhausted("empty");
    }
}
