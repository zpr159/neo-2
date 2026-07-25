use std::fmt;

use crate::error::NeoError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum ConversationErrorCode {
    IntentClassificationFailed = 3000,
    ExecutiveRejected = 3001,
    PlanningFailed = 3002,
    ReasoningFailed = 3003,
    MemoryRetrievalFailed = 3004,
    KnowledgeRetrievalFailed = 3005,
    WorldModelFailed = 3006,
    WorkflowFailed = 3007,
    AgentFailed = 3008,
    ToolExecutionFailed = 3009,
    ToolAuthorizationDenied = 3010,
    ToolNotFound = 3011,
    FunctionCallFailed = 3012,
    PromptBuildFailed = 3013,
    ResponseValidationFailed = 3014,
    ContextAssemblyFailed = 3015,
    PipelineError = 3016,
    SessionNotFound = 3017,
    ConversationNotFound = 3018,
    ConversationCancelled = 3019,
    ConversationTimeout = 3020,
    ProviderError = 3021,
    SerializationFailed = 3022,
    ConfigurationInvalid = 3023,
    InternalError = 3099,
}

#[derive(Debug, Clone)]
pub enum ConversationError {
    IntentClassificationFailed(String),
    ExecutiveRejected(String),
    PlanningFailed(String),
    ReasoningFailed(String),
    MemoryRetrievalFailed(String),
    KnowledgeRetrievalFailed(String),
    WorldModelFailed(String),
    WorkflowFailed(String),
    AgentFailed(String),
    ToolExecutionFailed(String),
    ToolAuthorizationDenied(String),
    ToolNotFound(String),
    FunctionCallFailed(String),
    PromptBuildFailed(String),
    ResponseValidationFailed(String),
    ContextAssemblyFailed(String),
    PipelineError(String),
    SessionNotFound(String),
    ConversationNotFound(String),
    ConversationCancelled(String),
    ConversationTimeout(String),
    ProviderError(String),
    SerializationFailed(String),
    ConfigurationInvalid(String),
    InternalError(String),
}

impl ConversationError {
    pub fn code(&self) -> ConversationErrorCode {
        match self {
            Self::IntentClassificationFailed(_) => ConversationErrorCode::IntentClassificationFailed,
            Self::ExecutiveRejected(_) => ConversationErrorCode::ExecutiveRejected,
            Self::PlanningFailed(_) => ConversationErrorCode::PlanningFailed,
            Self::ReasoningFailed(_) => ConversationErrorCode::ReasoningFailed,
            Self::MemoryRetrievalFailed(_) => ConversationErrorCode::MemoryRetrievalFailed,
            Self::KnowledgeRetrievalFailed(_) => ConversationErrorCode::KnowledgeRetrievalFailed,
            Self::WorldModelFailed(_) => ConversationErrorCode::WorldModelFailed,
            Self::WorkflowFailed(_) => ConversationErrorCode::WorkflowFailed,
            Self::AgentFailed(_) => ConversationErrorCode::AgentFailed,
            Self::ToolExecutionFailed(_) => ConversationErrorCode::ToolExecutionFailed,
            Self::ToolAuthorizationDenied(_) => ConversationErrorCode::ToolAuthorizationDenied,
            Self::ToolNotFound(_) => ConversationErrorCode::ToolNotFound,
            Self::FunctionCallFailed(_) => ConversationErrorCode::FunctionCallFailed,
            Self::PromptBuildFailed(_) => ConversationErrorCode::PromptBuildFailed,
            Self::ResponseValidationFailed(_) => ConversationErrorCode::ResponseValidationFailed,
            Self::ContextAssemblyFailed(_) => ConversationErrorCode::ContextAssemblyFailed,
            Self::PipelineError(_) => ConversationErrorCode::PipelineError,
            Self::SessionNotFound(_) => ConversationErrorCode::SessionNotFound,
            Self::ConversationNotFound(_) => ConversationErrorCode::ConversationNotFound,
            Self::ConversationCancelled(_) => ConversationErrorCode::ConversationCancelled,
            Self::ConversationTimeout(_) => ConversationErrorCode::ConversationTimeout,
            Self::ProviderError(_) => ConversationErrorCode::ProviderError,
            Self::SerializationFailed(_) => ConversationErrorCode::SerializationFailed,
            Self::ConfigurationInvalid(_) => ConversationErrorCode::ConfigurationInvalid,
            Self::InternalError(_) => ConversationErrorCode::InternalError,
        }
    }
}

impl fmt::Display for ConversationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IntentClassificationFailed(m) => write!(f, "[intent classification failed] {}", m),
            Self::ExecutiveRejected(m) => write!(f, "[executive rejected] {}", m),
            Self::PlanningFailed(m) => write!(f, "[planning failed] {}", m),
            Self::ReasoningFailed(m) => write!(f, "[reasoning failed] {}", m),
            Self::MemoryRetrievalFailed(m) => write!(f, "[memory retrieval failed] {}", m),
            Self::KnowledgeRetrievalFailed(m) => write!(f, "[knowledge retrieval failed] {}", m),
            Self::WorldModelFailed(m) => write!(f, "[world model failed] {}", m),
            Self::WorkflowFailed(m) => write!(f, "[workflow failed] {}", m),
            Self::AgentFailed(m) => write!(f, "[agent failed] {}", m),
            Self::ToolExecutionFailed(m) => write!(f, "[tool execution failed] {}", m),
            Self::ToolAuthorizationDenied(m) => write!(f, "[tool authorization denied] {}", m),
            Self::ToolNotFound(m) => write!(f, "[tool not found] {}", m),
            Self::FunctionCallFailed(m) => write!(f, "[function call failed] {}", m),
            Self::PromptBuildFailed(m) => write!(f, "[prompt build failed] {}", m),
            Self::ResponseValidationFailed(m) => write!(f, "[response validation failed] {}", m),
            Self::ContextAssemblyFailed(m) => write!(f, "[context assembly failed] {}", m),
            Self::PipelineError(m) => write!(f, "[pipeline error] {}", m),
            Self::SessionNotFound(m) => write!(f, "[session not found] {}", m),
            Self::ConversationNotFound(m) => write!(f, "[conversation not found] {}", m),
            Self::ConversationCancelled(m) => write!(f, "[conversation cancelled] {}", m),
            Self::ConversationTimeout(m) => write!(f, "[conversation timeout] {}", m),
            Self::ProviderError(m) => write!(f, "[provider error] {}", m),
            Self::SerializationFailed(m) => write!(f, "[serialization failed] {}", m),
            Self::ConfigurationInvalid(m) => write!(f, "[configuration invalid] {}", m),
            Self::InternalError(m) => write!(f, "[internal error] {}", m),
        }
    }
}

impl std::error::Error for ConversationError {}

impl From<ConversationError> for NeoError {
    fn from(err: ConversationError) -> Self {
        NeoError::Internal(format!("conversation: {}", err))
    }
}

pub type ConversationResult<T> = Result<T, ConversationError>;
