use serde::{Deserialize, Serialize};
use std::fmt;

use crate::id::AgentId;
use crate::time::Timestamp;
use crate::language::types::{Message, MessageRole};

pub type ConversationId = uuid::Uuid;
pub type SessionId = uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Intent {
    Question,
    Conversation,
    Research,
    Coding,
    Planning,
    Analysis,
    Debugging,
    Automation,
    ToolRequest,
    WorkflowExecution,
    KnowledgeLookup,
    MemoryRecall,
    WorldQuery,
    Creative,
    Summarization,
    Translation,
    Explanation,
    Simulation,
    Custom(String),
}

impl fmt::Display for Intent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Intent::Question => write!(f, "question"),
            Intent::Conversation => write!(f, "conversation"),
            Intent::Research => write!(f, "research"),
            Intent::Coding => write!(f, "coding"),
            Intent::Planning => write!(f, "planning"),
            Intent::Analysis => write!(f, "analysis"),
            Intent::Debugging => write!(f, "debugging"),
            Intent::Automation => write!(f, "automation"),
            Intent::ToolRequest => write!(f, "tool_request"),
            Intent::WorkflowExecution => write!(f, "workflow_execution"),
            Intent::KnowledgeLookup => write!(f, "knowledge_lookup"),
            Intent::MemoryRecall => write!(f, "memory_recall"),
            Intent::WorldQuery => write!(f, "world_query"),
            Intent::Creative => write!(f, "creative"),
            Intent::Summarization => write!(f, "summarization"),
            Intent::Translation => write!(f, "translation"),
            Intent::Explanation => write!(f, "explanation"),
            Intent::Simulation => write!(f, "simulation"),
            Intent::Custom(s) => write!(f, "custom:{}", s),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Urgency {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningDepth {
    None,
    Shallow,
    Normal,
    Deep,
    Exhaustive,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestClassification {
    SimpleQuery,
    ComplexQuery,
    MultiStepTask,
    ToolRequired,
    WorkflowRequired,
    AgentRequired,
    Declarative,
    MetaCommand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionPolicy {
    Immediate,
    Planned,
    Delegated,
    Deferred,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolAuthorization {
    Auto,
    RequireApproval,
    Denied,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    Text,
    Markdown,
    Json,
    Code,
    Structured,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: uuid::Uuid,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: Timestamp,
    pub intent: Option<Intent>,
    pub metadata: std::collections::HashMap<String, String>,
    pub evidence: Vec<super::evidence::Evidence>,
    pub tool_calls: Option<Vec<crate::language::types::ToolCall>>,
    pub tool_call_id: Option<String>,
}

impl ConversationMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            role: MessageRole::User,
            content: content.into(),
            timestamp: Timestamp::now(),
            intent: None,
            metadata: std::collections::HashMap::new(),
            evidence: Vec::new(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            role: MessageRole::Assistant,
            content: content.into(),
            timestamp: Timestamp::now(),
            intent: None,
            metadata: std::collections::HashMap::new(),
            evidence: Vec::new(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            role: MessageRole::System,
            content: content.into(),
            timestamp: Timestamp::now(),
            intent: None,
            metadata: std::collections::HashMap::new(),
            evidence: Vec::new(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn tool(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            role: MessageRole::Tool,
            content: content.into(),
            timestamp: Timestamp::now(),
            intent: None,
            metadata: std::collections::HashMap::new(),
            evidence: Vec::new(),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }

    pub fn to_language_message(&self) -> Message {
        Message {
            role: self.role.clone(),
            content: self.content.clone(),
            name: self.metadata.get("name").cloned(),
            tool_calls: self.tool_calls.clone(),
            tool_call_id: self.tool_call_id.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    pub conversation_id: ConversationId,
    pub session_id: SessionId,
    pub messages: Vec<ConversationMessage>,
    pub intent: Option<Intent>,
    pub urgency: Urgency,
    pub classification: Option<RequestClassification>,
    pub execution_policy: Option<ExecutionPolicy>,
    pub reasoning_depth: ReasoningDepth,
    pub tool_authorizations: Vec<ToolAuthorization>,
    pub user_id: Option<String>,
    pub agent_id: Option<AgentId>,
    pub timestamp: Timestamp,
    pub metadata: std::collections::HashMap<String, String>,
}

impl ConversationContext {
    pub fn new(conversation_id: ConversationId, session_id: SessionId) -> Self {
        Self {
            conversation_id,
            session_id,
            messages: Vec::new(),
            intent: None,
            urgency: Urgency::Normal,
            classification: None,
            execution_policy: None,
            reasoning_depth: ReasoningDepth::Normal,
            tool_authorizations: Vec::new(),
            user_id: None,
            agent_id: None,
            timestamp: Timestamp::now(),
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn push_message(&mut self, msg: ConversationMessage) {
        self.messages.push(msg);
    }

    pub fn last_user_message(&self) -> Option<&ConversationMessage> {
        self.messages.iter().rev().find(|m| m.role == MessageRole::User)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationResponse {
    pub conversation_id: ConversationId,
    pub message: ConversationMessage,
    pub tool_calls: Option<Vec<crate::language::types::ToolCall>>,
    pub requires_continuation: bool,
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMetrics {
    pub conversation_id: ConversationId,
    pub message_count: usize,
    pub total_tokens_used: usize,
    pub tool_executions: usize,
    pub reasoning_cycles: usize,
    pub memory_retrievals: usize,
    pub knowledge_lookups: usize,
    pub world_model_queries: usize,
    pub latency_ms: f64,
    pub started_at: Timestamp,
    pub last_activity: Timestamp,
}

impl ConversationMetrics {
    pub fn new(conversation_id: ConversationId) -> Self {
        Self {
            conversation_id,
            message_count: 0,
            total_tokens_used: 0,
            tool_executions: 0,
            reasoning_cycles: 0,
            memory_retrievals: 0,
            knowledge_lookups: 0,
            world_model_queries: 0,
            latency_ms: 0.0,
            started_at: Timestamp::now(),
            last_activity: Timestamp::now(),
        }
    }
}
