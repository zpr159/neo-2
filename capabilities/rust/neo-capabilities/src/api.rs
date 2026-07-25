//! # Capability API
//!
//! Provides the complete API surface for managing capabilities in the Neo AGI OS.
//! Handles registration, lifecycle, execution, indexing, search, import/export,
//! and execution record management.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use parking_lot::RwLock;
use uuid::Uuid;

use crate::core::{
    Capability, CapabilityCategory, CapabilityEntry, CapabilityId, CapabilityMetadata,
    CapabilityNamespace, CapabilityResult_output, CapabilityState, CapabilitySummary,
    ExecutionContext,
};
use crate::discovery::{CapabilitySource, DiscoveryEngine, DiscoveryStrategy};
use crate::error::{CapabilityError, CapabilityResult};
use crate::execution::{CapabilityExecutor, ExecutionRecord};

/// The complete capability API surface for the Neo AGI OS.
///
/// Provides registration, lifecycle management, execution, indexing,
/// search, import/export, and execution record querying.
pub struct CapabilityApi {
    entries: RwLock<HashMap<CapabilityId, CapabilityEntry>>,
    capabilities: RwLock<HashMap<CapabilityId, Arc<RwLock<dyn Capability>>>>,
    executor: CapabilityExecutor,
    discovery: DiscoveryEngine,
    by_name: RwLock<HashMap<String, CapabilityId>>,
    by_alias: RwLock<HashMap<String, CapabilityId>>,
    by_namespace: RwLock<HashMap<String, Vec<CapabilityId>>>,
    by_tag: RwLock<HashMap<String, Vec<CapabilityId>>>,
    by_category: RwLock<HashMap<String, Vec<CapabilityId>>>,
}

