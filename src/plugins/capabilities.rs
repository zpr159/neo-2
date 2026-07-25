use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Description of a tool exposed by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCapability {
    /// Unique tool name within the plugin.
    pub name: String,
    /// Human-readable description of what the tool does.
    pub description: String,
    /// JSON-Schema describing the tool's accepted parameters.
    pub parameters_schema: Option<serde_json::Value>,
    /// Whether the tool requires explicit user approval before execution.
    pub requires_approval: bool,
}

/// Description of a workflow exposed by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCapability {
    /// Unique workflow name within the plugin.
    pub name: String,
    /// Human-readable description of the workflow.
    pub description: String,
    /// Ordered list of accepted input type identifiers.
    pub input_types: Vec<String>,
    /// Ordered list of produced output type identifiers.
    pub output_types: Vec<String>,
}

/// Description of a model-provider exposed by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapability {
    /// Provider identifier (e.g. `"openai"`, `"anthropic"`).
    pub name: String,
    /// Model identifiers this provider supports.
    pub supported_models: Vec<String>,
    /// Feature flags or capability strings advertised by the provider.
    pub features: Vec<String>,
}

/// Aggregate of every capability a single plugin may advertise.
///
/// Each field is a map from a capability key to its descriptor, allowing
/// efficient lookup by name at runtime.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginCapabilities {
    /// Tools provided by the plugin.
    pub tools: HashMap<String, ToolCapability>,
    /// Workflows provided by the plugin.
    pub workflows: HashMap<String, WorkflowCapability>,
    /// Model providers provided by the plugin.
    pub providers: HashMap<String, ProviderCapability>,
    /// Prompt templates provided by the plugin.
    pub prompt_templates: HashMap<String, String>,
    /// Retriever implementations provided by the plugin.
    pub retrievers: HashMap<String, String>,
    /// Planner implementations provided by the plugin.
    pub planners: HashMap<String, String>,
    /// Free-form custom capabilities keyed by name.
    pub custom: HashMap<String, serde_json::Value>,
}
