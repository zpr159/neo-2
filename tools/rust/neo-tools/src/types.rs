//! Core type definitions for the Tool Ecosystem.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use crate::error::ToolResult;

// ---------------------------------------------------------------------------
// ToolId
// ---------------------------------------------------------------------------

/// Unique identifier for a registered tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ToolId(pub uuid::Uuid);

impl ToolId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    pub fn as_uuid(&self) -> &uuid::Uuid {
        &self.0
    }
}

impl Default for ToolId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ToolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for ToolId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(uuid::Uuid::parse_str(s)?))
    }
}

impl From<uuid::Uuid> for ToolId {
    fn from(id: uuid::Uuid) -> Self {
        Self(id)
    }
}

impl From<ToolId> for uuid::Uuid {
    fn from(id: ToolId) -> uuid::Uuid {
        id.0
    }
}

// ---------------------------------------------------------------------------
// ExecutionId
// ---------------------------------------------------------------------------

/// Unique identifier for a single tool execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExecutionId(pub uuid::Uuid);

impl ExecutionId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for ExecutionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ExecutionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// ToolType — what kind of integration this tool provides
// ---------------------------------------------------------------------------

/// Broad classification of tool integration type.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
#[strum(serialize_all = "snake_case")]
pub enum ToolType {
    Filesystem,
    Shell,
    Git,
    Browser,
    HttpClient,
    Database,
    Cloud,
    Container,
    Ide,
    Networking,
    Custom(String),
}

// ---------------------------------------------------------------------------
// ToolCategory — fine-grained grouping within a ToolType
// ---------------------------------------------------------------------------

/// Fine-grained category for tool classification.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
#[strum(serialize_all = "snake_case")]
pub enum ToolCategory {
    Read,
    Write,
    Transform,
    Search,
    Execute,
    Query,
    Deploy,
    Monitor,
    Communicate,
    Analyze,
    Custom(String),
}

// ---------------------------------------------------------------------------
// ToolVersion — semantic version
// ---------------------------------------------------------------------------

/// Semantic version for tools.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ToolVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre_release: Option<String>,
}

impl ToolVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            pre_release: None,
        }
    }

    pub fn is_compatible(&self, required: &ToolVersion) -> bool {
        self.major == required.major && self >= required
    }
}

impl Ord for ToolVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.major
            .cmp(&other.major)
            .then_with(|| self.minor.cmp(&other.minor))
            .then_with(|| self.patch.cmp(&other.patch))
    }
}

impl PartialOrd for ToolVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for ToolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(ref pre) = self.pre_release {
            write!(f, "-{}", pre)?;
        }
        Ok(())
    }
}

impl FromStr for ToolVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let pre_str;
        let version_part = if let Some(idx) = s.find('-') {
            pre_str = Some(s[idx + 1..].to_string());
            &s[..idx]
        } else {
            pre_str = None;
            s
        };

        let parts: Vec<&str> = version_part.split('.').collect();
        if parts.len() != 3 {
            return Err(format!("expected MAJOR.MINOR.PATCH, got '{}'", s));
        }

        Ok(Self {
            major: parts[0]
                .parse()
                .map_err(|e| format!("invalid major: {e}"))?,
            minor: parts[1]
                .parse()
                .map_err(|e| format!("invalid minor: {e}"))?,
            patch: parts[2]
                .parse()
                .map_err(|e| format!("invalid patch: {e}"))?,
            pre_release: pre_str,
        })
    }
}

// ---------------------------------------------------------------------------
// ToolMetadata
// ---------------------------------------------------------------------------

/// Descriptive metadata for a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub tool_type: ToolType,
    pub category: ToolCategory,
    pub version: ToolVersion,
    pub author: String,
    pub license: Option<String>,
    pub tags: Vec<String>,
    pub input_schema: serde_json::Value,
    pub output_schema: Option<serde_json::Value>,
    pub timeout_ms: Option<u64>,
    pub max_retries: u32,
    pub requires_permission: bool,
}

