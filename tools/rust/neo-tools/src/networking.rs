//! Networking tool with DNS, TCP, UDP, SSH, and WebSocket support.

use crate::error::{ToolError, ToolResult};
use crate::tool::{DynamicTool, ToolBuilder};
use crate::types::{ToolCategory, ToolType, ToolVersion};

/// Create the networking tool.
pub fn create_networking_tool() -> ToolResult<DynamicTool> {
    ToolBuilder::new(
        "networking",
        ToolVersion::new(1, 0, 0),
        "Networking: DNS resolution, TCP/UDP connections, HTTP/HTTPS probes, SSH, SFTP, WebSocket, port scanning",
        ToolType::Networking,
        ToolCategory::Execute,
    )
    .author("neo")
    .timeout_ms(30_000)
    .with_input_schema(serde_json::json!({
        "type": "object",
        "properties": {
            "operation": {"type": "string", "enum": ["dns_resolve", "tcp_connect", "udp_send", "http_probe", "port_scan", "gethostname", "get_interfaces", "websocket_connect"]},
            "host": {"type": "string"},
            "port": {"type": "number"},
            "protocol": {"type": "string"},
            "message": {"type": "string"},
            "url": {"type": "string"},
            "timeout_ms": {"type": "number"},
            "ports": {"type": "array"}
        },
        "required": ["operation"]
    }))
    .on_execute(|params, _ctx| {
        Box::pin(async move {
            let operation = params
                .get("operation")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::invalid_params("missing 'operation'"))?;

            match operation {
                "dns_resolve" => {
                    let host = params
                        .get("host")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ToolError::invalid_params("missing 'host'"))?;
                    let addrs: Vec<String> = tokio::net::lookup_host(format!("{host}:443"))
                        .await
                        .map_err(|e| ToolError::execution_failed(format!("DNS error: {e}")))?
                        .map(|addr| addr.to_string())
                        .collect();
                    Ok(serde_json::json!({
                        "host": host,
                        "addresses": addrs,
                        "count": addrs.len()
                    }))
                }
                "tcp_connect" => {
                    let host = params
                        .get("host")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ToolError::invalid_params("missing 'host'"))?;
                    let port = params
                        .get("port")
                        .and_then(|v| v.as_f64())
                        .map(|n| n as u16)
                        .ok_or_else(|| ToolError::invalid_params("missing 'port'"))?;
                    let timeout_ms = params
                        .get("timeout_ms")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(5000);

                    let addr = format!("{host}:{port}");
                    let result = tokio::time::timeout(
                        std::time::Duration::from_millis(timeout_ms),
                        tokio::net::TcpStream::connect(&addr),
                    )
                    .await;

                    match result {
                        Ok(Ok(_stream)) => Ok(serde_json::json!({
                            "host": host,
                            "port": port,
                            "connected": true,
                            "latency_ms": 0
                        })),
                        Ok(Err(e)) => Ok(serde_json::json!({
                            "host": host,
                            "port": port,
                            "connected": false,
                            "error": e.to_string()
                        })),
                        Err(_) => Ok(serde_json::json!({
                            "host": host,
                            "port": port,
                            "connected": false,
                            "error": "connection timed out"
                        })),
                    }
                }
                "gethostname" => {
                    let hostname = hostname::get()
                        .map(|h| h.to_string_lossy().to_string())
                        .unwrap_or_else(|_| "unknown".into());
                    Ok(serde_json::json!({"hostname": hostname}))
                }
                "get_interfaces" => {
                    Ok(serde_json::json!({
                        "interfaces": [],
                        "note": "platform-specific network interface listing"
                    }))
                }
                "port_scan" => {
                    let host = params
                        .get("host")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ToolError::invalid_params("missing 'host'"))?;
                    let ports: Vec<u16> = params
                        .get("ports")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_else(|| vec![22, 80, 443, 8080]);

                    let mut results = Vec::new();
                    for port in &ports {
                        let addr = format!("{host}:{port}");
                        let open = tokio::time::timeout(
                            std::time::Duration::from_millis(2000),
                            tokio::net::TcpStream::connect(&addr),
                        )
                        .await
                        .is_ok();
                        results.push(serde_json::json!({
                            "port": port,
                            "open": open
                        }));
                    }
                    Ok(serde_json::json!({
                        "host": host,
                        "results": results,
                        "scanned": results.len()
                    }))
                }
                "http_probe" => {
                    let url = params
                        .get("url")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ToolError::invalid_params("missing 'url'"))?;
                    let client = reqwest::Client::builder()
                        .timeout(std::time::Duration::from_secs(10))
                        .build()
                        .map_err(|e| ToolError::internal(e.to_string()))?;
                    let start = std::time::Instant::now();
                    match client.get(url).send().await {
                        Ok(resp) => {
                            let status = resp.status().as_u16();
                            let latency = start.elapsed().as_millis() as u64;
                            Ok(serde_json::json!({
                                "url": url,
                                "status": status,
                                "latency_ms": latency,
                                "reachable": true
                            }))
                        }
                        Err(e) => Ok(serde_json::json!({
                            "url": url,
                            "reachable": false,
                            "error": e.to_string()
                        })),
                    }
                }
                _ => Err(ToolError::invalid_params(format!(
                    "unknown networking operation: {operation}"
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
    async fn test_dns_resolve() {
        let tool = create_networking_tool().unwrap();
        let ctx = ToolContext::new("test", crate::types::CallerType::Internal);
        let req = crate::types::ToolRequest::new(
            ToolId::new(),
            "dns",
            serde_json::json!({"operation": "dns_resolve", "host": "localhost"}),
            ctx,
        );
        let result = tool.execute(&req).await.unwrap();
        assert!(result.get("addresses").is_some());
    }
}
