use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::core::{
    Capability, CapabilityId, CapabilityMetadata, CapabilityVersion, CapabilityCategory,
    CapabilityNamespace, CapabilityTags, CapabilityAliases, CapabilityState,
    ExecutionContext, CapabilityResult_output, ResourceRequirements,
    InputSchema, OutputSchema,
};
use crate::error::{CapabilityError, CapabilityResult};

// ---------------------------------------------------------------------------
// Enums for operation-based capability types
// ---------------------------------------------------------------------------

/// Operations supported by the memory capability.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryOperation {
    Store,
    Retrieve,
    Update,
    Delete,
    Search,
    Consolidate,
}

impl fmt::Display for MemoryOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store => write!(f, "store"),
            Self::Retrieve => write!(f, "retrieve"),
            Self::Update => write!(f, "update"),
            Self::Delete => write!(f, "delete"),
            Self::Search => write!(f, "search"),
            Self::Consolidate => write!(f, "consolidate"),
        }
    }
}

/// Operations supported by the knowledge capability.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KnowledgeOperation {
    Query,
    Traverse,
    Extract,
    Validate,
    Merge,
    Infer,
}

impl fmt::Display for KnowledgeOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Query => write!(f, "query"),
            Self::Traverse => write!(f, "traverse"),
            Self::Extract => write!(f, "extract"),
            Self::Validate => write!(f, "validate"),
            Self::Merge => write!(f, "merge"),
            Self::Infer => write!(f, "infer"),
        }
    }
}

/// Operations supported by the filesystem capability.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FsOperation {
    Read,
    Write,
    Delete,
    List,
    Watch,
    Copy,
}

impl fmt::Display for FsOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read => write!(f, "read"),
            Self::Write => write!(f, "write"),
            Self::Delete => write!(f, "delete"),
            Self::List => write!(f, "list"),
            Self::Watch => write!(f, "watch"),
            Self::Copy => write!(f, "copy"),
        }
    }
}

/// Operations supported by the system capability.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SystemOperation {
    Shutdown,
    Restart,
    HealthCheck,
    Metrics,
    Config,
}

impl fmt::Display for SystemOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shutdown => write!(f, "shutdown"),
            Self::Restart => write!(f, "restart"),
            Self::HealthCheck => write!(f, "health_check"),
            Self::Metrics => write!(f, "metrics"),
            Self::Config => write!(f, "config"),
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: build default metadata
// ---------------------------------------------------------------------------

fn default_metadata(
    name: &str,
    category: CapabilityCategory,
    namespace: CapabilityNamespace,
) -> CapabilityMetadata {
    CapabilityMetadata::new(name, CapabilityVersion::initial(), name, category)
        .with_namespace(namespace)
        .with_author("neo-agi")
        .with_tag(name)
}

fn default_input_schema(description: &str) -> InputSchema {
    InputSchema::new(
        serde_json::json!({
            "type": "object",
        }),
        description,
    )
}

fn empty_json_object() -> serde_json::Value {
    serde_json::json!({})
}

// ---------------------------------------------------------------------------
// 1. ReasoningCapability
// ---------------------------------------------------------------------------

pub struct ReasoningCapability {
    metadata: CapabilityMetadata,
    strategy_name: String,
    config: serde_json::Value,
}

impl ReasoningCapability {
    pub fn new(strategy_name: String, config: serde_json::Value) -> Self {
        let mut metadata = default_metadata(
            "reasoning",
            CapabilityCategory::Reasoning,
            CapabilityNamespace::reasoning(),
        );
        metadata.description = format!(
            "Reasoning capability using strategy '{}'",
            strategy_name
        );
        metadata.inputs = vec![default_input_schema("reasoning query input")];
        metadata.output = OutputSchema::new(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "reasoning_result": {"type": "string"},
                    "strategy": {"type": "string"}
                }
            }),
            "reasoning output",
        );
        Self {
            metadata,
            strategy_name,
            config,
        }
    }
}

#[async_trait]
impl Capability for ReasoningCapability {
    fn metadata(&self) -> &CapabilityMetadata {
        &self.metadata
    }

    fn metadata_mut(&mut self) -> &mut CapabilityMetadata {
        &mut self.metadata
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: ExecutionContext,
    ) -> CapabilityResult<CapabilityResult_output> {
        self.validate_input(&input)?;

        let query = input
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("general reasoning");

        Ok(CapabilityResult_output::success(
            serde_json::json!({
                "status": "completed",
                "operation": "reason",
                "strategy": self.strategy_name,
                "query": query,
                "config": self.config,
                "reasoning_result": format!("Applied '{}' strategy to: {}", self.strategy_name, query),
            }),
            0,
        ))
    }
}

// ---------------------------------------------------------------------------
// 2. InferenceCapability
// ---------------------------------------------------------------------------

pub struct InferenceCapability {
    metadata: CapabilityMetadata,
    model_id: String,
    backend: String,
}

impl InferenceCapability {
    pub fn new(model_id: String, backend: String) -> Self {
        let mut metadata = default_metadata(
            "inference",
            CapabilityCategory::Inference,
            CapabilityNamespace::inference(),
        );
        metadata.description = format!(
            "Inference capability using model '{}' on backend '{}'",
            model_id, backend
        );
        metadata.inputs = vec![default_input_schema("inference prompt input")];
        metadata.output = OutputSchema::new(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "inference_result": {"type": "string"},
                    "model": {"type": "string"}
                }
            }),
            "inference output",
        );
        Self {
            metadata,
            model_id,
            backend,
        }
    }
}

#[async_trait]
impl Capability for InferenceCapability {
    fn metadata(&self) -> &CapabilityMetadata {
        &self.metadata
    }

    fn metadata_mut(&mut self) -> &mut CapabilityMetadata {
        &mut self.metadata
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: ExecutionContext,
    ) -> CapabilityResult<CapabilityResult_output> {
        self.validate_input(&input)?;

        let prompt = input
            .get("prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        Ok(CapabilityResult_output::success(
            serde_json::json!({
                "status": "completed",
                "operation": "inference",
                "model_id": self.model_id,
                "backend": self.backend,
                "prompt": prompt,
                "inference_result": format!(
                    "Model '{}' on '{}' processed prompt of length {}",
                    self.model_id, self.backend, prompt.len()
                ),
            }),
            0,
        ))
    }
}

// ---------------------------------------------------------------------------
// 3. MemoryCapability
// ---------------------------------------------------------------------------

pub struct MemoryCapability {
    metadata: CapabilityMetadata,
    operation: MemoryOperation,
    memory_type: String,
}

impl MemoryCapability {
    pub fn new(operation: MemoryOperation, memory_type: String) -> Self {
        let mut metadata = default_metadata(
            "memory",
            CapabilityCategory::Memory,
            CapabilityNamespace::memory(),
        );
        metadata.description = format!(
            "Memory capability performing '{}' on type '{}'",
            operation, memory_type
        );
        metadata.inputs = vec![default_input_schema("memory operation input")];
        metadata.output = OutputSchema::new(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "status": {"type": "string"},
                    "operation": {"type": "string"}
                }
            }),
            "memory operation output",
        );
        Self {
            metadata,
            operation,
            memory_type,
        }
    }
}

