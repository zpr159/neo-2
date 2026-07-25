use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use super::types::{Message, TokenUsage};

/// Estimate tokens using a simple heuristic (words * 1.3 for English).
pub struct TokenEstimator {
    words_per_token: f64,
}

impl TokenEstimator {
    pub fn new() -> Self {
        Self {
            words_per_token: 1.3,
        }
    }

    pub fn with_ratio(words_per_token: f64) -> Self {
        Self { words_per_token }
    }

    /// Estimate token count from text.
    pub fn estimate(&self, text: &str) -> usize {
        let word_count = text.split_whitespace().count();
        (word_count as f64 / self.words_per_token).ceil() as usize
    }

    /// Estimate tokens for a message.
    pub fn estimate_message(&self, message: &Message) -> usize {
        let base = self.estimate(&message.content);
        let overhead = match message.role {
            super::types::MessageRole::System => 4,
            super::types::MessageRole::User => 4,
            super::types::MessageRole::Assistant => 4,
            super::types::MessageRole::Tool => 6,
        };
        base + overhead
    }

    /// Estimate tokens for a list of messages.
    pub fn estimate_messages(&self, messages: &[Message]) -> usize {
        messages.iter().map(|m| self.estimate_message(m)).sum()
    }
}

impl Default for TokenEstimator {
    fn default() -> Self {
        Self::new()
    }
}

/// Adapter for provider-specific tokenization.
///
/// Wraps a provider's token counting function and provides a unified interface.
pub struct TokenizerAdapter {
    provider_name: String,
    model_name: String,
    estimator: TokenEstimator,
    cache: HashMap<String, usize>,
}

impl TokenizerAdapter {
    pub fn new(provider_name: impl Into<String>, model_name: impl Into<String>) -> Self {
        Self {
            provider_name: provider_name.into(),
            model_name: model_name.into(),
            estimator: TokenEstimator::new(),
            cache: HashMap::new(),
        }
    }

    pub fn count(&self, text: &str) -> usize {
        if let Some(&cached) = self.cache.get(text) {
            return cached;
        }
        self.estimator.estimate(text)
    }

    pub fn provider_name(&self) -> &str {
        &self.provider_name
    }

    pub fn model_name(&self) -> &str {
        &self.model_name
    }
}

/// Thread-safe token counter that tracks usage across sessions.
pub struct TokenCounter {
    total_prompt_tokens: AtomicU64,
    total_completion_tokens: AtomicU64,
    total_tokens: AtomicU64,
    request_count: AtomicU64,
    per_session: tokio::sync::RwLock<HashMap<String, TokenUsage>>,
}

impl TokenCounter {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            total_prompt_tokens: AtomicU64::new(0),
            total_completion_tokens: AtomicU64::new(0),
            total_tokens: AtomicU64::new(0),
            request_count: AtomicU64::new(0),
            per_session: tokio::sync::RwLock::new(HashMap::new()),
        })
    }

    /// Record token usage for a request.
    pub async fn record(&self, session_id: &str, usage: &TokenUsage) {
        self.total_prompt_tokens
            .fetch_add(usage.prompt_tokens as u64, Ordering::Relaxed);
        self.total_completion_tokens
            .fetch_add(usage.completion_tokens as u64, Ordering::Relaxed);
        self.total_tokens
            .fetch_add(usage.total_tokens as u64, Ordering::Relaxed);
        self.request_count.fetch_add(1, Ordering::Relaxed);

        let mut sessions = self.per_session.write().await;
        let entry = sessions
            .entry(session_id.to_string())
            .or_insert_with(TokenUsage::default);
        entry.prompt_tokens += usage.prompt_tokens;
        entry.completion_tokens += usage.completion_tokens;
        entry.total_tokens += usage.total_tokens;
    }

    /// Get total token usage across all requests.
    pub fn total_usage(&self) -> TokenUsage {
        TokenUsage {
            prompt_tokens: self.total_prompt_tokens.load(Ordering::Relaxed) as usize,
            completion_tokens: self.total_completion_tokens.load(Ordering::Relaxed) as usize,
            total_tokens: self.total_tokens.load(Ordering::Relaxed) as usize,
        }
    }

    /// Get total request count.
    pub fn request_count(&self) -> u64 {
        self.request_count.load(Ordering::Relaxed)
    }

    /// Get token usage for a specific session.
    pub async fn session_usage(&self, session_id: &str) -> TokenUsage {
        let sessions = self.per_session.read().await;
        sessions
            .get(session_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Reset all counters.
    pub fn reset(&self) {
        self.total_prompt_tokens.store(0, Ordering::Relaxed);
        self.total_completion_tokens.store(0, Ordering::Relaxed);
        self.total_tokens.store(0, Ordering::Relaxed);
        self.request_count.store(0, Ordering::Relaxed);
    }

    /// Reset counters for a specific session.
    pub async fn reset_session(&self, session_id: &str) {
        let mut sessions = self.per_session.write().await;
        sessions.remove(session_id);
    }
}

impl Default for TokenCounter {
    fn default() -> Self {
        Self {
            total_prompt_tokens: AtomicU64::new(0),
            total_completion_tokens: AtomicU64::new(0),
            total_tokens: AtomicU64::new(0),
            request_count: AtomicU64::new(0),
            per_session: tokio::sync::RwLock::new(HashMap::new()),
        }
    }
}