impl CapabilityApi {
    /// Create a new empty CapabilityApi.
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            capabilities: RwLock::new(HashMap::new()),
            executor: CapabilityExecutor::new(),
            discovery: DiscoveryEngine::new(),
            by_name: RwLock::new(HashMap::new()),
            by_alias: RwLock::new(HashMap::new()),
            by_namespace: RwLock::new(HashMap::new()),
            by_tag: RwLock::new(HashMap::new()),
            by_category: RwLock::new(HashMap::new()),
        }
    }

    /// Register a capability.
    ///
    /// Validates metadata, checks for name/alias conflicts, stores the
    /// capability implementation, transitions state to `Registered`, and
    /// populates all secondary indexes.
    pub fn register(
        &self,
        capability: Arc<RwLock<dyn Capability>>,
    ) -> CapabilityResult<CapabilityId> {
        let metadata = capability.read().metadata().clone();
        metadata.validate()?;

        {
            let by_name = self.by_name.read();
            if by_name.contains_key(&metadata.name) {
                return Err(CapabilityError::already_registered(format!(
                    "capability '{}' is already registered",
                    metadata.name
                )));
            }
        }

        {
            let by_alias = self.by_alias.read();
            for alias in metadata.aliases.as_slice() {
                if by_alias.contains_key(alias) {
                    return Err(CapabilityError::already_registered(format!(
                        "alias '{}' is already registered",
                        alias
                    )));
                }
            }
        }

        let id = metadata.id;

        let mut entry = CapabilityEntry::new(metadata.clone());
        entry.transition(CapabilityState::Registered)?;

        self.entries.write().insert(id, entry);
        self.capabilities.write().insert(id, capability);

        self.by_name.write().insert(metadata.name.clone(), id);

        {
            let mut idx = self.by_alias.write();
            for alias in metadata.aliases.as_slice() {
                idx.insert(alias.clone(), id);
            }
        }

        self.by_namespace
            .write()
            .entry(metadata.namespace.0.clone())
            .or_default()
            .push(id);

        {
            let mut idx = self.by_tag.write();
            for tag in metadata.tags.as_set() {
                idx.entry(tag.clone()).or_default().push(id);
            }
        }

        self.by_category
            .write()
            .entry(metadata.category.to_string())
            .or_default()
            .push(id);

        let source = CapabilitySource {
            source_type: DiscoveryStrategy::BuiltIn,
            location: format!("api://{}", metadata.name),
            discovered_at: Utc::now(),
            checksum: String::new(),
        };
        let _ = self.discovery.register_source(id, source);

        Ok(id)
    }

    /// Unregister a capability.
    ///
    /// Removes from all indexes, transitions state to `Revoked`, and drops
    /// the stored capability implementation.
    pub fn unregister(&self, id: CapabilityId) -> CapabilityResult<()> {
        let metadata = {
            let mut entries = self.entries.write();
            let entry = entries
                .get_mut(&id)
                .ok_or_else(|| CapabilityError::not_found(format!("capability {}", id)))?;
            entry.transition(CapabilityState::Revoked)?;
            entry.metadata.clone()
        };

        self.by_name.write().remove(&metadata.name);

        {
            let mut idx = self.by_alias.write();
            for alias in metadata.aliases.as_slice() {
                idx.remove(alias);
            }
        }

        {
            let mut idx = self.by_namespace.write();
            if let Some(ids) = idx.get_mut(&metadata.namespace.0) {
                ids.retain(|&x| x != id);
                if ids.is_empty() {
                    idx.remove(&metadata.namespace.0);
                }
            }
        }

        {
            let mut idx = self.by_tag.write();
            for tag in metadata.tags.as_set() {
                if let Some(ids) = idx.get_mut(tag) {
                    ids.retain(|&x| x != id);
                    if ids.is_empty() {
                        idx.remove(tag);
                    }
                }
            }
        }

        {
            let mut idx = self.by_category.write();
            let key = metadata.category.to_string();
            if let Some(ids) = idx.get_mut(&key) {
                ids.retain(|&x| x != id);
                if ids.is_empty() {
                    idx.remove(&key);
                }
            }
        }

        self.entries.write().remove(&id);
        self.capabilities.write().remove(&id);
        self.discovery.remove_source(&id);

        Ok(())
    }

    /// Execute a capability by ID.
    ///
    /// Verifies the capability is in the `Enabled` state, then delegates
    /// to the executor. Updates execution statistics on the entry afterward.
    pub async fn execute(
        &self,
        id: CapabilityId,
        input: serde_json::Value,
        context: ExecutionContext,
    ) -> CapabilityResult<CapabilityResult_output> {
        {
            let entries = self.entries.read();
            let entry = entries
                .get(&id)
                .ok_or_else(|| CapabilityError::not_found(format!("capability {}", id)))?;
            if entry.state != CapabilityState::Enabled {
                return Err(CapabilityError::invalid_state(format!(
                    "capability '{}' is not enabled (current state: {})",
                    entry.metadata.name, entry.state
                )));
            }
        }

        let cap_arc = {
            let caps = self.capabilities.read();
            caps.get(&id)
                .ok_or_else(|| {
                    CapabilityError::not_found(format!(
                        "no implementation registered for capability '{}'",
                        id
                    ))
                })?
                .clone()
        };

        let result = {
            let guard = cap_arc.read();
            self.executor
                .execute_capability(&*guard, context, input)
                .await?
        };

        {
            let mut entries = self.entries.write();
            if let Some(entry) = entries.get_mut(&id) {
                entry.execution_count += 1;
                entry.last_executed_at = Some(Utc::now());
            }
        }

        Ok(result)
    }

    /// Inspect a capability and return its summary.
    pub fn inspect(&self, id: CapabilityId) -> CapabilityResult<CapabilitySummary> {
        let entries = self.entries.read();
        let entry = entries
            .get(&id)
            .ok_or_else(|| CapabilityError::not_found(format!("capability {}", id)))?;
        Ok(CapabilitySummary::from(entry))
    }

    /// List all registered capability summaries.
    pub fn list(&self) -> Vec<CapabilitySummary> {
        self.entries
            .read()
            .values()
            .map(CapabilitySummary::from)
            .collect()
    }

    /// List capabilities matching a given category.
    pub fn list_by_category(&self, category: CapabilityCategory) -> Vec<CapabilitySummary> {
        let key = category.to_string();
        let ids = {
            let idx = self.by_category.read();
            match idx.get(&key) {
                Some(ids) => ids.clone(),
                None => return Vec::new(),
            }
        };
        let entries = self.entries.read();
        ids.iter()
            .filter_map(|id| entries.get(id).map(CapabilitySummary::from))
            .collect()
    }

    /// List capabilities matching a given namespace.
    pub fn list_by_namespace(&self, namespace: CapabilityNamespace) -> Vec<CapabilitySummary> {
        let ids = {
            let idx = self.by_namespace.read();
            match idx.get(&namespace.0) {
                Some(ids) => ids.clone(),
                None => return Vec::new(),
            }
        };
        let entries = self.entries.read();
        ids.iter()
            .filter_map(|id| entries.get(id).map(CapabilitySummary::from))
            .collect()
    }

    /// List capabilities that have a given tag.
    pub fn list_by_tag(&self, tag: &str) -> Vec<CapabilitySummary> {
        let ids = {
            let idx = self.by_tag.read();
            match idx.get(tag) {
                Some(ids) => ids.clone(),
                None => return Vec::new(),
            }
        };
        let entries = self.entries.read();
        ids.iter()
            .filter_map(|id| entries.get(id).map(CapabilitySummary::from))
            .collect()
    }

    /// Search capabilities by a query string.
    ///
    /// Matches case-insensitively against name, description, tags, and aliases.
    pub fn search(&self, query: &str) -> Vec<CapabilitySummary> {
        let q = query.to_lowercase();
        self.entries
            .read()
            .values()
            .filter(|entry| {
                entry.metadata.name.to_lowercase().contains(&q)
                    || entry
                        .metadata
                        .description
                        .to_lowercase()
                        .contains(&q)
                    || entry
                        .metadata
                        .tags
                        .0
                        .iter()
                        .any(|t| t.to_lowercase().contains(&q))
                    || entry
                        .metadata
                        .aliases
                        .0
                        .iter()
                        .any(|a| a.to_lowercase().contains(&q))
            })
            .map(CapabilitySummary::from)
            .collect()
    }

    /// Enable a capability (transition to `Enabled`).
    pub fn enable(&self, id: CapabilityId) -> CapabilityResult<()> {
        let mut entries = self.entries.write();
        let entry = entries
            .get_mut(&id)
            .ok_or_else(|| CapabilityError::not_found(format!("capability {}", id)))?;
        entry.transition(CapabilityState::Enabled)
    }

    /// Disable a capability (transition to `Disabled`).
    pub fn disable(&self, id: CapabilityId) -> CapabilityResult<()> {
        let mut entries = self.entries.write();
        let entry = entries
            .get_mut(&id)
            .ok_or_else(|| CapabilityError::not_found(format!("capability {}", id)))?;
        entry.transition(CapabilityState::Disabled)
    }

    /// Export a capability's metadata as JSON.
    pub fn export_capability(&self, id: CapabilityId) -> CapabilityResult<serde_json::Value> {
        let entries = self.entries.read();
        let entry = entries
            .get(&id)
            .ok_or_else(|| CapabilityError::not_found(format!("capability {}", id)))?;
        serde_json::to_value(&entry.metadata).map_err(|e| {
            CapabilityError::validation_failed(format!("failed to serialize metadata: {}", e))
        })
    }

    /// Import a capability from JSON metadata.
    ///
    /// Deserializes the metadata, creates an entry in `Registered` state,
    /// and populates all indexes. No capability implementation is stored;
    /// this is useful for cataloguing external capabilities.
    pub fn import_capability(&self, data: serde_json::Value) -> CapabilityResult<CapabilityId> {
        let metadata: CapabilityMetadata = serde_json::from_value(data).map_err(|e| {
            CapabilityError::validation_failed(format!("invalid capability data: {}", e))
        })?;
        metadata.validate()?;

        {
            let by_name = self.by_name.read();
            if by_name.contains_key(&metadata.name) {
                return Err(CapabilityError::already_registered(format!(
                    "capability '{}' already exists",
                    metadata.name
                )));
            }
        }

        {
            let entries = self.entries.read();
            if entries.contains_key(&metadata.id) {
                return Err(CapabilityError::already_registered(format!(
                    "capability with ID {} already exists",
                    metadata.id
                )));
            }
        }

        let id = metadata.id;
        let mut entry = CapabilityEntry::new(metadata.clone());
        entry.transition(CapabilityState::Registered)?;

        self.entries.write().insert(id, entry);
        self.by_name.write().insert(metadata.name.clone(), id);

        {
            let mut idx = self.by_alias.write();
            for alias in metadata.aliases.as_slice() {
                idx.insert(alias.clone(), id);
            }
        }

        self.by_namespace
            .write()
            .entry(metadata.namespace.0.clone())
            .or_default()
            .push(id);

        {
            let mut idx = self.by_tag.write();
            for tag in metadata.tags.as_set() {
                idx.entry(tag.clone()).or_default().push(id);
            }
        }

        self.by_category
            .write()
            .entry(metadata.category.to_string())
            .or_default()
            .push(id);

        Ok(id)
    }

    /// Get the full metadata for a capability.
    pub fn get_metadata(&self, id: CapabilityId) -> CapabilityResult<CapabilityMetadata> {
        let entries = self.entries.read();
        let entry = entries
            .get(&id)
            .ok_or_else(|| CapabilityError::not_found(format!("capability {}", id)))?;
        Ok(entry.metadata.clone())
    }

    /// Get an execution record by its ID.
    pub fn get_execution_record(&self, execution_id: Uuid) -> Option<ExecutionRecord> {
        self.executor.get_record(&execution_id)
    }

    /// List all execution records.
    pub fn list_executions(&self) -> Vec<ExecutionRecord> {
        self.executor.list_executions()
    }

    /// Get IDs of all currently active (in-progress) executions.
    pub fn active_executions(&self) -> Vec<Uuid> {
        self.executor.active_executions()
    }

    /// Cancel an active execution. Returns `true` if found and cancelled.
    pub fn cancel_execution(&self, execution_id: Uuid) -> bool {
        self.executor.cancel_execution(&execution_id)
    }

    /// Total number of registered capabilities.
    pub fn capability_count(&self) -> usize {
        self.entries.read().len()
    }

    /// Number of capabilities currently in the `Enabled` state.
    pub fn enabled_count(&self) -> usize {
        self.entries
            .read()
            .values()
            .filter(|e| e.state == CapabilityState::Enabled)
            .count()
    }
}