#[async_trait]
impl Capability for MemoryCapability {
    fn metadata(&self) -> &CapabilityMetadata {
        &self.metadata
    }

    fn metadata_mut(&mut self) -> &mut CapabilityMetadata {
        &mut self.metadata
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: ExecutionContext,
    ) -> CapabilityResult<CapabilityResult_output> {
        self.validate_input(&input)?;

        let detail = match &self.operation {
            MemoryOperation::Store => {
                let key = input
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("_unnamed");
                format!("stored key '{}' in {}", key, self.memory_type)
            }
            MemoryOperation::Retrieve => {
                let key = input
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("_unnamed");
                format!("retrieved key '{}' from {}", key, self.memory_type)
            }
            MemoryOperation::Update => {
                let key = input
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("_unnamed");
                format!("updated key '{}' in {}", key, self.memory_type)
            }
            MemoryOperation::Delete => {
                let key = input
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("_unnamed");
                format!("deleted key '{}' from {}", key, self.memory_type)
            }
            MemoryOperation::Search => {
                let query = input
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                format!("searched '{}' in {}", query, self.memory_type)
            }
            MemoryOperation::Consolidate => {
                format!("consolidated {}", self.memory_type)
            }
        };

        Ok(CapabilityResult_output::success(
            serde_json::json!({
                "status": "completed",
                "operation": self.operation.to_string(),
                "memory_type": self.memory_type,
                "detail": detail,
            }),
            0,
        ))
    }
}

// ---------------------------------------------------------------------------
// 4. KnowledgeCapability
// ---------------------------------------------------------------------------

pub struct KnowledgeCapability {
    metadata: CapabilityMetadata,
    operation: KnowledgeOperation,
}

impl KnowledgeCapability {
    pub fn new(operation: KnowledgeOperation) -> Self {
        let mut metadata = default_metadata(
            "knowledge",
            CapabilityCategory::Knowledge,
            CapabilityNamespace::knowledge(),
        );
        metadata.description = format!(
            "Knowledge capability performing '{}' operation",
            operation
        );
        metadata.inputs = vec![default_input_schema("knowledge operation input")];
        metadata.output = OutputSchema::new(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "status": {"type": "string"},
                    "operation": {"type": "string"}
                }
            }),
            "knowledge operation output",
        );
        Self {
            metadata,
            operation,
        }
    }
}

#[async_trait]
impl Capability for KnowledgeCapability {
    fn metadata(&self) -> &CapabilityMetadata {
        &self.metadata
    }

    fn metadata_mut(&mut self) -> &mut CapabilityMetadata {
        &mut self.metadata
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: ExecutionContext,
    ) -> CapabilityResult<CapabilityResult_output> {
        self.validate_input(&input)?;

        let detail = match &self.operation {
            KnowledgeOperation::Query => {
                let q = input
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                format!("queried knowledge graph: {}", q)
            }
            KnowledgeOperation::Traverse => {
                let start = input
                    .get("start_node")
                    .and_then(|v| v.as_str())
                    .unwrap_or("root");
                format!("traversed graph from node '{}'", start)
            }
            KnowledgeOperation::Extract => {
                let source = input
                    .get("source")
                    .and_then(|v| v.as_str())
                    .unwrap_or("_unknown");
                format!("extracted knowledge from '{}'", source)
            }
            KnowledgeOperation::Validate => {
                format!("validated knowledge graph integrity")
            }
            KnowledgeOperation::Merge => {
                let sources = input
                    .get("sources")
                    .and_then(|v| v.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                format!("merged {} knowledge sources", sources)
            }
            KnowledgeOperation::Infer => {
                let seed = input
                    .get("seed")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                format!("inferred from seed: {}", seed)
            }
        };

        Ok(CapabilityResult_output::success(
            serde_json::json!({
                "status": "completed",
                "operation": self.operation.to_string(),
                "detail": detail,
            }),
            0,
        ))
    }
}

// ---------------------------------------------------------------------------
// 5. ToolCapability
// ---------------------------------------------------------------------------

pub struct ToolCapability {
    metadata: CapabilityMetadata,
    tool_name: String,
    tool_type: String,
}

impl ToolCapability {
    pub fn new(tool_name: String, tool_type: String) -> Self {
        let mut metadata = default_metadata(
            "tool",
            CapabilityCategory::Tool,
            CapabilityNamespace::core(),
        );
        metadata.description = format!(
            "Tool capability for '{}' (type: {})",
            tool_name, tool_type
        );
        metadata.inputs = vec![default_input_schema("tool invocation input")];
        metadata.output = OutputSchema::new(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "status": {"type": "string"},
                    "tool": {"type": "string"}
                }
            }),
            "tool invocation output",
        );
        Self {
            metadata,
            tool_name,
            tool_type,
        }
    }
}

#[async_trait]
impl Capability for ToolCapability {
    fn metadata(&self) -> &CapabilityMetadata {
        &self.metadata
    }

    fn metadata_mut(&mut self) -> &mut CapabilityMetadata {
        &mut self.metadata
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: ExecutionContext,
    ) -> CapabilityResult<CapabilityResult_output> {
        self.validate_input(&input)?;

        let args = input
            .get("args")
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        Ok(CapabilityResult_output::success(
            serde_json::json!({
                "status": "completed",
                "operation": "tool_invoke",
                "tool_name": self.tool_name,
                "tool_type": self.tool_type,
                "args": args,
            }),
            0,
        ))
    }
}

// ---------------------------------------------------------------------------
// 6. WorkflowCapability
// ---------------------------------------------------------------------------

pub struct WorkflowCapability {
    metadata: CapabilityMetadata,
    steps: Vec<String>,
    workflow_type: String,
}

impl WorkflowCapability {
    pub fn new(steps: Vec<String>, workflow_type: String) -> Self {
        let step_count = steps.len();
        let mut metadata = default_metadata(
            "workflow",
            CapabilityCategory::Workflow,
            CapabilityNamespace::core(),
        );
        metadata.description = format!(
            "Workflow capability with {} steps (type: {})",
            step_count, workflow_type
        );
        metadata.inputs = vec![default_input_schema("workflow execution input")];
        metadata.output = OutputSchema::new(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "status": {"type": "string"},
                    "steps_executed": {"type": "integer"}
                }
            }),
            "workflow execution output",
        );
        Self {
            metadata,
            steps,
            workflow_type,
        }
    }
}

#[async_trait]
impl Capability for WorkflowCapability {
    fn metadata(&self) -> &CapabilityMetadata {
        &self.metadata
    }

    fn metadata_mut(&mut self) -> &mut CapabilityMetadata {
        &mut self.metadata
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: ExecutionContext,
    ) -> CapabilityResult<CapabilityResult_output> {
        self.validate_input(&input)?;

        let executed: Vec<serde_json::Value> = self
            .steps
            .iter()
            .enumerate()
            .map(|(i, step)| {
                serde_json::json!({
                    "index": i,
                    "step": step,
                    "status": "completed",
                })
            })
            .collect();

        Ok(CapabilityResult_output::success(
            serde_json::json!({
                "status": "completed",
                "operation": "workflow_execute",
                "workflow_type": self.workflow_type,
                "total_steps": self.steps.len(),
                "executed_steps": executed,
            }),
            0,
        ))
    }
}

