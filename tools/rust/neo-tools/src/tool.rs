//! The core `Tool` trait and concrete implementations.

use async_trait::async_trait;
use std::fmt;
use std::sync::Arc;

use crate::error::ToolResult;
use crate::lifecycle::{LifecycleTracker, ToolLifecycleState};
use crate::types::{
    ExecuteFn, HealthCheckFn, ToolConfiguration, ToolContext, ToolHealth,
    ToolManifest, ToolMetadata, ToolMetrics, ToolRequest, ToolResponse, ToolVersion,
};

/// The core trait that all tools must implement.
///
/// Tools are the fundamental building blocks for system interaction in Neo.
/// They provide a uniform interface for executing operations across diverse
/// subsystems (filesystem, shell, browser, HTTP, database, etc.).
#[async_trait]
pub trait Tool: Send + Sync {
    /// Returns the tool's metadata.
    fn metadata(&self) -> &ToolMetadata;

    /// Returns a mutable reference to the tool's metadata.
    fn metadata_mut(&mut self) -> &mut ToolMetadata;

    /// Returns the current lifecycle state.
    fn state(&self) -> ToolLifecycleState;

    /// Returns the tool's configuration.
    fn config(&self) -> &ToolConfiguration;

    /// Returns the tool's execution metrics.
    fn metrics(&self) -> &ToolMetrics;

    /// Execute a request against this tool.
    async fn execute(&self, request: &ToolRequest) -> ToolResult<ToolResponse>;

    /// Validate that the given parameters are acceptable.
    fn validate_input(&self, _operation: &str, _params: &serde_json::Value) -> ToolResult<()> {
        Ok(())
    }

    /// Health check for this tool.
    async fn health_check(&self) -> ToolHealth {
        ToolHealth::healthy(self.id())
    }

    /// Called when the tool is registered in the registry.
    async fn on_register(&mut self) -> ToolResult<()> {
        Ok(())
    }

    /// Called when the tool is loaded.
    async fn on_load(&mut self) -> ToolResult<()> {
        Ok(())
    }

    /// Called when the tool is initialized.
    async fn on_initialize(&mut self) -> ToolResult<()> {
        Ok(())
    }

    /// Called when the tool is started.
    async fn on_start(&mut self) -> ToolResult<()> {
        Ok(())
    }

    /// Called when the tool is stopped.
    async fn on_stop(&mut self) -> ToolResult<()> {
        Ok(())
    }

    /// Called when the tool is disabled.
    async fn on_disable(&mut self) -> ToolResult<()> {
        Ok(())
    }

    /// Called when the tool is unloaded.
    async fn on_unload(&mut self) -> ToolResult<()> {
        Ok(())
    }

    /// Returns the tool's unique ID (shortcut for metadata).
    fn id(&self) -> crate::types::ToolId {
        crate::types::ToolId(uuid::Uuid::nil())
    }

    /// Whether this tool requires permission checks.
    fn requires_permission(&self) -> bool {
        self.metadata().requires_permission
    }

    /// The permissions required to use this tool.
    fn required_permissions(&self) -> Vec<String> {
        Vec::new()
    }

    /// Estimated execution time in milliseconds.
    fn estimated_duration_ms(&self) -> Option<u64> {
        self.metadata().timeout_ms
    }
}

// ---------------------------------------------------------------------------
// DynamicTool — a trait-object-friendly wrapper
// ---------------------------------------------------------------------------

/// A type-erased tool object stored in the registry.
pub struct DynamicTool {
    pub manifest: ToolManifest,
    pub lifecycle: LifecycleTracker,
    pub metrics: ToolMetrics,
    pub execute_fn: ExecuteFn,
    pub health_fn: Option<HealthCheckFn>,
}

impl fmt::Debug for DynamicTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynamicTool")
            .field("manifest", &self.manifest)
            .field("lifecycle", &self.lifecycle)
            .field("metrics", &self.metrics)
            .field("execute_fn", &"<fn>")
            .field("health_fn", &self.health_fn.as_ref().map(|_| "<fn>"))
            .finish()
    }
}

impl DynamicTool {
    pub fn new(manifest: ToolManifest, execute_fn: ExecuteFn) -> Self {
        Self {
            manifest,
            lifecycle: LifecycleTracker::new(ToolLifecycleState::Registered),
            metrics: ToolMetrics::default(),
            execute_fn,
            health_fn: None,
        }
    }

    pub fn with_health_check(mut self, health_fn: HealthCheckFn) -> Self {
        self.health_fn = Some(health_fn);
        self
    }

