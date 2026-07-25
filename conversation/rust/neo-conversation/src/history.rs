use crate::error::{ConversationError, ConversationResult};
use crate::types::{ConversationMessage, MessageId, MessageRole, TokenUsage};

/// Manages the message history for a conversation session.
///
/// Supports append, truncate, query, summarization, compression,
/// duplicate detection, and token accounting.
#[derive(Debug, Clone)]
pub struct ConversationHistory {
    messages: Vec<ConversationMessage>,
    total_tokens: TokenUsage,
    max_messages: usize,
    duplicate_hashes: Vec<u64>,
}

impl ConversationHistory {
    pub fn new(max_messages: usize) -> Self {
        Self {
            messages: Vec::new(),
            total_tokens: TokenUsage::default(),
            max_messages,
            duplicate_hashes: Vec::new(),
        }
    }

    /// Append a message to the history.
    pub fn append(&mut self, message: ConversationMessage) -> ConversationResult<()> {
        if self.is_duplicate(&message) {
            return Err(ConversationError::InvalidInput(
                "duplicate message detected".into(),
            ));
        }
        let tokens = message.estimate_tokens();
        self.total_tokens.prompt_tokens += tokens;
        self.total_tokens.total_tokens += tokens;
        self.duplicate_hashes.push(hash_content(&message.content));
        self.messages.push(message);
        self.enforce_limit();
        Ok(())
    }

    /// Append a message without duplicate checking.
    pub fn append_unchecked(&mut self, message: ConversationMessage) {
        let tokens = message.estimate_tokens();
        self.total_tokens.prompt_tokens += tokens;
        self.total_tokens.total_tokens += tokens;
        self.messages.push(message);
        self.enforce_limit();
    }

    /// Truncate history to the last `count` messages.
    pub fn truncate(&mut self, count: usize) {
        if self.messages.len() > count {
            self.messages.drain(..self.messages.len() - count);
            self.recompute_tokens();
        }
    }

    /// Truncate to fit within a token budget, keeping system messages and recent messages.
    pub fn truncate_to_tokens(&mut self, budget: usize) {
        let mut current = self.estimate_total_tokens();
        while current > budget && self.messages.len() > 1 {
            // Remove the oldest non-system message.
            if self.messages[0].role == MessageRole::System && self.messages.len() > 2 {
                let removed = self.messages.remove(1);
                current -= removed.estimate_tokens();
            } else if self.messages[0].role != MessageRole::System && !self.messages.is_empty() {
                let removed = self.messages.remove(0);
                current -= removed.estimate_tokens();
            } else {
                break;
            }
        }
        self.recompute_tokens();
    }

    /// Get all messages.
    #[must_use]
    pub fn messages(&self) -> &[ConversationMessage] {
        &self.messages
    }

    /// Get mutable access to messages.
    pub fn messages_mut(&mut self) -> &mut [ConversationMessage] {
        &mut self.messages
    }

    /// Get messages by role.
    #[must_use]
    pub fn messages_by_role(&self, role: &MessageRole) -> Vec<&ConversationMessage> {
        self.messages.iter().filter(|m| m.role == *role).collect()
    }

    /// Get a message by ID.
    #[must_use]
    pub fn get(&self, id: &MessageId) -> Option<&ConversationMessage> {
        self.messages.iter().find(|m| m.id == *id)
    }

    /// Query messages matching a predicate.
    #[must_use]
    pub fn query<F>(&self, predicate: F) -> Vec<&ConversationMessage>
    where
        F: Fn(&ConversationMessage) -> bool,
    {
        self.messages.iter().filter(|m| predicate(m)).collect()
    }

    /// Get the last N messages.
    #[must_use]
    pub fn last_n(&self, n: usize) -> Vec<&ConversationMessage> {
        self.messages.iter().rev().take(n).collect::<Vec<_>>().into_iter().rev().collect()
    }

    /// Get the conversation summary by extracting key exchanges.
    #[must_use]
    pub fn summarize(&self) -> String {
        let user_count = self
            .messages
            .iter()
            .filter(|m| m.role == MessageRole::User)
            .count();
        let assistant_count = self
            .messages
            .iter()
            .filter(|m| m.role == MessageRole::Assistant)
            .count();

        let last_user = self
            .messages
            .iter()
            .rev()
            .find(|m| m.role == MessageRole::User)
            .map(|m| m.content.as_str())
            .unwrap_or("");

        format!(
            "Conversation with {} user messages and {} assistant messages. Last user message: \"{}\"",
            user_count, assistant_count, last_user,
        )
    }

    /// Compress by summarizing older messages and keeping recent ones intact.
    pub fn compress(&mut self, keep_recent: usize) -> String {
        if self.messages.len() <= keep_recent + 1 {
            return self.summarize();
        }

        let summary = self.summarize();
        let system = self
            .messages
            .first()
            .filter(|m| m.role == MessageRole::System)
            .cloned();
        let recent: Vec<ConversationMessage> =
            self.messages[self.messages.len() - keep_recent..].to_vec();

        self.messages.clear();
        if let Some(sys) = system {
            self.messages.push(sys);
        }
        self.messages.push(ConversationMessage::internal(
            format!("[Conversation Summary]\n{summary}"),
        ));
        self.messages.extend(recent);
        self.recompute_tokens();

        summary
    }

    /// Check for duplicate messages.
    fn is_duplicate(&self, message: &ConversationMessage) -> bool {
        let hash = hash_content(&message.content);
        self.duplicate_hashes.contains(&hash)
    }

    fn enforce_limit(&mut self) {
        if self.messages.len() > self.max_messages {
            let excess = self.messages.len() - self.max_messages;
            self.messages.drain(..excess);
            self.duplicate_hashes.drain(..excess);
        }
    }

    fn estimate_total_tokens(&self) -> usize {
        self.messages.iter().map(|m| m.estimate_tokens()).sum()
    }

    fn recompute_tokens(&mut self) {
        let prompt: usize = self.messages.iter().map(|m| m.estimate_tokens()).sum();
        self.total_tokens = TokenUsage {
            prompt_tokens: prompt,
            completion_tokens: self.total_tokens.completion_tokens,
            total_tokens: prompt + self.total_tokens.completion_tokens,
        };
    }

    /// Get token usage.
    #[must_use]
    pub fn token_usage(&self) -> TokenUsage {
        self.total_tokens
    }

    /// Get the number of messages.
    #[must_use]
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check if history is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Clear the history.
    pub fn clear(&mut self) {
        self.messages.clear();
        self.total_tokens = TokenUsage::default();
        self.duplicate_hashes.clear();
    }
}

fn hash_content(content: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}
