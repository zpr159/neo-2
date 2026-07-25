use std::collections::HashMap;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::error::{AgentError, AgentResult};
use crate::types::AgentId;

// ---------------------------------------------------------------------------
// ContextVersion
// ---------------------------------------------------------------------------

/// A monotonically increasing version counter for optimistic locking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContextVersion(pub u64);

impl ContextVersion {
    /// Create the initial version.
    #[must_use]
    pub fn initial() -> Self {
        Self(1)
    }

    /// Advance to the next version.
    #[must_use]
    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}

impl Default for ContextVersion {
    fn default() -> Self {
        Self::initial()
    }
}

// ---------------------------------------------------------------------------
// ContextEntry
// ---------------------------------------------------------------------------

/// A single entry in a shared context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEntry {
    /// The key.
    pub key: String,
    /// The value.
    pub value: serde_json::Value,
    /// The version when this entry was last modified.
    pub version: ContextVersion,
    /// Who last modified this entry.
    pub last_writer: AgentId,
    /// When this entry was last modified.
    pub modified_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// ContextSnapshot
// ---------------------------------------------------------------------------

/// A point-in-time snapshot of a shared context for safe read access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSnapshot {
    /// The version of the context at snapshot time.
    pub version: ContextVersion,
    /// All entries in the context.
    pub entries: HashMap<String, serde_json::Value>,
    /// When the snapshot was taken.
    pub taken_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// SharedContext
// ---------------------------------------------------------------------------

/// A thread-safe, versioned shared context for inter-agent collaboration.
///
/// Supports optimistic locking via version numbers, conflict detection,
/// and merge operations.
pub struct SharedContext {
    /// The context entries.
    entries: DashMap<String, RwLock<ContextEntry>>,
    /// Current version of the context.
    version: RwLock<ContextVersion>,
    /// Name/description of this context.
    name: String,
    /// When the context was created.
    created_at: DateTime<Utc>,
}