impl ToolMetadata {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        tool_type: ToolType,
        category: ToolCategory,
        version: ToolVersion,
    ) -> Self {
        let name = name.into();
        Self {
            display_name: name.clone(),
            name,
            description: description.into(),
            tool_type,
            category,
            version,
            author: "neo".into(),
            license: None,
            tags: Vec::new(),
            input_schema: serde_json::json!({"type": "object"}),
            output_schema: None,
            timeout_ms: None,
            max_retries: 0,
            requires_permission: false,
        }
    }

    pub fn with_display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = name.into();
        self
    }

    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = author.into();
        self
    }

    pub fn with_license(mut self, license: impl Into<String>) -> Self {
        self.license = Some(license.into());
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn with_input_schema(mut self, schema: serde_json::Value) -> Self {
        self.input_schema = schema;
        self
    }

    pub fn with_output_schema(mut self, schema: serde_json::Value) -> Self {
        self.output_schema = Some(schema);
        self
    }

    pub fn with_timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ms = Some(ms);
        self
    }

    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    pub fn requiring_permission(mut self) -> Self {
        self.requires_permission = true;
        self
    }
}

// ---------------------------------------------------------------------------
// ToolManifest
// ---------------------------------------------------------------------------

/// Complete manifest for a tool, including metadata, configuration, and dependencies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolManifest {
    pub metadata: ToolMetadata,
    pub config: ToolConfiguration,
    pub dependencies: Vec<ToolDependency>,
    pub permissions: Vec<String>,
    pub capabilities: Vec<String>,
}

impl ToolManifest {
    pub fn new(metadata: ToolMetadata, config: ToolConfiguration) -> Self {
        Self {
            metadata,
            config,
            dependencies: Vec::new(),
            permissions: Vec::new(),
            capabilities: Vec::new(),
        }
    }

    pub fn with_dependency(mut self, dep: ToolDependency) -> Self {
        self.dependencies.push(dep);
        self
    }

    pub fn with_permission(mut self, perm: impl Into<String>) -> Self {
        self.permissions.push(perm.into());
        self
    }

    pub fn with_capability(mut self, cap: impl Into<String>) -> Self {
        self.capabilities.push(cap.into());
        self
    }
}

// ---------------------------------------------------------------------------
// ToolConfiguration
// ---------------------------------------------------------------------------

/// Runtime configuration for a tool.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolConfiguration {
    pub enabled: bool,
    pub auto_start: bool,
    pub sandboxed: bool,
    pub priority: u32,
    pub max_concurrent: usize,
    pub settings: serde_json::Value,
}

impl ToolConfiguration {
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }

    pub fn with_auto_start(mut self, val: bool) -> Self {
        self.auto_start = val;
        self
    }

    pub fn with_sandboxed(mut self, val: bool) -> Self {
        self.sandboxed = val;
        self
    }

    pub fn with_priority(mut self, val: u32) -> Self {
        self.priority = val;
        self
    }

    pub fn with_max_concurrent(mut self, val: usize) -> Self {
        self.max_concurrent = val;
        self
    }

    pub fn with_settings(mut self, val: serde_json::Value) -> Self {
        self.settings = val;
        self
    }
}

// ---------------------------------------------------------------------------
// ToolDependency
// ---------------------------------------------------------------------------

/// Declares a dependency on another tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDependency {
    pub name: String,
    pub version_requirement: ToolVersion,
    pub optional: bool,
}

impl ToolDependency {
    pub fn required(name: impl Into<String>, version: ToolVersion) -> Self {
        Self {
            name: name.into(),
            version_requirement: version,
            optional: false,
        }
    }

    pub fn optional(name: impl Into<String>, version: ToolVersion) -> Self {
        Self {
            name: name.into(),
            version_requirement: version,
            optional: true,
        }
    }
}

