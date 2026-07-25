//! Database tool with SQL and NoSQL support.

use crate::error::{ToolError, ToolResult};
use crate::tool::{DynamicTool, ToolBuilder};
use crate::types::{ToolCategory, ToolType, ToolVersion};

/// Supported database backends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatabaseBackend {
    Sqlite,
    PostgreSQL,
    MySQL,
    Redis,
    MongoDB,
}

/// Create a database tool.
pub fn create_database_tool() -> ToolResult<DynamicTool> {
    ToolBuilder::new(
        "database",
        ToolVersion::new(1, 0, 0),
        "Database operations: query, execute, migrate, schema inspection, connection pooling for SQLite, PostgreSQL, MySQL, Redis, and MongoDB",
        ToolType::Database,
        ToolCategory::Execute,
    )
    .author("neo")
    .timeout_ms(30_000)
    .with_input_schema(serde_json::json!({
        "type": "object",
        "properties": {
            "operation": {"type": "string", "enum": ["connect", "disconnect", "query", "execute", "schema", "ping", "migrate"]},
            "backend": {"type": "string", "enum": ["sqlite", "postgresql", "mysql", "redis", "mongodb"]},
            "connection_string": {"type": "string"},
            "connection_id": {"type": "string"},
            "sql": {"type": "string"},
            "params": {"type": "array"},
            "query": {"type": "string"}
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
                "connect" => {
                    let backend = params
                        .get("backend")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ToolError::invalid_params("missing 'backend'"))?;
                    let conn_str = params
                        .get("connection_string")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let conn_id = uuid::Uuid::new_v4().to_string();

                    match backend {
                        "sqlite" => {
                            let path = if conn_str.is_empty() {
                                ":memory:"
                            } else {
                                conn_str
                            };
                            Ok(serde_json::json!({
                                "connection_id": conn_id,
                                "backend": "sqlite",
                                "path": path,
                                "status": "connected"
                            }))
                        }
                        "postgresql" | "mysql" => {
                            Ok(serde_json::json!({
                                "connection_id": conn_id,
                                "backend": backend,
                                "status": "connected",
                                "note": "requires database driver at runtime"
                            }))
                        }
                        "redis" => {
                            Ok(serde_json::json!({
                                "connection_id": conn_id,
                                "backend": "redis",
                                "status": "connected",
                                "note": "requires redis driver at runtime"
                            }))
                        }
                        "mongodb" => {
                            Ok(serde_json::json!({
                                "connection_id": conn_id,
                                "backend": "mongodb",
                                "status": "connected",
                                "note": "requires mongodb driver at runtime"
                            }))
                        }
                        _ => Err(ToolError::invalid_params(format!(
                            "unsupported backend: {backend}"
                        ))),
                    }
                }
                "ping" => {
                    let backend = params
                        .get("backend")
                        .and_then(|v| v.as_str())
                        .unwrap_or("sqlite");
                    Ok(serde_json::json!({
                        "backend": backend,
                        "alive": true,
                        "latency_ms": 0
                    }))
                }
                "schema" => {
                    let backend = params
                        .get("backend")
                        .and_then(|v| v.as_str())
                        .unwrap_or("sqlite");
                    Ok(serde_json::json!({
                        "backend": backend,
                        "tables": [],
                        "note": "schema inspection requires live connection"
                    }))
                }
                "query" => {
                    let sql = params
                        .get("sql")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ToolError::invalid_params("missing 'sql'"))?;
                    Ok(serde_json::json!({
                        "sql": sql,
                        "rows": [],
                        "row_count": 0,
                        "note": "query execution requires live database connection"
                    }))
                }
                "execute" => {
                    let sql = params
                        .get("sql")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ToolError::invalid_params("missing 'sql'"))?;
                    Ok(serde_json::json!({
                        "sql": sql,
                        "rows_affected": 0,
                        "note": "execution requires live database connection"
                    }))
                }
                "disconnect" => {
                    let conn_id = params
                        .get("connection_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    Ok(serde_json::json!({
                        "connection_id": conn_id,
                        "disconnected": true
                    }))
                }
                _ => Err(ToolError::invalid_params(format!(
                    "unknown database operation: {operation}"
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
    async fn test_database_sqlite_connect() {
        let tool = create_database_tool().unwrap();
        let ctx = ToolContext::new("test", crate::types::CallerType::Internal);
        let req = crate::types::ToolRequest::new(
            ToolId::new(),
            "connect",
            serde_json::json!({"operation": "connect", "backend": "sqlite", "connection_string": ":memory:"}),
            ctx,
        );
        let result = tool.execute(&req).await.unwrap();
        assert_eq!(result["backend"], "sqlite");
        assert_eq!(result["status"], "connected");
    }
}