impl Default for CapabilityApi {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use crate::core::{CapabilityCategory, CapabilityNamespace, CapabilityVersion};

    struct TestCap {
        meta: CapabilityMetadata,
    }

    impl TestCap {
        fn new(name: &str) -> Self {
            Self {
                meta: CapabilityMetadata::new(
                    name,
                    CapabilityVersion::initial(),
                    format!("Test capability: {}", name),
                    CapabilityCategory::System,
                ),
            }
        }

        fn with_category(name: &str, category: CapabilityCategory) -> Self {
            Self {
                meta: CapabilityMetadata::new(
                    name,
                    CapabilityVersion::initial(),
                    format!("Test capability: {}", name),
                    category,
                ),
            }
        }

        fn with_meta(meta: CapabilityMetadata) -> Self {
            Self { meta }
        }
    }

    #[async_trait]
    impl Capability for TestCap {
        fn metadata(&self) -> &CapabilityMetadata {
            &self.meta
        }

        fn metadata_mut(&mut self) -> &mut CapabilityMetadata {
            &mut self.meta
        }

        async fn execute(
            &self,
            _input: serde_json::Value,
            _context: ExecutionContext,
        ) -> CapabilityResult<CapabilityResult_output> {
            Ok(CapabilityResult_output::success(
                serde_json::json!({"status": "ok"}),
                0,
            ))
        }
    }

    fn make_cap(name: &str) -> Arc<RwLock<dyn Capability>> {
        Arc::new(RwLock::new(TestCap::new(name)))
    }

    fn make_cap_with_category(
        name: &str,
        category: CapabilityCategory,
    ) -> Arc<RwLock<dyn Capability>> {
        Arc::new(RwLock::new(TestCap::with_category(name, category)))
    }

    // ── API creation ─────────────────────────────────────────────────────

    #[test]
    fn api_creation() {
        let api = CapabilityApi::new();
        assert_eq!(api.capability_count(), 0);
        assert_eq!(api.enabled_count(), 0);
        assert!(api.list().is_empty());
    }

