use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum ReasoningErrorCode {
    SessionNotFound = 2000,
    SessionCancelled = 2001,
    SessionTimeout = 2002,
    SessionCompleted = 2003,
    InvalidState = 2004,
    StrategyNotFound = 2010,
    StrategyExecutionFailed = 2011,
    PlanningFailed = 2020,
    CircularDependency = 2021,
    NoPlan = 2022,
    ReflectionFailed = 2030,
    InconsistentResult = 2031,
    HypothesisRejected = 2040,
    NoHypotheses = 2041,
    DecisionFailed = 2050,
    NoOptions = 2051,
    KnowledgeIntegrationFailed = 2060,
    CacheError = 2070,
    ToolPlanFailed = 2080,
    ToolExecutionFailed = 2081,
    ModelNotFound = 2090,
    AllModelsFailed = 2091,
    ConsensusNotReached = 2092,
    ExplanationFailed = 2100,
    InternalError = 9000,
}

#[derive(Debug)]
pub enum ReasoningError {
    SessionNotFound(String),
    SessionCancelled(String),
    SessionTimeout(String),
    SessionCompleted(String),
    InvalidState(String),
    StrategyNotFound(String),
    StrategyExecutionFailed(String),
    PlanningFailed(String),
    CircularDependency(String),
    NoPlan(String),
    ReflectionFailed(String),
    InconsistentResult(String),
    HypothesisRejected(String),
    NoHypotheses(String),
    DecisionFailed(String),
    NoOptions(String),
    KnowledgeIntegrationFailed(String),
    CacheError(String),
    ToolPlanFailed(String),
    ToolExecutionFailed(String),
    ModelNotFound(String),
    AllModelsFailed(String),
    ConsensusNotReached(String),
    ExplanationFailed(String),
    InternalError(String),
}

impl ReasoningError {
    pub fn code(&self) -> ReasoningErrorCode {
        match self {
            ReasoningError::SessionNotFound(_) => ReasoningErrorCode::SessionNotFound,
            ReasoningError::SessionCancelled(_) => ReasoningErrorCode::SessionCancelled,
            ReasoningError::SessionTimeout(_) => ReasoningErrorCode::SessionTimeout,
            ReasoningError::SessionCompleted(_) => ReasoningErrorCode::SessionCompleted,
            ReasoningError::InvalidState(_) => ReasoningErrorCode::InvalidState,
            ReasoningError::StrategyNotFound(_) => ReasoningErrorCode::StrategyNotFound,
            ReasoningError::StrategyExecutionFailed(_) => {
                ReasoningErrorCode::StrategyExecutionFailed
            }
            ReasoningError::PlanningFailed(_) => ReasoningErrorCode::PlanningFailed,
            ReasoningError::CircularDependency(_) => ReasoningErrorCode::CircularDependency,
            ReasoningError::NoPlan(_) => ReasoningErrorCode::NoPlan,
            ReasoningError::ReflectionFailed(_) => ReasoningErrorCode::ReflectionFailed,
            ReasoningError::InconsistentResult(_) => ReasoningErrorCode::InconsistentResult,
            ReasoningError::HypothesisRejected(_) => ReasoningErrorCode::HypothesisRejected,
            ReasoningError::NoHypotheses(_) => ReasoningErrorCode::NoHypotheses,
            ReasoningError::DecisionFailed(_) => ReasoningErrorCode::DecisionFailed,
            ReasoningError::NoOptions(_) => ReasoningErrorCode::NoOptions,
            ReasoningError::KnowledgeIntegrationFailed(_) => {
                ReasoningErrorCode::KnowledgeIntegrationFailed
            }
            ReasoningError::CacheError(_) => ReasoningErrorCode::CacheError,
            ReasoningError::ToolPlanFailed(_) => ReasoningErrorCode::ToolPlanFailed,
            ReasoningError::ToolExecutionFailed(_) => ReasoningErrorCode::ToolExecutionFailed,
            ReasoningError::ModelNotFound(_) => ReasoningErrorCode::ModelNotFound,
            ReasoningError::AllModelsFailed(_) => ReasoningErrorCode::AllModelsFailed,
            ReasoningError::ConsensusNotReached(_) => ReasoningErrorCode::ConsensusNotReached,
            ReasoningError::ExplanationFailed(_) => ReasoningErrorCode::ExplanationFailed,
            ReasoningError::InternalError(_) => ReasoningErrorCode::InternalError,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            ReasoningError::SessionNotFound(m)
            | ReasoningError::SessionCancelled(m)
            | ReasoningError::SessionTimeout(m)
            | ReasoningError::SessionCompleted(m)
            | ReasoningError::InvalidState(m)
            | ReasoningError::StrategyNotFound(m)
            | ReasoningError::StrategyExecutionFailed(m)
            | ReasoningError::PlanningFailed(m)
            | ReasoningError::CircularDependency(m)
            | ReasoningError::NoPlan(m)
            | ReasoningError::ReflectionFailed(m)
            | ReasoningError::InconsistentResult(m)
            | ReasoningError::HypothesisRejected(m)
            | ReasoningError::NoHypotheses(m)
            | ReasoningError::DecisionFailed(m)
            | ReasoningError::NoOptions(m)
            | ReasoningError::KnowledgeIntegrationFailed(m)
            | ReasoningError::CacheError(m)
            | ReasoningError::ToolPlanFailed(m)
            | ReasoningError::ToolExecutionFailed(m)
            | ReasoningError::ModelNotFound(m)
            | ReasoningError::AllModelsFailed(m)
            | ReasoningError::ConsensusNotReached(m)
            | ReasoningError::ExplanationFailed(m)
            | ReasoningError::InternalError(m) => m,
        }
    }
}

impl fmt::Display for ReasoningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[reasoning:{:?}] {}", self.code(), self.message())
    }
}

impl std::error::Error for ReasoningError {}

impl From<ReasoningError> for neo_core::error::NeoError {
    fn from(e: ReasoningError) -> Self {
        neo_core::error::NeoError::Internal(e.to_string())
    }
}

impl From<neo_core::error::NeoError> for ReasoningError {
    fn from(e: neo_core::error::NeoError) -> Self {
        ReasoningError::InternalError(e.to_string())
    }
}

pub type ReasoningResult<T> = Result<T, ReasoningError>;
