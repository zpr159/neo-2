use crate::error::ConversationResult;

/// Interface to the memory subsystem.
///
/// Provides retrieval of relevant memories, episodic memory access,
/// and working memory management.
pub trait MemoryInterface: Send + Sync {
    /// Retrieve memories relevant to the given query.
    fn retrieve(&self, query: &str, limit: usize) -> ConversationResult<Vec<MemoryResult>>;

    /// Store a new memory.
    fn store(&self, memory: NewMemory) -> ConversationResult<String>;

    /// Get working memory contents.
    fn working_memory(&self) -> ConversationResult<Vec<String>>;

    /// Update working memory.
    fn update_working_memory(&self, entries: Vec<String>) -> ConversationResult<()>;
}

/// A retrieved memory.
#[derive(Debug, Clone)]
pub struct MemoryResult {
    pub id: String,
    pub content: String,
    pub relevance: f64,
    pub source: String,
    pub timestamp: Option<String>,
}

/// A new memory to store.
#[derive(Debug, Clone)]
pub struct NewMemory {
    pub content: String,
    pub source: String,
    pub importance: f64,
    pub tags: Vec<String>,
}

/// Default in-memory memory interface.
pub struct DefaultMemory {
    memories: std::sync::Mutex<Vec<MemoryResult>>,
    working: std::sync::Mutex<Vec<String>>,
}

impl DefaultMemory {
    pub fn new() -> Self {
        Self {
            memories: std::sync::Mutex::new(Vec::new()),
            working: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl Default for DefaultMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryInterface for DefaultMemory {
    fn retrieve(&self, _query: &str, limit: usize) -> ConversationResult<Vec<MemoryResult>> {
        let memories = self.memories.lock().map_err(|e| {
            crate::error::ConversationError::Internal(format!("lock poisoned: {e}"))
        })?;
        Ok(memories.iter().take(limit).cloned().collect())
    }

    fn store(&self, memory: NewMemory) -> ConversationResult<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let result = MemoryResult {
            id: id.clone(),
            content: memory.content,
            relevance: memory.importance,
            source: memory.source,
            timestamp: None,
        };
        let mut memories = self.memories.lock().map_err(|e| {
            crate::error::ConversationError::Internal(format!("lock poisoned: {e}"))
        })?;
        memories.push(result);
        Ok(id)
    }

    fn working_memory(&self) -> ConversationResult<Vec<String>> {
        let working = self.working.lock().map_err(|e| {
            crate::error::ConversationError::Internal(format!("lock poisoned: {e}"))
        })?;
        Ok(working.clone())
    }

    fn update_working_memory(&self, entries: Vec<String>) -> ConversationResult<()> {
        let mut working = self.working.lock().map_err(|e| {
            crate::error::ConversationError::Internal(format!("lock poisoned: {e}"))
        })?;
        *working = entries;
        Ok(())
    }
}
