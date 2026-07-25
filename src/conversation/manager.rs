use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use crate::component::{Component, ComponentState};
use crate::conversation::config::ConversationConfig;
use crate::conversation::error::{ConversationError, ConversationResult};
use crate::conversation::pipeline::{ConversationPipeline, PipelineEvent};
use crate::conversation::types::*;
use crate::error::NeoResult;
use crate::id::ComponentId;

/// Manages multiple concurrent conversations.
///
/// The ConversationManager is the entry point for all conversation interactions.
/// It maintains session state, routes messages to the pipeline, and handles
/// session lifecycle.
pub struct ConversationManager {
    id: ComponentId,
    state: ComponentState,
    config: ConversationConfig,
    pipeline: Arc<ConversationPipeline>,
    sessions: RwLock<HashMap<SessionId, SessionState>>,
    conversations: RwLock<HashMap<ConversationId, ConversationContext>>,
}

struct SessionState {
    conversation_ids: Vec<ConversationId>,
    active_conversation: Option<ConversationId>,
    user_id: Option<String>,
    created_at: crate::time::Timestamp,
}

impl ConversationManager {
    pub fn new(config: ConversationConfig, pipeline: Arc<ConversationPipeline>) -> Self {
        Self {
            id: ComponentId::new(),
            state: ComponentState::Created,
            config,
            pipeline,
            sessions: RwLock::new(HashMap::new()),
            conversations: RwLock::new(HashMap::new()),
        }
    }

    /// Create a new session.
    pub async fn create_session(&self, user_id: Option<String>) -> SessionId {
        let session_id = SessionId::new_v4();
        let state = SessionState {
            conversation_ids: Vec::new(),
            active_conversation: None,
            user_id,
            created_at: crate::time::Timestamp::now(),
        };
        self.sessions.write().await.insert(session_id, state);
        session_id
    }

    /// Create a new conversation within a session.
    pub async fn create_conversation(
        &self,
        session_id: SessionId,
    ) -> ConversationResult<ConversationId> {
        let conversation_id = ConversationId::new_v4();
        let context = ConversationContext::new(conversation_id, session_id);

        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(&session_id)
            .ok_or_else(|| ConversationError::SessionNotFound(format!("{}", session_id)))?;

        session.conversation_ids.push(conversation_id);
        session.active_conversation = Some(conversation_id);
        drop(sessions);

        self.conversations
            .write()
            .await
            .insert(conversation_id, context);

        Ok(conversation_id)
    }

    /// Send a user message and receive a response.
    pub async fn send_message(
        &self,
        session_id: SessionId,
        conversation_id: ConversationId,
        message: &str,
    ) -> ConversationResult<ConversationResponse> {
        let mut conversations = self.conversations.write().await;
        let context = conversations
            .get_mut(&conversation_id)
            .ok_or_else(|| {
                ConversationError::ConversationNotFound(format!("{}", conversation_id))
            })?;

        if context.session_id != session_id {
            return Err(ConversationError::SessionNotFound(
                "Session mismatch".to_string(),
            ));
        }

        let user_msg = ConversationMessage::user(message);
        context.push_message(user_msg);

        let result = self
            .pipeline
            .process(context, message, None)
            .await?;

        Ok(result)
    }

    /// Send a message with event streaming.
    pub async fn send_message_streaming(
        &self,
        session_id: SessionId,
        conversation_id: ConversationId,
        message: &str,
    ) -> ConversationResult<(ConversationResponse, mpsc::Receiver<PipelineEvent>)> {
        let (event_tx, event_rx) = mpsc::channel(64);

        let mut conversations = self.conversations.write().await;
        let context = conversations
            .get_mut(&conversation_id)
            .ok_or_else(|| {
                ConversationError::ConversationNotFound(format!("{}", conversation_id))
            })?;

        if context.session_id != session_id {
            return Err(ConversationError::SessionNotFound(
                "Session mismatch".to_string(),
            ));
        }

        let user_msg = ConversationMessage::user(message);
        context.push_message(user_msg);

        let result = self
            .pipeline
            .process(context, message, Some(event_tx))
            .await?;

        Ok((result, event_rx))
    }

