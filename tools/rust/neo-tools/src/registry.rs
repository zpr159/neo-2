//! Tool registry, manager, catalog, and related infrastructure.

use dashmap::DashMap;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::{ToolError, ToolResult};
use crate::lifecycle::ToolLifecycleState;
use crate::tool::DynamicTool;
use crate::types::{
    ToolCategory, ToolId, ToolManifest, ToolMetadata, ToolType, ToolVersion,
};

// ---------------------------------------------------------------------------
// ToolRegistry — central concurrent registry
// ---------------------------------------------------------------------------

/// Central concurrent registry for all tools.
pub struct ToolRegistry {
    tools: DashMap<String, Arc<RwLock<DynamicTool>>>,
    by_id: DashMap<ToolId, String>,
    by_type: DashMap<ToolType, HashSet<String>>,
    by_category: DashMap<ToolCategory, HashSet<String>>,
    by_tag: DashMap<String, HashSet<String>>,
}

impl std::fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolRegistry")
            .field("tool_count", &self.tools.len())
            .finish()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: DashMap::new(),
            by_id: DashMap::new(),
            by_type: DashMap::new(),
            by_category: DashMap::new(),
            by_tag: DashMap::new(),
        }
    }

    /// Register a tool. Returns the tool's name on success.
    pub async fn register(&self, tool: DynamicTool) -> ToolResult<String> {
        let name = tool.name().to_string();
        let tool_type = tool.manifest.metadata.tool_type.clone();
        let category = tool.manifest.metadata.category.clone();
        let tags: Vec<String> = tool.manifest.metadata.tags.clone();
        let tool_id = ToolId::new();

        if self.tools.contains_key(&name) {
            return Err(ToolError::already_exists(format!(
                "tool '{name}' is already registered"
            )));
        }

        let mut tool = tool;
        tool.lifecycle.force_transition(ToolLifecycleState::Loading);
        tool.lifecycle.force_transition(ToolLifecycleState::Loaded);
        tool.lifecycle
            .force_transition(ToolLifecycleState::Initializing);
        tool.lifecycle.force_transition(ToolLifecycleState::Ready);

        let arc_tool = Arc::new(RwLock::new(tool));

        self.tools.insert(name.clone(), Arc::clone(&arc_tool));
        self.by_id.insert(tool_id, name.clone());

        self.by_type
            .entry(tool_type)
            .or_default()
            .insert(name.clone());

        self.by_category
            .entry(category)
            .or_default()
            .insert(name.clone());

        for tag in &tags {
            self.by_tag
                .entry(tag.clone())
                .or_default()
                .insert(name.clone());
        }

        tracing::info!(tool = %name, "tool registered");
        Ok(name)
    }

    /// Unregister a tool by name.
    pub async fn unregister(&self, name: &str) -> ToolResult<()> {
        let tool_arc = self
            .tools
            .remove(name)
            .ok_or_else(|| ToolError::not_found(format!("tool '{name}' not found")))?
            .1;

        let tool = tool_arc.read().await;
        let tool_type = tool.manifest.metadata.tool_type.clone();
        let category = tool.manifest.metadata.category.clone();
        let tags: Vec<String> = tool.manifest.metadata.tags.clone();
        drop(tool);

        let id_to_remove = self
            .by_id
            .iter()
            .find(|entry| entry.value() == name)
            .map(|entry| *entry.key());
        if let Some(id) = id_to_remove {
            self.by_id.remove(&id);
        }
        if let Some(mut set) = self.by_type.get_mut(&tool_type) {
            set.remove(name);
        }
        if let Some(mut set) = self.by_category.get_mut(&category) {
            set.remove(name);
        }
        for tag in &tags {
            if let Some(mut set) = self.by_tag.get_mut(tag) {
                set.remove(name);
            }
        }

        tracing::info!(tool = %name, "tool unregistered");
        Ok(())
    }

    /// Get a tool by name.
    pub async fn get(&self, name: &str) -> ToolResult<Arc<RwLock<DynamicTool>>> {
        self.tools
            .get(name)
            .map(|entry| Arc::clone(entry.value()))
            .ok_or_else(|| ToolError::not_found(format!("tool '{name}' not found")))
    }

    /// Check if a tool is registered.
    pub async fn contains(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Enable a tool.
    pub async fn enable(&self, name: &str) -> ToolResult<()> {
        let tool = self.get(name).await?;
        let mut t = tool.write().await;
        t.manifest.config.enabled = true;
        tracing::info!(tool = %name, "tool enabled");
        Ok(())
    }

    /// Disable a tool.
    pub async fn disable(&self, name: &str) -> ToolResult<()> {
        let tool = self.get(name).await?;
        let mut t = tool.write().await;
        t.manifest.config.enabled = false;
        tracing::info!(tool = %name, "tool disabled");
        Ok(())
    }

    /// Transition a tool's lifecycle state.
    pub async fn transition(&self, name: &str, target: ToolLifecycleState) -> ToolResult<()> {
        let tool = self.get(name).await?;
        let mut t = tool.write().await;
        t.lifecycle.transition(target)?;
        tracing::debug!(tool = %name, state = %t.lifecycle.current(), "tool state transitioned");
        Ok(())
    }

    /// List all registered tool names.
    pub async fn list_names(&self) -> Vec<String> {
        self.tools.iter().map(|entry| entry.key().clone()).collect()
    }

    /// List tools by type.
    pub async fn list_by_type(&self, tool_type: &ToolType) -> Vec<String> {
        self.by_type
            .get(tool_type)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// List tools by category.
    pub async fn list_by_category(&self, category: &ToolCategory) -> Vec<String> {
        self.by_category
            .get(category)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// List tools by tag.
    pub async fn list_by_tag(&self, tag: &str) -> Vec<String> {
        self.by_tag
            .get(tag)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Search tools by name/description query.
    pub async fn search(&self, query: &str) -> Vec<String> {
        let q = query.to_lowercase();
        self.tools
            .iter()
            .filter_map(|entry| {
                let name = entry.key().clone();
                let tool = entry.value();
                let meta = tool.try_read().ok()?;
                let m = &meta.manifest.metadata;
                if m.name.to_lowercase().contains(&q)
                    || m.description.to_lowercase().contains(&q)
                    || m.display_name.to_lowercase().contains(&q)
                {
                    Some(name)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get metadata for a tool.
    pub async fn metadata(&self, name: &str) -> ToolResult<ToolMetadata> {
        let tool = self.get(name).await?;
        let t = tool.read().await;
        Ok(t.manifest.metadata.clone())
    }

    /// Get the manifest for a tool.
    pub async fn manifest(&self, name: &str) -> ToolResult<ToolManifest> {
        let tool = self.get(name).await?;
        let t = tool.read().await;
        Ok(t.manifest.clone())
    }

    /// Get the current state of a tool.
    pub async fn state(&self, name: &str) -> ToolResult<ToolLifecycleState> {
        let tool = self.get(name).await?;
        let t = tool.read().await;
        Ok(t.lifecycle.current())
    }

    /// Count of registered tools.
    pub async fn count(&self) -> usize {
        self.tools.len()
    }

    /// Get all tool names and their states as a snapshot.
    pub async fn snapshot(&self) -> Vec<(String, ToolLifecycleState, ToolVersion)> {
        let mut result = Vec::new();
        for entry in self.tools.iter() {
            let t = entry.value().read().await;
            result.push((
                entry.key().clone(),
                t.lifecycle.current(),
                t.manifest.metadata.version.clone(),
            ));
        }
        result
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// ToolManager — high-level orchestration
// ---------------------------------------------------------------------------

/// High-level tool lifecycle manager.
pub struct ToolManager {
    registry: Arc<ToolRegistry>,
}

impl std::fmt::Debug for ToolManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolManager").finish()
    }
}

impl ToolManager {
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        Self { registry }
    }

    /// Full lifecycle: register → load → initialize → ready.
    pub async fn activate_tool(&self, mut tool: DynamicTool) -> ToolResult<()> {
        let name = tool.name().to_string();

        tool.lifecycle.transition(ToolLifecycleState::Loading)?;
        self.registry.register(tool).await?;

        let tool_arc = self.registry.get(&name).await?;
        {
            let t = tool_arc.read().await;
            if t.lifecycle.current() != ToolLifecycleState::Ready {
                return Err(ToolError::lifecycle_violation(format!(
                    "tool '{name}' is not in Ready state after activation"
                )));
            }
        }

        tracing::info!(tool = %name, "tool activated");
        Ok(())
    }

    /// Full lifecycle: stop → unload → unregister.
    pub async fn deactivate_tool(&self, name: &str) -> ToolResult<()> {
        {
            let tool_arc = self.registry.get(name).await?;
            let mut t = tool_arc.write().await;
            if t.lifecycle.current().can_execute() {
                t.lifecycle.transition(ToolLifecycleState::Stopping)?;
            }
            t.lifecycle.transition(ToolLifecycleState::Stopped)?;
        }

        self.registry.unregister(name).await?;
        tracing::info!(tool = %name, "tool deactivated");
        Ok(())
    }

    /// Health-check all tools.
    pub async fn health_check_all(&self) -> Vec<crate::types::ToolHealth> {
        let names = self.registry.list_names().await;
        let mut results = Vec::new();
        for name in &names {
            if let Ok(tool_arc) = self.registry.get(name).await {
                let t = tool_arc.read().await;
                let health = if let Some(ref health_fn) = t.health_fn {
                    health_fn().await
                } else {
                    crate::types::ToolHealth::healthy(crate::types::ToolId(uuid::Uuid::nil()))
                };
                results.push(health);
            }
        }
        results
    }

    pub fn registry(&self) -> &Arc<ToolRegistry> {
        &self.registry
    }
}

// ---------------------------------------------------------------------------
// ToolCatalog — searchable index
// ---------------------------------------------------------------------------

/// Read-only searchable catalog of tool metadata.
pub struct ToolCatalog {
    entries: DashMap<String, ToolMetadata>,
}

impl std::fmt::Debug for ToolCatalog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolCatalog")
            .field("entries", &self.entries.len())
            .finish()
    }
}

impl ToolCatalog {
    pub fn new() -> Self {
        Self {
            entries: DashMap::new(),
        }
    }

    pub async fn index(&self, registry: &ToolRegistry) -> ToolResult<()> {
        self.entries.clear();
        for name in registry.list_names().await {
            if let Ok(meta) = registry.metadata(&name).await {
                self.entries.insert(name, meta);
            }
        }
        Ok(())
    }

    pub fn search(&self, query: &str) -> Vec<ToolMetadata> {
        let q = query.to_lowercase();
        self.entries
            .iter()
            .filter(|entry| {
                let m = entry.value();
                m.name.to_lowercase().contains(&q)
                    || m.description.to_lowercase().contains(&q)
                    || m.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn by_type(&self, tool_type: &ToolType) -> Vec<ToolMetadata> {
        self.entries
            .iter()
            .filter(|entry| &entry.value().tool_type == tool_type)
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn by_category(&self, category: &ToolCategory) -> Vec<ToolMetadata> {
        self.entries
            .iter()
            .filter(|entry| &entry.value().category == category)
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn by_tag(&self, tag: &str) -> Vec<ToolMetadata> {
        self.entries
            .iter()
            .filter(|entry| entry.value().tags.iter().any(|t| t == tag))
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }
}

impl Default for ToolCatalog {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// ToolFactory — creates tools from manifests
// ---------------------------------------------------------------------------

/// Factory for creating tool instances from manifests.
pub struct ToolFactory {
    builders: DashMap<String, Box<dyn Fn() -> ToolResult<DynamicTool> + Send + Sync>>,
}

impl std::fmt::Debug for ToolFactory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolFactory")
            .field("builder_count", &self.builders.len())
            .finish()
    }
}

impl ToolFactory {
    pub fn new() -> Self {
        Self {
            builders: DashMap::new(),
        }
    }

    pub fn register_builder<F>(&self, name: impl Into<String>, builder: F)
    where
        F: Fn() -> ToolResult<DynamicTool> + Send + Sync + 'static,
    {
        self.builders.insert(name.into(), Box::new(builder));
    }

    pub fn create(&self, name: &str) -> ToolResult<DynamicTool> {
        let builder = self
            .builders
            .get(name)
            .ok_or_else(|| ToolError::not_found(format!("no builder for tool '{name}'")))?;
        builder()
    }

    pub fn has_builder(&self, name: &str) -> bool {
        self.builders.contains_key(name)
    }
}

impl Default for ToolFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::ToolBuilder;
    use crate::types::ToolVersion;

    fn make_test_tool(name: &str) -> DynamicTool {
        ToolBuilder::new(
            name,
            ToolVersion::new(1, 0, 0),
            "Test tool",
            ToolType::Custom("test".into()),
            ToolCategory::Execute,
        )
        .on_execute(|params, _ctx| Box::pin(async move { Ok(params) }))
        .build()
        .unwrap()
    }

    #[tokio::test]
    async fn test_register_and_get() {
        let registry = ToolRegistry::new();
        let tool = make_test_tool("test_tool");
        registry.register(tool).await.unwrap();

        assert!(registry.contains("test_tool").await);
        assert_eq!(registry.count().await, 1);
    }

    #[tokio::test]
    async fn test_unregister() {
        let registry = ToolRegistry::new();
        let tool = make_test_tool("test_tool");
        registry.register(tool).await.unwrap();

        registry.unregister("test_tool").await.unwrap();
        assert!(!registry.contains("test_tool").await);
        assert_eq!(registry.count().await, 0);
    }

    #[tokio::test]
    async fn test_duplicate_registration() {
        let registry = ToolRegistry::new();
        let tool1 = make_test_tool("dup_tool");
        let tool2 = make_test_tool("dup_tool");
        registry.register(tool1).await.unwrap();
        assert!(registry.register(tool2).await.is_err());
    }

    #[tokio::test]
    async fn test_search() {
        let registry = ToolRegistry::new();
        registry
            .register(make_test_tool("http_client"))
            .await
            .unwrap();
        registry
            .register(make_test_tool("file_reader"))
            .await
            .unwrap();

        let results = registry.search("http").await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], "http_client");
    }

    #[tokio::test]
    async fn test_enable_disable() {
        let registry = ToolRegistry::new();
        registry
            .register(make_test_tool("toggle_tool"))
            .await
            .unwrap();

        registry.disable("toggle_tool").await.unwrap();
        let tool = registry.get("toggle_tool").await.unwrap();
        assert!(!tool.read().await.manifest.config.enabled);

        registry.enable("toggle_tool").await.unwrap();
        let tool = registry.get("toggle_tool").await.unwrap();
        assert!(tool.read().await.manifest.config.enabled);
    }
}
