use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::{
    CognitiveContext, ConversationMessage, MessageId, MessageRole, SessionConfig, SessionId,
    TokenUsage, ToolCall, UserId,
};

/// State of a conversation session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SessionState {
    Created,
    Initializing,
    Active,
    WaitingForTool,
    Streaming,
    Paused,
    Completed,
    Cancelled,
    Failed,
    Expired,
}

impl std::fmt::Display for SessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::Initializing => write!(f, "initializing"),
            Self::Active => write!(f, "active"),
            Self::WaitingForTool => write!(f, "waiting_for_tool"),
            Self::Streaming => write!(f, "streaming"),
            Self::Paused => write!(f, "paused"),
            Self::Completed => write!(f, "completed"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Failed => write!(f, "failed"),
            Self::Expired => write!(f, "expired"),
        }
    }
}

/// Valid state transitions for a session.
fn is_valid_transition(from: &SessionState, to: &SessionState) -> bool {
    matches!(
        (from, to),
        (SessionState::Created, SessionState::Initializing)
            | (SessionState::Created, SessionState::Active)
            | (SessionState::Created, SessionState::Cancelled)
            | (SessionState::Initializing, SessionState::Active)
            | (SessionState::Initializing, SessionState::Failed)
            | (SessionState::Active, SessionState::Streaming)
            | (SessionState::Active, SessionState::WaitingForTool)
            | (SessionState::Active, SessionState::Paused)
            | (SessionState::Active, SessionState::Completed)
            | (SessionState::Active, SessionState::Failed)
            | (SessionState::Active, SessionState::Cancelled)
            | (SessionState::Active, SessionState::Expired)
            | (SessionState::WaitingForTool, SessionState::Active)
            | (SessionState::WaitingForTool, SessionState::Failed)
            | (SessionState::WaitingForTool, SessionState::Cancelled)
            | (SessionState::Streaming, SessionState::Active)
            | (SessionState::Streaming, SessionState::Failed)
            | (SessionState::Streaming, SessionState::Cancelled)
            | (SessionState::Paused, SessionState::Active)
            | (SessionState::Paused, SessionState::Cancelled)
    )
}

/// Memory state snapshot within a session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionMemoryState {
    pub relevant_memories: Vec<String>,
    pub episodic_memories: Vec<String>,
    pub working_memory: Vec<String>,
}

/// Planning state snapshot within a session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionPlanningState {
    pub active_plan: Option<String>,
    pub plan_steps: Vec<String>,
    pub completed_steps: Vec<String>,
}

/// Reasoning state snapshot within a session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionReasoningState {
    pub reasoning_chains: Vec<String>,
    pub conclusions: Vec<String>,
    pub confidence: Option<f64>,
}

/// World model state reference within a session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionWorldState {
    pub observations: Vec<String>,
    pub entities: Vec<String>,
    pub relationships: Vec<String>,
}

/// Tool state within a session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionToolState {
    pub available_tools: Vec<String>,
    pub pending_calls: Vec<ToolCall>,
    pub call_history: Vec<ToolCallRecord>,
}

/// Record of a past tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub call: ToolCall,
    pub result: Option<String>,
    pub success: bool,
    pub timestamp: DateTime<Utc>,
}

/// A conversation session maintaining full dialogue history and context.
pub struct ConversationSession {
    pub id: SessionId,
    pub state: SessionState,
    pub config: SessionConfig,
    pub messages: Vec<ConversationMessage>,
    pub cognitive_context: CognitiveContext,
    pub user_id: Option<UserId>,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub total_tokens: TokenUsage,
    pub turn_count: usize,
    pub pending_tool_calls: Vec<ToolCall>,
    pub memory_state: SessionMemoryState,
    pub planning_state: SessionPlanningState,
    pub reasoning_state: SessionReasoningState,
    pub world_state: SessionWorldState,
    pub tool_state: SessionToolState,
}

impl ConversationSession {
    pub fn new(config: SessionConfig) -> Self {
        let now = Utc::now();
        let mut session = Self {
            id: SessionId::random(),
            state: SessionState::Created,
            config,
            messages: Vec::new(),
            cognitive_context: CognitiveContext::empty(),
            user_id: None,
            metadata: HashMap::new(),
            created_at: now,
            last_active: now,
            total_tokens: TokenUsage::default(),
            turn_count: 0,
            pending_tool_calls: Vec::new(),
            memory_state: SessionMemoryState::default(),
            planning_state: SessionPlanningState::default(),
            reasoning_state: SessionReasoningState::default(),
            world_state: SessionWorldState::default(),
            tool_state: SessionToolState::default(),
        };

        let system_msg = ConversationMessage::system(&session.config.system_prompt);
        session.messages.push(system_msg);
        session
    }

    /// Attempt a state transition. Returns Err on illegal transitions.
    pub fn transition(&mut self, to: SessionState) -> Result<(), crate::error::ConversationError> {
        if !is_valid_transition(&self.state, &to) {
            return Err(crate::error::ConversationError::InvalidStateTransition {
                from: self.state.to_string(),
                to: to.to_string(),
            });
        }
        self.state = to;
        self.last_active = Utc::now();
        Ok(())
    }

