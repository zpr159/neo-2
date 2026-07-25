use crate::types::{SessionId, StreamChunk};

/// Accumulates and manages streaming responses.
pub struct ResponseStreamer {
    /// Accumulated responses keyed by session ID.
    responses: dashmap::DashMap<SessionId, StreamAccumulator>,
}

/// Accumulated state for a streaming response.
#[derive(Debug, Clone)]
pub struct StreamAccumulator {
    /// Full accumulated text.
    pub text: String,
    /// Number of chunks received.
    pub chunk_count: usize,
    /// Whether the stream is complete.
    pub complete: bool,
}

impl ResponseStreamer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            responses: dashmap::DashMap::new(),
        }
    }

    /// Record a stream chunk.
    pub fn record_chunk(&self, chunk: &StreamChunk) {
        let mut acc = self
            .responses
            .entry(chunk.session_id.clone())
            .or_insert_with(|| StreamAccumulator {
                text: String::new(),
                chunk_count: 0,
                complete: false,
            });
        acc.text.push_str(&chunk.text);
        acc.chunk_count += 1;
        acc.complete = chunk.done;
    }

    /// Record a completed response (non-streaming or final accumulation).
    pub async fn record_completed(&self, session_id: &SessionId, text: &str) {
        self.responses.insert(
            session_id.clone(),
            StreamAccumulator {
                text: text.to_string(),
                chunk_count: 1,
                complete: true,
            },
        );
    }

    /// Get the accumulated text for a session.
    pub fn get_accumulated(&self, session_id: &SessionId) -> Option<String> {
        self.responses.get(session_id).map(|a| a.text.clone())
    }

    /// Check if a session's stream is complete.
    pub fn is_complete(&self, session_id: &SessionId) -> bool {
        self.responses
            .get(session_id)
            .map_or(true, |a| a.complete)
    }

    /// Get the chunk count for a session.
    pub fn chunk_count(&self, session_id: &SessionId) -> usize {
        self.responses
            .get(session_id)
            .map_or(0, |a| a.chunk_count)
    }

    /// Remove a session's accumulated response.
    pub fn remove(&self, session_id: &SessionId) -> bool {
        self.responses.remove(session_id).is_some()
    }

    /// Clear all accumulated responses.
    pub fn clear(&self) {
        self.responses.clear();
    }
}

impl Default for ResponseStreamer {
    fn default() -> Self {
        Self::new()
    }
}

/// Wraps a stream receiver with automatic accumulation.
pub struct StreamAccumulatorWrapper {
    accumulated: String,
}

impl StreamAccumulatorWrapper {
    #[must_use]
    pub fn new() -> Self {
        Self {
            accumulated: String::new(),
        }
    }

    /// Process a chunk and return whether the stream is done.
    pub fn process_chunk(&mut self, chunk: &StreamChunk) -> bool {
        self.accumulated.push_str(&chunk.text);
        chunk.done
    }

    /// Get the accumulated text.
    #[must_use]
    pub fn accumulated_text(&self) -> &str {
        &self.accumulated
    }
}

impl Default for StreamAccumulatorWrapper {
    fn default() -> Self {
        Self::new()
    }
}
