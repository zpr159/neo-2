use crate::error::ConversationResult;
use crate::session::ConversationSession;
use crate::types::{SessionId, TokenUsage};

/// Persistence backend trait for conversation state.
///
/// Implementations provide pluggable storage for session state,
/// conversation history, summaries, and checkpoints.
pub trait PersistenceBackend: Send + Sync {
    /// Save a session's state.
    fn save_session(&self, session: &ConversationSession) -> ConversationResult<()>;

    /// Load a session by ID.
    fn load_session(&self, session_id: &SessionId) -> ConversationResult<Option<SessionData>>;

    /// Delete a session.
    fn delete_session(&self, session_id: &SessionId) -> ConversationResult<bool>;

    /// List all stored session IDs.
    fn list_sessions(&self) -> ConversationResult<Vec<SessionId>>;

    /// Save a summary for a session.
    fn save_summary(&self, session_id: &SessionId, summary: &str) -> ConversationResult<()>;

    /// Load a summary for a session.
    fn load_summary(&self, session_id: &SessionId) -> ConversationResult<Option<String>>;

    /// Save token usage checkpoint.
    fn save_token_usage(
        &self,
        session_id: &SessionId,
        usage: TokenUsage,
    ) -> ConversationResult<()>;

    /// Load token usage.
    fn load_token_usage(&self, session_id: &SessionId) -> ConversationResult<Option<TokenUsage>>;
}

/// Stored session data for persistence.
#[derive(Debug, Clone)]
pub struct SessionData {
    pub session_id: SessionId,
    pub messages: Vec<crate::types::ConversationMessage>,
    pub state: crate::session::SessionState,
    pub token_usage: TokenUsage,
    pub metadata: std::collections::HashMap<String, String>,
}

/// In-memory persistence backend (for testing and development).
pub struct InMemoryPersistence {
    sessions: parking_lot::RwLock<
        std::collections::HashMap<SessionId, SessionData>,
    >,
    summaries: parking_lot::RwLock<std::collections::HashMap<SessionId, String>>,
    token_usage: parking_lot::RwLock<std::collections::HashMap<SessionId, TokenUsage>>,
}

impl InMemoryPersistence {
    pub fn new() -> Self {
        Self {
            sessions: parking_lot::RwLock::new(std::collections::HashMap::new()),
            summaries: parking_lot::RwLock::new(std::collections::HashMap::new()),
            token_usage: parking_lot::RwLock::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for InMemoryPersistence {
    fn default() -> Self {
        Self::new()
    }
}

impl PersistenceBackend for InMemoryPersistence {
    fn save_session(&self, session: &ConversationSession) -> ConversationResult<()> {
        let data = SessionData {
            session_id: session.id.clone(),
            messages: session.messages.clone(),
            state: session.state.clone(),
            token_usage: session.total_tokens,
            metadata: session.metadata.clone(),
        };
        self.sessions.write().insert(session.id.clone(), data);
        Ok(())
    }

    fn load_session(&self, session_id: &SessionId) -> ConversationResult<Option<SessionData>> {
        Ok(self.sessions.read().get(session_id).cloned())
    }

    fn delete_session(&self, session_id: &SessionId) -> ConversationResult<bool> {
        Ok(self.sessions.write().remove(session_id).is_some())
    }

    fn list_sessions(&self) -> ConversationResult<Vec<SessionId>> {
        Ok(self.sessions.read().keys().cloned().collect())
    }

    fn save_summary(&self, session_id: &SessionId, summary: &str) -> ConversationResult<()> {
        self.summaries
            .write()
            .insert(session_id.clone(), summary.to_string());
        Ok(())
    }

    fn load_summary(&self, session_id: &SessionId) -> ConversationResult<Option<String>> {
        Ok(self.summaries.read().get(session_id).cloned())
    }

    fn save_token_usage(
        &self,
        session_id: &SessionId,
        usage: TokenUsage,
    ) -> ConversationResult<()> {
        self.token_usage.write().insert(session_id.clone(), usage);
        Ok(())
    }

    fn load_token_usage(&self, session_id: &SessionId) -> ConversationResult<Option<TokenUsage>> {
        Ok(self.token_usage.read().get(session_id).copied())
    }
}