    #[test]
    fn api_default() {
        let api = CapabilityApi::default();
        assert_eq!(api.capability_count(), 0);
    }

    // ── Registration ─────────────────────────────────────────────────────

    #[test]
    fn register_and_inspect() {
        let api = CapabilityApi::new();
        let cap = make_cap("test-cap");
        let id = api.register(cap).unwrap();

        let summary = api.inspect(id).unwrap();
        assert_eq!(summary.name, "test-cap");
        assert_eq!(summary.state, CapabilityState::Registered);
        assert_eq!(summary.version, CapabilityVersion::initial());
        assert_eq!(summary.category, CapabilityCategory::System);
    }

    #[test]
    fn register_duplicate_name_fails() {
        let api = CapabilityApi::new();
        api.register(make_cap("dup")).unwrap();
        assert!(api.register(make_cap("dup")).is_err());
    }

    #[test]
    fn register_empty_name_fails() {
        let api = CapabilityApi::new();
        let cap = Arc::new(RwLock::new(TestCap::with_meta(
            CapabilityMetadata::new(
                "",
                CapabilityVersion::initial(),
                "desc",
                CapabilityCategory::System,
            ),
        )));
        assert!(api.register(cap).is_err());
    }

    #[test]
    fn register_empty_description_fails() {
        let api = CapabilityApi::new();
        let cap = Arc::new(RwLock::new(TestCap::with_meta(
            CapabilityMetadata::new(
                "valid-name",
                CapabilityVersion::initial(),
                "",
                CapabilityCategory::System,
            ),
        )));
        assert!(api.register(cap).is_err());
    }

    #[test]
    fn register_duplicate_alias_fails() {
        let api = CapabilityApi::new();
        let cap1 = Arc::new(RwLock::new(TestCap::with_meta(
            CapabilityMetadata::new(
                "cap-a",
                CapabilityVersion::initial(),
                "a",
                CapabilityCategory::System,
            )
            .with_alias("shared-alias"),
        )));
        let cap2 = Arc::new(RwLock::new(TestCap::with_meta(
            CapabilityMetadata::new(
                "cap-b",
                CapabilityVersion::initial(),
                "b",
                CapabilityCategory::System,
            )
            .with_alias("shared-alias"),
        )));

        api.register(cap1).unwrap();
        assert!(api.register(cap2).is_err());
    }

    #[test]
    fn register_increments_count() {
        let api = CapabilityApi::new();
        assert_eq!(api.capability_count(), 0);

        api.register(make_cap("c1")).unwrap();
        assert_eq!(api.capability_count(), 1);

        api.register(make_cap("c2")).unwrap();
        assert_eq!(api.capability_count(), 2);

        api.register(make_cap("c3")).unwrap();
        assert_eq!(api.capability_count(), 3);
    }

    // ── Unregistration ───────────────────────────────────────────────────

    #[test]
    fn unregister_removes_entry() {
        let api = CapabilityApi::new();
        let id = api.register(make_cap("unreg")).unwrap();
        assert_eq!(api.capability_count(), 1);

        api.unregister(id).unwrap();
        assert_eq!(api.capability_count(), 0);
        assert!(api.inspect(id).is_err());
    }

    #[test]
    fn unregister_nonexistent_fails() {
        let api = CapabilityApi::new();
        assert!(api.unregister(CapabilityId::new()).is_err());
    }

    #[test]
    fn unregister_removes_from_name_index() {
        let api = CapabilityApi::new();
        let id = api.register(make_cap("named")).unwrap();

        let results = api.search("named");
        assert_eq!(results.len(), 1);

        api.unregister(id).unwrap();

        let results = api.search("named");
        assert!(results.is_empty());
    }

    #[test]
    fn unregister_removes_from_tag_index() {
        let api = CapabilityApi::new();
        let cap = Arc::new(RwLock::new(TestCap::with_meta(
            CapabilityMetadata::new(
                "tagged",
                CapabilityVersion::initial(),
                "tagged cap",
                CapabilityCategory::System,
            )
            .with_tag("removable"),
        )));
        let id = api.register(cap).unwrap();
        assert_eq!(api.list_by_tag("removable").len(), 1);

        api.unregister(id).unwrap();
        assert!(api.list_by_tag("removable").is_empty());
    }

    #[test]
    fn unregister_removes_from_category_index() {
        let api = CapabilityApi::new();
        let id = api
            .register(make_cap_with_category("tool-cap", CapabilityCategory::Tool))
            .unwrap();
        assert_eq!(
            api.list_by_category(CapabilityCategory::Tool).len(),
            1
        );

        api.unregister(id).unwrap();
        assert!(api
            .list_by_category(CapabilityCategory::Tool)
            .is_empty());
    }

    #[test]
    fn unregister_removes_from_namespace_index() {
        let api = CapabilityApi::new();
        let cap = Arc::new(RwLock::new(TestCap::with_meta(
            CapabilityMetadata::new(
                "ns-cap",
                CapabilityVersion::initial(),
                "ns",
                CapabilityCategory::System,
            )
            .with_namespace(CapabilityNamespace::inference()),
        )));
        let id = api.register(cap).unwrap();
        assert_eq!(
            api.list_by_namespace(CapabilityNamespace::inference())
                .len(),
            1
        );

        api.unregister(id).unwrap();
        assert!(api
            .list_by_namespace(CapabilityNamespace::inference())
            .is_empty());
    }

