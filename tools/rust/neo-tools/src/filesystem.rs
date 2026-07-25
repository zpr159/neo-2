//! Filesystem tool implementations.

use std::path::PathBuf;
use tokio::fs;

use crate::error::{ToolError, ToolResult};
use crate::tool::{DynamicTool, ToolBuilder};
use crate::types::{
    ToolCategory, ToolType, ToolVersion,
};

/// Create the filesystem tool.
pub fn create_filesystem_tool() -> ToolResult<DynamicTool> {
    ToolBuilder::new(
        "filesystem",
        ToolVersion::new(1, 0, 0),
        "Filesystem operations: read, write, create, delete, move, copy, rename, search, and metadata",
        ToolType::Filesystem,
        ToolCategory::Execute,
    )
    .author("neo")
    .with_input_schema(serde_json::json!({
        "type": "object",
        "properties": {
            "operation": {"type": "string", "enum": ["create", "read", "write", "append", "delete", "move", "copy", "rename", "mkdir", "rmdir", "exists", "metadata", "hash", "search", "list", "glob"]},
            "path": {"type": "string"},
            "content": {"type": "string"},
            "destination": {"type": "string"},
            "pattern": {"type": "string"},
            "recursive": {"type": "boolean"}
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
                "create" => handle_create(&params).await,
                "read" => handle_read(&params).await,
                "write" => handle_write(&params).await,
                "append" => handle_append(&params).await,
                "delete" => handle_delete(&params).await,
                "move" => handle_move(&params).await,
                "copy" => handle_copy(&params).await,
                "rename" => handle_rename(&params).await,
                "mkdir" => handle_mkdir(&params).await,
                "rmdir" => handle_rmdir(&params).await,
                "exists" => handle_exists(&params).await,
                "metadata" => handle_metadata(&params).await,
                "hash" => handle_hash(&params).await,
                "search" => handle_search(&params).await,
                "list" => handle_list(&params).await,
                "glob" => handle_glob(&params).await,
                _ => Err(ToolError::invalid_params(format!(
                    "unknown filesystem operation: {operation}"
                ))),
            }
        })
    })
    .build()
}

fn require_path(params: &serde_json::Value) -> ToolResult<PathBuf> {
    params
        .get("path")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .ok_or_else(|| ToolError::invalid_params("missing 'path'"))
}

fn require_dest(params: &serde_json::Value) -> ToolResult<PathBuf> {
    params
        .get("destination")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .ok_or_else(|| ToolError::invalid_params("missing 'destination'"))
}

async fn handle_create(params: &serde_json::Value) -> ToolResult<serde_json::Value> {
    let path = require_path(params)?;
    let content = params.get("content").and_then(|v| v.as_str()).unwrap_or("");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&path, content).await?;
    Ok(serde_json::json!({"created": path.to_string_lossy()}))
}

async fn handle_read(params: &serde_json::Value) -> ToolResult<serde_json::Value> {
    let path = require_path(params)?;
    let content = fs::read_to_string(&path).await?;
    Ok(serde_json::json!({
        "path": path.to_string_lossy(),
        "content": content,
        "size_bytes": content.len()
    }))
}

async fn handle_write(params: &serde_json::Value) -> ToolResult<serde_json::Value> {
    let path = require_path(params)?;
    let content = params.get("content").and_then(|v| v.as_str()).unwrap_or("");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&path, content).await?;
    Ok(serde_json::json!({"written": content.len(), "path": path.to_string_lossy()}))
}

async fn handle_append(params: &serde_json::Value) -> ToolResult<serde_json::Value> {
    let path = require_path(params)?;
    let content = params.get("content").and_then(|v| v.as_str()).unwrap_or("");
    use tokio::io::AsyncWriteExt;
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .await?;
    file.write_all(content.as_bytes()).await?;
    Ok(serde_json::json!({"appended": content.len(), "path": path.to_string_lossy()}))
}

async fn handle_delete(params: &serde_json::Value) -> ToolResult<serde_json::Value> {
    let path = require_path(params)?;
    if path.is_dir() {
        fs::remove_dir_all(&path).await?;
    } else {
        fs::remove_file(&path).await?;
    }
    Ok(serde_json::json!({"deleted": path.to_string_lossy()}))
}

async fn handle_move(params: &serde_json::Value) -> ToolResult<serde_json::Value> {
    let src = require_path(params)?;
    let dst = require_dest(params)?;
    fs::rename(&src, &dst).await?;
    Ok(serde_json::json!({"from": src.to_string_lossy(), "to": dst.to_string_lossy()}))
}

async fn handle_copy(params: &serde_json::Value) -> ToolResult<serde_json::Value> {
    let src = require_path(params)?;
    let dst = require_dest(params)?;
    fs::copy(&src, &dst).await?;
    Ok(serde_json::json!({"from": src.to_string_lossy(), "to": dst.to_string_lossy()}))
}

async fn handle_rename(params: &serde_json::Value) -> ToolResult<serde_json::Value> {
    let src = require_path(params)?;
    let dst = require_dest(params)?;
    fs::rename(&src, &dst).await?;
    Ok(serde_json::json!({"from": src.to_string_lossy(), "to": dst.to_string_lossy()}))
}

async fn handle_mkdir(params: &serde_json::Value) -> ToolResult<serde_json::Value> {
    let path = require_path(params)?;
    let recursive = params
        .get("recursive")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    if recursive {
        fs::create_dir_all(&path).await?;
    } else {
        fs::create_dir(&path).await?;
    }
    Ok(serde_json::json!({"created": path.to_string_lossy()}))
}