    pub fn tool_id(&self) -> crate::types::ToolId {
        crate::types::ToolId(uuid::Uuid::new_v4())
    }

    pub fn name(&self) -> &str {
        &self.manifest.metadata.name
    }

    pub fn version(&self) -> &ToolVersion {
        &self.manifest.metadata.version
    }

    pub fn state(&self) -> ToolLifecycleState {
        self.lifecycle.current()
    }

    pub fn is_executable(&self) -> bool {
        self.lifecycle.current().can_execute() && self.manifest.config.enabled
    }

    /// Execute a request against this tool.
    pub async fn execute(&self, request: &ToolRequest) -> ToolResult<serde_json::Value> {
        (self.execute_fn)(request.parameters.clone(), request.context.clone()).await
    }
}

// ---------------------------------------------------------------------------
// ToolBuilder — fluent builder for DynamicTool
// ---------------------------------------------------------------------------

/// Fluent builder for constructing tools.
pub struct ToolBuilder {
    manifest: ToolManifest,
    execute_fn: Option<ExecuteFn>,
    health_fn: Option<HealthCheckFn>,
}

impl fmt::Debug for ToolBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ToolBuilder")
            .field("manifest", &self.manifest)
            .field("execute_fn", &self.execute_fn.as_ref().map(|_| "<fn>"))
            .field("health_fn", &self.health_fn.as_ref().map(|_| "<fn>"))
            .finish()
    }
}

impl ToolBuilder {
    pub fn new(
        name: impl Into<String>,
        version: ToolVersion,
        description: impl Into<String>,
        tool_type: crate::types::ToolType,
        category: crate::types::ToolCategory,
    ) -> Self {
        let metadata = ToolMetadata::new(name, description, tool_type, category, version);
        let config = ToolConfiguration::enabled();
        Self {
            manifest: ToolManifest::new(metadata, config),
            execute_fn: None,
            health_fn: None,
        }
    }

    pub fn display_name(mut self, name: impl Into<String>) -> Self {
        self.manifest.metadata.display_name = name.into();
        self
    }

    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.manifest.metadata.author = author.into();
        self
    }

    pub fn license(mut self, license: impl Into<String>) -> Self {
        self.manifest.metadata.license = Some(license.into());
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.manifest.metadata.tags.push(tag.into());
        self
    }

    pub fn timeout_ms(mut self, ms: u64) -> Self {
        self.manifest.metadata.timeout_ms = Some(ms);
        self
    }

    pub fn max_retries(mut self, retries: u32) -> Self {
        self.manifest.metadata.max_retries = retries;
        self
    }

    pub fn requiring_permission(mut self) -> Self {
        self.manifest.metadata.requires_permission = true;
        self
    }

    pub fn with_input_schema(mut self, schema: serde_json::Value) -> Self {
        self.manifest.metadata.input_schema = schema;
        self
    }

    pub fn with_output_schema(mut self, schema: serde_json::Value) -> Self {
        self.manifest.metadata.output_schema = Some(schema);
        self
    }

    pub fn with_config(mut self, config: ToolConfiguration) -> Self {
        self.manifest.config = config;
        self
    }

    pub fn dependency(mut self, name: impl Into<String>, version: ToolVersion) -> Self {
        self.manifest
            .dependencies
            .push(crate::types::ToolDependency::required(name, version));
        self
    }

    pub fn permission(mut self, perm: impl Into<String>) -> Self {
        self.manifest.permissions.push(perm.into());
        self
    }

    pub fn on_execute<F>(mut self, f: F) -> Self
    where
        F: Fn(
                serde_json::Value,
                ToolContext,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = ToolResult<serde_json::Value>> + Send>,
            > + Send
            + Sync
            + 'static,
    {
        self.execute_fn = Some(Arc::new(f));
        self
    }

    pub fn on_health_check<F>(mut self, f: F) -> Self
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = ToolHealth> + Send>>
            + Send
            + Sync
            + 'static,
    {
        self.health_fn = Some(Arc::new(f));
        self
    }

    pub fn build(self) -> ToolResult<DynamicTool> {
        let execute_fn = self
            .execute_fn
            .ok_or_else(|| crate::error::ToolError::config("on_execute handler is required"))?;

        let mut tool = DynamicTool::new(self.manifest, execute_fn);
        if let Some(health_fn) = self.health_fn {
            tool.health_fn = Some(health_fn);
        }
        Ok(tool)
    }
}
