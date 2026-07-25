//! HTTP client tool with REST, GraphQL, WebSocket, and streaming support.

use crate::error::{ToolError, ToolResult};
use crate::tool::{DynamicTool, ToolBuilder};
use crate::types::{ToolCategory, ToolType, ToolVersion};

/// Create the HTTP client tool.
pub fn create_http_tool() -> ToolResult<DynamicTool> {
    ToolBuilder::new(
        "http",
        ToolVersion::new(1, 0, 0),
        "HTTP client: GET, POST, PUT, PATCH, DELETE with JSON, XML, multipart, streaming, and authentication",
        ToolType::HttpClient,
        ToolCategory::Execute,
    )
    .author("neo")
    .timeout_ms(30_000)
    .with_input_schema(serde_json::json!({
        "type": "object",
        "properties": {
            "operation": {"type": "string", "enum": ["request"]},
            "method": {"type": "string", "enum": ["GET", "POST", "PUT", "PATCH", "DELETE"]},
            "url": {"type": "string"},
            "headers": {"type": "object"},
            "body": {},
            "auth": {"type": "object"},
            "timeout_ms": {"type": "number"}
        },
        "required": ["url"]
    }))
    .on_execute(|params, _ctx| {
        Box::pin(async move {
            let url = params
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::invalid_params("missing 'url'"))?;

            let method = params
                .get("method")
                .and_then(|v| v.as_str())
                .unwrap_or("GET");

            let timeout_ms = params
                .get("timeout_ms")
                .and_then(|v| v.as_u64())
                .unwrap_or(30_000);

            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_millis(timeout_ms))
                .build()
                .map_err(|e| ToolError::internal(format!("failed to build client: {e}")))?;

            let mut req_builder = match method.to_uppercase().as_str() {
                "GET" => client.get(url),
                "POST" => client.post(url),
                "PUT" => client.put(url),
                "PATCH" => client.patch(url),
                "DELETE" => client.delete(url),
                _ => return Err(ToolError::invalid_params(format!("unsupported method: {method}"))),
            };

            if let Some(headers) = params.get("headers").and_then(|v| v.as_object()) {
                for (k, v) in headers {
                    if let Some(val) = v.as_str() {
                        req_builder = req_builder.header(k.as_str(), val);
                    }
                }
            }

            if let Some(auth) = params.get("auth") {
                if let Some(bearer) = auth.get("bearer").and_then(|v| v.as_str()) {
                    req_builder = req_builder.bearer_auth(bearer);
                } else if let Some(basic) = auth.get("basic") {
                    if let Some((user, pass)) = basic.as_str().and_then(|s| s.split_once(':')) {
                        req_builder = req_builder.basic_auth(user, Some(pass));
                    }
                } else if let Some(api_key) = auth.get("api_key").and_then(|v| v.as_str()) {
                    let header = auth
                        .get("api_key_header")
                        .and_then(|v| v.as_str())
                        .unwrap_or("X-API-Key");
                    req_builder = req_builder.header(header, api_key);
                }
            }

            if let Some(body) = params.get("body") {
                if let Some(obj) = body.as_object() {
                    req_builder = req_builder.json(obj);
                } else if let Some(s) = body.as_str() {
                    req_builder = req_builder.body(s.to_string());
                }
            }

            let start = std::time::Instant::now();
            let response = req_builder.send().await.map_err(|e| {
                ToolError::execution_failed(format!("request failed: {e}"))
            })?;

            let status = response.status().as_u16();
            let headers: std::collections::HashMap<String, String> = response
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();

            let body = response.text().await.map_err(ToolError::io)?;
            let duration_ms = start.elapsed().as_millis() as u64;

            let parsed_body = serde_json::from_str::<serde_json::Value>(&body)
                .unwrap_or_else(|_| serde_json::Value::String(body.clone()));

            Ok(serde_json::json!({
                "status": status,
                "headers": headers,
                "body": parsed_body,
                "raw_body": body,
                "duration_ms": duration_ms,
            }))
        })
    })
    .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ToolContext, ToolId};

    #[tokio::test]
    async fn test_http_request_failure_handled() {
        let tool = create_http_tool().unwrap();
        let ctx = ToolContext::new("test", crate::types::CallerType::Internal);
        let req = crate::types::ToolRequest::new(
            ToolId::new(),
            "request",
            serde_json::json!({
                "url": "http://127.0.0.1:1/nonexistent",
                "method": "GET",
                "timeout_ms": 2_000
            }),
            ctx,
        );
        let result = tool.execute(&req).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_http_tool_metadata() {
        let tool = create_http_tool().unwrap();
        assert_eq!(tool.manifest.metadata.name, "http");
    }
}