async fn handle_rmdir(params: &serde_json::Value) -> ToolResult<serde_json::Value> {
    let path = require_path(params)?;
    let recursive = params
        .get("recursive")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if recursive {
        fs::remove_dir_all(&path).await?;
    } else {
        fs::remove_dir(&path).await?;
    }
    Ok(serde_json::json!({"removed": path.to_string_lossy()}))
}

async fn handle_exists(params: &serde_json::Value) -> ToolResult<serde_json::Value> {
    let path = require_path(params)?;
    let exists = path.exists();
    let is_file = path.is_file();
    let is_dir = path.is_dir();
    Ok(serde_json::json!({
        "path": path.to_string_lossy(),
        "exists": exists,
        "is_file": is_file,
        "is_dir": is_dir
    }))
}

async fn handle_metadata(params: &serde_json::Value) -> ToolResult<serde_json::Value> {
    let path = require_path(params)?;
    let meta = fs::metadata(&path).await?;
    Ok(serde_json::json!({
        "path": path.to_string_lossy(),
        "size_bytes": meta.len(),
        "is_file": meta.is_file(),
        "is_dir": meta.is_dir(),
        "readonly": meta.permissions().readonly(),
        "modified": meta.modified().ok().map(|t| {
            t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
        })
    }))
}

async fn handle_hash(params: &serde_json::Value) -> ToolResult<serde_json::Value> {
    use sha2::{Digest, Sha256};
    let path = require_path(params)?;
    let bytes = fs::read(&path).await?;
    let hash = Sha256::digest(&bytes);
    Ok(serde_json::json!({
        "path": path.to_string_lossy(),
        "sha256": format!("{:x}", hash),
        "size_bytes": bytes.len()
    }))
}

async fn handle_search(params: &serde_json::Value) -> ToolResult<serde_json::Value> {
    let path = require_path(params)?;
    let pattern = params
        .get("pattern")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ToolError::invalid_params("missing 'pattern'"))?;
    let recursive = params
        .get("recursive")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let mut matches = Vec::new();
    if recursive {
        for entry in walkdir::WalkDir::new(&path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if let Some(name) = entry.file_name().to_str() {
                if name.contains(pattern) {
                    matches.push(entry.path().to_string_lossy().to_string());
                }
            }
        }
    } else {
        let mut dir = fs::read_dir(&path).await?;
        while let Some(entry) = dir.next_entry().await? {
            if let Some(name) = entry.file_name().to_str() {
                if name.contains(pattern) {
                    matches.push(entry.path().to_string_lossy().to_string());
                }
            }
        }
    }

    Ok(serde_json::json!({
        "matches": matches,
        "count": matches.len()
    }))
}

async fn handle_list(params: &serde_json::Value) -> ToolResult<serde_json::Value> {
    let path = require_path(params)?;
    let mut entries = Vec::new();
    let mut dir = fs::read_dir(&path).await?;
    while let Some(entry) = dir.next_entry().await? {
        let meta = entry.metadata().await?;
        entries.push(serde_json::json!({
            "name": entry.file_name().to_string_lossy(),
            "path": entry.path().to_string_lossy(),
            "is_file": meta.is_file(),
            "is_dir": meta.is_dir(),
            "size_bytes": meta.len()
        }));
    }
    Ok(serde_json::json!({"entries": entries, "count": entries.len()}))
}

async fn handle_glob(params: &serde_json::Value) -> ToolResult<serde_json::Value> {
    let pattern = params
        .get("pattern")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ToolError::invalid_params("missing 'pattern'"))?;

    let mut matches = Vec::new();
    for entry in
        glob::glob(pattern).map_err(|e| ToolError::invalid_params(format!("invalid glob: {e}")))?
    {
        match entry {
            Ok(path) => matches.push(path.to_string_lossy().to_string()),
            Err(e) => {
                tracing::warn!("glob error: {e}");
            }
        }
    }
    Ok(serde_json::json!({"matches": matches, "count": matches.len()}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ToolContext, ToolId};

    #[tokio::test]
    async fn test_filesystem_create_and_read() {
        let tool = create_filesystem_tool().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");

        let ctx = ToolContext::new("test", crate::types::CallerType::Internal);
        let create_req = crate::types::ToolRequest::new(
            ToolId::new(),
            "create",
            serde_json::json!({"operation": "create", "path": path.to_string_lossy(), "content": "hello world"}),
            ctx.clone(),
        );
        let result = tool.execute(&create_req).await.unwrap();
        assert!(result.get("created").is_some());

        let read_req = crate::types::ToolRequest::new(
            ToolId::new(),
            "read",
            serde_json::json!({"operation": "read", "path": path.to_string_lossy()}),
            ctx,
        );
        let result = tool.execute(&read_req).await.unwrap();
        assert_eq!(result["content"], "hello world");
    }

    #[tokio::test]
    async fn test_filesystem_metadata() {
        let tool = create_filesystem_tool().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("meta.txt");
        fs::write(&path, "test content").await.unwrap();

        let ctx = ToolContext::new("test", crate::types::CallerType::Internal);
        let req = crate::types::ToolRequest::new(
            ToolId::new(),
            "metadata",
            serde_json::json!({"operation": "metadata", "path": path.to_string_lossy()}),
            ctx,
        );
        let result = tool.execute(&req).await.unwrap();
        assert_eq!(result["size_bytes"], 12);
    }
}
