//! Cloud service tool with multi-provider support.

use crate::error::{ToolError, ToolResult};
use crate::tool::{DynamicTool, ToolBuilder};
use crate::types::{ToolCategory, ToolType, ToolVersion};

/// Create the cloud tool.
pub fn create_cloud_tool() -> ToolResult<DynamicTool> {
    ToolBuilder::new(
        "cloud",
        ToolVersion::new(1, 0, 0),
        "Cloud services: AWS, Azure, GCP — S3, Blob Storage, Cloud Storage, Secrets Managers, Cloud Functions",
        ToolType::Cloud,
        ToolCategory::Execute,
    )
    .author("neo")
    .timeout_ms(60_000)
    .with_input_schema(serde_json::json!({
        "type": "object",
        "properties": {
            "operation": {"type": "string", "enum": ["list_buckets", "upload", "download", "delete_object", "list_objects", "get_secret", "set_secret", "invoke_function", "list_services"]},
            "provider": {"type": "string", "enum": ["aws", "azure", "gcp"]},
            "service": {"type": "string", "enum": ["s3", "blob", "storage", "secrets", "functions"]},
            "bucket": {"type": "string"},
            "key": {"type": "string"},
            "path": {"type": "string"},
            "content": {"type": "string"},
            "secret_name": {"type": "string"},
            "function_name": {"type": "string"},
            "payload": {"type": "object"},
            "region": {"type": "string"},
            "config": {"type": "object"}
        },
        "required": ["operation", "provider"]
    }))
    .on_execute(|params, _ctx| {
        Box::pin(async move {
            let operation = params
                .get("operation")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::invalid_params("missing 'operation'"))?;
            let provider = params
                .get("provider")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::invalid_params("missing 'provider'"))?;

            match operation {
                "list_buckets" => {
                    let service = params
                        .get("service")
                        .and_then(|v| v.as_str())
                        .unwrap_or("s3");
                    Ok(serde_json::json!({
                        "provider": provider,
                        "service": service,
                        "buckets": [],
                        "note": "requires cloud provider credentials and SDK"
                    }))
                }
                "upload" => {
                    let bucket = params
                        .get("bucket")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let key = params
                        .get("key")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    Ok(serde_json::json!({
                        "provider": provider,
                        "uploaded": true,
                        "bucket": bucket,
                        "key": key,
                        "note": "requires cloud provider credentials and SDK"
                    }))
                }
                "download" => {
                    let bucket = params
                        .get("bucket")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let key = params
                        .get("key")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    Ok(serde_json::json!({
                        "provider": provider,
                        "bucket": bucket,
                        "key": key,
                        "content": "",
                        "note": "requires cloud provider credentials and SDK"
                    }))
                }
                "get_secret" => {
                    let secret_name = params
                        .get("secret_name")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ToolError::invalid_params("missing 'secret_name'"))?;
                    Ok(serde_json::json!({
                        "provider": provider,
                        "secret_name": secret_name,
                        "value": "",
                        "note": "requires cloud provider credentials and SDK"
                    }))
                }
                "invoke_function" => {
                    let function_name = params
                        .get("function_name")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ToolError::invalid_params("missing 'function_name'"))?;
                    let payload = params
                        .get("payload")
                        .cloned()
                        .unwrap_or(serde_json::json!({}));
                    Ok(serde_json::json!({
                        "provider": provider,
                        "function_name": function_name,
                        "payload": payload,
                        "response": {},
                        "note": "requires cloud provider credentials and SDK"
                    }))
                }
                _ => Ok(serde_json::json!({
                    "provider": provider,
                    "operation": operation,
                    "status": "stub",
                    "note": "requires cloud provider SDK integration"
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
    async fn test_cloud_list_buckets() {
        let tool = create_cloud_tool().unwrap();
        let ctx = ToolContext::new("test", crate::types::CallerType::Internal);
        let req = crate::types::ToolRequest::new(
            ToolId::new(),
            "list",
            serde_json::json!({"operation": "list_buckets", "provider": "aws", "service": "s3"}),
            ctx,
        );
        let result = tool.execute(&req).await.unwrap();
        assert_eq!(result["provider"], "aws");
    }
}
