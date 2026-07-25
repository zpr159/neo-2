//! Git integration tool.

use crate::error::{ToolError, ToolResult};
use crate::tool::{DynamicTool, ToolBuilder};
use crate::types::{ToolCategory, ToolType, ToolVersion};
use tokio::process::Command;

/// Create the git tool.
pub fn create_git_tool() -> ToolResult<DynamicTool> {
    ToolBuilder::new(
        "git",
        ToolVersion::new(1, 0, 0),
        "Git operations: clone, pull, push, commit, branch, diff, status, log, stash, tag, and worktree management",
        ToolType::Git,
        ToolCategory::Execute,
    )
    .author("neo")
    .timeout_ms(120_000)
    .with_input_schema(serde_json::json!({
        "type": "object",
        "properties": {
            "operation": {"type": "string", "enum": ["clone", "pull", "push", "commit", "branch", "checkout", "diff", "status", "log", "stash", "tag", "merge", "rebase", "fetch", "blame", "worktree"]},
            "repo_path": {"type": "string"},
            "url": {"type": "string"},
            "branch": {"type": "string"},
            "message": {"type": "string"},
            "files": {"type": "array"},
            "remote": {"type": "string"},
            "args": {"type": "array"}
        },
        "required": ["operation"]
    }))
    .on_execute(|params, _ctx| {
        Box::pin(async move {
            let operation = params
                .get("operation")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::invalid_params("missing 'operation'"))?;

            let repo_path = params
                .get("repo_path")
                .and_then(|v| v.as_str())
                .unwrap_or(".");

            match operation {
                "clone" => {
                    let url = params
                        .get("url")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ToolError::invalid_params("missing 'url'"))?;
                    let output = Command::new("git")
                        .args(["clone", url])
                        .current_dir(repo_path)
                        .output()
                        .await?;
                    Ok(serde_json::json!({
                        "stdout": String::from_utf8_lossy(&output.stdout),
                        "stderr": String::from_utf8_lossy(&output.stderr),
                        "exit_code": output.status.code(),
                    }))
                }
                "pull" | "fetch" => {
                    let remote = params
                        .get("remote")
                        .and_then(|v| v.as_str())
                        .unwrap_or("origin");
                    let output = Command::new("git")
                        .args([operation, remote])
                        .current_dir(repo_path)
                        .output()
                        .await?;
                    Ok(serde_json::json!({
                        "stdout": String::from_utf8_lossy(&output.stdout),
                        "stderr": String::from_utf8_lossy(&output.stderr),
                        "exit_code": output.status.code(),
                    }))
                }
                "push" => {
                    let remote = params
                        .get("remote")
                        .and_then(|v| v.as_str())
                        .unwrap_or("origin");
                    let output = Command::new("git")
                        .args(["push", remote])
                        .current_dir(repo_path)
                        .output()
                        .await?;
                    Ok(serde_json::json!({
                        "stdout": String::from_utf8_lossy(&output.stdout),
                        "stderr": String::from_utf8_lossy(&output.stderr),
                        "exit_code": output.status.code(),
                    }))
                }
                "commit" => {
                    let message = params
                        .get("message")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ToolError::invalid_params("missing 'message'"))?;
                    let output = Command::new("git")
                        .args(["commit", "-m", message])
                        .current_dir(repo_path)
                        .output()
                        .await?;
                    Ok(serde_json::json!({
                        "stdout": String::from_utf8_lossy(&output.stdout),
                        "stderr": String::from_utf8_lossy(&output.stderr),
                        "exit_code": output.status.code(),
                    }))
                }
                "status" | "diff" | "log" | "blame" => {
                    let output = Command::new("git")
                        .arg(operation)
                        .current_dir(repo_path)
                        .output()
                        .await?;
                    Ok(serde_json::json!({
                        "stdout": String::from_utf8_lossy(&output.stdout),
                        "stderr": String::from_utf8_lossy(&output.stderr),
                        "exit_code": output.status.code(),
                    }))
                }
                "branch" | "checkout" | "merge" | "rebase" => {
                    let mut args = vec![operation];
                    let mut cmd = Command::new("git");
                    cmd.current_dir(repo_path);
                    if let Some(branch) = params.get("branch").and_then(|v| v.as_str()) {
                        args.push(branch);
                    }
                    let output = cmd.args(&args).output().await?;
                    Ok(serde_json::json!({
                        "stdout": String::from_utf8_lossy(&output.stdout),
                        "stderr": String::from_utf8_lossy(&output.stderr),
                        "exit_code": output.status.code(),
                    }))
                }
                _ => Err(ToolError::invalid_params(format!(
                    "unknown git operation: {operation}"
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
    async fn test_git_status() {
        let tool = create_git_tool().unwrap();
        let ctx = ToolContext::new("test", crate::types::CallerType::Internal);
        let req = crate::types::ToolRequest::new(
            ToolId::new(),
            "status",
            serde_json::json!({"operation": "status", "repo_path": "."}),
            ctx,
        );
        let result = tool.execute(&req).await.unwrap();
        assert!(result.get("exit_code").is_some());
    }
}