    pub fn add_user_message(&mut self, content: impl Into<String>) -> MessageId {
        let msg = ConversationMessage::user(content);
        let id = msg.id.clone();
        self.messages.push(msg);
        self.last_active = Utc::now();
        let _ = self.transition(SessionState::Active);
        id
    }

    pub fn add_assistant_message(&mut self, content: impl Into<String>) -> MessageId {
        let msg = ConversationMessage::assistant(content);
        let id = msg.id.clone();
        self.messages.push(msg);
        self.last_active = Utc::now();
        self.trim_to_window();
        id
    }

    pub fn add_tool_result(&mut self, content: impl Into<String>) -> MessageId {
        let msg = ConversationMessage::tool(content);
        let id = msg.id.clone();
        self.messages.push(msg);
        self.last_active = Utc::now();
        id
    }

    pub fn add_system_message(&mut self, content: impl Into<String>) -> MessageId {
        let msg = ConversationMessage::system(content);
        let id = msg.id.clone();
        self.messages.push(msg);
        self.last_active = Utc::now();
        id
    }

    pub fn set_cognitive_context(&mut self, context: CognitiveContext) {
        self.cognitive_context = context;
    }

    pub fn messages_for_llm(&self) -> Vec<crate::types::LlmMessage> {
        self.messages
            .iter()
            .filter(|m| m.role != MessageRole::Tool || self.config.enable_tools)
            .map(|m| crate::types::LlmMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect()
    }

    pub fn history_text(&self) -> String {
        self.messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    MessageRole::System => "System",
                    MessageRole::User => "User",
                    MessageRole::Assistant => "Neo",
                    MessageRole::Tool => "Tool",
                    MessageRole::Planner => "Planner",
                    MessageRole::Reasoner => "Reasoner",
                    MessageRole::Executive => "Executive",
                    MessageRole::Developer => "Developer",
                    MessageRole::Internal => "Internal",
                };
                format!("[{role}]: {}", m.content)
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    pub fn recent_user_messages(&self, count: usize) -> Vec<&ConversationMessage> {
        self.messages
            .iter()
            .rev()
            .filter(|m| m.role == MessageRole::User)
            .take(count)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    pub fn messages_in_window(&self) -> &[ConversationMessage] {
        if self.messages.is_empty() {
            return &[];
        }
        let system_len = if self.messages[0].role == MessageRole::System {
            1
        } else {
            0
        };
        let max = self.config.max_messages;
        let start = system_len;
        let end = self.messages.len();
        if end - start > max {
            &self.messages[end - max..]
        } else {
            &self.messages[start..]
        }
    }

    fn trim_to_window(&mut self) {
        let max = self.config.max_messages;
        if self.messages.len() > max + 1 {
            let system = self.messages[0].clone();
            let tail: Vec<ConversationMessage> =
                self.messages[self.messages.len() - max..].to_vec();
            self.messages = Vec::with_capacity(max + 1);
            self.messages.push(system);
            self.messages.extend(tail);
        }
    }

    pub fn record_tokens(&mut self, prompt_tokens: usize, completion_tokens: usize) {
        self.total_tokens.prompt_tokens += prompt_tokens;
        self.total_tokens.completion_tokens += completion_tokens;
        self.total_tokens.total_tokens += prompt_tokens + completion_tokens;
        self.turn_count += 1;
    }

    pub fn complete(&mut self) {
        let _ = self.transition(SessionState::Completed);
    }

    pub fn fail(&mut self, reason: &str) {
        self.state = SessionState::Failed;
        self.metadata
            .insert("failure_reason".into(), reason.to_string());
    }

    pub fn cancel(&mut self) {
        let _ = self.transition(SessionState::Cancelled);
    }

    pub fn pause(&mut self) {
        let _ = self.transition(SessionState::Paused);
    }

    pub fn resume(&mut self) {
        let _ = self.transition(SessionState::Active);
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(
            self.state,
            SessionState::Created
                | SessionState::Initializing
                | SessionState::Active
                | SessionState::WaitingForTool
                | SessionState::Streaming
        )
    }

    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            SessionState::Completed | SessionState::Cancelled | SessionState::Failed | SessionState::Expired
        )
    }

    #[must_use]
    pub fn total_message_count(&self) -> usize {
        self.messages.len()
    }

    #[must_use]
    pub fn user_message_count(&self) -> usize {
        self.messages
            .iter()
            .filter(|m| m.role == MessageRole::User)
            .count()
    }

    #[must_use]
    pub fn assistant_message_count(&self) -> usize {
        self.messages
            .iter()
            .filter(|m| m.role == MessageRole::Assistant)
            .count()
    }

    #[must_use]
    pub fn last_message(&self) -> Option<&ConversationMessage> {
        self.messages.last()
    }

    #[must_use]
    pub fn last_user_message(&self) -> Option<&ConversationMessage> {
        self.messages
            .iter()
            .rev()
            .find(|m| m.role == MessageRole::User)
    }
}