// ---------------------------------------------------------------------------
// ToolContext
// ---------------------------------------------------------------------------

/// Context passed to a tool during execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolContext {
    pub execution_id: ExecutionId,
    pub caller_id: String,
    pub caller_type: CallerType,
    pub permissions: Vec<String>,
    pub environment: std::collections::HashMap<String, String>,
    pub timeout_ms: Option<u64>,
    pub sandbox_config: Option<SandboxConfig>,
}

/// Who is invoking the tool.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CallerType {
    Agent,
    Workflow,
    Executive,
    Cli,
    Api,
    Internal,
    Custom(String),
}

/// Sandbox configuration for an execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub cpu_limit_pct: Option<f64>,
    pub memory_limit_bytes: Option<u64>,
    pub disk_limit_bytes: Option<u64>,
    pub network_allowed: bool,
    pub allowed_paths: Vec<String>,
    pub denied_paths: Vec<String>,
    pub temp_dir: Option<String>,
}

impl ToolContext {
    pub fn new(caller_id: impl Into<String>, caller_type: CallerType) -> Self {
        Self {
            execution_id: ExecutionId::new(),
            caller_id: caller_id.into(),
            caller_type,
            permissions: Vec::new(),
            environment: std::collections::HashMap::new(),
            timeout_ms: None,
            sandbox_config: None,
        }
    }

    pub fn with_timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ms = Some(ms);
        self
    }

    pub fn with_permission(mut self, perm: impl Into<String>) -> Self {
        self.permissions.push(perm.into());
        self
    }

    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.environment.insert(key.into(), value.into());
        self
    }

    pub fn with_sandbox(mut self, config: SandboxConfig) -> Self {
        self.sandbox_config = Some(config);
        self
    }
}

// ---------------------------------------------------------------------------
// ToolRequest
// ---------------------------------------------------------------------------

/// A request to execute a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRequest {
    pub tool_id: ToolId,
    pub operation: String,
    pub parameters: serde_json::Value,
    pub context: ToolContext,
}

impl ToolRequest {
    pub fn new(
        tool_id: ToolId,
        operation: impl Into<String>,
        parameters: serde_json::Value,
        context: ToolContext,
    ) -> Self {
        Self {
            tool_id,
            operation: operation.into(),
            parameters,
            context,
        }
    }

    /// Create a named request (tool name stored in parameters for registry lookup).
    pub fn named(
        tool_id: ToolId,
        tool_name: impl Into<String>,
        operation: impl Into<String>,
        parameters: serde_json::Value,
        context: ToolContext,
    ) -> Self {
        let mut params = parameters;
        if let Some(obj) = params.as_object_mut() {
            obj.insert(
                "_tool_name".into(),
                serde_json::Value::String(tool_name.into()),
            );
        }
        Self {
            tool_id,
            operation: operation.into(),
            parameters: params,
            context,
        }
    }

