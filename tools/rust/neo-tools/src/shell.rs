//! Shell execution tool implementations.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;

use crate::error::{ToolError, ToolResult};
use crate::tool::{DynamicTool, ToolBuilder};
use crate::types::{ToolCategory, ToolType, ToolVersion};

/// Active shell sessions.
pub struct ShellSessionManager {
    sessions: DashMap<String, Arc<RwLock<ShellSession>>>,
}

impl std::fmt::Debug for ShellSessionManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShellSessionManager")
            .field("session_count", &self.sessions.len())
            .finish()
    }
}

use dashmap::DashMap;

pub struct ShellSession {
    pub id: String,
    pub shell: String,
    pub working_dir: String,
    pub env: HashMap<String, String>,
    pub history: Vec<ShellCommandRecord>,
}

impl std::fmt::Debug for ShellSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShellSession")
            .field("id", &self.id)
            .field("shell", &self.shell)
            .field("working_dir", &self.working_dir)
            .field("history_len", &self.history.len())
            .finish()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ShellCommandRecord {
    pub command: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

impl ShellSessionManager {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
        }
    }

    pub fn create_session(&self, shell: &str, working_dir: &str) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        self.sessions.insert(
            id.clone(),
            Arc::new(RwLock::new(ShellSession {
                id: id.clone(),
                shell: shell.to_string(),
                working_dir: working_dir.to_string(),
                env: HashMap::new(),
                history: Vec::new(),
            })),
        );
        id
    }

    pub fn get_session(&self, id: &str) -> Option<Arc<RwLock<ShellSession>>> {
        self.sessions.get(id).map(|e| Arc::clone(e.value()))
    }

    pub fn close_session(&self, id: &str) -> bool {
        self.sessions.remove(id).is_some()
    }
}

