use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::error::{AgentError, AgentResult};
use crate::types::AgentId;

// ---------------------------------------------------------------------------
// MemoryId
// ---------------------------------------------------------------------------

/// Identifier for a memory entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryId(pub uuid::Uuid);

impl MemoryId {
    /// Create a new memory ID.
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for MemoryId {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// MemoryTier
// ---------------------------------------------------------------------------

/// The tier of memory an entry belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryTier {
    /// Short-term working memory for current task.
    Working,
    /// Event-based memories of past experiences.
    Episodic,
    /// Long-term persistent knowledge.
    LongTerm,
    /// Skills and execution procedures.
    Procedural,
}

// ---------------------------------------------------------------------------
// MemoryEntry
// ---------------------------------------------------------------------------

/// A single memory entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Unique memory identifier.
    pub id: MemoryId,
    /// The content of the memory.
    pub content: serde_json::Value,
    /// Which tier this memory belongs to.
    pub tier: MemoryTier,
    /// Importance score (0.0 to 1.0).
    pub importance: f64,
    /// Access count.
    pub access_count: u32,
    /// When the memory was created.
    pub created_at: DateTime<Utc>,
    /// When the memory was last accessed.
    pub last_accessed: DateTime<Utc>,
    /// Tags for categorization.
    pub tags: Vec<String>,
    /// Source agent.
    pub source_agent: AgentId,
}

impl MemoryEntry {
    /// Create a new memory entry.
    #[must_use]
    pub fn new(content: serde_json::Value, tier: MemoryTier, source_agent: AgentId) -> Self {
        let now = Utc::now();
        Self {
            id: MemoryId::new(),
            content,
            tier,
            importance: 0.5,
            access_count: 0,
            created_at: now,
            last_accessed: now,
            tags: Vec::new(),
            source_agent,
        }
    }

    /// Record an access to this memory.
    pub fn record_access(&mut self) {
        self.access_count += 1;
        self.last_accessed = Utc::now();
    }

    /// Calculate a relevance score combining importance and recency.
    #[must_use]
    pub fn relevance_score(&self) -> f64 {
        let recency = {
            let elapsed = Utc::now()
                .signed_duration_since(self.last_accessed)
                .num_seconds() as f64;
            // Exponential decay with half-life of 1 hour
            (-elapsed / std::f64::consts::LN_2 * 3600.0).exp()
        };
        let frequency = (self.access_count as f64).ln_1p() / 10.0_f64.ln_1p();
        (self.importance * 0.4 + recency * 0.3 + frequency * 0.3).clamp(0.0, 1.0)
    }
}

// ---------------------------------------------------------------------------
// AgentMemory
// ---------------------------------------------------------------------------

/// The memory system for a single agent.
///
/// Provides tiered memory (working, episodic, long-term, procedural) with
/// importance-based retrieval and automatic decay.
pub struct AgentMemory {
    /// The agent this memory belongs to.
    agent_id: AgentId,
    /// Working memory (short-term, limited capacity).
    working_memory: VecDeque<MemoryEntry>,
    /// Working memory capacity.
    working_capacity: usize,
    /// Episodic memory.
    episodic_memory: Vec<MemoryEntry>,
    /// Long-term memory.
    long_term_memory: DashMap<MemoryId, MemoryEntry>,
    /// Procedural memory.
    procedural_memory: DashMap<MemoryId, MemoryEntry>,
    /// Maximum entries per tier.
    max_episodic: usize,
    max_long_term: usize,
    max_procedural: usize,
}

impl AgentMemory {
    /// Create a new agent memory system.
    #[must_use]
    pub fn new(
        agent_id: AgentId,
        working_capacity: usize,
        max_episodic: usize,
        max_long_term: usize,
        max_procedural: usize,
    ) -> Self {
        Self {
            agent_id,
            working_memory: VecDeque::with_capacity(working_capacity),
            working_capacity,
            episodic_memory: Vec::with_capacity(max_episodic),
            long_term_memory: DashMap::new(),
            procedural_memory: DashMap::new(),
            max_episodic,
            max_long_term,
            max_procedural,
        }
    }

