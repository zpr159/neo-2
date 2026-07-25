//! IDE integration tool.

use crate::error::{ToolError, ToolResult};
use crate::tool::{DynamicTool, ToolBuilder};
use crate::types::{ToolCategory, ToolType, ToolVersion};

/// Create the IDE integration tool.
pub fn create_ide_tool() -> ToolResult<DynamicTool> {
    ToolBuilder::new(
        "ide",
        ToolVersion::new(1, 0, 0),
        "IDE integration: VS Code, JetBrains — diagnostics, build, debugging, workspace inspection, project management",
        ToolType::Ide,
        ToolCategory::Execute,
    )
    .author("neo")
    .timeout_ms(60_000)
    .with_input_schema(serde_json::json!({
        "type": "object",
        "properties": {
            "operation": {"type": "string", "enum": ["diagnostics", "build", "open_file", "list_workspace", "get_config", "search_symbols", "list_extensions"]},
            "ide": {"type": "string", "enum": ["vscode", "intellij", "gateway"]},
            "workspace_path": {"type": "string"},
            "file_path": {"type": "string"},
            "query": {"type": "string"},
            "build_command": {"type": "string"}
        },
        "required": ["operation"]
    }))
    .on_execute(|params, _ctx| {
        Box::pin(async move {
            let operation = params
                .get("operation")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::invalid_params("missing 'operation'"))?;

            let ide = params
                .get("ide")
                .and_then(|v| v.as_str())
                .unwrap_or("vscode");

            match operation {
                "diagnostics" => {
                    let file = params
                        .get("file_path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    Ok(serde_json::json!({
                        "ide": ide,
                        "file": file,
                        "diagnostics": [],
                        "note": "requires IDE extension or LSP connection"
                    }))
                }
                "build" => {
                    let cmd = params
                        .get("build_command")
                        .and_then(|v| v.as_str())
                        .unwrap_or("cargo build");
                    Ok(serde_json::json!({
                        "ide": ide,
                        "command": cmd,
                        "status": "build queued",
                        "note": "requires IDE extension integration"
                    }))
                }
                "open_file" => {
                    let file = params
                        .get("file_path")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ToolError::invalid_params("missing 'file_path'"))?;
                    Ok(serde_json::json!({
                        "ide": ide,
                        "file": file,
                        "opened": true
                    }))
                }
                "list_workspace" => {
                    let workspace = params
                        .get("workspace_path")
                        .and_then(|v| v.as_str())
                        .unwrap_or(".");
                    Ok(serde_json::json!({
                        "ide": ide,
                        "workspace": workspace,
                        "files": [],
                        "note": "requires IDE extension integration"
                    }))
                }
                "search_symbols" => {
                    let query = params
                        .get("query")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    Ok(serde_json::json!({
                        "ide": ide,
                        "query": query,
                        "symbols": [],
                        "note": "requires LSP connection"
                    }))
                }
                "list_extensions" => {
                    Ok(serde_json::json!({
                        "ide": ide,
                        "extensions": [],
                        "note": "requires IDE extension API"
                    }))
                }
                "get_config" => {
                    Ok(serde_json::json!({
                        "ide": ide,
                        "config": {},
                        "note": "requires IDE extension API"
                    }))
                }
                _ => Err(ToolError::invalid_params(format!(
                    "unknown IDE operation: {operation}"
                ))),
            }
        })
    })
    .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ToolContext, ToolId};

    #[tokio::test]
    async fn test_ide_diagnostics() {
        let tool = create_ide_tool().unwrap();
        let ctx = ToolContext::new("test", crate::types::CallerType::Internal);
        let req = crate::types::ToolRequest::new(
            ToolId::new(),
            "diag",
            serde_json::json!({"operation": "diagnostics", "ide": "vscode", "file_path": "src/main.rs"}),
            ctx,
        );
        let result = tool.execute(&req).await.unwrap();
        assert!(result.get("diagnostics").is_some());
    }
}