impl Default for ShellSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Create the shell tool.
pub fn create_shell_tool() -> ToolResult<DynamicTool> {
    let sessions = Arc::new(ShellSessionManager::new());
    let sessions_clone = Arc::clone(&sessions);

    ToolBuilder::new(
        "shell",
        ToolVersion::new(1, 0, 0),
        "Shell execution: run commands in Bash, Zsh, PowerShell, or CMD with timeout, streaming, and history",
        ToolType::Shell,
        ToolCategory::Execute,
    )
    .author("neo")
    .timeout_ms(60_000)
    .with_input_schema(serde_json::json!({
        "type": "object",
        "properties": {
            "operation": {"type": "string", "enum": ["exec", "exec_stream", "session_create", "session_exec", "session_close", "history"]},
            "command": {"type": "string"},
            "shell": {"type": "string", "enum": ["bash", "zsh", "powershell", "cmd", "sh"]},
            "working_dir": {"type": "string"},
            "timeout_ms": {"type": "number"},
            "session_id": {"type": "string"},
            "env": {"type": "object"}
        },
        "required": ["operation"]
    }))
    .on_execute(move |params, _ctx| {
        let sessions = Arc::clone(&sessions_clone);
        Box::pin(async move {
            let operation = params
                .get("operation")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::invalid_params("missing 'operation'"))?;

            match operation {
                "exec" => exec_command(&params).await,
                "session_create" => {
                    let shell = params
                        .get("shell")
                        .and_then(|v| v.as_str())
                        .unwrap_or("bash");
                    let dir = params
                        .get("working_dir")
                        .and_then(|v| v.as_str())
                        .unwrap_or("/tmp");
                    let session_id = sessions.create_session(shell, dir);
                    Ok(serde_json::json!({"session_id": session_id}))
                }
                "session_exec" => {
                    let session_id = params
                        .get("session_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ToolError::invalid_params("missing 'session_id'"))?;
                    let command = params
                        .get("command")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ToolError::invalid_params("missing 'command'"))?;

                    let session_arc = sessions
                        .get_session(session_id)
                        .ok_or_else(|| ToolError::not_found("session not found"))?;
                    let mut session = session_arc.write().await;

                    let mut cmd = Command::new(&session.shell);
                    cmd.arg("-c").arg(command);
                    cmd.current_dir(&session.working_dir);

                    for (k, v) in &session.env {
                        cmd.env(k, v);
                    }

                    let output = cmd.output().await?;
                    let record = ShellCommandRecord {
                        command: command.to_string(),
                        exit_code: output.status.code(),
                        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                        duration_ms: 0,
                    };

                    let result = serde_json::json!({
                        "stdout": record.stdout,
                        "stderr": record.stderr,
                        "exit_code": record.exit_code,
                    });
                    session.history.push(record);
                    Ok(result)
                }
                "session_close" => {
                    let session_id = params
                        .get("session_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ToolError::invalid_params("missing 'session_id'"))?;
                    let closed = sessions.close_session(session_id);
                    Ok(serde_json::json!({"closed": closed}))
                }
                "history" => {
                    let session_id = params
                        .get("session_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ToolError::invalid_params("missing 'session_id'"))?;
                    let session_arc = sessions
                        .get_session(session_id)
                        .ok_or_else(|| ToolError::not_found("session not found"))?;
                    let session = session_arc.read().await;
                    Ok(serde_json::json!({"history": session.history}))
                }
                _ => Err(ToolError::invalid_params(format!(
                    "unknown shell operation: {operation}"
                ))),
            }
        })
    })
    .build()
}

async fn exec_command(params: &serde_json::Value) -> ToolResult<serde_json::Value> {
    let command = params
        .get("command")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ToolError::invalid_params("missing 'command'"))?;

    let shell = params
        .get("shell")
        .and_then(|v| v.as_str())
        .unwrap_or("bash");

    let timeout_ms = params
        .get("timeout_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(30_000);

    let mut cmd = Command::new(shell);
    cmd.arg("-c").arg(command);

    if let Some(dir) = params.get("working_dir").and_then(|v| v.as_str()) {
        cmd.current_dir(dir);
    }

    if let Some(env_obj) = params.get("env").and_then(|v| v.as_object()) {
        for (k, v) in env_obj {
            if let Some(val) = v.as_str() {
                cmd.env(k, val);
            }
        }
    }

    let start = std::time::Instant::now();
    let output = tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), cmd.output())
        .await
        .map_err(|_| ToolError::timeout(format!("command timed out after {timeout_ms}ms")))?
        .map_err(ToolError::io)?;

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(serde_json::json!({
        "stdout": String::from_utf8_lossy(&output.stdout),
        "stderr": String::from_utf8_lossy(&output.stderr),
        "exit_code": output.status.code(),
        "duration_ms": duration_ms,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ToolContext, ToolId};

    #[tokio::test]
    async fn test_shell_exec() {
        let tool = create_shell_tool().unwrap();
        let ctx = ToolContext::new("test", crate::types::CallerType::Internal);
        let req = crate::types::ToolRequest::new(
            ToolId::new(),
            "exec",
            serde_json::json!({"operation": "exec", "command": "echo hello", "shell": "bash"}),
            ctx,
        );
        let result = tool.execute(&req).await.unwrap();
        assert!(result["stdout"].as_str().unwrap_or("").contains("hello"));
    }

    #[tokio::test]
    async fn test_shell_session() {
        let tool = create_shell_tool().unwrap();
        let ctx = ToolContext::new("test", crate::types::CallerType::Internal);

        let req = crate::types::ToolRequest::new(
            ToolId::new(),
            "session",
            serde_json::json!({"operation": "session_create", "shell": "bash"}),
            ctx.clone(),
        );
        let result = tool.execute(&req).await.unwrap();
        let session_id = result["session_id"].as_str().unwrap().to_string();

        let req = crate::types::ToolRequest::new(
            ToolId::new(),
            "session_exec",
            serde_json::json!({"operation": "session_exec", "session_id": session_id, "command": "echo world"}),
            ctx.clone(),
        );
        let result = tool.execute(&req).await.unwrap();
        assert!(result["stdout"].as_str().unwrap_or("").contains("world"));
    }
}
