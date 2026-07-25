use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;

use crate::error::{ConversationError, ConversationResult};
use crate::language::{LanguageEngine, LanguageEngineConfig, OllamaEngine};
use crate::metrics::ConversationMetrics;
use crate::persistence::{InMemoryPersistence, PersistenceBackend};
use crate::pipeline::{ConversationPipeline, ConversationResponse};
use crate::session::{ConversationSession, SessionState};
use crate::types::{CognitiveContext, SessionConfig, SessionId, StreamChunk, UserId};

/// Manages multiple conversation sessions and the language engine.
///
/// Supports thousands of concurrent sessions with async request handling,
/// cancellation-safe operations, and pluggable persistence.
pub struct ConversationManager {
    sessions: DashMap<SessionId, Arc<tokio::sync::RwLock<ConversationSession>>>,
    engine: Arc<tokio::sync::RwLock<Box<dyn LanguageEngine>>>,
    pipeline: Arc<ConversationPipeline>,
    persistence: Arc<dyn PersistenceBackend>,
    metrics: ConversationMetrics,
    default_config: SessionConfig,
    initialized: bool,
}

impl ConversationManager {
    /// Create a new conversation manager with the Ollama backend.
    pub fn new() -> Self {
        Self::with_engine(Box::new(OllamaEngine::new()))
    }

    /// Create a new conversation manager with a custom language engine.
    pub fn with_engine(engine: Box<dyn LanguageEngine>) -> Self {
        let default_config = SessionConfig::default();
        let metrics = ConversationMetrics::new();
        Self {
            sessions: DashMap::new(),
            engine: Arc::new(tokio::sync::RwLock::new(engine)),
            pipeline: Arc::new(ConversationPipeline::with_metrics(&default_config, metrics.clone())),
            persistence: Arc::new(InMemoryPersistence::new()),
            metrics,
            default_config,
            initialized: false,
        }
    }

    /// Create a new conversation manager with custom persistence.
    pub fn with_persistence(
        engine: Box<dyn LanguageEngine>,
        persistence: Arc<dyn PersistenceBackend>,
    ) -> Self {
        let default_config = SessionConfig::default();
        let metrics = ConversationMetrics::new();
        Self {
            sessions: DashMap::new(),
            engine: Arc::new(tokio::sync::RwLock::new(engine)),
            pipeline: Arc::new(ConversationPipeline::with_metrics(&default_config, metrics.clone())),
            persistence,
            metrics,
            default_config,
            initialized: false,
        }
    }

    /// Initialize the manager and language engine.
    pub async fn initialize(
        &mut self,
        engine_config: &LanguageEngineConfig,
    ) -> ConversationResult<()> {
        {
            let mut engine = self.engine.write().await;
            engine.initialize(engine_config).await?;
        }
        self.initialized = true;
        tracing::info!(
            "Conversation manager initialized with {} backend",
            engine_config.backend_type
        );
        Ok(())
    }

    /// Create a new conversation session.
    pub fn create_session(&self, config: Option<SessionConfig>) -> SessionId {
        let cfg = config.unwrap_or_else(|| self.default_config.clone());
        let session = ConversationSession::new(cfg);
        let id = session.id.clone();
        self.sessions
            .insert(id.clone(), Arc::new(tokio::sync::RwLock::new(session)));
        self.metrics.session_created();
        tracing::debug!("Created session {id}");
        id
    }

    /// Create a session for a specific user.
    pub fn create_user_session(
        &self,
        user_id: UserId,
        config: Option<SessionConfig>,
    ) -> SessionId {
        let cfg = config.unwrap_or_else(|| self.default_config.clone());
        let mut session = ConversationSession::new(cfg);
        session.user_id = Some(user_id.clone());
        let id = session.id.clone();
        self.sessions
            .insert(id.clone(), Arc::new(tokio::sync::RwLock::new(session)));
        self.metrics.session_created();
        tracing::debug!("Created session {id} for user {user_id}");
        id
    }

    /// Send a message and get a response.
    pub async fn chat(
        &self,
        session_id: &SessionId,
        message: &str,
    ) -> ConversationResult<ConversationResponse> {
        let session_ref = self
            .sessions
            .get(session_id)
            .ok_or_else(|| ConversationError::SessionNotFound(session_id.to_string()))?;

        let engine = self.engine.read().await;
        let pipeline = self.pipeline.clone();

        let mut session = session_ref.write().await;
        pipeline.process_turn(&mut session, message, engine.as_ref()).await
    }

    /// Send a message and get a streaming response.
    pub async fn chat_streaming(
        &self,
        session_id: &SessionId,
        message: &str,
    ) -> ConversationResult<tokio::sync::mpsc::Receiver<ConversationResult<StreamChunk>>> {
        let session_ref = self
            .sessions
            .get(session_id)
            .ok_or_else(|| ConversationError::SessionNotFound(session_id.to_string()))?;

        let engine = self.engine.read().await;
        let pipeline = self.pipeline.clone();

        let mut session = session_ref.write().await;
        pipeline
            .process_turn_streaming(&mut session, message, engine.as_ref())
            .await
    }