    /// Get the tool name from parameters.
    pub fn tool_name(&self) -> String {
        self.parameters
            .get("_tool_name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string()
    }
}

// ---------------------------------------------------------------------------
// ToolResponse
// ---------------------------------------------------------------------------

/// Response from a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResponse {
    pub execution_id: ExecutionId,
    pub tool_id: ToolId,
    pub success: bool,
    pub output: serde_json::Value,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl ToolResponse {
    pub fn success(
        execution_id: ExecutionId,
        tool_id: ToolId,
        output: serde_json::Value,
        duration_ms: u64,
    ) -> Self {
        Self {
            execution_id,
            tool_id,
            success: true,
            output,
            error: None,
            duration_ms,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn failure(
        execution_id: ExecutionId,
        tool_id: ToolId,
        error: String,
        duration_ms: u64,
    ) -> Self {
        Self {
            execution_id,
            tool_id,
            success: false,
            output: serde_json::json!(null),
            error: Some(error),
            duration_ms,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

// ---------------------------------------------------------------------------
// ToolHealth
// ---------------------------------------------------------------------------

/// Health status of a tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Healthy => write!(f, "healthy"),
            Self::Degraded => write!(f, "degraded"),
            Self::Unhealthy => write!(f, "unhealthy"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Health check result for a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolHealth {
    pub tool_id: ToolId,
    pub status: HealthStatus,
    pub message: String,
    pub checked_at: DateTime<Utc>,
    pub latency_ms: Option<f64>,
}

impl ToolHealth {
    pub fn healthy(tool_id: ToolId) -> Self {
        Self {
            tool_id,
            status: HealthStatus::Healthy,
            message: "OK".into(),
            checked_at: Utc::now(),
            latency_ms: None,
        }
    }

    pub fn unhealthy(tool_id: ToolId, message: impl Into<String>) -> Self {
        Self {
            tool_id,
            status: HealthStatus::Unhealthy,
            message: message.into(),
            checked_at: Utc::now(),
            latency_ms: None,
        }
    }

    pub fn with_latency_ms(mut self, ms: f64) -> Self {
        self.latency_ms = Some(ms);
        self
    }
}

// ---------------------------------------------------------------------------
// ToolMetrics
// ---------------------------------------------------------------------------

/// Execution metrics for a tool.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolMetrics {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub total_duration_ms: u64,
    pub avg_duration_ms: f64,
    pub p95_duration_ms: f64,
    pub p99_duration_ms: u64,
    pub last_executed_at: Option<DateTime<Utc>>,
    pub retry_count: u64,
    pub cancellation_count: u64,
}

impl ToolMetrics {
    pub fn record_success(&mut self, duration_ms: u64) {
        self.total_executions += 1;
        self.successful_executions += 1;
        self.total_duration_ms += duration_ms;
        self.avg_duration_ms = self.total_duration_ms as f64 / self.total_executions as f64;
        self.last_executed_at = Some(Utc::now());
    }

    pub fn record_failure(&mut self, duration_ms: u64) {
        self.total_executions += 1;
        self.failed_executions += 1;
        self.total_duration_ms += duration_ms;
        self.avg_duration_ms = self.total_duration_ms as f64 / self.total_executions as f64;
        self.last_executed_at = Some(Utc::now());
    }

    pub fn record_retry(&mut self) {
        self.retry_count += 1;
    }

    pub fn record_cancellation(&mut self) {
        self.cancellation_count += 1;
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_executions == 0 {
            return 0.0;
        }
        self.successful_executions as f64 / self.total_executions as f64
    }
}

// ---------------------------------------------------------------------------
// ToolStatistics
// ---------------------------------------------------------------------------

/// Aggregate statistics across all tools.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolStatistics {
    pub total_tools: usize,
    pub active_tools: usize,
    pub total_executions: u64,
    pub total_failures: u64,
    pub avg_global_latency_ms: f64,
    pub tools_by_type: std::collections::HashMap<String, usize>,
}

// ---------------------------------------------------------------------------
// ToolSnapshot
// ---------------------------------------------------------------------------

/// Point-in-time snapshot of a tool's state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSnapshot {
    pub tool_id: ToolId,
    pub metadata: ToolMetadata,
    pub state: crate::lifecycle::ToolLifecycleState,
    pub config: ToolConfiguration,
    pub metrics: ToolMetrics,
    pub health: HealthStatus,
    pub captured_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// ExecuteFn — async function pointer for tool execution
// ---------------------------------------------------------------------------

/// The function signature for tool execution handlers.
pub type ExecuteFn = Arc<
    dyn Fn(
            serde_json::Value,
            ToolContext,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = ToolResult<serde_json::Value>> + Send>,
        > + Send
        + Sync,
>;

/// The function signature for health check handlers.
pub type HealthCheckFn = Arc<
    dyn Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = ToolHealth> + Send>>
        + Send
        + Sync,
>;