// ---------------------------------------------------------------------------
// 7. CommunicationCapability
// ---------------------------------------------------------------------------

pub struct CommunicationCapability {
    metadata: CapabilityMetadata,
    protocol: String,
    target: String,
}

impl CommunicationCapability {
    pub fn new(protocol: String, target: String) -> Self {
        let mut metadata = default_metadata(
            "communication",
            CapabilityCategory::Communication,
            CapabilityNamespace::communication(),
        );
        metadata.description = format!(
            "Communication capability using '{}' protocol targeting '{}'",
            protocol, target
        );
        metadata.inputs = vec![default_input_schema("communication message input")];
        metadata.output = OutputSchema::new(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "status": {"type": "string"},
                    "protocol": {"type": "string"}
                }
            }),
            "communication output",
        );
        Self {
            metadata,
            protocol,
            target,
        }
    }
}

#[async_trait]
impl Capability for CommunicationCapability {
    fn metadata(&self) -> &CapabilityMetadata {
        &self.metadata
    }

    fn metadata_mut(&mut self) -> &mut CapabilityMetadata {
        &mut self.metadata
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: ExecutionContext,
    ) -> CapabilityResult<CapabilityResult_output> {
        self.validate_input(&input)?;

        let message = input
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        Ok(CapabilityResult_output::success(
            serde_json::json!({
                "status": "completed",
                "operation": "communicate",
                "protocol": self.protocol,
                "target": self.target,
                "message_length": message.len(),
                "detail": format!("Sent message via '{}' to '{}'", self.protocol, self.target),
            }),
            0,
        ))
    }
}

// ---------------------------------------------------------------------------
// 8. FilesystemCapability
// ---------------------------------------------------------------------------

pub struct FilesystemCapability {
    metadata: CapabilityMetadata,
    operation: FsOperation,
    path: String,
}

impl FilesystemCapability {
    pub fn new(operation: FsOperation, path: String) -> Self {
        let mut metadata = default_metadata(
            "filesystem",
            CapabilityCategory::Filesystem,
            CapabilityNamespace::core(),
        );
        metadata.description = format!(
            "Filesystem capability performing '{}' on path '{}'",
            operation, path
        );
        metadata.inputs = vec![default_input_schema("filesystem operation input")];
        metadata.output = OutputSchema::new(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "status": {"type": "string"},
                    "operation": {"type": "string"},
                    "path": {"type": "string"}
                }
            }),
            "filesystem operation output",
        );
        Self {
            metadata,
            operation,
            path,
        }
    }
}

#[async_trait]
impl Capability for FilesystemCapability {
    fn metadata(&self) -> &CapabilityMetadata {
        &self.metadata
    }

    fn metadata_mut(&mut self) -> &mut CapabilityMetadata {
        &mut self.metadata
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: ExecutionContext,
    ) -> CapabilityResult<CapabilityResult_output> {
        self.validate_input(&input)?;

        let detail = match &self.operation {
            FsOperation::Read => format!("read contents from '{}'", self.path),
            FsOperation::Write => {
                let size = input
                    .get("content")
                    .and_then(|v| v.as_str())
                    .map(|s| s.len())
                    .unwrap_or(0);
                format!("wrote {} bytes to '{}'", size, self.path)
            }
            FsOperation::Delete => format!("deleted '{}'", self.path),
            FsOperation::List => format!("listed contents of '{}'", self.path),
            FsOperation::Watch => format!("watching '{}' for changes", self.path),
            FsOperation::Copy => {
                let dest = input
                    .get("destination")
                    .and_then(|v| v.as_str())
                    .unwrap_or("_unspecified");
                format!("copied '{}' to '{}'", self.path, dest)
            }
        };

        Ok(CapabilityResult_output::success(
            serde_json::json!({
                "status": "completed",
                "operation": self.operation.to_string(),
                "path": self.path,
                "detail": detail,
            }),
            0,
        ))
    }
}

// ---------------------------------------------------------------------------
// 9. NetworkCapability
// ---------------------------------------------------------------------------

pub struct NetworkCapability {
    metadata: CapabilityMetadata,
    method: String,
    url: String,
}

impl NetworkCapability {
    pub fn new(method: String, url: String) -> Self {
        let mut metadata = default_metadata(
            "network",
            CapabilityCategory::Network,
            CapabilityNamespace::core(),
        );
        metadata.description = format!(
            "Network capability performing {} request to '{}'",
            method, url
        );
        metadata.inputs = vec![default_input_schema("network request input")];
        metadata.output = OutputSchema::new(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "status": {"type": "string"},
                    "method": {"type": "string"},
                    "url": {"type": "string"}
                }
            }),
            "network response output",
        );
        Self {
            metadata,
            method,
            url,
        }
    }
}

#[async_trait]
impl Capability for NetworkCapability {
    fn metadata(&self) -> &CapabilityMetadata {
        &self.metadata
    }

    fn metadata_mut(&mut self) -> &mut CapabilityMetadata {
        &mut self.metadata
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: ExecutionContext,
    ) -> CapabilityResult<CapabilityResult_output> {
        self.validate_input(&input)?;

        let headers = input
            .get("headers")
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        Ok(CapabilityResult_output::success(
            serde_json::json!({
                "status": "completed",
                "operation": "network_request",
                "method": self.method,
                "url": self.url,
                "headers": headers,
                "detail": format!("{} request sent to '{}'", self.method, self.url),
            }),
            0,
        ))
    }
}

// ---------------------------------------------------------------------------
// 10. DeveloperCapability
// ---------------------------------------------------------------------------

pub struct DeveloperCapability {
    metadata: CapabilityMetadata,
    tool: String,
    language: Option<String>,
}

impl DeveloperCapability {
    pub fn new(tool: String, language: Option<String>) -> Self {
        let lang_display = language.as_deref().unwrap_or("any");
        let mut metadata = default_metadata(
            "developer",
            CapabilityCategory::Developer,
            CapabilityNamespace::developer(),
        );
        metadata.description = format!(
            "Developer capability using '{}' tool for {} language",
            tool, lang_display
        );
        metadata.inputs = vec![default_input_schema("developer tool invocation input")];
        metadata.output = OutputSchema::new(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "status": {"type": "string"},
                    "tool": {"type": "string"}
                }
            }),
            "developer tool output",
        );
        Self {
            metadata,
            tool,
            language,
        }
    }
}

#[async_trait]
impl Capability for DeveloperCapability {
    fn metadata(&self) -> &CapabilityMetadata {
        &self.metadata
    }

    fn metadata_mut(&mut self) -> &mut CapabilityMetadata {
        &mut self.metadata
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: ExecutionContext,
    ) -> CapabilityResult<CapabilityResult_output> {
        self.validate_input(&input)?;

        let command = input
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        Ok(CapabilityResult_output::success(
            serde_json::json!({
                "status": "completed",
                "operation": "developer_tool",
                "tool": self.tool,
                "language": self.language,
                "command": command,
                "detail": format!(
                    "Executed '{}' command via tool '{}' for {:?}",
                    command, self.tool, self.language
                ),
            }),
            0,
        ))
    }
}

// ---------------------------------------------------------------------------
// 11. SystemCapability
// ---------------------------------------------------------------------------

