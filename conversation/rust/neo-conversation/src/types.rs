use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

// ── Identifiers ──────────────────────────────────────────────────────

/// Unique identifier for a conversation session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl Default for SessionId {
    fn default() -> Self {
        Self::random()
    }
}

impl SessionId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn random() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for SessionId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Unique identifier for a message within a conversation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(pub String);

impl MessageId {
    pub fn random() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl fmt::Display for MessageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a user.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub String);

impl UserId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── Message Roles ────────────────────────────────────────────────────

/// Role of a message participant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
    Planner,
    Reasoner,
    Executive,
    Developer,
    Internal,
}

impl fmt::Display for MessageRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::System => write!(f, "system"),
            Self::User => write!(f, "user"),
            Self::Assistant => write!(f, "assistant"),
            Self::Tool => write!(f, "tool"),
            Self::Planner => write!(f, "planner"),
            Self::Reasoner => write!(f, "reasoner"),
            Self::Executive => write!(f, "executive"),
            Self::Developer => write!(f, "developer"),
            Self::Internal => write!(f, "internal"),
        }
    }
}

// ── Message Model ────────────────────────────────────────────────────

/// Metadata attached to a conversation message.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MessageMetadata {
    pub source: Option<String>,
    pub latency_ms: Option<f64>,
    pub token_count: Option<usize>,
    pub confidence: Option<f64>,
    pub tags: Vec<String>,
    pub extra: HashMap<String, serde_json::Value>,
}

/// An attachment within a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageAttachment {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub data: AttachmentData,
}

/// Data payload for an attachment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttachmentData {
    Inline(String),
    Reference(String),
}

/// A reference to another message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageReference {
    pub message_id: MessageId,
    pub relation: String,
}

/// A single message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: MessageId,
    pub role: MessageRole,
    pub content: String,
    pub name: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub token_count: Option<usize>,
    pub metadata: MessageMetadata,
    pub attachments: Vec<MessageAttachment>,
    pub references: Vec<MessageReference>,
}

impl ConversationMessage {
    pub fn new(role: MessageRole, content: impl Into<String>) -> Self {
        Self {
            id: MessageId::random(),
            role,
            content: content.into(),
            name: None,
            timestamp: Utc::now(),
            token_count: None,
            metadata: MessageMetadata::default(),
            attachments: Vec::new(),
            references: Vec::new(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::new(MessageRole::User, content)
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(MessageRole::Assistant, content)
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self::new(MessageRole::System, content)
    }

    pub fn tool(content: impl Into<String>) -> Self {
        Self::new(MessageRole::Tool, content)
    }

    pub fn planner(content: impl Into<String>) -> Self {
        Self::new(MessageRole::Planner, content)
    }

    pub fn reasoner(content: impl Into<String>) -> Self {
        Self::new(MessageRole::Reasoner, content)
    }

    pub fn executive(content: impl Into<String>) -> Self {
        Self::new(MessageRole::Executive, content)
    }

    pub fn developer(content: impl Into<String>) -> Self {
        Self::new(MessageRole::Developer, content)
    }

    pub fn internal(content: impl Into<String>) -> Self {
        Self::new(MessageRole::Internal, content)
    }

    #[must_use]
    pub fn estimate_tokens(&self) -> usize {
        self.content.len() / 4 + 4
    }
}

// ── Streaming ────────────────────────────────────────────────────────

/// A chunk of a streaming response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub session_id: SessionId,
    pub text: String,
    pub done: bool,
    pub usage: Option<TokenUsage>,
}

// ── Token Usage ──────────────────────────────────────────────────────

/// Token usage statistics.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

// ── Tool Types ───────────────────────────────────────────────────────

/// Tool definition for function calling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Tool invocation request extracted from a response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Result of a tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub call_id: String,
    pub name: String,
    pub result: String,
    pub success: bool,
    pub error: Option<String>,
}

// ── Cognitive Context ────────────────────────────────────────────────

/// The cognitive pipeline step that produced context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CognitiveSource {
    Memory,
    KnowledgeGraph,
    Reasoning,
    WorldModel,
    Planning,
    Executive,
    ToolUse,
    Direct,
}

impl fmt::Display for CognitiveSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Memory => write!(f, "memory"),
            Self::KnowledgeGraph => write!(f, "knowledge_graph"),
            Self::Reasoning => write!(f, "reasoning"),
            Self::WorldModel => write!(f, "world_model"),
            Self::Planning => write!(f, "planning"),
            Self::Executive => write!(f, "executive"),
            Self::ToolUse => write!(f, "tool_use"),
            Self::Direct => write!(f, "direct"),
        }
    }
}

/// Enriched context provided to the language model.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CognitiveContext {
    pub sources: Vec<CognitiveSource>,
    pub memories: Vec<String>,
    pub knowledge: Vec<String>,
    pub reasoning: Vec<String>,
    pub world_state: Vec<String>,
    pub plan_context: Option<String>,
    pub tool_results: Vec<ToolResult>,
    pub agent_outputs: Vec<String>,
    pub workflow_outputs: Vec<String>,
    pub executive_decisions: Vec<String>,
    pub additional: Vec<String>,
}

