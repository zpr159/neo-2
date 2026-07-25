use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::conversation::config::ToolConfig;
use crate::conversation::error::{ConversationError, ConversationResult};
use crate::conversation::executive_bridge::ExecutiveConversationBridge;
use crate::conversation::types::{ConversationContext, ToolAuthorization};
use crate::language::types::{FunctionDefinition, ToolDefinition};

/// Capability metadata for tool discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCapability {
    pub name: String,
    pub description: String,
    pub category: String,
    pub safe: bool,
    pub requires_approval: bool,
    pub estimated_cost: f64,
    pub tags: Vec<String>,
}

/// Definition of a tool available for execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinitionFull {
    pub capability: ToolCapability,
    pub schema: serde_json::Value,
    pub version: String,
    pub source: ToolSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolSource {
    Local,
    Remote,
    Distributed,
    Capability,
    Plugin,
    Workflow,
}

/// Request to execute a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionRequest {
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub timeout_ms: Option<u64>,
    pub retries: Option<u32>,
    pub chain_id: Option<String>,
}

/// Result from tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResult {
    pub tool_name: String,
    pub status: ToolExecutionStatus,
    pub output: serde_json::Value,
    pub execution_time_ms: u64,
    pub logs: Vec<String>,
    pub warnings: Vec<String>,
    pub confidence: f32,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolExecutionStatus {
    Success,
    Failure,
    Timeout,
    Cancelled,
    Unauthorized,
}

/// Definition of a function for function calling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinitionExt {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub return_type: Option<String>,
    pub version: String,
    pub async_execution: bool,
}

/// A function call request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCallRequest {
    pub function_name: String,
    pub arguments: HashMap<String, serde_json::Value>,
    pub call_id: String,
}

/// Result of a function call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCallResult {
    pub call_id: String,
    pub function_name: String,
    pub result: serde_json::Value,
    pub success: bool,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

/// A step in a tool chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolChainStep {
    pub step_index: usize,
    pub tool_name: String,
    pub input_mapping: std::collections::HashMap<String, String>,
    pub timeout_ms: Option<u64>,
}

/// A chain of tools to execute in sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolChain {
    pub id: String,
    pub name: String,
    pub steps: Vec<ToolChainStep>,
    pub description: String,
}

/// Result of executing a tool chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolChainResult {
    pub chain_id: String,
    pub step_results: Vec<ToolExecutionResult>,
    pub final_output: serde_json::Value,
    pub success: bool,
    pub total_time_ms: u64,
}

/// Trait for actually executing tools (provided by the tool runtime).
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    async fn execute(
        &self,
        tool_name: &str,
        arguments: &serde_json::Value,
        timeout_ms: u64,
    ) -> ConversationResult<ToolExecutionResult>;
}

/// Coordinates tool discovery, validation, execution, and chaining.
pub struct ToolCoordinator {
    config: ToolConfig,
    registered_tools: tokio::sync::RwLock<HashMap<String, ToolDefinitionFull>>,
    function_registry: tokio::sync::RwLock<HashMap<String, FunctionDefinitionExt>>,
    executor: Arc<dyn ToolExecutor>,
}

impl ToolCoordinator {
    pub fn new(config: ToolConfig, executor: Arc<dyn ToolExecutor>) -> Self {
        Self {
            config,
            registered_tools: tokio::sync::RwLock::new(HashMap::new()),
            function_registry: tokio::sync::RwLock::new(HashMap::new()),
            executor,
        }
    }

