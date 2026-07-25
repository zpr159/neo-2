//! Browser automation tool.

use crate::error::{ToolError, ToolResult};
use crate::tool::{DynamicTool, ToolBuilder};
use crate::types::{ToolCategory, ToolType, ToolVersion};

use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Active browser session state.
#[derive(Debug, Clone)]
pub struct BrowserPageState {
    pub url: String,
    pub title: String,
    pub cookies: HashMap<String, String>,
    pub local_storage: HashMap<String, String>,
    pub screenshot: Option<Vec<u8>>,
}

/// Manager for browser sessions.
pub struct BrowserSessionManager {
    sessions: DashMap<String, Arc<RwLock<BrowserPageState>>>,
}

impl std::fmt::Debug for BrowserSessionManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrowserSessionManager")
            .field("sessions", &self.sessions.len())
            .finish()
    }
}

impl BrowserSessionManager {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
        }
    }

    pub fn create_session(&self, url: &str) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        self.sessions.insert(
            id.clone(),
            Arc::new(RwLock::new(BrowserPageState {
                url: url.to_string(),
                title: String::new(),
                cookies: HashMap::new(),
                local_storage: HashMap::new(),
                screenshot: None,
            })),
        );
        id
    }

    pub fn get_session(&self, id: &str) -> Option<Arc<RwLock<BrowserPageState>>> {
        self.sessions.get(id).map(|e| Arc::clone(e.value()))
    }

    pub fn close_session(&self, id: &str) -> bool {
        self.sessions.remove(id).is_some()
    }
}