impl SharedContext {
    /// Create a new shared context.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            entries: DashMap::new(),
            version: RwLock::new(ContextVersion::initial()),
            name: name.into(),
            created_at: Utc::now(),
        }
    }

    /// Get the current version of the context.
    pub async fn current_version(&self) -> ContextVersion {
        *self.version.read().await
    }

    /// Get the context name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the creation time.
    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Read a value from the context.
    pub async fn get(&self, key: &str) -> Option<serde_json::Value> {
        self.entries
            .get(key)
            .and_then(|entry| entry.try_read().ok().map(|e| e.value.clone()))
    }

    /// Write a value to the context.
    pub async fn set(
        &self,
        key: String,
        value: serde_json::Value,
        writer: AgentId,
    ) -> AgentResult<ContextVersion> {
        let new_version = {
            let mut version = self.version.write().await;
            *version = version.next();
            *version
        };

        let entry = ContextEntry {
            key: key.clone(),
            value,
            version: new_version,
            last_writer: writer,
            modified_at: Utc::now(),
        };

        self.entries.insert(key, RwLock::new(entry));
        Ok(new_version)
    }

    /// Update a value with optimistic locking.
    ///
    /// The update only succeeds if the context version matches the expected version.
    pub async fn update_with_version(
        &self,
        key: &str,
        value: serde_json::Value,
        writer: AgentId,
        expected_version: ContextVersion,
    ) -> AgentResult<ContextVersion> {
        let current = *self.version.read().await;
        if current != expected_version {
            return Err(AgentError::ContextConflict(format!(
                "expected version {} but current is {}",
                expected_version.0, current.0
            )));
        }
        self.set(key.to_string(), value, writer).await
    }

    /// Remove a key from the context.
    pub async fn remove(&self, key: &str) -> bool {
        self.entries.remove(key).is_some()
    }

    /// Check if a key exists.
    pub async fn contains_key(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    /// Take a snapshot of the current context state.
    pub async fn snapshot(&self) -> ContextSnapshot {
        let version = *self.version.read().await;
        let mut entries = HashMap::new();
        for entry in self.entries.iter() {
            if let Ok(e) = entry.value().try_read() {
                entries.insert(e.key.clone(), e.value.clone());
            }
        }
        ContextSnapshot {
            version,
            entries,
            taken_at: Utc::now(),
        }
    }

    /// Merge another snapshot into this context.
    pub async fn merge(
        &self,
        snapshot: ContextSnapshot,
        merger: AgentId,
    ) -> AgentResult<ContextVersion> {
        let mut merged_count = 0;
        for (key, value) in snapshot.entries {
            self.set(key, value, merger).await?;
            merged_count += 1;
        }
        tracing::debug!(
            "Merged {} entries into context '{}'",
            merged_count,
            self.name
        );
        Ok(self.current_version().await)
    }

    /// List all keys in the context.
    #[must_use]
    pub async fn keys(&self) -> Vec<String> {
        self.entries
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Return the number of entries.
    #[must_use]
    pub async fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the context is empty.
    #[must_use]
    pub async fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all entries from the context.
    pub async fn clear(&self) {
        self.entries.clear();
    }
}

// ---------------------------------------------------------------------------
// SharedBlackboard
// ---------------------------------------------------------------------------

/// A shared blackboard for agent collaboration.
///
/// Unlike `SharedContext`, the blackboard is organized into named sections
/// and supports read/write locks at the section level.
pub struct SharedBlackboard {
    /// Sections: section_name -> entries.
    sections: DashMap<String, DashMap<String, RwLock<serde_json::Value>>>,
}

impl SharedBlackboard {
    /// Create a new shared blackboard.
    #[must_use]
    pub fn new() -> Self {
        Self {
            sections: DashMap::new(),
        }
    }

    /// Create a new section.
    pub fn create_section(&self, name: &str) {
        self.sections.entry(name.to_string()).or_default();
    }

    /// Write to a section.
    pub async fn write(
        &self,
        section: &str,
        key: &str,
        value: serde_json::Value,
    ) -> AgentResult<()> {
        let entries = self
            .sections
            .get(section)
            .ok_or_else(|| AgentError::NotFound(format!("section '{section}' not found")))?;

        entries.insert(key.to_string(), RwLock::new(value));
        Ok(())
    }

    /// Read from a section.
    pub async fn read(&self, section: &str, key: &str) -> AgentResult<serde_json::Value> {
        let entries = self
            .sections
            .get(section)
            .ok_or_else(|| AgentError::NotFound(format!("section '{section}' not found")))?;

        entries
            .get(key)
            .and_then(|v| v.try_read().ok().map(|v| v.clone()))
            .ok_or_else(|| {
                AgentError::NotFound(format!("key '{key}' not found in section '{section}'"))
            })
    }

    /// List all keys in a section.
    pub async fn list_section(&self, section: &str) -> AgentResult<Vec<String>> {
        let entries = self
            .sections
            .get(section)
            .ok_or_else(|| AgentError::NotFound(format!("section '{section}' not found")))?;

        Ok(entries.iter().map(|e| e.key().clone()).collect())
    }

    /// List all sections.
    #[must_use]
    pub fn list_sections(&self) -> Vec<String> {
        self.sections.iter().map(|e| e.key().clone()).collect()
    }

    /// Remove a key from a section.
    pub async fn remove(&self, section: &str, key: &str) -> bool {
        self.sections
            .get(section)
            .map(|entries| entries.remove(key).is_some())
            .unwrap_or(false)
    }
}

impl Default for SharedBlackboard {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// SharedWorkspace
// ---------------------------------------------------------------------------

/// A workspace that agents can share for collaborative work.
pub struct SharedWorkspace {
    /// The shared context.
    pub context: Arc<SharedContext>,
    /// The shared blackboard.
    pub blackboard: Arc<SharedBlackboard>,
    /// Working memory buffers per agent.
    working_memory: DashMap<AgentId, WorkingMemory>,
}

use std::sync::Arc;

/// Per-agent working memory.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkingMemory {
    /// Temporary data store for the agent's current task.
    pub data: HashMap<String, serde_json::Value>,
    /// Scratch pad for intermediate results.
    pub scratch: Vec<serde_json::Value>,
    /// Maximum capacity (number of entries).
    pub capacity: usize,
}

impl WorkingMemory {
    /// Create a new working memory with the given capacity.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            data: HashMap::new(),
            scratch: Vec::new(),
            capacity,
        }
    }

    /// Store a value in working memory.
    pub fn store(&mut self, key: String, value: serde_json::Value) -> AgentResult<()> {
        if self.data.len() >= self.capacity {
            return Err(AgentError::QuotaExceeded(
                "working memory capacity reached".into(),
            ));
        }
        self.data.insert(key, value);
        Ok(())
    }

    /// Retrieve a value from working memory.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.data.get(key)
    }

    /// Remove a value from working memory.
    pub fn remove(&mut self, key: &str) -> Option<serde_json::Value> {
        self.data.remove(key)
    }

    /// Push to the scratch pad.
    pub fn push_scratch(&mut self, value: serde_json::Value) {
        self.scratch.push(value);
    }

    /// Pop from the scratch pad.
    pub fn pop_scratch(&mut self) -> Option<serde_json::Value> {
        self.scratch.pop()
    }

    /// Clear working memory.
    pub fn clear(&mut self) {
        self.data.clear();
        self.scratch.clear();
    }
}