    /// Store a memory entry.
    pub fn store(&mut self, entry: MemoryEntry) -> AgentResult<MemoryId> {
        let id = entry.id;
        match entry.tier {
            MemoryTier::Working => {
                if self.working_memory.len() >= self.working_capacity {
                    // Evict least relevant
                    self.working_memory.pop_front();
                }
                self.working_memory.push_back(entry);
            }
            MemoryTier::Episodic => {
                if self.episodic_memory.len() >= self.max_episodic {
                    // Evict least relevant episodic memory
                    if let Some(min_idx) = self
                        .episodic_memory
                        .iter()
                        .enumerate()
                        .min_by(|a, b| {
                            a.1.relevance_score()
                                .partial_cmp(&b.1.relevance_score())
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                        .map(|(i, _)| i)
                    {
                        self.episodic_memory.remove(min_idx);
                    }
                }
                self.episodic_memory.push(entry);
            }
            MemoryTier::LongTerm => {
                if self.long_term_memory.len() >= self.max_long_term {
                    return Err(AgentError::QuotaExceeded(
                        "long-term memory capacity reached".into(),
                    ));
                }
                self.long_term_memory.insert(id, entry);
            }
            MemoryTier::Procedural => {
                if self.procedural_memory.len() >= self.max_procedural {
                    return Err(AgentError::QuotaExceeded(
                        "procedural memory capacity reached".into(),
                    ));
                }
                self.procedural_memory.insert(id, entry);
            }
        }
        Ok(id)
    }

    /// Retrieve the most relevant memories matching a query.
    #[must_use]
    pub fn retrieve(&self, query: &str, limit: usize) -> Vec<MemoryEntry> {
        let query_lower = query.to_lowercase();
        let mut results: Vec<MemoryEntry> = Vec::new();

        // Search working memory
        for entry in &self.working_memory {
            let content_str = entry.content.to_string().to_lowercase();
            if content_str.contains(&query_lower)
                || entry
                    .tags
                    .iter()
                    .any(|t| t.to_lowercase().contains(&query_lower))
            {
                results.push(entry.clone());
            }
        }

        // Search episodic memory
        for entry in &self.episodic_memory {
            let content_str = entry.content.to_string().to_lowercase();
            if content_str.contains(&query_lower)
                || entry
                    .tags
                    .iter()
                    .any(|t| t.to_lowercase().contains(&query_lower))
            {
                results.push(entry.clone());
            }
        }

        // Search long-term memory
        for entry in self.long_term_memory.iter() {
            let content_str = entry.value().content.to_string().to_lowercase();
            if content_str.contains(&query_lower)
                || entry
                    .value()
                    .tags
                    .iter()
                    .any(|t| t.to_lowercase().contains(&query_lower))
            {
                results.push(entry.value().clone());
            }
        }

        // Sort by relevance
        results.sort_by(|a, b| {
            b.relevance_score()
                .partial_cmp(&a.relevance_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results.truncate(limit);
        results
    }

    /// Get all memories in a specific tier.
    #[must_use]
    pub fn get_tier(&self, tier: MemoryTier) -> Vec<MemoryEntry> {
        match tier {
            MemoryTier::Working => self.working_memory.iter().cloned().collect(),
            MemoryTier::Episodic => self.episodic_memory.clone(),
            MemoryTier::LongTerm => self
                .long_term_memory
                .iter()
                .map(|e| e.value().clone())
                .collect(),
            MemoryTier::Procedural => self
                .procedural_memory
                .iter()
                .map(|e| e.value().clone())
                .collect(),
        }
    }

    /// Remove a memory entry by ID.
    pub fn remove(&mut self, id: &MemoryId) -> bool {
        // Check long-term
        if self.long_term_memory.remove(id).is_some() {
            return true;
        }
        // Check procedural
        if self.procedural_memory.remove(id).is_some() {
            return true;
        }
        // Check episodic
        if let Some(pos) = self.episodic_memory.iter().position(|e| e.id == *id) {
            self.episodic_memory.remove(pos);
            return true;
        }
        // Check working
        if let Some(pos) = self.working_memory.iter().position(|e| e.id == *id) {
            self.working_memory.remove(pos);
            return true;
        }
        false
    }

    /// Consolidate memories: move important working memories to episodic.
    pub fn consolidate(&mut self, importance_threshold: f64) -> u32 {
        let mut consolidated = 0;
        let mut to_move = Vec::new();

        for (i, entry) in self.working_memory.iter().enumerate() {
            if entry.importance >= importance_threshold {
                to_move.push(i);
            }
        }

        // Move in reverse order to maintain indices
        for &idx in to_move.iter().rev() {
            if let Some(entry) = self.working_memory.remove(idx) {
                self.episodic_memory.push(entry);
                consolidated += 1;
            }
        }

        consolidated
    }

    /// Get total memory count across all tiers.
    #[must_use]
    pub fn total_count(&self) -> usize {
        self.working_memory.len()
            + self.episodic_memory.len()
            + self.long_term_memory.len()
            + self.procedural_memory.len()
    }

    /// Get the agent ID this memory belongs to.
    #[must_use]
    pub fn agent_id(&self) -> AgentId {
        self.agent_id
    }
}

// ---------------------------------------------------------------------------
// AgentMemoryManager
// ---------------------------------------------------------------------------

/// Manages memory for all agents in the system.
pub struct AgentMemoryManager {
    /// Memory systems per agent.
    memories: DashMap<AgentId, AgentMemory>,
    /// Default capacities.
    default_working_capacity: usize,
    default_episodic_capacity: usize,
    default_long_term_capacity: usize,
    default_procedural_capacity: usize,
}

impl AgentMemoryManager {
    /// Create a new memory manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            memories: DashMap::new(),
            default_working_capacity: 100,
            default_episodic_capacity: 10_000,
            default_long_term_capacity: 100_000,
            default_procedural_capacity: 10_000,
        }
    }

    /// Register an agent with default capacities.
    pub fn register_agent(&self, agent_id: AgentId) {
        let memory = AgentMemory::new(
            agent_id,
            self.default_working_capacity,
            self.default_episodic_capacity,
            self.default_long_term_capacity,
            self.default_procedural_capacity,
        );
        self.memories.insert(agent_id, memory);
    }

    /// Get a reference to an agent's memory.
    pub fn get_memory(
        &self,
        agent_id: &AgentId,
    ) -> Option<dashmap::mapref::one::Ref<'_, AgentId, AgentMemory>> {
        self.memories.get(agent_id)
    }

    /// Get a mutable reference to an agent's memory.
    pub fn get_memory_mut(
        &self,
        agent_id: &AgentId,
    ) -> Option<dashmap::mapref::one::RefMut<'_, AgentId, AgentMemory>> {
        self.memories.get_mut(agent_id)
    }

    /// Unregister an agent's memory.
    pub fn unregister_agent(&self, agent_id: &AgentId) {
        self.memories.remove(agent_id);
    }

    /// Return the number of registered agents.
    #[must_use]
    pub fn agent_count(&self) -> usize {
        self.memories.len()
    }
}

impl Default for AgentMemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_agent() -> AgentId {
        AgentId::new()
    }