    /// Cancel a conversation.
    pub async fn cancel_conversation(
        &self,
        session_id: SessionId,
        conversation_id: ConversationId,
    ) -> ConversationResult<()> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(&session_id)
            .ok_or_else(|| ConversationError::SessionNotFound(format!("{}", session_id)))?;

        if session.active_conversation == Some(conversation_id) {
            session.active_conversation = None;
        }
        session
            .conversation_ids
            .retain(|&id| id != conversation_id);

        self.conversations.write().await.remove(&conversation_id);

        Ok(())
    }

    /// Get conversation history.
    pub async fn get_history(
        &self,
        session_id: SessionId,
        conversation_id: ConversationId,
    ) -> ConversationResult<Vec<ConversationMessage>> {
        let conversations = self.conversations.read().await;
        let context = conversations
            .get(&conversation_id)
            .ok_or_else(|| {
                ConversationError::ConversationNotFound(format!("{}", conversation_id))
            })?;

        if context.session_id != session_id {
            return Err(ConversationError::SessionNotFound(
                "Session mismatch".to_string(),
            ));
        }

        Ok(context.messages.clone())
    }

    /// Get conversation metrics.
    pub async fn get_metrics(
        &self,
        _session_id: SessionId,
        conversation_id: ConversationId,
    ) -> ConversationResult<ConversationMetrics> {
        let conversations = self.conversations.read().await;
        let context = conversations
            .get(&conversation_id)
            .ok_or_else(|| {
                ConversationError::ConversationNotFound(format!("{}", conversation_id))
            })?;

        let mut metrics = ConversationMetrics::new(conversation_id);
        metrics.message_count = context.messages.len();

        Ok(metrics)
    }

    /// List active conversations for a session.
    pub async fn list_conversations(
        &self,
        session_id: SessionId,
    ) -> ConversationResult<Vec<ConversationId>> {
        let sessions = self.sessions.read().await;
        let session = sessions
            .get(&session_id)
            .ok_or_else(|| ConversationError::SessionNotFound(format!("{}", session_id)))?;

        Ok(session.conversation_ids.clone())
    }

    /// Get the number of active conversations.
    pub async fn active_conversation_count(&self) -> usize {
        self.conversations.read().await.len()
    }

    /// Get the number of active sessions.
    pub async fn active_session_count(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// Clean up expired sessions and conversations.
    pub async fn cleanup(&self, max_age_secs: u64) -> usize {
        let mut sessions = self.sessions.write().await;
        let mut conversations = self.conversations.write().await;
        let mut removed = 0;

        let expired: Vec<SessionId> = sessions
            .iter()
                .filter(|(_, s)| s.created_at.is_expired(max_age_secs))
            .map(|(id, _)| *id)
            .collect();

        for session_id in &expired {
            if let Some(session) = sessions.remove(session_id) {
                for conv_id in &session.conversation_ids {
                    conversations.remove(conv_id);
                    removed += 1;
                }
            }
        }

        removed
    }
}

impl Component for ConversationManager {
    fn name(&self) -> &str {
        "ConversationManager"
    }

    fn state(&self) -> ComponentState {
        self.state
    }

    async fn initialize(&mut self) -> NeoResult<()> {
        self.state = ComponentState::Initializing;
        self.state = ComponentState::Running;
        Ok(())
    }

    async fn start(&mut self) -> NeoResult<()> {
        self.state = ComponentState::Running;
        Ok(())
    }

    async fn stop(&mut self) -> NeoResult<()> {
        self.state = ComponentState::Stopping;
        let _ = self.cleanup(0).await;
        self.state = ComponentState::Stopped;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