    /// Set cognitive context for a session.
    pub async fn set_context(
        &self,
        session_id: &SessionId,
        context: CognitiveContext,
    ) -> ConversationResult<()> {
        let session_ref = self
            .sessions
            .get(session_id)
            .ok_or_else(|| ConversationError::SessionNotFound(session_id.to_string()))?;

        let mut session = session_ref.write().await;
        session.set_cognitive_context(context);
        Ok(())
    }

    /// Get conversation history as text.
    pub async fn history(&self, session_id: &SessionId) -> ConversationResult<String> {
        let session_ref = self
            .sessions
            .get(session_id)
            .ok_or_else(|| ConversationError::SessionNotFound(session_id.to_string()))?;

        let session = session_ref.read().await;
        Ok(session.history_text())
    }

    /// Cancel a session.
    pub async fn cancel_session(&self, session_id: &SessionId) -> ConversationResult<()> {
        let session_ref = self
            .sessions
            .get(session_id)
            .ok_or_else(|| ConversationError::SessionNotFound(session_id.to_string()))?;

        let mut session = session_ref.write().await;
        session.cancel();
        self.metrics.session_destroyed();
        Ok(())
    }

    /// Pause a session.
    pub async fn pause_session(&self, session_id: &SessionId) -> ConversationResult<()> {
        let session_ref = self
            .sessions
            .get(session_id)
            .ok_or_else(|| ConversationError::SessionNotFound(session_id.to_string()))?;

        let mut session = session_ref.write().await;
        session.pause();
        Ok(())
    }

    /// Resume a paused session.
    pub async fn resume_session(&self, session_id: &SessionId) -> ConversationResult<()> {
        let session_ref = self
            .sessions
            .get(session_id)
            .ok_or_else(|| ConversationError::SessionNotFound(session_id.to_string()))?;

        let mut session = session_ref.write().await;
        session.resume();
        Ok(())
    }

    /// Complete a session.
    pub async fn complete_session(&self, session_id: &SessionId) -> ConversationResult<()> {
        let session_ref = self
            .sessions
            .get(session_id)
            .ok_or_else(|| ConversationError::SessionNotFound(session_id.to_string()))?;

        let mut session = session_ref.write().await;
        session.complete();
        self.metrics.session_destroyed();
        Ok(())
    }

    /// Restore a session from persistence.
    pub async fn restore_session(
        &self,
        session_id: &SessionId,
    ) -> ConversationResult<bool> {
        match self.persistence.load_session(session_id)? {
            Some(data) => {
                let mut session = ConversationSession::new(self.default_config.clone());
                session.id = data.session_id;
                session.messages = data.messages;
                session.state = data.state;
                session.total_tokens = data.token_usage;
                session.metadata = data.metadata;

                self.sessions
                    .insert(session_id.clone(), Arc::new(tokio::sync::RwLock::new(session)));
                tracing::info!("Restored session {session_id}");
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Save a session to persistence.
    pub async fn save_session(&self, session_id: &SessionId) -> ConversationResult<()> {
        let session_ref = self
            .sessions
            .get(session_id)
            .ok_or_else(|| ConversationError::SessionNotFound(session_id.to_string()))?;

        let session = session_ref.read().await;
        self.persistence.save_session(&session)?;
        Ok(())
    }

    /// Remove a session completely.
    pub async fn remove_session(&self, session_id: &SessionId) -> ConversationResult<bool> {
        if self.sessions.remove(session_id).is_some() {
            self.persistence.delete_session(session_id)?;
            self.metrics.session_destroyed();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get active session IDs.
    pub fn active_sessions(&self) -> Vec<SessionId> {
        self.sessions.iter().map(|s| s.key().clone()).collect()
    }

    /// Get the number of active sessions.
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Check if a session exists.
    pub fn has_session(&self, session_id: &SessionId) -> bool {
        self.sessions.contains_key(session_id)
    }

    /// Get session state.
    pub async fn session_state(
        &self,
        session_id: &SessionId,
    ) -> ConversationResult<SessionState> {
        let session_ref = self
            .sessions
            .get(session_id)
            .ok_or_else(|| ConversationError::SessionNotFound(session_id.to_string()))?;

        let session = session_ref.read().await;
        Ok(session.state.clone())
    }

    /// Get metrics snapshot.
    #[must_use]
    pub fn metrics_snapshot(&self) -> crate::metrics::MetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Clean up expired sessions.
    pub async fn cleanup_expired(&self, timeout: Duration) {
        let mut to_remove = Vec::new();

        for entry in self.sessions.iter() {
            let session = entry.value().read().await;
            let idle_duration = chrono::Utc::now()
                .signed_duration_since(session.last_active)
                .to_std()
                .unwrap_or(Duration::ZERO);

            if idle_duration > timeout && session.is_terminal() {
                to_remove.push(entry.key().clone());
            }
        }

        for id in to_remove {
            self.sessions.remove(&id);
            tracing::debug!("Cleaned up expired session {id}");
        }
    }

    /// Shutdown the manager.
    pub async fn shutdown(&self) -> ConversationResult<()> {
        // Complete all sessions.
        for session_ref in self.sessions.iter() {
            let mut session = session_ref.value().write().await;
            session.complete();
        }
        self.sessions.clear();

        // Shutdown the engine.
        let mut engine = self.engine.write().await;
        engine.shutdown().await?;

        tracing::info!("Conversation manager shut down");
        Ok(())
    }
}

impl Default for ConversationManager {
    fn default() -> Self {
        Self::new()
    }
}
