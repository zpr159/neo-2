use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::types::SessionId;

/// Tracks metrics for the conversation subsystem.
#[derive(Debug)]
pub struct ConversationMetrics {
    inner: Arc<MetricsInner>,
}

#[derive(Debug)]
struct MetricsInner {
    active_sessions: AtomicU64,
    total_sessions: AtomicU64,
    total_messages: AtomicU64,
    total_tokens: AtomicU64,
    total_tool_calls: AtomicU64,
    context_assembly_time_ns: AtomicU64,
    provider_latency_ns: AtomicU64,
    reasoning_latency_ns: AtomicU64,
    planning_latency_ns: AtomicU64,
    memory_retrieval_latency_ns: AtomicU64,
    stream_chunks_sent: AtomicU64,
}

impl ConversationMetrics {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(MetricsInner {
                active_sessions: AtomicU64::new(0),
                total_sessions: AtomicU64::new(0),
                total_messages: AtomicU64::new(0),
                total_tokens: AtomicU64::new(0),
                total_tool_calls: AtomicU64::new(0),
                context_assembly_time_ns: AtomicU64::new(0),
                provider_latency_ns: AtomicU64::new(0),
                reasoning_latency_ns: AtomicU64::new(0),
                planning_latency_ns: AtomicU64::new(0),
                memory_retrieval_latency_ns: AtomicU64::new(0),
                stream_chunks_sent: AtomicU64::new(0),
            }),
        }
    }

    pub fn session_created(&self) {
        self.inner.total_sessions.fetch_add(1, Ordering::Relaxed);
        self.inner.active_sessions.fetch_add(1, Ordering::Relaxed);
    }

    pub fn session_destroyed(&self) {
        self.inner.active_sessions.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn message_sent(&self) {
        self.inner.total_messages.fetch_add(1, Ordering::Relaxed);
    }

    pub fn tokens_used(&self, count: usize) {
        self.inner
            .total_tokens
            .fetch_add(count as u64, Ordering::Relaxed);
    }

    pub fn tool_call_recorded(&self) {
        self.inner
            .total_tool_calls
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_context_assembly_time(&self, duration: Duration) {
        self.inner
            .context_assembly_time_ns
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    pub fn record_provider_latency(&self, duration: Duration) {
        self.inner
            .provider_latency_ns
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    pub fn record_reasoning_latency(&self, duration: Duration) {
        self.inner
            .reasoning_latency_ns
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    pub fn record_planning_latency(&self, duration: Duration) {
        self.inner
            .planning_latency_ns
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    pub fn record_memory_retrieval_latency(&self, duration: Duration) {
        self.inner
            .memory_retrieval_latency_ns
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    pub fn stream_chunk_sent(&self) {
        self.inner
            .stream_chunks_sent
            .fetch_add(1, Ordering::Relaxed);
    }

    #[must_use]
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            active_sessions: self.inner.active_sessions.load(Ordering::Relaxed),
            total_sessions: self.inner.total_sessions.load(Ordering::Relaxed),
            total_messages: self.inner.total_messages.load(Ordering::Relaxed),
            total_tokens: self.inner.total_tokens.load(Ordering::Relaxed),
            total_tool_calls: self.inner.total_tool_calls.load(Ordering::Relaxed),
            context_assembly_time_ns: self.inner.context_assembly_time_ns.load(Ordering::Relaxed),
            provider_latency_ns: self.inner.provider_latency_ns.load(Ordering::Relaxed),
            reasoning_latency_ns: self.inner.reasoning_latency_ns.load(Ordering::Relaxed),
            planning_latency_ns: self.inner.planning_latency_ns.load(Ordering::Relaxed),
            memory_retrieval_latency_ns: self
                .inner
                .memory_retrieval_latency_ns
                .load(Ordering::Relaxed),
            stream_chunks_sent: self.inner.stream_chunks_sent.load(Ordering::Relaxed),
        }
    }
}

impl Default for ConversationMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ConversationMetrics {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Point-in-time snapshot of all metrics.
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub active_sessions: u64,
    pub total_sessions: u64,
    pub total_messages: u64,
    pub total_tokens: u64,
    pub total_tool_calls: u64,
    pub context_assembly_time_ns: u64,
    pub provider_latency_ns: u64,
    pub reasoning_latency_ns: u64,
    pub planning_latency_ns: u64,
    pub memory_retrieval_latency_ns: u64,
    pub stream_chunks_sent: u64,
}

/// Per-session metrics tracker.
#[derive(Debug, Clone, Default)]
pub struct SessionMetrics {
    pub session_id: SessionId,
    pub messages_exchanged: usize,
    pub tokens_used: usize,
    pub tool_calls: usize,
    pub context_assembly_time: Duration,
    pub provider_latency: Duration,
    pub reasoning_latency: Duration,
    pub planning_latency: Duration,
    pub memory_retrieval_latency: Duration,
}

impl SessionMetrics {
    pub fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            ..Default::default()
        }
    }
}