impl SharedWorkspace {
    /// Create a new shared workspace.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            context: Arc::new(SharedContext::new(name)),
            blackboard: Arc::new(SharedBlackboard::new()),
            working_memory: DashMap::new(),
        }
    }

    /// Register an agent's working memory.
    pub fn register_agent(&self, agent_id: AgentId, capacity: usize) {
        self.working_memory
            .insert(agent_id, WorkingMemory::new(capacity));
    }

    /// Unregister an agent.
    pub fn unregister_agent(&self, agent_id: &AgentId) {
        self.working_memory.remove(agent_id);
    }

    /// Get an agent's working memory (read-only).
    pub fn get_working_memory(&self, agent_id: &AgentId) -> Option<WorkingMemory> {
        self.working_memory.get(agent_id).map(|wm| wm.clone())
    }

    /// Update an agent's working memory.
    pub fn update_working_memory(
        &self,
        agent_id: &AgentId,
        key: String,
        value: serde_json::Value,
    ) -> AgentResult<()> {
        if let Some(mut wm) = self.working_memory.get_mut(agent_id) {
            wm.store(key, value)
        } else {
            Err(AgentError::NotFound(format!(
                "no working memory for agent {agent_id}"
            )))
        }
    }

    /// Get a reference to the shared context.
    #[must_use]
    pub fn context(&self) -> &Arc<SharedContext> {
        &self.context
    }

    /// Get a reference to the shared blackboard.
    #[must_use]
    pub fn blackboard(&self) -> &Arc<SharedBlackboard> {
        &self.blackboard
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shared_context() {
        let ctx = SharedContext::new("test-ctx");
        let agent = AgentId::new();

        let v1 = ctx
            .set("key1".into(), serde_json::json!("value1"), agent)
            .await
            .unwrap();
        assert_eq!(v1, ContextVersion(2));

        let val = ctx.get("key1").await;
        assert_eq!(val, Some(serde_json::json!("value1")));

        // Optimistic update
        let v2 = ctx
            .update_with_version("key1", serde_json::json!("value2"), agent, v1)
            .await
            .unwrap();
        assert_eq!(v2, ContextVersion(3));

        // Conflicting update
        let result = ctx
            .update_with_version(
                "key1",
                serde_json::json!("value3"),
                agent,
                ContextVersion(1),
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_shared_context_snapshot() {
        let ctx = SharedContext::new("snap-ctx");
        let agent = AgentId::new();

        ctx.set("a".into(), serde_json::json!(1), agent)
            .await
            .unwrap();
        ctx.set("b".into(), serde_json::json!(2), agent)
            .await
            .unwrap();

        let snap = ctx.snapshot().await;
        assert_eq!(snap.entries.len(), 2);
    }

    #[tokio::test]
    async fn test_shared_blackboard() {
        let bb = SharedBlackboard::new();
        bb.create_section("ideas");
        bb.write("ideas", "agent1", serde_json::json!("idea1"))
            .await
            .unwrap();

        let val = bb.read("ideas", "agent1").await.unwrap();
        assert_eq!(val, serde_json::json!("idea1"));

        let sections = bb.list_sections();
        assert!(sections.contains(&"ideas".to_string()));
    }

    #[test]
    fn test_working_memory() {
        let mut wm = WorkingMemory::new(5);
        wm.store("k1".into(), serde_json::json!("v1")).unwrap();
        assert_eq!(wm.get("k1"), Some(&serde_json::json!("v1")));

        wm.push_scratch(serde_json::json!("scratch"));
        assert_eq!(wm.pop_scratch(), Some(serde_json::json!("scratch")));
    }

    #[tokio::test]
    async fn test_shared_workspace() {
        let ws = SharedWorkspace::new("test-workspace");
        let agent = AgentId::new();
        ws.register_agent(agent, 100);

        ws.context()
            .set("goal".into(), serde_json::json!("achieve_agi"), agent)
            .await
            .unwrap();

        ws.blackboard().create_section("plans");
        ws.blackboard()
            .write("plans", "step1", serde_json::json!("research"))
            .await
            .unwrap();

        ws.update_working_memory(&agent, "current_task".into(), serde_json::json!("coding"))
            .unwrap();
    }
}
