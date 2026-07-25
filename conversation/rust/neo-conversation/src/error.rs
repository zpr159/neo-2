use std::fmt;

/// Error type for the conversation subsystem.
#[derive(Debug)]
pub enum ConversationError {
    SessionNotFound(String),
    SessionExpired(String),
    LanguageEngineError(String),
    ContextError(String),
    ContextOverflow(String),
    ToolError(String),
    PipelineError(String),
    StreamError(String),
    Serialization(String),
    InvalidInput(String),
    Internal(String),
    NotInitialized,
    EngineUnavailable(String),
    TokenLimitExceeded { limit: usize, actual: usize },
    PromptConstructionFailed(String),
    ResponseValidationFailed(String),
    HistoryCorrupted(String),
    ProviderUnavailable(String),
    StreamingInterrupted(String),
    ToolExecutionFailed(String),
    InvalidStateTransition {
        from: String,
        to: String,
    },
    HallucinationDetected(String),
    PolicyViolation(String),
    Cancelled(String),
}

impl fmt::Display for ConversationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SessionNotFound(msg) => write!(f, "[session not found] {msg}"),
            Self::SessionExpired(msg) => write!(f, "[session expired] {msg}"),
            Self::LanguageEngineError(msg) => write!(f, "[language engine] {msg}"),
            Self::ContextError(msg) => write!(f, "[context] {msg}"),
            Self::ContextOverflow(msg) => write!(f, "[context overflow] {msg}"),
            Self::ToolError(msg) => write!(f, "[tool] {msg}"),
            Self::PipelineError(msg) => write!(f, "[pipeline] {msg}"),
            Self::StreamError(msg) => write!(f, "[stream] {msg}"),
            Self::Serialization(msg) => write!(f, "[serialization] {msg}"),
            Self::InvalidInput(msg) => write!(f, "[invalid input] {msg}"),
            Self::Internal(msg) => write!(f, "[internal] {msg}"),
            Self::NotInitialized => write!(f, "[not initialized]"),
            Self::EngineUnavailable(msg) => write!(f, "[engine unavailable] {msg}"),
            Self::TokenLimitExceeded { limit, actual } => {
                write!(f, "[token limit] exceeded limit of {limit} with {actual} tokens")
            }
            Self::PromptConstructionFailed(msg) => {
                write!(f, "[prompt construction] {msg}")
            }
            Self::ResponseValidationFailed(msg) => {
                write!(f, "[response validation] {msg}")
            }
            Self::HistoryCorrupted(msg) => write!(f, "[history corrupted] {msg}"),
            Self::ProviderUnavailable(msg) => write!(f, "[provider unavailable] {msg}"),
            Self::StreamingInterrupted(msg) => {
                write!(f, "[streaming interrupted] {msg}")
            }
            Self::ToolExecutionFailed(msg) => {
                write!(f, "[tool execution failed] {msg}")
            }
            Self::InvalidStateTransition { from, to } => {
                write!(f, "[invalid state] cannot transition from '{from}' to '{to}'")
            }
            Self::HallucinationDetected(msg) => {
                write!(f, "[hallucination] {msg}")
            }
            Self::PolicyViolation(msg) => write!(f, "[policy violation] {msg}"),
            Self::Cancelled(msg) => write!(f, "[cancelled] {msg}"),
        }
    }
}

impl std::error::Error for ConversationError {}

impl From<ConversationError> for neo_core::NeoError {
    fn from(e: ConversationError) -> Self {
        neo_core::NeoError::Internal(e.to_string())
    }
}

impl From<serde_json::Error> for ConversationError {
    fn from(e: serde_json::Error) -> Self {
        Self::Serialization(e.to_string())
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for ConversationError {
    fn from(e: tokio::sync::oneshot::error::RecvError) -> Self {
        Self::Internal(e.to_string())
    }
}

/// Result type for conversation operations.
pub type ConversationResult<T> = Result<T, ConversationError>;
