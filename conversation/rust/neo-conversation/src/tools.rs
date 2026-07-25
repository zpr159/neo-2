use std::collections::HashMap;
use std::sync::Arc;

use crate::error::ConversationResult;
use crate::types::{ToolCall, ToolDefinition, ToolResult};

/// A tool that can be invoked from conversation.
#[async_trait::async_trait]
pub trait ConversationTool: Send + Sync {
    /// Tool name.
    fn name(&self) -> &str;

    /// Tool description.
    fn description(&self) -> &str;

    /// Tool parameter schema.
    fn parameters(&self) -> serde_json::Value;

    /// Execute the tool with the given arguments.
    async fn execute(&self, arguments: &serde_json::Value) -> ConversationResult<String>;
}

/// Bridges conversation with the tool ecosystem.
///
/// Extracts tool calls from LLM responses and executes them.
pub struct ToolBridge {
    /// Registered tools by name.
    tools: HashMap<String, Arc<dyn ConversationTool>>,
}

impl ToolBridge {
    #[must_use]
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool.
    pub fn register(&mut self, tool: Arc<dyn ConversationTool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// Get tool definitions for the LLM prompt.
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools
            .values()
            .map(|t| ToolDefinition {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.parameters(),
            })
            .collect()
    }

    /// Extract tool calls from an LLM response text.
    ///
    /// Supports common tool call formats:
    /// - JSON code blocks with tool_call markers
    /// - Inline function calls
    pub fn extract_tool_calls(&self, response_text: &str) -> Vec<ToolCall> {
        let mut calls = Vec::new();

        // Look for ```json tool_call blocks.
        let blocks = extract_json_blocks(response_text);
        for block in blocks {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&block) {
                // Check if it looks like a tool call.
                if let (Some(name), Some(args)) = (
                    value.get("tool").or_else(|| value.get("name")),
                    value.get("arguments").or_else(|| value.get("args")),
                ) {
                    if let Some(name_str) = name.as_str() {
                        if self.tools.contains_key(name_str) {
                            calls.push(ToolCall {
                                id: uuid::Uuid::new_v4().to_string(),
                                name: name_str.to_string(),
                                arguments: args.clone(),
                            });
                        }
                    }
                }
            }
        }

        // Also look for simple function call patterns.
        for line in response_text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("TOOL_CALL:") || trimmed.starts_with("tool_call:") {
                if let Some(json_str) = trimmed.split(':').skip(1).collect::<String>().strip_prefix(' ') {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
                        if let (Some(name), Some(args)) = (
                            value.get("name"),
                            value.get("arguments").or_else(|| value.get("args")),
                        ) {
                            if let Some(name_str) = name.as_str() {
                                calls.push(ToolCall {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    name: name_str.to_string(),
                                    arguments: args.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }

        calls
    }

    /// Execute tool calls and return results.
    pub async fn execute_tools(&self, tool_calls: &[ToolCall]) -> Vec<ToolResult> {
        let mut results = Vec::new();

        for call in tool_calls {
            let result = if let Some(tool) = self.tools.get(&call.name) {
                match tool.execute(&call.arguments).await {
                    Ok(output) => ToolResult {
                        call_id: call.id.clone(),
                        name: call.name.clone(),
                        result: output,
                        success: true,
                        error: None,
                    },
                    Err(e) => ToolResult {
                        call_id: call.id.clone(),
                        name: call.name.clone(),
                        result: String::new(),
                        success: false,
                        error: Some(e.to_string()),
                    },
                }
            } else {
                ToolResult {
                    call_id: call.id.clone(),
                    name: call.name.clone(),
                    result: String::new(),
                    success: false,
                    error: Some(format!("Tool '{}' not found", call.name)),
                }
            };
            results.push(result);
        }

        results
    }

    /// Check if a tool is registered.
    #[must_use]
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get the number of registered tools.
    #[must_use]
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }
}

impl Default for ToolBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract JSON code blocks from text.
fn extract_json_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut in_block = false;
    let mut current = String::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```json") || trimmed.starts_with("```JSON") {
            in_block = true;
            current.clear();
        } else if trimmed == "```" && in_block {
            in_block = false;
            if !current.is_empty() {
                blocks.push(current.clone());
            }
        } else if in_block {
            current.push_str(line);
            current.push('\n');
        }
    }

    blocks
}

/// A simple key-value tool for testing and demonstration.
pub struct KeyValueTool {
    store: Arc<tokio::sync::RwLock<HashMap<String, String>>>,
}

impl KeyValueTool {
    #[must_use]
    pub fn new() -> Self {
        Self {
            store: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }
}

impl Default for KeyValueTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ConversationTool for KeyValueTool {
    fn name(&self) -> &str {
        "key_value_store"
    }

    fn description(&self) -> &str {
        "Store and retrieve key-value pairs"
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["get", "set"],
                    "description": "Action to perform"
                },
                "key": {
                    "type": "string",
                    "description": "The key"
                },
                "value": {
                    "type": "string",
                    "description": "The value (for set action)"
                }
            },
            "required": ["action", "key"]
        })
    }

    async fn execute(&self, arguments: &serde_json::Value) -> ConversationResult<String> {
        let action = arguments["action"]
            .as_str()
            .ok_or_else(|| crate::error::ConversationError::ToolError("missing action".into()))?;
        let key = arguments["key"]
            .as_str()
            .ok_or_else(|| crate::error::ConversationError::ToolError("missing key".into()))?;

        match action {
            "get" => {
                let store = self.store.read().await;
                Ok(store
                    .get(key)
                    .cloned()
                    .unwrap_or_else(|| format!("key '{key}' not found")))
            }
            "set" => {
                let value = arguments["value"]
                    .as_str()
                    .ok_or_else(|| {
                        crate::error::ConversationError::ToolError("missing value for set".into())
                    })?;
                let mut store = self.store.write().await;
                store.insert(key.to_string(), value.to_string());
                Ok(format!("set '{key}' = '{value}'"))
            }
            _ => Err(crate::error::ConversationError::ToolError(format!(
                "unknown action: {action}"
            ))),
        }
    }
}
