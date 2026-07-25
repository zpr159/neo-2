use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContextId(pub Uuid);

impl ContextId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ContextId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::System => write!(f, "system"),
            Self::User => write!(f, "user"),
            Self::Assistant => write!(f, "assistant"),
            Self::Tool => write!(f, "tool"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub name: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub token_count: Option<usize>,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            name: None,
            timestamp: Utc::now(),
            token_count: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            name: None,
            timestamp: Utc::now(),
            token_count: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            name: None,
            timestamp: Utc::now(),
            token_count: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn tool(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Tool,
            content: content.into(),
            name: None,
            timestamp: Utc::now(),
            token_count: None,
            metadata: std::collections::HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    pub id: ContextId,
    pub messages: Vec<Message>,
    pub system_prompt: Option<String>,
    pub max_tokens: usize,
    pub sliding_window_size: Option<usize>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ConversationContext {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            id: ContextId::new(),
            messages: Vec::new(),
            system_prompt: None,
            max_tokens,
            sliding_window_size: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    pub fn with_sliding_window(mut self, window_size: usize) -> Self {
        self.sliding_window_size = Some(window_size);
        self
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        self.updated_at = Utc::now();
        if let Some(window) = self.sliding_window_size {
            let max_messages = window;
            if self.messages.len() > max_messages {
                let drain_count = self.messages.len() - max_messages;
                self.messages.drain(..drain_count);
            }
        }
    }

    #[must_use]
    pub fn total_tokens(&self) -> usize {
        self.messages
            .iter()
            .filter_map(|m| m.token_count)
            .sum()
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.updated_at = Utc::now();
    }

    pub fn remove_oldest(&mut self, count: usize) {
        let remove_count = count.min(self.messages.len());
        self.messages.drain(..remove_count);
        self.updated_at = Utc::now();
    }

    #[must_use]
    pub fn messages_in_window(&self) -> &[Message] {
        &self.messages
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    pub max_context_tokens: usize,
    pub sliding_window_size: usize,
    pub compression_threshold: usize,
    pub enable_compression: bool,
    pub enable_persistence: bool,
    pub persistence_path: Option<String>,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_context_tokens: 8192,
            sliding_window_size: 4096,
            compression_threshold: 2048,
            enable_compression: false,
            enable_persistence: false,
            persistence_path: None,
        }
    }
}

pub mod engine;
