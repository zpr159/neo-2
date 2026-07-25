//! SDK builders for fluent tool construction.

use std::sync::Arc;

use crate::error::ToolResult;
use crate::executor::ToolExecutor;
use crate::registry::ToolRegistry;
use crate::tool::{DynamicTool, ToolBuilder};
use crate::types::{
    ToolCategory, ToolConfiguration, ToolContext, ToolType, ToolVersion,
};

/// Fluent SDK for building tools.
pub struct ToolSdk;

impl std::fmt::Debug for ToolSdk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolSdk").finish()
    }
}

impl ToolSdk {
    /// Create a new tool builder.
    pub fn tool(
        name: impl Into<String>,
        version: ToolVersion,
        description: impl Into<String>,
        tool_type: ToolType,
        category: ToolCategory,
    ) -> ToolBuilder {
        ToolBuilder::new(name, version, description, tool_type, category)
    }

    /// Create a new registry.
    pub fn registry() -> ToolRegistry {
        ToolRegistry::new()
    }

    /// Create a new executor for a registry.
    pub fn executor(registry: Arc<ToolRegistry>, max_concurrent: usize) -> ToolExecutor {
        ToolExecutor::new(registry, max_concurrent)
    }

    /// Create a new executor builder.
    pub fn executor_builder() -> crate::executor::ToolExecutorBuilder {
        crate::executor::ToolExecutorBuilder::new()
    }
}

/// Pre-built tool configurations for common patterns.
pub struct ToolPresets;

impl std::fmt::Debug for ToolPresets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolPresets").finish()
    }
}

impl ToolPresets {
    /// Create a read-only filesystem tool.
    pub fn readonly_filesystem() -> ToolResult<DynamicTool> {
        use crate::types::ToolVersion;
        ToolBuilder::new(
            "readonly_fs",
            ToolVersion::new(1, 0, 0),
            "Read-only filesystem access",
            ToolType::Filesystem,
            ToolCategory::Read,
        )
        .with_config(
            ToolConfiguration::enabled()
                .with_sandboxed(true)
                .with_priority(100),
        )
        .on_execute(|params, _ctx| Box::pin(async move { Ok(params) }))
        .build()
    }

    /// Create a safe shell tool with restricted commands.
    pub fn safe_shell() -> ToolResult<DynamicTool> {
        use crate::types::ToolVersion;
        ToolBuilder::new(
            "safe_shell",
            ToolVersion::new(1, 0, 0),
            "Safe shell with restricted command set",
            ToolType::Shell,
            ToolCategory::Execute,
        )
        .with_config(
            ToolConfiguration::enabled()
                .with_sandboxed(true)
                .with_priority(50),
        )
        .requiring_permission()
        .on_execute(|params, _ctx| Box::pin(async move { Ok(params) }))
        .build()
    }

    /// Create a monitored HTTP client.
    pub fn monitored_http() -> ToolResult<DynamicTool> {
        use crate::types::ToolVersion;
        ToolBuilder::new(
            "monitored_http",
            ToolVersion::new(1, 0, 0),
            "HTTP client with request/response monitoring",
            ToolType::HttpClient,
            ToolCategory::Execute,
        )
        .with_config(ToolConfiguration::enabled().with_max_concurrent(20))
        .requiring_permission()
        .on_execute(|params, _ctx| Box::pin(async move { Ok(params) }))
        .build()
    }
}

/// Batch tool registration helper.
pub struct ToolBatchRegistrar<'a> {
    registry: &'a ToolRegistry,
    tools: Vec<DynamicTool>,
}

impl<'a> std::fmt::Debug for ToolBatchRegistrar<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolBatchRegistrar")
            .field("pending_count", &self.tools.len())
            .finish()
    }
}

impl<'a> ToolBatchRegistrar<'a> {
    pub fn new(registry: &'a ToolRegistry) -> Self {
        Self {
            registry,
            tools: Vec::new(),
        }
    }

    pub fn add(mut self, tool: DynamicTool) -> Self {
        self.tools.push(tool);
        self
    }

    pub async fn register_all(self) -> ToolResult<Vec<String>> {
        let mut names = Vec::new();
        for tool in self.tools {
            let name = self.registry.register(tool).await?;
            names.push(name);
        }
        Ok(names)
    }
}

/// Convenience traits for tool operations.
pub trait ToolExt {
    /// Execute this tool with default context.
    fn execute_simple(
        &self,
        operation: &str,
        params: serde_json::Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToolResult<serde_json::Value>> + Send>>;
}

impl ToolExt for DynamicTool {
    fn execute_simple(
        &self,
        _operation: &str,
        params: serde_json::Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToolResult<serde_json::Value>> + Send>>
    {
        let ctx = ToolContext::new("sdk", crate::types::CallerType::Internal);
        let fn_ptr = Arc::clone(&self.execute_fn);
        Box::pin(async move { (fn_ptr)(params, ctx).await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sdk_builder() {
        let tool = ToolSdk::tool(
            "my_tool",
            ToolVersion::new(1, 0, 0),
            "A custom tool",
            ToolType::Custom("custom".into()),
            ToolCategory::Execute,
        )
        .author("test")
        .on_execute(|params, _ctx| Box::pin(async move { Ok(params) }))
        .build();

        assert!(tool.is_ok());
    }

    #[test]
    fn test_presets() {
        assert!(ToolPresets::readonly_filesystem().is_ok());
        assert!(ToolPresets::safe_shell().is_ok());
        assert!(ToolPresets::monitored_http().is_ok());
    }
}