    #[test]
    fn unregister_removes_from_alias_index() {
        let api = CapabilityApi::new();
        let cap = Arc::new(RwLock::new(TestCap::with_meta(
            CapabilityMetadata::new(
                "aliased",
                CapabilityVersion::initial(),
                "aliased",
                CapabilityCategory::System,
            )
            .with_alias("temp-alias"),
        )));
        let id = api.register(cap).unwrap();
        assert_eq!(api.search("temp-alias").len(), 1);

        api.unregister(id).unwrap();
        assert!(api.search("temp-alias").is_empty());
    }

    // ── Execution ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn execute_enabled_capability() {
        let api = CapabilityApi::new();
        let id = api.register(make_cap("exec-ok")).unwrap();
        api.enable(id).unwrap();

        let ctx = ExecutionContext::new(id);
        let result = api
            .execute(id, serde_json::json!({"x": 1}), ctx)
            .await
            .unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn execute_not_enabled_fails() {
        let api = CapabilityApi::new();
        let id = api.register(make_cap("not-enabled")).unwrap();

        let ctx = ExecutionContext::new(id);
        let err = api.execute(id, serde_json::json!({}), ctx).await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn execute_not_found_fails() {
        let api = CapabilityApi::new();
        let ctx = ExecutionContext::new(CapabilityId::new());
        let err = api
            .execute(CapabilityId::new(), serde_json::json!({}), ctx)
            .await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn execute_updates_execution_count() {
        let api = CapabilityApi::new();
        let id = api.register(make_cap("counted")).unwrap();
        api.enable(id).unwrap();

        assert_eq!(api.inspect(id).unwrap().execution_count, 0);

        let ctx = ExecutionContext::new(id);
        api.execute(id, serde_json::json!({}), ctx).await
            .unwrap();

        assert_eq!(api.inspect(id).unwrap().execution_count, 1);
        assert!(api.inspect(id).unwrap().last_executed_at.is_some());
    }

    #[tokio::test]
    async fn execute_multiple_times() {
        let api = CapabilityApi::new();
        let id = api.register(make_cap("multi-exec")).unwrap();
        api.enable(id).unwrap();

        for _ in 0..5 {
            let ctx = ExecutionContext::new(id);
            api.execute(id, serde_json::json!({}), ctx).await
                .unwrap();
        }

        assert_eq!(api.inspect(id).unwrap().execution_count, 5);
    }

    // ── Listing ──────────────────────────────────────────────────────────

    #[test]
    fn list_returns_all() {
        let api = CapabilityApi::new();
        api.register(make_cap("a")).unwrap();
        api.register(make_cap("b")).unwrap();
        api.register(make_cap("c")).unwrap();

        let all = api.list();
        assert_eq!(all.len(), 3);

        let names: Vec<&str> = all.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
        assert!(names.contains(&"c"));
    }

    #[test]
    fn list_empty_registry() {
        let api = CapabilityApi::new();
        assert!(api.list().is_empty());
    }

    #[test]
    fn list_by_category_filters_correctly() {
        let api = CapabilityApi::new();
        api.register(make_cap_with_category("sys", CapabilityCategory::System))
            .unwrap();
        api.register(make_cap_with_category("reason", CapabilityCategory::Reasoning))
            .unwrap();
        api.register(make_cap_with_category("sys2", CapabilityCategory::System))
            .unwrap();

        let sys = api.list_by_category(CapabilityCategory::System);
        assert_eq!(sys.len(), 2);
        let names: Vec<&str> = sys.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"sys"));
        assert!(names.contains(&"sys2"));

        let reasoning = api.list_by_category(CapabilityCategory::Reasoning);
        assert_eq!(reasoning.len(), 1);

        let inference = api.list_by_category(CapabilityCategory::Inference);
        assert!(inference.is_empty());
    }

    #[test]
    fn list_by_category_custom() {
        let api = CapabilityApi::new();
        api.register(make_cap_with_category(
            "custom-a",
            CapabilityCategory::Custom("plugin_x".to_string()),
        ))
        .unwrap();

        let results = api.list_by_category(CapabilityCategory::Custom("plugin_x".to_string()));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "custom-a");