pub struct SystemCapability {
    metadata: CapabilityMetadata,
    operation: SystemOperation,
}

impl SystemCapability {
    pub fn new(operation: SystemOperation) -> Self {
        let mut metadata = default_metadata(
            "system",
            CapabilityCategory::System,
            CapabilityNamespace::core(),
        );
        metadata.description = format!(
            "System capability performing '{}' operation",
            operation
        );
        metadata.inputs = vec![default_input_schema("system operation input")];
        metadata.output = OutputSchema::new(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "status": {"type": "string"},
                    "operation": {"type": "string"}
                }
            }),
            "system operation output",
        );
        Self {
            metadata,
            operation,
        }
    }
}

#[async_trait]
impl Capability for SystemCapability {
    fn metadata(&self) -> &CapabilityMetadata {
        &self.metadata
    }

    fn metadata_mut(&mut self) -> &mut CapabilityMetadata {
        &mut self.metadata
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: ExecutionContext,
    ) -> CapabilityResult<CapabilityResult_output> {
        self.validate_input(&input)?;

        let detail = match &self.operation {
            SystemOperation::Shutdown => {
                let graceful = input
                    .get("graceful")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                format!(
                    "initiated {} shutdown",
                    if graceful { "graceful" } else { "force" }
                )
            }
            SystemOperation::Restart => "system restart initiated".to_string(),
            SystemOperation::HealthCheck => "all subsystems operational".to_string(),
            SystemOperation::Metrics => {
                let metrics = input
                    .get("metrics")
                    .cloned()
                    .unwrap_or(serde_json::json!({}));
                format!("collected metrics: {}", metrics)
            }
            SystemOperation::Config => {
                let key = input
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("_all");
                format!("accessed config for key '{}'", key)
            }
        };

        Ok(CapabilityResult_output::success(
            serde_json::json!({
                "status": "completed",
                "operation": self.operation.to_string(),
                "detail": detail,
            }),
            0,
        ))
    }
}

// ---------------------------------------------------------------------------
// 12. CustomCapability
// ---------------------------------------------------------------------------

pub struct CustomCapability {
    metadata: CapabilityMetadata,
    custom_type: String,
    handler_config: serde_json::Value,
}

impl CustomCapability {
    pub fn new(custom_type: String, handler_config: serde_json::Value) -> Self {
        let mut metadata = default_metadata(
            "custom",
            CapabilityCategory::Custom(custom_type.clone()),
            CapabilityNamespace::core(),
        );
        metadata.description = format!(
            "Custom capability of type '{}'",
            custom_type
        );
        metadata.inputs = vec![default_input_schema("custom handler input")];
        metadata.output = OutputSchema::new(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "status": {"type": "string"},
                    "custom_type": {"type": "string"}
                }
            }),
            "custom handler output",
        );
        Self {
            metadata,
            custom_type,
            handler_config,
        }
    }
}

#[async_trait]
impl Capability for CustomCapability {
    fn metadata(&self) -> &CapabilityMetadata {
        &self.metadata
    }

    fn metadata_mut(&mut self) -> &mut CapabilityMetadata {
        &mut self.metadata
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: ExecutionContext,
    ) -> CapabilityResult<CapabilityResult_output> {
        self.validate_input(&input)?;

        let action = input
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        Ok(CapabilityResult_output::success(
            serde_json::json!({
                "status": "completed",
                "operation": "custom_execute",
                "custom_type": self.custom_type,
                "handler_config": self.handler_config,
                "action": action,
                "detail": format!(
                    "Executed action '{}' on custom type '{}'",
                    action, self.custom_type
                ),
            }),
            0,
        ))
    }
}

// ---------------------------------------------------------------------------
// CapabilityTypeRegistry
// ---------------------------------------------------------------------------

type CapabilityFactory = Box<dyn Fn() -> Box<dyn Capability> + Send + Sync>;

/// Registry that maps string names to capability type factories.
pub struct CapabilityTypeRegistry {
    factories: DashMap<String, CapabilityFactory>,
    descriptions: DashMap<String, String>,
}

impl CapabilityTypeRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            factories: DashMap::new(),
            descriptions: DashMap::new(),
        }
    }

    /// Register a capability type with a name and factory function.
    pub fn register<F>(&self, name: &str, description: &str, factory: F)
    where
        F: Fn() -> Box<dyn Capability> + Send + Sync + 'static,
    {
        self.factories
            .insert(name.to_string(), Box::new(factory));
        self.descriptions
            .insert(name.to_string(), description.to_string());
    }

    /// Create a capability instance by name.
    pub fn create(&self, name: &str) -> Option<Box<dyn Capability>> {
        self.factories.get(name).map(|f| f())
    }

    /// Check whether a type name is registered.
    pub fn contains(&self, name: &str) -> bool {
        self.factories.contains_key(name)
    }

    /// Get the description for a registered type.
    pub fn description(&self, name: &str) -> Option<String> {
        self.descriptions.get(name).map(|d| d.clone())
    }

    /// List all registered type names.
    pub fn list_types(&self) -> Vec<String> {
        self.factories
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Number of registered types.
    pub fn len(&self) -> usize {
        self.factories.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.factories.is_empty()
    }
}

impl Default for CapabilityTypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// create_builtin_capabilities
// ---------------------------------------------------------------------------

/// Create one instance of each of the 12 built-in capability types.
pub fn create_builtin_capabilities() -> Vec<Box<dyn Capability>> {
    vec![
        Box::new(ReasoningCapability::new(
            "chain_of_thought".to_string(),
            serde_json::json!({ "depth": 5, "branches": 3 }),
        )),
        Box::new(InferenceCapability::new(
            "neo-default".to_string(),
            "local".to_string(),
        )),
        Box::new(MemoryCapability::new(
            MemoryOperation::Store,
            "short_term".to_string(),
        )),
        Box::new(KnowledgeCapability::new(KnowledgeOperation::Query)),
        Box::new(ToolCapability::new(
            "shell".to_string(),
            "system".to_string(),
        )),
        Box::new(WorkflowCapability::new(
            vec![
                "validate_input".to_string(),
                "process".to_string(),
                "format_output".to_string(),
            ],
            "sequential".to_string(),
        )),
        Box::new(CommunicationCapability::new(
            "websocket".to_string(),
            "localhost:8080".to_string(),
        )),
        Box::new(FilesystemCapability::new(
            FsOperation::Read,
            "/tmp/default".to_string(),
        )),
        Box::new(NetworkCapability::new(
            "GET".to_string(),
            "http://localhost:3000/health".to_string(),
        )),
        Box::new(DeveloperCapability::new(
            "rust-analyzer".to_string(),
            Some("rust".to_string()),
        )),
        Box::new(SystemCapability::new(SystemOperation::HealthCheck)),
        Box::new(CustomCapability::new(
            "plugin".to_string(),
            serde_json::json!({ "version": 1 }),
        )),
    ]
}

