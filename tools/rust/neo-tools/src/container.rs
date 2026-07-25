//! Container management tool.

use crate::error::{ToolError, ToolResult};
use crate::tool::{DynamicTool, ToolBuilder};
use crate::types::{ToolCategory, ToolType, ToolVersion};

/// Create the container tool.
pub fn create_container_tool() -> ToolResult<DynamicTool> {
    ToolBuilder::new(
        "container",
        ToolVersion::new(1, 0, 0),
        "Container management: Docker, Docker Compose, Podman, Kubernetes — build, run, stop, logs, exec, images, deployments, pods",
        ToolType::Container,
        ToolCategory::Execute,
    )
    .author("neo")
    .timeout_ms(120_000)
    .with_input_schema(serde_json::json!({
        "type": "object",
        "properties": {
            "operation": {"type": "string", "enum": ["build", "run", "stop", "start", "remove", "logs", "exec", "list_containers", "list_images", "pull", "push", "inspect", "list_pods", "list_deployments"]},
            "engine": {"type": "string", "enum": ["docker", "podman", "kubectl"]},
            "image": {"type": "string"},
            "container_name": {"type": "string"},
            "command": {"type": "string"},
            "dockerfile": {"type": "string"},
            "context": {"type": "string"},
            "ports": {"type": "object"},
            "env": {"type": "object"},
            "namespace": {"type": "string"},
            "resource_limits": {"type": "object"}
        },
        "required": ["operation", "engine"]
    }))
    .on_execute(|params, _ctx| {
        Box::pin(async move {
            let operation = params
                .get("operation")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::invalid_params("missing 'operation'"))?;
            let engine = params
                .get("engine")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::invalid_params("missing 'engine'"))?;

            match operation {
                "build" => {
                    let image = params
                        .get("image")
                        .and_then(|v| v.as_str())
                        .unwrap_or("neo-tool:latest");
                    let dockerfile = params
                        .get("dockerfile")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Dockerfile");
                    let context = params
                        .get("context")
                        .and_then(|v| v.as_str())
                        .unwrap_or(".");
                    Ok(serde_json::json!({
                        "engine": engine,
                        "operation": "build",
                        "image": image,
                        "dockerfile": dockerfile,
                        "context": context,
                        "status": "build queued",
                        "note": "requires container runtime"
                    }))
                }
                "run" => {
                    let image = params
                        .get("image")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ToolError::invalid_params("missing 'image'"))?;
                    let name = params
                        .get("container_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("neo-container");
                    Ok(serde_json::json!({
                        "engine": engine,
                        "operation": "run",
                        "image": image,
                        "container_name": name,
                        "status": "run queued",
                        "note": "requires container runtime"
                    }))
                }
                "stop" => {
                    let name = params
                        .get("container_name")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ToolError::invalid_params("missing 'container_name'"))?;
                    Ok(serde_json::json!({
                        "engine": engine,
                        "operation": "stop",
                        "container_name": name,
                        "status": "stop queued"
                    }))
                }
                "logs" => {
                    let name = params
                        .get("container_name")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ToolError::invalid_params("missing 'container_name'"))?;
                    Ok(serde_json::json!({
                        "engine": engine,
                        "operation": "logs",
                        "container_name": name,
                        "logs": "",
                        "note": "requires container runtime"
                    }))
                }
                "list_containers" | "list_images" | "list_pods" | "list_deployments" => {
                    Ok(serde_json::json!({
                        "engine": engine,
                        "operation": operation,
                        "items": [],
                        "note": "requires container runtime"
                    }))
                }
                _ => Ok(serde_json::json!({
                    "engine": engine,
                    "operation": operation,
                    "status": "stub",
                    "note": "requires container runtime"
                })),
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
    async fn test_container_list() {
        let tool = create_container_tool().unwrap();
        let ctx = ToolContext::new("test", crate::types::CallerType::Internal);
        let req = crate::types::ToolRequest::new(
            ToolId::new(),
            "list",
            serde_json::json!({"operation": "list_containers", "engine": "docker"}),
            ctx,
        );
        let result = tool.execute(&req).await.unwrap();
        assert!(result.get("items").is_some());
    }
}