    /// Discover available tools matching a query.
    pub async fn discover_tools(&self, query: &str) -> Vec<ToolCapability> {
        let tools = self.registered_tools.read().await;
        let query_lower = query.to_lowercase();
        tools
            .values()
            .filter(|t| {
                t.capability.name.to_lowercase().contains(&query_lower)
                    || t.capability.description.to_lowercase().contains(&query_lower)
                    || t.capability.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .map(|t| t.capability.clone())
            .collect()
    }

    /// Register a tool.
    pub async fn register_tool(&self, tool: ToolDefinitionFull) {
        let name = tool.capability.name.clone();
        self.registered_tools.write().await.insert(name, tool);
    }

    /// Register a function for function calling.
    pub async fn register_function(&self, function: FunctionDefinitionExt) {
        let name = function.name.clone();
        self.function_registry.write().await.insert(name, function);
    }

    /// Validate tool permissions through the Executive.
    pub async fn validate_permissions(
        &self,
        context: &ConversationContext,
        tool_name: &str,
        arguments: &serde_json::Value,
        executive: &dyn ExecutiveConversationBridge,
    ) -> ConversationResult<ToolAuthorization> {
        let tools = self.registered_tools.read().await;
        if let Some(tool) = tools.get(tool_name) {
            if self.config.deny_patterns.iter().any(|p| tool_name.contains(p.as_str())) {
                return Ok(ToolAuthorization::Denied);
            }
            if !tool.capability.safe && self.config.auto_approve_safe_tools {
                return executive.authorize_tool(context, tool_name, arguments).await;
            }
        }
        drop(tools);
        executive.authorize_tool(context, tool_name, arguments).await
    }

    /// Execute a tool with validation.
    pub async fn execute_tool(
        &self,
        context: &ConversationContext,
        request: &ToolExecutionRequest,
        executive: &dyn ExecutiveConversationBridge,
    ) -> ConversationResult<ToolExecutionResult> {
        let auth = self.validate_permissions(
            context,
            &request.tool_name,
            &request.arguments,
            executive,
        )
        .await?;

        match auth {
            ToolAuthorization::Denied => {
                return Err(ConversationError::ToolAuthorizationDenied(
                    format!("Tool '{}' denied by executive", request.tool_name),
                ));
            }
            ToolAuthorization::RequireApproval => {
                // In a real implementation, this would prompt the user.
                // For now, we proceed.
            }
            ToolAuthorization::Auto => {}
        }

        let timeout = request
            .timeout_ms
            .unwrap_or(self.config.default_timeout_ms);

        self.executor
            .execute(&request.tool_name, &request.arguments, timeout)
            .await
    }

    /// Execute a chain of tools.
    pub async fn execute_chain(
        &self,
        context: &ConversationContext,
        chain: &ToolChain,
        initial_input: &serde_json::Value,
        executive: &dyn ExecutiveConversationBridge,
    ) -> ConversationResult<ToolChainResult> {
        let start = std::time::Instant::now();
        let mut step_results = Vec::new();
        let mut current_output = initial_input.clone();

        for step in &chain.steps {
            let mut arguments: std::collections::HashMap<String, String> = step.input_mapping.clone();
            // Map output from previous step
            for (key, mapping) in &step.input_mapping {
                if mapping == "$previous_output" {
                    if let Some(val) = current_output.get(key) {
                        arguments.insert(key.clone(), val.to_string());
                    }
                }
            }

            let request = ToolExecutionRequest {
                tool_name: step.tool_name.clone(),
                arguments: serde_json::to_value(&arguments)
                    .map_err(|e| ConversationError::ToolExecutionFailed(e.to_string()))?,
                timeout_ms: step.timeout_ms,
                retries: None,
                chain_id: Some(chain.id.clone()),
            };

            let result = self.execute_tool(context, &request, executive).await?;
            let success = result.status == ToolExecutionStatus::Success;
            step_results.push(result);

            if !success {
                return Ok(ToolChainResult {
                    chain_id: chain.id.clone(),
                    step_results,
                    final_output: current_output,
                    success: false,
                    total_time_ms: start.elapsed().as_millis() as u64,
                });
            }

            if let Some(last) = step_results.last() {
                current_output = last.output.clone();
            }
        }

        Ok(ToolChainResult {
            chain_id: chain.id.clone(),
            step_results,
            final_output: current_output,
            success: true,
            total_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Convert registered tools to LanguageEngine tool definitions.
    pub async fn to_tool_definitions(&self) -> Vec<ToolDefinition> {
        let tools = self.registered_tools.read().await;
        tools
            .values()
            .map(|t| ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: t.capability.name.clone(),
                    description: t.capability.description.clone(),
                    parameters: t.schema.clone(),
                },
            })
            .collect()
    }

    pub async fn list_functions(&self) -> Vec<FunctionDefinitionExt> {
        self.function_registry.read().await.values().cloned().collect()
    }
}
