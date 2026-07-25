use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::CapabilityApi;
use crate::core::{
    Capability, CapabilityCategory, CapabilityId, CapabilityMetadata, CapabilityNamespace,
    CapabilityState, CapabilitySummary, CapabilityTags, CapabilityVersion, ExecutionContext,
};
use crate::error::{CapabilityError, CapabilityResult};
use crate::execution::ExecutionRecord;

/// Central capability registry that combines all registry operations.
///
/// The `CapabilityRegistry` is the primary entry point for managing capabilities
/// in the Neo system. It provides a unified interface for registration, lookup,
/// search, and lifecycle management.
pub struct CapabilityRegistry {
    api: Arc<CapabilityApi>,
}

impl CapabilityRegistry {
    /// Create a new capability registry.
    pub fn new() -> Self {
        Self {
            api: Arc::new(CapabilityApi::new()),
        }
    }

    /// Register a capability.
    pub fn register(&self, capability: Arc<RwLock<dyn Capability>>) -> CapabilityResult<CapabilityId> {
        self.api.register(capability)
    }

    /// Unregister a capability by ID.
    pub fn unregister(&self, id: CapabilityId) -> CapabilityResult<()> {
        self.api.unregister(id)
    }

    /// Execute a capability by ID.
    pub async fn execute(
        &self,
        id: CapabilityId,
        input: serde_json::Value,
        context: ExecutionContext,
    ) -> CapabilityResult<crate::core::CapabilityResult_output> {
        self.api.execute(id, input, context).await
    }

    /// Inspect a capability by ID.
    pub fn inspect(&self, id: CapabilityId) -> CapabilityResult<CapabilitySummary> {
        self.api.inspect(id)
    }

    /// List all registered capabilities.
    pub fn list(&self) -> Vec<CapabilitySummary> {
        self.api.list()
    }

    /// List capabilities by category.
    pub fn list_by_category(&self, category: CapabilityCategory) -> Vec<CapabilitySummary> {
        self.api.list_by_category(category)
    }

    /// List capabilities by namespace.
    pub fn list_by_namespace(&self, namespace: CapabilityNamespace) -> Vec<CapabilitySummary> {
        self.api.list_by_namespace(namespace)
    }

    /// List capabilities by tag.
    pub fn list_by_tag(&self, tag: &str) -> Vec<CapabilitySummary> {
        self.api.list_by_tag(tag)
    }

    /// Search capabilities by query string.
    pub fn search(&self, query: &str) -> Vec<CapabilitySummary> {
        self.api.search(query)
    }

    /// Enable a capability.
    pub fn enable(&self, id: CapabilityId) -> CapabilityResult<()> {
        self.api.enable(id)
    }

    /// Disable a capability.
    pub fn disable(&self, id: CapabilityId) -> CapabilityResult<()> {
        self.api.disable(id)
    }

    /// Export a capability as JSON.
    pub fn export_capability(&self, id: CapabilityId) -> CapabilityResult<serde_json::Value> {
        self.api.export_capability(id)
    }

    /// Import a capability from JSON.
    pub fn import_capability(&self, data: serde_json::Value) -> CapabilityResult<CapabilityId> {
        self.api.import_capability(data)
    }

    /// Get metadata for a capability.
    pub fn get_metadata(&self, id: CapabilityId) -> CapabilityResult<CapabilityMetadata> {
        self.api.get_metadata(id)
    }

    /// Get execution record by ID.
    pub fn get_execution_record(&self, execution_id: Uuid) -> Option<ExecutionRecord> {
        self.api.get_execution_record(execution_id)
    }

    /// List all execution records.
    pub fn list_executions(&self) -> Vec<ExecutionRecord> {
        self.api.list_executions()
    }

    /// Get active execution IDs.
    pub fn active_executions(&self) -> Vec<Uuid> {
        self.api.active_executions()
    }

    /// Cancel an execution.
    pub fn cancel_execution(&self, execution_id: Uuid) -> bool {
        self.api.cancel_execution(execution_id)
    }

    /// Get total capability count.
    pub fn capability_count(&self) -> usize {
        self.api.capability_count()
    }

    /// Get enabled capability count.
    pub fn enabled_count(&self) -> usize {
        self.api.enabled_count()
    }

    /// Get a reference to the underlying API.
    pub fn api(&self) -> &Arc<CapabilityApi> {
        &self.api
    }

    /// Get a summary of the registry state.
    pub fn registry_summary(&self) -> RegistrySummary {
        let all = self.list();
        let mut by_category: HashMap<String, usize> = HashMap::new();
        let mut by_state: HashMap<String, usize> = HashMap::new();
        let mut by_namespace: HashMap<String, usize> = HashMap::new();

        for cap in &all {
            *by_category
                .entry(cap.category.to_string())
                .or_insert(0) += 1;
            *by_state
                .entry(cap.state.to_string())
                .or_insert(0) += 1;
            *by_namespace
                .entry(cap.namespace.to_string())
                .or_insert(0) += 1;
        }

        RegistrySummary {
            total_capabilities: all.len(),
            enabled_capabilities: all.iter().filter(|c| c.state == CapabilityState::Enabled).count(),
            by_category,
            by_state,
            by_namespace,
            total_executions: all.iter().map(|c| c.execution_count).sum(),
        }
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of the registry state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySummary {
    pub total_capabilities: usize,
    pub enabled_capabilities: usize,
    pub by_category: HashMap<String, usize>,
    pub by_state: HashMap<String, usize>,
    pub by_namespace: HashMap<String, usize>,
    pub total_executions: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_creation() {
        let registry = CapabilityRegistry::new();
        assert_eq!(registry.capability_count(), 0);
    }

    #[test]
    fn registry_summary_empty() {
        let registry = CapabilityRegistry::new();
        let summary = registry.registry_summary();
        assert_eq!(summary.total_capabilities, 0);
        assert_eq!(summary.enabled_capabilities, 0);
    }

    #[test]
    fn registry_list_empty() {
        let registry = CapabilityRegistry::new();
        assert!(registry.list().is_empty());
    }

    #[test]
    fn registry_search_empty() {
        let registry = CapabilityRegistry::new();
        assert!(registry.search("test").is_empty());
    }

    #[test]
    fn registry_default() {
        let registry = CapabilityRegistry::default();
        assert_eq!(registry.capability_count(), 0);
    }
}