impl CognitiveContext {
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.memories.is_empty()
            && self.knowledge.is_empty()
            && self.reasoning.is_empty()
            && self.world_state.is_empty()
            && self.plan_context.is_none()
            && self.tool_results.is_empty()
            && self.agent_outputs.is_empty()
            && self.workflow_outputs.is_empty()
            && self.executive_decisions.is_empty()
            && self.additional.is_empty()
    }

    pub fn build_context_block(&self) -> String {
        let mut parts = Vec::new();

        if !self.memories.is_empty() {
            parts.push(format!(
                "## Relevant Memories\n{}",
                self.memories.join("\n")
            ));
        }
        if !self.knowledge.is_empty() {
            parts.push(format!(
                "## Relevant Knowledge\n{}",
                self.knowledge.join("\n")
            ));
        }
        if !self.reasoning.is_empty() {
            parts.push(format!(
                "## Reasoning Context\n{}",
                self.reasoning.join("\n")
            ));
        }
        if !self.world_state.is_empty() {
            parts.push(format!(
                "## World State\n{}",
                self.world_state.join("\n")
            ));
        }
        if let Some(plan) = &self.plan_context {
            parts.push(format!("## Active Plan\n{plan}"));
        }
        if !self.executive_decisions.is_empty() {
            parts.push(format!(
                "## Executive Decisions\n{}",
                self.executive_decisions.join("\n")
            ));
        }
        if !self.agent_outputs.is_empty() {
            parts.push(format!(
                "## Agent Outputs\n{}",
                self.agent_outputs.join("\n")
            ));
        }
        if !self.workflow_outputs.is_empty() {
            parts.push(format!(
                "## Workflow Outputs\n{}",
                self.workflow_outputs.join("\n")
            ));
        }
        if !self.tool_results.is_empty() {
            let results: Vec<String> = self
                .tool_results
                .iter()
                .map(|r| format!("- {}: {}", r.name, r.result))
                .collect();
            parts.push(format!("## Tool Results\n{}", results.join("\n")));
        }
        if !self.additional.is_empty() {
            parts.push(format!(
                "## Additional Context\n{}",
                self.additional.join("\n")
            ));
        }

        if parts.is_empty() {
            String::new()
        } else {
            parts.join("\n\n")
        }
    }
}

// ── LLM Message Format ───────────────────────────────────────────────

/// Message format for the language model API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: MessageRole,
    pub content: String,
}

// ── Conversation Modes ───────────────────────────────────────────────

/// Operating mode for a conversation session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConversationMode {
    Assistant,
    Research,
    Coding,
    Planning,
    Automation,
    Analysis,
    Debugging,
    Creative,
    Teaching,
    Simulation,
    Custom(String),
}

impl fmt::Display for ConversationMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Assistant => write!(f, "assistant"),
            Self::Research => write!(f, "research"),
            Self::Coding => write!(f, "coding"),
            Self::Planning => write!(f, "planning"),
            Self::Automation => write!(f, "automation"),
            Self::Analysis => write!(f, "analysis"),
            Self::Debugging => write!(f, "debugging"),
            Self::Creative => write!(f, "creative"),
            Self::Teaching => write!(f, "teaching"),
            Self::Simulation => write!(f, "simulation"),
            Self::Custom(name) => write!(f, "{name}"),
        }
    }
}

impl Default for ConversationMode {
    fn default() -> Self {
        Self::Assistant
    }
}

// ── User Model ───────────────────────────────────────────────────────

/// Model of a user built gradually through conversations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserModel {
    pub preferences: HashMap<String, String>,
    pub frequently_discussed_topics: Vec<String>,
    pub goals: Vec<String>,
    pub projects: Vec<String>,
    pub expertise: Vec<String>,
    pub communication_style: Option<String>,
    pub tool_preferences: Vec<String>,
    pub knowledge_level: Option<String>,
    pub privacy_settings: PrivacySettings,
}

/// User privacy settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacySettings {
    pub store_history: bool,
    pub share_metrics: bool,
    pub allow_learning: bool,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            store_history: true,
            share_metrics: true,
            allow_learning: true,
        }
    }
}

// ── Session Configuration ────────────────────────────────────────────

/// Configuration for a conversation session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub max_messages: usize,
    pub max_context_tokens: usize,
    pub system_prompt: String,
    pub enable_tools: bool,
    pub tools: Vec<ToolDefinition>,
    pub temperature: f32,
    pub top_p: f32,
    pub max_generation_tokens: usize,
    pub stop_sequences: Vec<String>,
    pub stream: bool,
    pub mode: ConversationMode,
    pub memory_enabled: bool,
    pub reasoning_enabled: bool,
    pub planning_enabled: bool,
    pub persistence_enabled: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_messages: 100,
            max_context_tokens: 8192,
            system_prompt: "You are Neo, an AI operating system assistant. You are thoughtful, precise, and helpful. You think before answering and use your knowledge, memory, and reasoning capabilities to provide accurate responses.".into(),
            enable_tools: true,
            tools: Vec::new(),
            temperature: 0.7,
            top_p: 0.9,
            max_generation_tokens: 2048,
            stop_sequences: Vec::new(),
            stream: true,
            mode: ConversationMode::Assistant,
            memory_enabled: true,
            reasoning_enabled: true,
            planning_enabled: true,
            persistence_enabled: false,
        }
    }
}