    #[test]
    fn test_agent_memory_store_and_retrieve() {
        let agent = test_agent();
        let mut mem = AgentMemory::new(agent, 10, 100, 1000, 100);

        let entry = MemoryEntry::new(
            serde_json::json!("important fact about Rust"),
            MemoryTier::Working,
            agent,
        );
        let id = mem.store(entry).unwrap();

        let results = mem.retrieve("Rust", 10);
        assert!(!results.is_empty());
        assert_eq!(results[0].id, id);
    }

    #[test]
    fn test_memory_tiers() {
        let agent = test_agent();
        let mut mem = AgentMemory::new(agent, 5, 10, 100, 10);

        // Working memory
        let entry = MemoryEntry::new(
            serde_json::json!("working data"),
            MemoryTier::Working,
            agent,
        );
        mem.store(entry).unwrap();
        assert_eq!(mem.get_tier(MemoryTier::Working).len(), 1);

        // Episodic memory
        let entry = MemoryEntry::new(
            serde_json::json!("past experience"),
            MemoryTier::Episodic,
            agent,
        );
        mem.store(entry).unwrap();
        assert_eq!(mem.get_tier(MemoryTier::Episodic).len(), 1);

        // Long-term memory
        let entry = MemoryEntry::new(
            serde_json::json!("permanent knowledge"),
            MemoryTier::LongTerm,
            agent,
        );
        mem.store(entry).unwrap();
        assert_eq!(mem.get_tier(MemoryTier::LongTerm).len(), 1);
    }

    #[test]
    fn test_memory_consolidation() {
        let agent = test_agent();
        let mut mem = AgentMemory::new(agent, 10, 100, 1000, 10);

        let mut entry =
            MemoryEntry::new(serde_json::json!("important"), MemoryTier::Working, agent);
        entry.importance = 0.9;
        mem.store(entry).unwrap();

        let mut entry2 = MemoryEntry::new(
            serde_json::json!("not important"),
            MemoryTier::Working,
            agent,
        );
        entry2.importance = 0.1;
        mem.store(entry2).unwrap();

        let consolidated = mem.consolidate(0.8);
        assert_eq!(consolidated, 1);
        assert_eq!(mem.get_tier(MemoryTier::Working).len(), 1);
        assert_eq!(mem.get_tier(MemoryTier::Episodic).len(), 1);
    }

    #[test]
    fn test_memory_relevance_score() {
        let agent = test_agent();
        let mut entry = MemoryEntry::new(serde_json::json!("test"), MemoryTier::Working, agent);
        entry.importance = 1.0;
        entry.access_count = 10;
        let score = entry.relevance_score();
        assert!(score > 0.0 && score <= 1.0);
    }

    #[test]
    fn test_memory_manager() {
        let mgr = AgentMemoryManager::new();
        let agent = test_agent();
        mgr.register_agent(agent);

        assert_eq!(mgr.agent_count(), 1);
        assert!(mgr.get_memory(&agent).is_some());

        mgr.unregister_agent(&agent);
        assert_eq!(mgr.agent_count(), 0);
    }
}
