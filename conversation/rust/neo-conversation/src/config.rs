use serde::{Deserialize, Serialize};

use crate::language::LanguageEngineConfig;
use crate::types::{ConversationMode, SessionConfig};

/// Top-level configuration for the conversation subsystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationConfig {
    pub engine: LanguageEngineConfig,
    pub session: SessionConfig,
    pub max_sessions: usize,
    pub session_timeout_secs: u64,
    pub persist_history: bool,
    pub history_dir: Option<String>,
    pub enable_cognitive_context: bool,
    pub context_gathering_timeout_ms: u64,
    pub default_mode: ConversationMode,
    pub streaming_enabled: bool,
    pub persistence_enabled: bool,
    pub memory_enabled: bool,
    pub tool_calling_enabled: bool,
    pub reasoning_enabled: bool,
    pub planning_enabled: bool,
    pub max_concurrent_requests: usize,
    pub response_validation_enabled: bool,
    pub long_context_enabled: bool,
    pub long_context_summary_threshold: usize,
    pub user_model_enabled: bool,
}

impl Default for ConversationConfig {
    fn default() -> Self {
        Self {
            engine: LanguageEngineConfig::default(),
            session: SessionConfig::default(),
            max_sessions: 1000,
            session_timeout_secs: 3600,
            persist_history: false,
            history_dir: None,
            enable_cognitive_context: true,
            context_gathering_timeout_ms: 5000,
            default_mode: ConversationMode::Assistant,
            streaming_enabled: true,
            persistence_enabled: false,
            memory_enabled: true,
            tool_calling_enabled: true,
            reasoning_enabled: true,
            planning_enabled: true,
            max_concurrent_requests: 32,
            response_validation_enabled: true,
            long_context_enabled: true,
            long_context_summary_threshold: 16384,
            user_model_enabled: true,
        }
    }
}