        let results = api.list_by_category(CapabilityCategory::Custom("plugin_y".to_string()));
        assert!(results.is_empty());
    }

    #[test]
    fn list_by_namespace_filters_correctly() {
        let api = CapabilityApi::new();
        let cap = Arc::new(RwLock::new(TestCap::with_meta(
            CapabilityMetadata::new(
                "inf-cap",
                CapabilityVersion::initial(),
                "inf",
                CapabilityCategory::Inference,
            )
            .with_namespace(CapabilityNamespace::inference()),
        )));
        api.register(cap).unwrap();

        assert_eq!(
            api.list_by_namespace(CapabilityNamespace::inference())
                .len(),
            1
        );
        assert!(api
            .list_by_namespace(CapabilityNamespace::memory())
            .is_empty());
    }

    #[test]
    fn list_by_tag_filters_correctly() {
        let api = CapabilityApi::new();
        let cap = Arc::new(RwLock::new(TestCap::with_meta(
            CapabilityMetadata::new(
                "tagged",
                CapabilityVersion::initial(),
                "tagged",
                CapabilityCategory::System,
            )
            .with_tag("ai")
            .with_tag("v2"),
        )));
        api.register(cap).unwrap();

        assert_eq!(api.list_by_tag("ai").len(), 1);
        assert_eq!(api.list_by_tag("v2").len(), 1);
        assert!(api.list_by_tag("missing").is_empty());
    }

    #[test]
    fn list_by_tag_multiple_capabilities() {
        let api = CapabilityApi::new();
        let cap1 = Arc::new(RwLock::new(TestCap::with_meta(
            CapabilityMetadata::new(
                "c1",
                CapabilityVersion::initial(),
                "c1",
                CapabilityCategory::System,
            )
            .with_tag("shared"),
        )));
        let cap2 = Arc::new(RwLock::new(TestCap::with_meta(
            CapabilityMetadata::new(
                "c2",
                CapabilityVersion::initial(),
                "c2",
                CapabilityCategory::System,
            )
            .with_tag("shared")
            .with_tag("extra"),
        )));
        api.register(cap1).unwrap();
        api.register(cap2).unwrap();

        assert_eq!(api.list_by_tag("shared").len(), 2);
        assert_eq!(api.list_by_tag("extra").len(), 1);
    }

    // ── Search ───────────────────────────────────────────────────────────

    #[test]
    fn search_by_name() {
        let api = CapabilityApi::new();
        api.register(make_cap("alpha-search")).unwrap();
        api.register(make_cap("beta-search")).unwrap();
        api.register(make_cap("gamma")).unwrap();

        let results = api.search("search");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn search_by_description() {
        let api = CapabilityApi::new();
        let cap = Arc::new(RwLock::new(TestCap::with_meta(
            CapabilityMetadata::new(
                "unique-name",
                CapabilityVersion::initial(),
                "Performs quantum computation on qubits",
                CapabilityCategory::System,
            ),
        )));
        api.register(cap).unwrap();

        let results = api.search("quantum");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "unique-name");
    }

    #[test]
    fn search_by_tag() {
        let api = CapabilityApi::new();
        let cap = Arc::new(RwLock::new(TestCap::with_meta(
            CapabilityMetadata::new(
                "ml-cap",
                CapabilityVersion::initial(),
                "ml cap",
                CapabilityCategory::System,
            )
            .with_tag("machine-learning"),
        )));
        api.register(cap).unwrap();

        let results = api.search("machine");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_by_alias() {
        let api = CapabilityApi::new();
        let cap = Arc::new(RwLock::new(TestCap::with_meta(
            CapabilityMetadata::new(
                "aliased",
                CapabilityVersion::initial(),
                "aliased",
                CapabilityCategory::System,
            )
            .with_alias("short"),
        )));
        api.register(cap).unwrap();

        let results = api.search("short");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "aliased");
    }

    #[test]
    fn search_case_insensitive() {
        let api = CapabilityApi::new();
        api.register(make_cap("MyCapability")).unwrap();

        assert_eq!(api.search("mycapability").len(), 1);
        assert_eq!(api.search("MYCAPABILITY").len(), 1);
        assert_eq!(api.search("MyCaPaBiLiTy").len(), 1);
    }

    #[test]
    fn search_empty_query_matches_all() {
        let api = CapabilityApi::new();
        api.register(make_cap("a")).unwrap();
        api.register(make_cap("b")).unwrap();

        let results = api.search("");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn search_no_results() {
        let api = CapabilityApi::new();
        api.register(make_cap("alpha")).unwrap();
        assert!(api.search("zzz-nonexistent").is_empty());
    }

    // ── Enable / Disable ─────────────────────────────────────────────────

    #[test]
    fn enable_transitions_to_enabled() {
        let api = CapabilityApi::new();
        let id = api.register(make_cap("en")).unwrap();
        assert_eq!(api.inspect(id).unwrap().state, CapabilityState::Registered);

        api.enable(id).unwrap();
        assert_eq!(api.inspect(id).unwrap().state, CapabilityState::Enabled);
    }

    #[test]
    fn disable_transitions_to_disabled() {
        let api = CapabilityApi::new();
        let id = api.register(make_cap("dis")).unwrap();
        api.enable(id).unwrap();
        assert_eq!(api.inspect(id).unwrap().state, CapabilityState::Enabled);

        api.disable(id).unwrap();
        assert_eq!(api.inspect(id).unwrap().state, CapabilityState::Disabled);
    }

    #[test]
    fn enable_then_disable_toggles_count() {
        let api = CapabilityApi::new();
        let id1 = api.register(make_cap("e1")).unwrap();
        let id2 = api.register(make_cap("e2")).unwrap();
        let _id3 = api.register(make_cap("e3")).unwrap();

        assert_eq!(api.enabled_count(), 0);

        api.enable(id1).unwrap();
        assert_eq!(api.enabled_count(), 1);

        api.enable(id2).unwrap();
        assert_eq!(api.enabled_count(), 2);

        api.disable(id1).unwrap();
        assert_eq!(api.enabled_count(), 1);
    }

    #[test]
    fn enable_nonexistent_fails() {
        let api = CapabilityApi::new();
        assert!(api.enable(CapabilityId::new()).is_err());
    }

    #[test]
    fn disable_nonexistent_fails() {
        let api = CapabilityApi::new();
        assert!(api.disable(CapabilityId::new()).is_err());
    }

    // ── Export / Import ──────────────────────────────────────────────────

    #[test]
    fn export_returns_metadata_json() {
        let api = CapabilityApi::new();
        let id = api.register(make_cap("export-me")).unwrap();

        let json = api.export_capability(id).unwrap();
        assert!(json.is_object());
        assert_eq!(json["name"], "export-me");
        assert_eq!(json["description"], "Test capability: export-me");
    }

    #[test]
    fn export_nonexistent_fails() {
        let api = CapabilityApi::new();
        assert!(api.export_capability(CapabilityId::new()).is_err());
    }

    #[test]
    fn import_roundtrip() {
        let api = CapabilityApi::new();
        let id = api.register(make_cap("roundtrip")).unwrap();
        let json = api.export_capability(id).unwrap();

        let api2 = CapabilityApi::new();
        let _imported_id = api2.import_capability(json).unwrap();
        let summary = api2.inspect(_imported_id).unwrap();
        assert_eq!(summary.name, "roundtrip");
        assert_eq!(summary.state, CapabilityState::Registered);
    }

    #[test]
    fn import_duplicate_name_fails() {
        let api = CapabilityApi::new();
        let id = api.register(make_cap("existing")).unwrap();
        let json = api.export_capability(id).unwrap();

        assert!(api.import_capability(json).is_err());
    }

    #[test]
    fn import_invalid_json_fails() {
        let api = CapabilityApi::new();
        assert!(api.import_capability(serde_json::json!("not valid")).is_err());
    }

    #[test]
    fn import_empty_name_fails() {
        let api = CapabilityApi::new();
        let json = serde_json::json!({
            "name": "",
            "id": {"0": Uuid::new_v4()},
            "version": {"major": 1, "minor": 0, "patch": 0},
            "description": "empty name",
            "category": "system",
            "namespace": {"0": "neo.core"},
            "tags": {"0": []},
            "aliases": {"0": []},
            "author": "",
            "license": "",
            "inputs": [],
            "output": {"schema": null, "description": "no output"},
            "dependencies": [],
            "required_permissions": [],
            "resource_requirements": {
                "cpu_units": 0.0,
                "gpu_units": 0.0,
                "memory_bytes": 0,
                "inference_tokens": 0,
                "disk_bytes": 0
            },
            "timeout_ms": null,
            "max_retries": 0,
            "requires_approval": false,
            "composable": true,
            "custom": {},
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        });
        assert!(api.import_capability(json).is_err());
    }

    #[test]
    fn import_stores_in_indexes() {
        let api = CapabilityApi::new();
        let cap = Arc::new(RwLock::new(TestCap::with_meta(
            CapabilityMetadata::new(
                "indexed-cap",
                CapabilityVersion::initial(),
                "indexed description",
                CapabilityCategory::Tool,
            )
            .with_namespace(CapabilityNamespace::developer())
            .with_tag("imported")
            .with_alias("imp-alias"),
        )));
        let id = api.register(cap).unwrap();
        let json = api.export_capability(id).unwrap();
        api.unregister(id).unwrap();

        let api2 = CapabilityApi::new();
        let _imported_id = api2.import_capability(json).unwrap();

        assert_eq!(api2.search("indexed-cap").len(), 1);
        assert_eq!(api2.list_by_category(CapabilityCategory::Tool).len(), 1);
        assert_eq!(
            api2.list_by_namespace(CapabilityNamespace::developer())
                .len(),
            1
        );
        assert_eq!(api2.list_by_tag("imported").len(), 1);
        assert_eq!(api2.search("imp-alias").len(), 1);
    }

    // ── Metadata ─────────────────────────────────────────────────────────

    #[test]
    fn get_metadata_returns_clone() {
        let api = CapabilityApi::new();
        let id = api.register(make_cap("meta-test")).unwrap();

        let meta = api.get_metadata(id).unwrap();
        assert_eq!(meta.name, "meta-test");
        assert_eq!(meta.version, CapabilityVersion::initial());
    }

    #[test]
    fn get_metadata_nonexistent_fails() {
        let api = CapabilityApi::new();
        assert!(api.get_metadata(CapabilityId::new()).is_err());
    }

    // ── Execution records ────────────────────────────────────────────────

    #[test]
    fn list_executions_empty() {
        let api = CapabilityApi::new();
        assert!(api.list_executions().is_empty());
    }

    #[test]
    fn active_executions_empty() {
        let api = CapabilityApi::new();
        assert!(api.active_executions().is_empty());
    }

    #[test]
    fn get_execution_record_none() {
        let api = CapabilityApi::new();
        assert!(api.get_execution_record(Uuid::new_v4()).is_none());
    }

    #[test]
    fn cancel_nonexistent_returns_false() {
        let api = CapabilityApi::new();
        assert!(!api.cancel_execution(Uuid::new_v4()));
    }

    #[tokio::test]
    async fn execute_creates_execution_record() {
        let api = CapabilityApi::new();
        let id = api.register(make_cap("recorded")).unwrap();
        api.enable(id).unwrap();

        let ctx = ExecutionContext::new(id);
        api.execute(id, serde_json::json!({}), ctx).await
            .unwrap();

        let records = api.list_executions();
        assert_eq!(records.len(), 1);
    }

    // ── Counts ───────────────────────────────────────────────────────────

    #[test]
    fn capability_count_after_register_unregister() {
        let api = CapabilityApi::new();
        assert_eq!(api.capability_count(), 0);

        let id1 = api.register(make_cap("c1")).unwrap();
        assert_eq!(api.capability_count(), 1);

        let _id2 = api.register(make_cap("c2")).unwrap();
        assert_eq!(api.capability_count(), 2);

        api.unregister(id1).unwrap();
        assert_eq!(api.capability_count(), 1);
    }

    #[test]
    fn enabled_count_reflects_state() {
        let api = CapabilityApi::new();
        let id1 = api.register(make_cap("e1")).unwrap();
        let id2 = api.register(make_cap("e2")).unwrap();

        assert_eq!(api.enabled_count(), 0);

        api.enable(id1).unwrap();
        assert_eq!(api.enabled_count(), 1);

        api.enable(id2).unwrap();
        assert_eq!(api.enabled_count(), 2);

        api.disable(id1).unwrap();
        assert_eq!(api.enabled_count(), 1);

        api.disable(id2).unwrap();
        assert_eq!(api.enabled_count(), 0);
    }

    // ── Edge cases ───────────────────────────────────────────────────────

    #[test]
    fn register_with_all_metadata_fields() {
        let api = CapabilityApi::new();
        let cap = Arc::new(RwLock::new(TestCap::with_meta(
            CapabilityMetadata::new(
                "full-cap",
                CapabilityVersion::new(2, 3, 4),
                "A fully configured capability",
                CapabilityCategory::Reasoning,
            )
            .with_namespace(CapabilityNamespace::reasoning())
            .with_tag("reasoning")
            .with_tag("advanced")
            .with_alias("full")
            .with_alias("advanced-reasoner")
            .with_author("neo-team")
            .with_timeout_ms(5000)
            .with_max_retries(3),
        )));
        let id = api.register(cap).unwrap();

        let meta = api.get_metadata(id).unwrap();
        assert_eq!(meta.name, "full-cap");
        assert_eq!(meta.version, CapabilityVersion::new(2, 3, 4));
        assert_eq!(meta.category, CapabilityCategory::Reasoning);
        assert_eq!(meta.namespace.as_str(), "neo.reasoning");
        assert!(meta.tags.contains("reasoning"));
        assert!(meta.tags.contains("advanced"));
        assert!(meta.aliases.matches("full"));
        assert!(meta.aliases.matches("advanced-reasoner"));
        assert_eq!(meta.author, "neo-team");
        assert_eq!(meta.timeout_ms, Some(5000));
        assert_eq!(meta.max_retries, 3);
    }

    #[test]
    fn search_finds_across_multiple_fields() {
        let api = CapabilityApi::new();
        let cap = Arc::new(RwLock::new(TestCap::with_meta(
            CapabilityMetadata::new(
                "nlp-processor",
                CapabilityVersion::initial(),
                "Natural language understanding for text analysis",
                CapabilityCategory::Inference,
            )
            .with_tag("nlp")
            .with_tag("text"),
        )));
        api.register(cap).unwrap();

        assert_eq!(api.search("nlp").len(), 1);
        assert_eq!(api.search("natural language").len(), 1);
        assert_eq!(api.search("text").len(), 1);
        assert_eq!(api.search("nlp-processor").len(), 1);
    }

    #[test]
    fn unregister_all_leaves_empty_indexes() {
        let api = CapabilityApi::new();
        let id1 = api
            .register(make_cap_with_category("s1", CapabilityCategory::Tool))
            .unwrap();
        let id2 = api
            .register(make_cap_with_category("s2", CapabilityCategory::Tool))
            .unwrap();

        api.unregister(id1).unwrap();
        api.unregister(id2).unwrap();

        assert!(api.list_by_category(CapabilityCategory::Tool).is_empty());
        assert!(api.list().is_empty());
    }

    #[test]
    fn multiple_namespaces_independent() {
        let api = CapabilityApi::new();
        let cap1 = Arc::new(RwLock::new(TestCap::with_meta(
            CapabilityMetadata::new(
                "mem-cap",
                CapabilityVersion::initial(),
                "memory",
                CapabilityCategory::Memory,
            )
            .with_namespace(CapabilityNamespace::memory()),
        )));
        let cap2 = Arc::new(RwLock::new(TestCap::with_meta(
            CapabilityMetadata::new(
                "know-cap",
                CapabilityVersion::initial(),
                "knowledge",
                CapabilityCategory::Knowledge,
            )
            .with_namespace(CapabilityNamespace::knowledge()),
        )));
        api.register(cap1).unwrap();
        api.register(cap2).unwrap();

        assert_eq!(
            api.list_by_namespace(CapabilityNamespace::memory())
                .len(),
            1
        );
        assert_eq!(
            api.list_by_namespace(CapabilityNamespace::knowledge())
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn execute_disabled_capability_fails() {
        let api = CapabilityApi::new();
        let id = api.register(make_cap("was-enabled")).unwrap();
        api.enable(id).unwrap();
        api.disable(id).unwrap();

        let ctx = ExecutionContext::new(id);
        let err = api.execute(id, serde_json::json!({}), ctx).await;
        assert!(err.is_err());
    }

    #[test]
    fn inspect_returns_summary_with_correct_fields() {
        let api = CapabilityApi::new();
        let cap = Arc::new(RwLock::new(TestCap::with_meta(
            CapabilityMetadata::new(
                "summary-cap",
                CapabilityVersion::new(3, 1, 0),
                "summary description",
                CapabilityCategory::Workflow,
            )
            .with_tag("workflow")
            .with_namespace(CapabilityNamespace::core()),
        )));
        let id = api.register(cap).unwrap();

        let summary = api.inspect(id).unwrap();
        assert_eq!(summary.id, id);
        assert_eq!(summary.name, "summary-cap");
        assert_eq!(summary.version, CapabilityVersion::new(3, 1, 0));
        assert_eq!(summary.category, CapabilityCategory::Workflow);
        assert_eq!(summary.namespace.as_str(), "neo.core");
        assert_eq!(summary.description, "summary description");
        assert!(summary.tags.contains(&"workflow".to_string()));
        assert_eq!(summary.execution_count, 0);
        assert_eq!(summary.state, CapabilityState::Registered);
    }
}