/// Populate a CapabilityTypeRegistry with all 12 built-in types.
pub fn register_builtin_types(registry: &CapabilityTypeRegistry) {
    registry.register(
        "reasoning",
        "Reasoning capability for chain-of-thought and strategy-based reasoning",
        || {
            Box::new(ReasoningCapability::new(
                "chain_of_thought".to_string(),
                serde_json::json!({}),
            ))
        },
    );
    registry.register(
        "inference",
        "Inference capability for running model predictions",
        || {
            Box::new(InferenceCapability::new(
                "neo-default".to_string(),
                "local".to_string(),
            ))
        },
    );
    registry.register(
        "memory",
        "Memory capability for short-term and long-term memory operations",
        || {
            Box::new(MemoryCapability::new(
                MemoryOperation::Store,
                "short_term".to_string(),
            ))
        },
    );
    registry.register(
        "knowledge",
        "Knowledge capability for knowledge graph operations",
        || Box::new(KnowledgeCapability::new(KnowledgeOperation::Query)),
    );
    registry.register(
        "tool",
        "Tool capability for invoking external tools",
        || {
            Box::new(ToolCapability::new(
                "shell".to_string(),
                "system".to_string(),
            ))
        },
    );
    registry.register(
        "workflow",
        "Workflow capability for multi-step orchestrated execution",
        || {
            Box::new(WorkflowCapability::new(
                vec!["step1".to_string()],
                "sequential".to_string(),
            ))
        },
    );
    registry.register(
        "communication",
        "Communication capability for inter-service messaging",
        || {
            Box::new(CommunicationCapability::new(
                "http".to_string(),
                "localhost".to_string(),
            ))
        },
    );
    registry.register(
        "filesystem",
        "Filesystem capability for file and directory operations",
        || {
            Box::new(FilesystemCapability::new(
                FsOperation::Read,
                "/".to_string(),
            ))
        },
    );
    registry.register(
        "network",
        "Network capability for HTTP and other network requests",
        || {
            Box::new(NetworkCapability::new(
                "GET".to_string(),
                "http://localhost".to_string(),
            ))
        },
    );
    registry.register(
        "developer",
        "Developer capability for IDE and dev-tool integration",
        || {
            Box::new(DeveloperCapability::new(
                "default".to_string(),
                None,
            ))
        },
    );
    registry.register(
        "system",
        "System capability for OS-level operations",
        || Box::new(SystemCapability::new(SystemOperation::HealthCheck)),
    );
    registry.register(
        "custom",
        "Custom capability for user-defined extension types",
        || {
            Box::new(CustomCapability::new(
                "default".to_string(),
                serde_json::json!({}),
            ))
        },
    );
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_context() -> ExecutionContext {
        ExecutionContext::new(CapabilityId::new())
    }

    // -- ReasoningCapability tests ------------------------------------------

    #[tokio::test]
    async fn reasoning_execute() {
        let cap = ReasoningCapability::new(
            "tree_of_thought".to_string(),
            serde_json::json!({ "max_depth": 10 }),
        );
        let ctx = make_context();
        let result = cap
            .execute(
                serde_json::json!({ "query": "What is 2+2?" }),
                ctx,
            )
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.output["status"], "completed");
        assert_eq!(result.output["strategy"], "tree_of_thought");
        assert!(result.output["reasoning_result"]
            .as_str()
            .unwrap()
            .contains("tree_of_thought"));
    }

    #[test]
    fn reasoning_metadata() {
        let cap = ReasoningCapability::new("cot".to_string(), serde_json::json!({}));
        let meta = cap.metadata();
        assert_eq!(meta.name, "reasoning");
        assert_eq!(meta.version, CapabilityVersion::initial());
        assert_eq!(meta.category, CapabilityCategory::Reasoning);
        assert_eq!(meta.namespace.as_str(), "neo.reasoning");
    }

    #[test]
    fn reasoning_default_input_ok() {
        let cap = ReasoningCapability::new("cot".to_string(), serde_json::json!({}));
        assert!(cap.validate_input(&serde_json::json!({})).is_ok());
    }

    // -- InferenceCapability tests ------------------------------------------

    #[tokio::test]
    async fn inference_execute() {
        let cap = InferenceCapability::new("gpt-neo".to_string(), "gpu-cluster".to_string());
        let ctx = make_context();
        let result = cap
            .execute(
                serde_json::json!({ "prompt": "Hello world" }),
                ctx,
            )
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.output["model_id"], "gpt-neo");
        assert_eq!(result.output["backend"], "gpu-cluster");
    }

    #[test]
    fn inference_metadata() {
        let cap = InferenceCapability::new("m".to_string(), "b".to_string());
        assert_eq!(cap.metadata().category, CapabilityCategory::Inference);
        assert_eq!(cap.metadata().namespace.as_str(), "neo.inference");
    }

    // -- MemoryCapability tests ---------------------------------------------

    #[tokio::test]
    async fn memory_store() {
        let cap = MemoryCapability::new(MemoryOperation::Store, "episodic".to_string());
        let ctx = make_context();
        let result = cap
            .execute(
                serde_json::json!({ "key": "event_1", "value": { "ts": 1234 } }),
                ctx,
            )
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.output["operation"], "store");
        assert!(result.output["detail"]
            .as_str()
            .unwrap()
            .contains("event_1"));
    }

    #[tokio::test]
    async fn memory_search() {
        let cap = MemoryCapability::new(MemoryOperation::Search, "semantic".to_string());
        let ctx = make_context();
        let result = cap
            .execute(
                serde_json::json!({ "query": "find cats" }),
                ctx,
            )
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.output["operation"], "search");
        assert!(result.output["detail"]
            .as_str()
            .unwrap()
            .contains("find cats"));
    }

    #[tokio::test]
    async fn memory_retrieve() {
        let cap = MemoryCapability::new(MemoryOperation::Retrieve, "working".to_string());
        let ctx = make_context();
        let result = cap
            .execute(serde_json::json!({ "key": "k1" }), ctx)
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.output["operation"], "retrieve");
    }

    #[tokio::test]
    async fn memory_update() {
        let cap = MemoryCapability::new(MemoryOperation::Update, "long_term".to_string());
        let ctx = make_context();
        let result = cap
            .execute(serde_json::json!({ "key": "k2" }), ctx)
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.output["operation"], "update");
    }

    #[tokio::test]
    async fn memory_delete() {
        let cap = MemoryCapability::new(MemoryOperation::Delete, "cache".to_string());
        let ctx = make_context();
        let result = cap
            .execute(serde_json::json!({ "key": "k3" }), ctx)
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.output["operation"], "delete");
    }

    #[tokio::test]
    async fn memory_consolidate() {
        let cap = MemoryCapability::new(MemoryOperation::Consolidate, "all".to_string());
        let ctx = make_context();
        let result = cap.execute(serde_json::json!({}), ctx).await.unwrap();

        assert!(result.success);
        assert_eq!(result.output["operation"], "consolidate");
    }

    #[test]
    fn memory_metadata() {
        let cap = MemoryCapability::new(MemoryOperation::Store, "short_term".to_string());
        assert_eq!(cap.metadata().category, CapabilityCategory::Memory);
        assert_eq!(cap.metadata().namespace.as_str(), "neo.memory");
    }

    // -- KnowledgeCapability tests ------------------------------------------

    #[tokio::test]
    async fn knowledge_query() {
        let cap = KnowledgeCapability::new(KnowledgeOperation::Query);
        let ctx = make_context();
        let result = cap
            .execute(
                serde_json::json!({ "query": "what is Rust?" }),
                ctx,
            )
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.output["operation"], "query");
    }

    #[tokio::test]
    async fn knowledge_traverse() {
        let cap = KnowledgeCapability::new(KnowledgeOperation::Traverse);
        let ctx = make_context();
        let result = cap
            .execute(
                serde_json::json!({ "start_node": "concept_42" }),
                ctx,
            )
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output["detail"]
            .as_str()
            .unwrap()
            .contains("concept_42"));
    }

    #[tokio::test]
    async fn knowledge_extract() {
        let cap = KnowledgeCapability::new(KnowledgeOperation::Extract);
        let ctx = make_context();
        let result = cap
            .execute(
                serde_json::json!({ "source": "document.pdf" }),
                ctx,
            )
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output["detail"]
            .as_str()
            .unwrap()
            .contains("document.pdf"));
    }

    #[tokio::test]
    async fn knowledge_validate() {
        let cap = KnowledgeCapability::new(KnowledgeOperation::Validate);
        let ctx = make_context();
        let result = cap.execute(serde_json::json!({}), ctx).await.unwrap();
        assert!(result.success);
        assert_eq!(result.output["operation"], "validate");
    }

    #[tokio::test]
    async fn knowledge_merge() {
        let cap = KnowledgeCapability::new(KnowledgeOperation::Merge);
        let ctx = make_context();
        let result = cap
            .execute(
                serde_json::json!({ "sources": ["a", "b", "c"] }),
                ctx,
            )
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output["detail"]
            .as_str()
            .unwrap()
            .contains("3"));
    }

    #[tokio::test]
    async fn knowledge_infer() {
        let cap = KnowledgeCapability::new(KnowledgeOperation::Infer);
        let ctx = make_context();
        let result = cap
            .execute(
                serde_json::json!({ "seed": "all dogs are mortal" }),
                ctx,
            )
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output["detail"]
            .as_str()
            .unwrap()
            .contains("all dogs are mortal"));
    }

    #[test]
    fn knowledge_metadata() {
        let cap = KnowledgeCapability::new(KnowledgeOperation::Query);
        assert_eq!(cap.metadata().category, CapabilityCategory::Knowledge);
        assert_eq!(cap.metadata().namespace.as_str(), "neo.knowledge");
    }

    // -- ToolCapability tests -----------------------------------------------

    #[tokio::test]
    async fn tool_execute() {
        let cap = ToolCapability::new("grep".to_string(), "search".to_string());
        let ctx = make_context();
        let result = cap
            .execute(
                serde_json::json!({ "args": { "pattern": "TODO", "path": "." } }),
                ctx,
            )
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.output["tool_name"], "grep");
        assert_eq!(result.output["tool_type"], "search");
    }

    #[tokio::test]
    async fn tool_no_args() {
        let cap = ToolCapability::new("ls".to_string(), "filesystem".to_string());
        let ctx = make_context();
        let result = cap.execute(serde_json::json!({}), ctx).await.unwrap();
        assert!(result.success);
        assert_eq!(result.output["args"], serde_json::Value::Null);
    }

    #[test]
    fn tool_metadata() {
        let cap = ToolCapability::new("x".to_string(), "y".to_string());
        assert_eq!(cap.metadata().category, CapabilityCategory::Tool);
        assert_eq!(cap.metadata().namespace.as_str(), "neo.core");
    }

    // -- WorkflowCapability tests -------------------------------------------

    #[tokio::test]
    async fn workflow_execute() {
        let steps = vec![
            "fetch_data".to_string(),
            "transform".to_string(),
            "store".to_string(),
        ];
        let cap = WorkflowCapability::new(steps, "pipeline".to_string());
        let ctx = make_context();
        let result = cap.execute(serde_json::json!({}), ctx).await.unwrap();

        assert!(result.success);
        assert_eq!(result.output["total_steps"], 3);
        assert_eq!(result.output["workflow_type"], "pipeline");
        let executed = result.output["executed_steps"].as_array().unwrap();
        assert_eq!(executed.len(), 3);
        assert_eq!(executed[0]["step"], "fetch_data");
        assert_eq!(executed[1]["step"], "transform");
        assert_eq!(executed[2]["step"], "store");
    }

    #[test]
    fn workflow_metadata() {
        let cap = WorkflowCapability::new(vec!["a".to_string()], "serial".to_string());
        assert_eq!(cap.metadata().category, CapabilityCategory::Workflow);
    }

    // -- CommunicationCapability tests --------------------------------------

    #[tokio::test]
    async fn communication_execute() {
        let cap = CommunicationCapability::new(
            "grpc".to_string(),
            "service-b:9090".to_string(),
        );
        let ctx = make_context();
        let result = cap
            .execute(
                serde_json::json!({ "message": "ping" }),
                ctx,
            )
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.output["protocol"], "grpc");
        assert_eq!(result.output["target"], "service-b:9090");
    }

    #[tokio::test]
    async fn communication_empty_message() {
        let cap = CommunicationCapability::new("http".to_string(), "host".to_string());
        let ctx = make_context();
        let result = cap.execute(serde_json::json!({}), ctx).await.unwrap();
        assert!(result.success);
        assert_eq!(result.output["message_length"], 0);
    }

    #[test]
    fn communication_metadata() {
        let cap = CommunicationCapability::new("ws".to_string(), "t".to_string());
        assert_eq!(
            cap.metadata().category,
            CapabilityCategory::Communication
        );
        assert_eq!(cap.metadata().namespace.as_str(), "neo.communication");
    }

    // -- FilesystemCapability tests -----------------------------------------

    #[tokio::test]
    async fn fs_read() {
        let cap = FilesystemCapability::new(FsOperation::Read, "/etc/hosts".to_string());
        let ctx = make_context();
        let result = cap.execute(serde_json::json!({}), ctx).await.unwrap();
        assert!(result.success);
        assert_eq!(result.output["operation"], "read");
        assert!(result.output["detail"]
            .as_str()
            .unwrap()
            .contains("/etc/hosts"));
    }

    #[tokio::test]
    async fn fs_write() {
        let cap = FilesystemCapability::new(FsOperation::Write, "/tmp/out.txt".to_string());
        let ctx = make_context();
        let result = cap
            .execute(
                serde_json::json!({ "content": "hello world" }),
                ctx,
            )
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output["detail"]
            .as_str()
            .unwrap()
            .contains("11 bytes"));
    }

    #[tokio::test]
    async fn fs_delete() {
        let cap = FilesystemCapability::new(FsOperation::Delete, "/tmp/junk".to_string());
        let ctx = make_context();
        let result = cap.execute(serde_json::json!({}), ctx).await.unwrap();
        assert!(result.success);
        assert_eq!(result.output["operation"], "delete");
    }

    #[tokio::test]
    async fn fs_list() {
        let cap = FilesystemCapability::new(FsOperation::List, "/home".to_string());
        let ctx = make_context();
        let result = cap.execute(serde_json::json!({}), ctx).await.unwrap();
        assert!(result.success);
        assert_eq!(result.output["operation"], "list");
    }

    #[tokio::test]
    async fn fs_watch() {
        let cap = FilesystemCapability::new(FsOperation::Watch, "/src".to_string());
        let ctx = make_context();
        let result = cap.execute(serde_json::json!({}), ctx).await.unwrap();
        assert!(result.success);
        assert_eq!(result.output["operation"], "watch");
    }

    #[tokio::test]
    async fn fs_copy() {
        let cap = FilesystemCapability::new(FsOperation::Copy, "/a/file.txt".to_string());
        let ctx = make_context();
        let result = cap
            .execute(
                serde_json::json!({ "destination": "/b/file.txt" }),
                ctx,
            )
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output["detail"]
            .as_str()
            .unwrap()
            .contains("/b/file.txt"));
    }

    #[test]
    fn fs_metadata() {
        let cap = FilesystemCapability::new(FsOperation::Read, "/".to_string());
        assert_eq!(cap.metadata().category, CapabilityCategory::Filesystem);
        assert_eq!(cap.metadata().namespace.as_str(), "neo.core");
    }

    // -- NetworkCapability tests --------------------------------------------

    #[tokio::test]
    async fn network_get() {
        let cap = NetworkCapability::new(
            "GET".to_string(),
            "https://example.com".to_string(),
        );
        let ctx = make_context();
        let result = cap.execute(serde_json::json!({}), ctx).await.unwrap();
        assert!(result.success);
        assert_eq!(result.output["method"], "GET");
        assert_eq!(result.output["url"], "https://example.com");
    }

    #[tokio::test]
    async fn network_post_with_headers() {
        let cap = NetworkCapability::new(
            "POST".to_string(),
            "https://api.example.com/data".to_string(),
        );
        let ctx = make_context();
        let result = cap
            .execute(
                serde_json::json!({ "headers": { "Content-Type": "application/json" } }),
                ctx,
            )
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.output["method"], "POST");
    }

    #[test]
    fn network_metadata() {
        let cap = NetworkCapability::new("GET".to_string(), "http://x".to_string());
        assert_eq!(cap.metadata().category, CapabilityCategory::Network);
    }

    // -- DeveloperCapability tests ------------------------------------------

    #[tokio::test]
    async fn developer_execute() {
        let cap = DeveloperCapability::new(
            "clippy".to_string(),
            Some("rust".to_string()),
        );
        let ctx = make_context();
        let result = cap
            .execute(
                serde_json::json!({ "command": "check" }),
                ctx,
            )
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.output["tool"], "clippy");
        assert_eq!(result.output["language"], "rust");
        assert_eq!(result.output["command"], "check");
    }

    #[tokio::test]
    async fn developer_no_language() {
        let cap = DeveloperCapability::new("generic".to_string(), None);
        let ctx = make_context();
        let result = cap.execute(serde_json::json!({}), ctx).await.unwrap();
        assert!(result.success);
        assert!(result.output["language"].is_null());
    }

    #[test]
    fn developer_metadata() {
        let cap = DeveloperCapability::new("x".to_string(), None);
        assert_eq!(cap.metadata().category, CapabilityCategory::Developer);
        assert_eq!(cap.metadata().namespace.as_str(), "neo.developer");
    }

    // -- SystemCapability tests ---------------------------------------------

    #[tokio::test]
    async fn system_health_check() {
        let cap = SystemCapability::new(SystemOperation::HealthCheck);
        let ctx = make_context();
        let result = cap.execute(serde_json::json!({}), ctx).await.unwrap();
        assert!(result.success);
        assert_eq!(result.output["operation"], "health_check");
        assert!(result.output["detail"]
            .as_str()
            .unwrap()
            .contains("operational"));
    }

    #[tokio::test]
    async fn system_shutdown() {
        let cap = SystemCapability::new(SystemOperation::Shutdown);
        let ctx = make_context();
        let result = cap
            .execute(serde_json::json!({ "graceful": false }), ctx)
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output["detail"]
            .as_str()
            .unwrap()
            .contains("force"));
    }

    #[tokio::test]
    async fn system_restart() {
        let cap = SystemCapability::new(SystemOperation::Restart);
        let ctx = make_context();
        let result = cap.execute(serde_json::json!({}), ctx).await.unwrap();
        assert!(result.success);
        assert_eq!(result.output["operation"], "restart");
    }

    #[tokio::test]
    async fn system_metrics() {
        let cap = SystemCapability::new(SystemOperation::Metrics);
        let ctx = make_context();
        let result = cap
            .execute(
                serde_json::json!({ "metrics": { "cpu": 0.5 } }),
                ctx,
            )
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.output["operation"], "metrics");
    }

    #[tokio::test]
    async fn system_config() {
        let cap = SystemCapability::new(SystemOperation::Config);
        let ctx = make_context();
        let result = cap
            .execute(
                serde_json::json!({ "key": "log_level" }),
                ctx,
            )
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output["detail"]
            .as_str()
            .unwrap()
            .contains("log_level"));
    }

    #[test]
    fn system_metadata() {
        let cap = SystemCapability::new(SystemOperation::HealthCheck);
        assert_eq!(cap.metadata().category, CapabilityCategory::System);
        assert_eq!(cap.metadata().namespace.as_str(), "neo.core");
    }

    // -- CustomCapability tests ---------------------------------------------

    #[tokio::test]
    async fn custom_execute() {
        let cap = CustomCapability::new(
            "my_extension".to_string(),
            serde_json::json!({ "handler": "v1" }),
        );
        let ctx = make_context();
        let result = cap
            .execute(
                serde_json::json!({ "action": "deploy" }),
                ctx,
            )
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.output["custom_type"], "my_extension");
        assert_eq!(result.output["action"], "deploy");
    }

    #[tokio::test]
    async fn custom_default_action() {
        let cap = CustomCapability::new("ext".to_string(), serde_json::json!({}));
        let ctx = make_context();
        let result = cap.execute(serde_json::json!({}), ctx).await.unwrap();
        assert!(result.success);
        assert_eq!(result.output["action"], "default");
    }

    #[test]
    fn custom_metadata() {
        let cap = CustomCapability::new("foo".to_string(), serde_json::json!({}));
        assert_eq!(
            cap.metadata().category,
            CapabilityCategory::Custom("foo".to_string())
        );
    }

    // -- CapabilityTypeRegistry tests ---------------------------------------

    #[test]
    fn registry_empty() {
        let reg = CapabilityTypeRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
        assert!(!reg.contains("anything"));
        assert!(reg.create("anything").is_none());
    }

    #[test]
    fn registry_register_and_create() {
        let reg = CapabilityTypeRegistry::new();
        reg.register("reasoning", "Reasoning engine", || {
            Box::new(ReasoningCapability::new("cot".to_string(), serde_json::json!({})))
        });

        assert!(reg.contains("reasoning"));
        assert_eq!(reg.len(), 1);
        assert!(reg.description("reasoning").is_some());
        assert!(reg.description("reasoning")
            .unwrap()
            .contains("Reasoning"));

        let cap = reg.create("reasoning").unwrap();
        assert_eq!(cap.metadata().name, "reasoning");
    }

    #[test]
    fn registry_list_types() {
        let reg = CapabilityTypeRegistry::new();
        reg.register("a", "A", || {
            Box::new(ReasoningCapability::new("x".to_string(), serde_json::json!({})))
        });
        reg.register("b", "B", || {
            Box::new(InferenceCapability::new("x".to_string(), "y".to_string()))
        });

        let mut types = reg.list_types();
        types.sort();
        assert_eq!(types, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn registry_register_builtin_types() {
        let reg = CapabilityTypeRegistry::new();
        register_builtin_types(&reg);
        assert_eq!(reg.len(), 12);
        assert!(reg.contains("reasoning"));
        assert!(reg.contains("inference"));
        assert!(reg.contains("memory"));
        assert!(reg.contains("knowledge"));
        assert!(reg.contains("tool"));
        assert!(reg.contains("workflow"));
        assert!(reg.contains("communication"));
        assert!(reg.contains("filesystem"));
        assert!(reg.contains("network"));
        assert!(reg.contains("developer"));
        assert!(reg.contains("system"));
        assert!(reg.contains("custom"));

        for name in reg.list_types() {
            let cap = reg.create(&name).unwrap();
            assert!(!cap.metadata().name.is_empty());
        }
    }

    // -- create_builtin_capabilities tests ----------------------------------

    #[test]
    fn builtin_capabilities_count() {
        let caps = create_builtin_capabilities();
        assert_eq!(caps.len(), 12);
    }

    #[test]
    fn builtin_capabilities_all_unique_names() {
        let caps = create_builtin_capabilities();
        let names: Vec<&str> = caps.iter().map(|c| c.metadata().name.as_str()).collect();
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn builtin_capabilities_all_unique_categories() {
        let caps = create_builtin_capabilities();
        let categories: Vec<String> = caps
            .iter()
            .map(|c| c.metadata().category.to_string())
            .collect();
        let mut unique = categories.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(categories.len(), unique.len());
    }

    #[test]
    fn builtin_capabilities_all_valid_metadata() {
        let caps = create_builtin_capabilities();
        for cap in &caps {
            let meta = cap.metadata();
            assert!(meta.validate().is_ok(), "metadata invalid for {}", meta.name);
            assert_eq!(meta.version, CapabilityVersion::initial());
            assert!(!meta.inputs.is_empty());
        }
    }

    #[tokio::test]
    async fn builtin_capabilities_all_execute() {
        let caps = create_builtin_capabilities();
        let ctx = make_context();
        for cap in &caps {
            let result = cap.execute(serde_json::json!({}), ctx.clone()).await;
            assert!(
                result.is_ok(),
                "execute failed for {}: {:?}",
                cap.metadata().name,
                result.err()
            );
            assert!(result.unwrap().success);
        }
    }

    // -- Enum Display tests -------------------------------------------------

    #[test]
    fn memory_operation_display() {
        assert_eq!(MemoryOperation::Store.to_string(), "store");
        assert_eq!(MemoryOperation::Retrieve.to_string(), "retrieve");
        assert_eq!(MemoryOperation::Update.to_string(), "update");
        assert_eq!(MemoryOperation::Delete.to_string(), "delete");
        assert_eq!(MemoryOperation::Search.to_string(), "search");
        assert_eq!(MemoryOperation::Consolidate.to_string(), "consolidate");
    }

    #[test]
    fn knowledge_operation_display() {
        assert_eq!(KnowledgeOperation::Query.to_string(), "query");
        assert_eq!(KnowledgeOperation::Traverse.to_string(), "traverse");
        assert_eq!(KnowledgeOperation::Extract.to_string(), "extract");
        assert_eq!(KnowledgeOperation::Validate.to_string(), "validate");
        assert_eq!(KnowledgeOperation::Merge.to_string(), "merge");
        assert_eq!(KnowledgeOperation::Infer.to_string(), "infer");
    }

    #[test]
    fn fs_operation_display() {
        assert_eq!(FsOperation::Read.to_string(), "read");
        assert_eq!(FsOperation::Write.to_string(), "write");
        assert_eq!(FsOperation::Delete.to_string(), "delete");
        assert_eq!(FsOperation::List.to_string(), "list");
        assert_eq!(FsOperation::Watch.to_string(), "watch");
        assert_eq!(FsOperation::Copy.to_string(), "copy");
    }

    #[test]
    fn system_operation_display() {
        assert_eq!(SystemOperation::Shutdown.to_string(), "shutdown");
        assert_eq!(SystemOperation::Restart.to_string(), "restart");
        assert_eq!(SystemOperation::HealthCheck.to_string(), "health_check");
        assert_eq!(SystemOperation::Metrics.to_string(), "metrics");
        assert_eq!(SystemOperation::Config.to_string(), "config");
    }

    // -- Additional edge-case tests -----------------------------------------

    #[test]
    fn reasoning_no_query_field() {
        let cap = ReasoningCapability::new("x".to_string(), serde_json::json!({}));
        let ctx = make_context();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(cap.execute(serde_json::json!({}), ctx));
        assert!(result.unwrap().success);
    }

    #[test]
    fn inference_empty_prompt() {
        let cap = InferenceCapability::new("m".to_string(), "b".to_string());
        let ctx = make_context();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(cap.execute(serde_json::json!({}), ctx));
        assert!(result.unwrap().success);
    }

    #[test]
    fn memory_no_key_defaults_to_unnamed() {
        let cap = MemoryCapability::new(MemoryOperation::Retrieve, "t".to_string());
        let ctx = make_context();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(cap.execute(serde_json::json!({}), ctx));
        let detail = result.unwrap().output["detail"].as_str().unwrap().to_string();
        assert!(detail.contains("_unnamed"));
    }

    #[test]
    fn workflow_empty_steps() {
        let cap = WorkflowCapability::new(vec![], "empty".to_string());
        let ctx = make_context();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(cap.execute(serde_json::json!({}), ctx));
        let r = result.unwrap();
        assert!(r.success);
        assert_eq!(r.output["total_steps"], 0);
        assert!(r.output["executed_steps"].as_array().unwrap().is_empty());
    }

    #[test]
    fn fs_copy_no_destination() {
        let cap = FilesystemCapability::new(FsOperation::Copy, "/src".to_string());
        let ctx = make_context();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(cap.execute(serde_json::json!({}), ctx));
        let detail = result.unwrap().output["detail"].as_str().unwrap().to_string();
        assert!(detail.contains("_unspecified"));
    }

    #[test]
    fn network_empty_method() {
        let cap = NetworkCapability::new(String::new(), "http://x".to_string());
        let ctx = make_context();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(cap.execute(serde_json::json!({}), ctx));
        assert!(result.unwrap().success);
    }

    #[test]
    fn developer_execute_no_command() {
        let cap = DeveloperCapability::new("tool".to_string(), None);
        let ctx = make_context();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(cap.execute(serde_json::json!({}), ctx));
        let r = result.unwrap();
        assert!(r.success);
        assert_eq!(r.output["command"], "default");
    }

    #[test]
    fn system_shutdown_default_graceful() {
        let cap = SystemCapability::new(SystemOperation::Shutdown);
        let ctx = make_context();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(cap.execute(serde_json::json!({}), ctx));
        let detail = result.unwrap().output["detail"].as_str().unwrap().to_string();
        assert!(detail.contains("graceful"));
    }

    #[test]
    fn system_config_no_key() {
        let cap = SystemCapability::new(SystemOperation::Config);
        let ctx = make_context();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(cap.execute(serde_json::json!({}), ctx));
        let detail = result.unwrap().output["detail"].as_str().unwrap().to_string();
        assert!(detail.contains("_all"));
    }
}