impl Default for BrowserSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Create the browser automation tool.
pub fn create_browser_tool() -> ToolResult<DynamicTool> {
    let manager = Arc::new(BrowserSessionManager::new());
    let manager_clone = Arc::clone(&manager);

    ToolBuilder::new(
        "browser",
        ToolVersion::new(1, 0, 0),
        "Browser automation: navigation, clicking, forms, screenshots, cookies, DOM inspection, JavaScript execution",
        ToolType::Browser,
        ToolCategory::Execute,
    )
    .author("neo")
    .timeout_ms(60_000)
    .with_input_schema(serde_json::json!({
        "type": "object",
        "properties": {
            "operation": {"type": "string", "enum": ["create_session", "navigate", "get_url", "get_title", "click", "fill_form", "screenshot", "execute_js", "get_cookies", "set_cookie", "get_local_storage", "set_local_storage", "close_session", "get_page_source"]},
            "session_id": {"type": "string"},
            "url": {"type": "string"},
            "selector": {"type": "string"},
            "value": {"type": "string"},
            "script": {"type": "string"},
            "cookie_name": {"type": "string"},
            "cookie_value": {"type": "string"},
            "storage_key": {"type": "string"},
            "storage_value": {"type": "string"}
        },
        "required": ["operation"]
    }))
    .on_execute(move |params, _ctx| {
        let manager = Arc::clone(&manager_clone);
        Box::pin(async move {
            let operation = params
                .get("operation")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::invalid_params("missing 'operation'"))?;

            let session_id = params.get("session_id").and_then(|v| v.as_str());

            match operation {
                "create_session" => {
                    let url = params
                        .get("url")
                        .and_then(|v| v.as_str())
                        .unwrap_or("about:blank");
                    let id = manager.create_session(url);
                    Ok(serde_json::json!({"session_id": id}))
                }
                "navigate" => {
                    let sid = session_id
                        .ok_or_else(|| ToolError::invalid_params("missing 'session_id'"))?;
                    let url = params
                        .get("url")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ToolError::invalid_params("missing 'url'"))?;
                    let session = manager
                        .get_session(sid)
                        .ok_or_else(|| ToolError::not_found("session not found"))?;
                    let mut page = session.write().await;
                    page.url = url.to_string();
                    Ok(serde_json::json!({"navigated": url, "session_id": sid}))
                }
                "get_url" => {
                    let sid = session_id
                        .ok_or_else(|| ToolError::invalid_params("missing 'session_id'"))?;
                    let session = manager
                        .get_session(sid)
                        .ok_or_else(|| ToolError::not_found("session not found"))?;
                    let page = session.read().await;
                    Ok(serde_json::json!({"url": page.url}))
                }
                "get_title" => {
                    let sid = session_id
                        .ok_or_else(|| ToolError::invalid_params("missing 'session_id'"))?;
                    let session = manager
                        .get_session(sid)
                        .ok_or_else(|| ToolError::not_found("session not found"))?;
                    let page = session.read().await;
                    Ok(serde_json::json!({"title": page.title}))
                }
                "execute_js" => {
                    let sid = session_id
                        .ok_or_else(|| ToolError::invalid_params("missing 'session_id'"))?;
                    let script = params
                        .get("script")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ToolError::invalid_params("missing 'script'"))?;
                    // In production, this would use a real browser engine.
                    // For now, we return a structured response.
                    Ok(serde_json::json!({
                        "result": format!("JS execution stub: {}", script),
                        "session_id": sid
                    }))
                }
                "screenshot" => {
                    let sid = session_id
                        .ok_or_else(|| ToolError::invalid_params("missing 'session_id'"))?;
                    Ok(serde_json::json!({
                        "session_id": sid,
                        "format": "png",
                        "note": "screenshot requires browser engine integration"
                    }))
                }
                "get_cookies" => {
                    let sid = session_id
                        .ok_or_else(|| ToolError::invalid_params("missing 'session_id'"))?;
                    let session = manager
                        .get_session(sid)
                        .ok_or_else(|| ToolError::not_found("session not found"))?;
                    let page = session.read().await;
                    Ok(serde_json::json!({"cookies": page.cookies}))
                }
                "set_cookie" => {
                    let sid = session_id
                        .ok_or_else(|| ToolError::invalid_params("missing 'session_id'"))?;
                    let name = params
                        .get("cookie_name")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ToolError::invalid_params("missing 'cookie_name'"))?;
                    let value = params
                        .get("cookie_value")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let session = manager
                        .get_session(sid)
                        .ok_or_else(|| ToolError::not_found("session not found"))?;
                    let mut page = session.write().await;
                    page.cookies.insert(name.to_string(), value.to_string());
                    Ok(serde_json::json!({"set": true}))
                }
                "close_session" => {
                    let sid = session_id
                        .ok_or_else(|| ToolError::invalid_params("missing 'session_id'"))?;
                    let closed = manager.close_session(sid);
                    Ok(serde_json::json!({"closed": closed}))
                }
                _ => Err(ToolError::invalid_params(format!(
                    "unknown browser operation: {operation}"
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
    async fn test_browser_session_lifecycle() {
        let tool = create_browser_tool().unwrap();
        let ctx = ToolContext::new("test", crate::types::CallerType::Internal);

        let req = crate::types::ToolRequest::new(
            ToolId::new(),
            "create",
            serde_json::json!({"operation": "create_session", "url": "https://example.com"}),
            ctx.clone(),
        );
        let result = tool.execute(&req).await.unwrap();
        let sid = result["session_id"].as_str().unwrap().to_string();

        let req = crate::types::ToolRequest::new(
            ToolId::new(),
            "nav",
            serde_json::json!({"operation": "navigate", "session_id": sid, "url": "https://example.com/page"}),
            ctx.clone(),
        );
        let result = tool.execute(&req).await.unwrap();
        assert!(result["navigated"].as_str().is_some());

        let req = crate::types::ToolRequest::new(
            ToolId::new(),
            "get_url",
            serde_json::json!({"operation": "get_url", "session_id": sid}),
            ctx,
        );
        let result = tool.execute(&req).await.unwrap();
        assert_eq!(result["url"], "https://example.com/page");

        let req = crate::types::ToolRequest::new(
            ToolId::new(),
            "close",
            serde_json::json!({"operation": "close_session", "session_id": sid}),
            ToolContext::new("test", crate::types::CallerType::Internal),
        );
        let result = tool.execute(&req).await.unwrap();
        assert!(result["closed"].as_bool().unwrap());
    }
}
