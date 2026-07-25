use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{CapabilityError, CapabilityResult};

/// Unique identifier for a capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CapabilityId(pub Uuid);

impl CapabilityId {
    /// Create a new random capability identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Get the inner UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for CapabilityId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CapabilityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Semantic version for capability versioning.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CapabilityVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl CapabilityVersion {
    /// Create a new version.
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Create version 1.0.0.
    pub fn initial() -> Self {
        Self::new(1, 0, 0)
    }

    /// Check if this version is compatible with the required version.
    /// Same major version required for compatibility.
    pub fn is_compatible_with(&self, required: &CapabilityVersion) -> bool {
        self.major == required.major && *self >= *required
    }

    /// Bump the patch version.
    pub fn bump_patch(&mut self) {
        self.patch += 1;
    }

    /// Bump the minor version and reset patch.
    pub fn bump_minor(&mut self) {
        self.minor += 1;
        self.patch = 0;
    }

    /// Bump the major version and reset minor and patch.
    pub fn bump_major(&mut self) {
        self.major += 1;
        self.minor = 0;
        self.patch = 0;
    }
}

impl PartialOrd for CapabilityVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CapabilityVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
    }
}

impl fmt::Display for CapabilityVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl std::str::FromStr for CapabilityVersion {
    type Err = CapabilityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(CapabilityError::validation_failed(format!(
                "invalid version format '{}': expected MAJOR.MINOR.PATCH",
                s
            )));
        }
        let major = parts[0].parse::<u32>().map_err(|e| {
            CapabilityError::validation_failed(format!("invalid major version: {}", e))
        })?;
        let minor = parts[1].parse::<u32>().map_err(|e| {
            CapabilityError::validation_failed(format!("invalid minor version: {}", e))
        })?;
        let patch = parts[2].parse::<u32>().map_err(|e| {
            CapabilityError::validation_failed(format!("invalid patch version: {}", e))
        })?;
        Ok(Self::new(major, minor, patch))
    }
}

/// Namespace for organizing capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CapabilityNamespace(pub String);

impl CapabilityNamespace {
    /// Create a new namespace.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Core system namespace.
    pub fn core() -> Self {
        Self::new("neo.core")
    }

    /// AI inference namespace.
    pub fn inference() -> Self {
        Self::new("neo.inference")
    }

    /// Reasoning namespace.
    pub fn reasoning() -> Self {
        Self::new("neo.reasoning")
    }

    /// Memory namespace.
    pub fn memory() -> Self {
        Self::new("neo.memory")
    }

    /// Knowledge namespace.
    pub fn knowledge() -> Self {
        Self::new("neo.knowledge")
    }

    /// Developer tools namespace.
    pub fn developer() -> Self {
        Self::new("neo.developer")
    }

    /// Communication namespace.
    pub fn communication() -> Self {
        Self::new("neo.communication")
    }

    /// Get the namespace string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CapabilityNamespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for CapabilityNamespace {
    fn default() -> Self {
        Self::core()
    }
}

/// Tags for categorizing capabilities.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityTags(pub HashSet<String>);

impl CapabilityTags {
    /// Create empty tags.
    pub fn empty() -> Self {
        Self(HashSet::new())
    }

    /// Create tags from a single tag.
    pub fn single(tag: impl Into<String>) -> Self {
        let mut set = HashSet::new();
        set.insert(tag.into());
        Self(set)
    }

    /// Create tags from an iterator.
    pub fn from_iter<T: IntoIterator<Item = impl Into<String>>>(iter: T) -> Self {
        Self(iter.into_iter().map(Into::into).collect())
    }

    /// Add a tag.
    pub fn add(&mut self, tag: impl Into<String>) {
        self.0.insert(tag.into());
    }

    /// Check if tags contain a specific tag.
    pub fn contains(&self, tag: &str) -> bool {
        self.0.contains(tag)
    }

    /// Check if tags contain all given tags.
    pub fn contains_all(&self, tags: &[&str]) -> bool {
        tags.iter().all(|t| self.0.contains(*t))
    }

    /// Check if tags contain any of the given tags.
    pub fn contains_any(&self, tags: &[&str]) -> bool {
        tags.iter().any(|t| self.0.contains(*t))
    }

    /// Get the set of tags.
    pub fn as_set(&self) -> &HashSet<String> {
        &self.0
    }
}

/// Aliases for capability lookup.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilityAliases(pub Vec<String>);

impl CapabilityAliases {
    /// Create empty aliases.
    pub fn empty() -> Self {
        Self(Vec::new())
    }

    /// Add an alias.
    pub fn add(&mut self, alias: impl Into<String>) {
        self.0.push(alias.into());
    }

    /// Check if the given name matches any alias.
    pub fn matches(&self, name: &str) -> bool {
        self.0.iter().any(|a| a.eq_ignore_ascii_case(name))
    }

    /// Get all aliases.
    pub fn as_slice(&self) -> &[String] {
        &self.0
    }
}

/// Lifecycle state of a capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CapabilityState {
    /// Capability has been defined but not yet registered.
    Defined,
    /// Capability is registered and ready to be enabled.
    Registered,
    /// Capability is enabled and can be executed.
    Enabled,
    /// Capability is temporarily disabled.
    Disabled,
    /// Capability is currently executing.
    Executing,
    /// Capability has been revoked and cannot be used.
    Revoked,
    /// Capability encountered an error.
    Failed,
}

impl CapabilityState {
    /// Check if the state is terminal (no further transitions possible).
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Revoked | Self::Failed)
    }

    /// Check if the capability is executable.
    pub fn is_executable(self) -> bool {
        matches!(self, Self::Enabled)
    }

    /// Valid transitions from this state.
    pub fn valid_transitions(self) -> &'static [CapabilityState] {
        match self {
            Self::Defined => &[Self::Registered, Self::Failed],
            Self::Registered => &[Self::Enabled, Self::Disabled, Self::Revoked],
            Self::Enabled => &[Self::Executing, Self::Disabled, Self::Revoked, Self::Failed],
            Self::Disabled => &[Self::Enabled, Self::Revoked],
            Self::Executing => &[Self::Enabled, Self::Failed],
            Self::Revoked => &[],
            Self::Failed => &[Self::Registered],
        }
    }

    /// Check if a transition to the target state is valid.
    pub fn can_transition_to(self, target: CapabilityState) -> bool {
        self.valid_transitions().contains(&target)
    }
}

impl fmt::Display for CapabilityState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Defined => write!(f, "defined"),
            Self::Registered => write!(f, "registered"),
            Self::Enabled => write!(f, "enabled"),
            Self::Disabled => write!(f, "disabled"),
            Self::Executing => write!(f, "executing"),
            Self::Revoked => write!(f, "revoked"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

/// High-level category of a capability.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CapabilityCategory {
    Reasoning,
    Inference,
    Memory,
    Knowledge,
    Tool,
    Workflow,
    Communication,
    Filesystem,
    Network,
    Developer,
    System,
    Custom(String),
}

impl fmt::Display for CapabilityCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Reasoning => write!(f, "reasoning"),
            Self::Inference => write!(f, "inference"),
            Self::Memory => write!(f, "memory"),
            Self::Knowledge => write!(f, "knowledge"),
            Self::Tool => write!(f, "tool"),
            Self::Workflow => write!(f, "workflow"),
            Self::Communication => write!(f, "communication"),
            Self::Filesystem => write!(f, "filesystem"),
            Self::Network => write!(f, "network"),
            Self::Developer => write!(f, "developer"),
            Self::System => write!(f, "system"),
            Self::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

impl Default for CapabilityCategory {
    fn default() -> Self {
        Self::System
    }
}

/// Input schema describing the expected arguments for a capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputSchema {
    /// JSON Schema describing the input format.
    pub schema: serde_json::Value,
    /// Human-readable description of inputs.
    pub description: String,
    /// Whether the input is required.
    pub required: bool,
    /// Default value as JSON.
    pub default: Option<serde_json::Value>,
}

impl InputSchema {
    /// Create a new input schema.
    pub fn new(schema: serde_json::Value, description: impl Into<String>) -> Self {
        Self {
            schema,
            description: description.into(),
            required: true,
            default: None,
        }
    }

    /// Make the input optional.
    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    /// Set a default value.
    pub fn with_default(mut self, default: serde_json::Value) -> Self {
        self.default = Some(default);
        self.required = false;
        self
    }

    /// Validate input against the schema.
    pub fn validate(&self, input: &serde_json::Value) -> CapabilityResult<()> {
        if input.is_null() && self.required {
            return Err(CapabilityError::validation_failed(format!(
                "required input '{}' is missing",
                self.description
            )));
        }
        if !input.is_null() && !self.schema.is_object() {
            return Ok(());
        }
        if let (Some(schema_obj), Some(input_obj)) = (self.schema.as_object(), input.as_object()) {
            if let Some(required_fields) = schema_obj.get("required").and_then(|v| v.as_array()) {
                for field in required_fields {
                    if let Some(field_name) = field.as_str() {
                        if !input_obj.contains_key(field_name) {
                            return Err(CapabilityError::validation_failed(format!(
                                "missing required field '{}'",
                                field_name
                            )));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

/// Output schema describing the output of a capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputSchema {
    /// JSON Schema describing the output format.
    pub schema: serde_json::Value,
    /// Human-readable description of the output.
    pub description: String,
}

impl OutputSchema {
    /// Create a new output schema.
    pub fn new(schema: serde_json::Value, description: impl Into<String>) -> Self {
        Self {
            schema,
            description: description.into(),
        }
    }
}

/// Metadata describing a capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityMetadata {
    /// Human-readable name.
    pub name: String,
    /// Unique identifier.
    pub id: CapabilityId,
    /// Version.
    pub version: CapabilityVersion,
    /// Description.
    pub description: String,
    /// Category.
    pub category: CapabilityCategory,
    /// Namespace.
    pub namespace: CapabilityNamespace,
    /// Tags.
    pub tags: CapabilityTags,
    /// Aliases.
    pub aliases: CapabilityAliases,
    /// Author.
    pub author: String,
    /// License.
    pub license: String,
    /// Input schemas.
    pub inputs: Vec<InputSchema>,
    /// Output schema.
    pub output: OutputSchema,
    /// Dependencies on other capabilities.
    pub dependencies: Vec<CapabilityId>,
    /// Required permissions.
    pub required_permissions: Vec<String>,
    /// Estimated resource requirements.
    pub resource_requirements: ResourceRequirements,
    /// Timeout in milliseconds.
    pub timeout_ms: Option<u64>,
    /// Maximum retry count.
    pub max_retries: u32,
    /// Whether this capability requires approval before execution.
    pub requires_approval: bool,
    /// Whether this capability can be composed.
    pub composable: bool,
    /// Custom metadata.
    pub custom: HashMap<String, serde_json::Value>,
    /// When this capability was created.
    pub created_at: DateTime<Utc>,
    /// When this capability was last updated.
    pub updated_at: DateTime<Utc>,
}

impl CapabilityMetadata {
    /// Create new metadata with sensible defaults.
    pub fn new(
        name: impl Into<String>,
        version: CapabilityVersion,
        description: impl Into<String>,
        category: CapabilityCategory,
    ) -> Self {
        let now = Utc::now();
        let name_str = name.into();
        Self {
            name: name_str,
            id: CapabilityId::new(),
            version,
            description: description.into(),
            category,
            namespace: CapabilityNamespace::core(),
            tags: CapabilityTags::empty(),
            aliases: CapabilityAliases::empty(),
            author: String::new(),
            license: String::new(),
            inputs: Vec::new(),
            output: OutputSchema::new(serde_json::Value::Null, "no output"),
            dependencies: Vec::new(),
            required_permissions: Vec::new(),
            resource_requirements: ResourceRequirements::default(),
            timeout_ms: None,
            max_retries: 0,
            requires_approval: false,
            composable: true,
            custom: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Set the namespace.
    pub fn with_namespace(mut self, namespace: CapabilityNamespace) -> Self {
        self.namespace = namespace;
        self
    }

    /// Set the author.
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = author.into();
        self
    }

    /// Add a tag.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.add(tag);
        self
    }

    /// Add an alias.
    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.aliases.add(alias);
        self
    }

    /// Add an input schema.
    pub fn with_input(mut self, input: InputSchema) -> Self {
        self.inputs.push(input);
        self
    }

    /// Set the output schema.
    pub fn with_output(mut self, output: OutputSchema) -> Self {
        self.output = output;
        self
    }

    /// Add a dependency.
    pub fn with_dependency(mut self, dep: CapabilityId) -> Self {
        self.dependencies.push(dep);
        self
    }

    /// Add a required permission.
    pub fn with_permission(mut self, perm: impl Into<String>) -> Self {
        self.required_permissions.push(perm.into());
        self
    }

    /// Set timeout.
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    /// Set max retries.
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Mark as requiring approval.
    pub fn requiring_approval(mut self) -> Self {
        self.requires_approval = true;
        self
    }

    /// Set resource requirements.
    pub fn with_resources(mut self, resources: ResourceRequirements) -> Self {
        self.resource_requirements = resources;
        self
    }

    /// Validate the metadata itself.
    pub fn validate(&self) -> CapabilityResult<()> {
        if self.name.is_empty() {
            return Err(CapabilityError::validation_failed(
                "capability name cannot be empty",
            ));
        }
        if self.description.is_empty() {
            return Err(CapabilityError::validation_failed(
                "capability description cannot be empty",
            ));
        }
        for input in &self.inputs {
            input.validate(&serde_json::Value::Null).ok();
        }
        Ok(())
    }
}

/// Resource requirements for executing a capability.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// CPU units required (0.0 - 1.0).
    pub cpu_units: f64,
    /// GPU units required (0.0 - 1.0).
    pub gpu_units: f64,
    /// Memory in bytes required.
    pub memory_bytes: u64,
    /// Inference budget in tokens.
    pub inference_tokens: u32,
    /// Disk bytes required.
    pub disk_bytes: u64,
}

impl ResourceRequirements {
    /// No resource requirements.
    pub fn none() -> Self {
        Self::default()
    }

    /// Minimal requirements.
    pub fn minimal() -> Self {
        Self {
            cpu_units: 0.01,
            gpu_units: 0.0,
            memory_bytes: 1024 * 1024,
            inference_tokens: 0,
            disk_bytes: 0,
        }
    }

    /// Moderate requirements.
    pub fn moderate() -> Self {
        Self {
            cpu_units: 0.25,
            gpu_units: 0.0,
            memory_bytes: 256 * 1024 * 1024,
            inference_tokens: 1024,
            disk_bytes: 1024 * 1024,
        }
    }

    /// Heavy requirements (GPU-intensive).
    pub fn heavy() -> Self {
        Self {
            cpu_units: 0.5,
            gpu_units: 0.5,
            memory_bytes: 1024 * 1024 * 1024,
            inference_tokens: 8192,
            disk_bytes: 100 * 1024 * 1024,
        }
    }
}

/// Progress update from a capability execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressUpdate {
    /// Current step (0-based).
    pub current: u32,
    /// Total steps.
    pub total: u32,
    /// Message describing current progress.
    pub message: String,
    /// Percentage complete (0.0 - 100.0).
    pub percentage: f64,
}

impl ProgressUpdate {
    /// Create a new progress update.
    pub fn new(current: u32, total: u32, message: impl Into<String>) -> Self {
        let percentage = if total > 0 {
            (current as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        Self {
            current,
            total,
            message: message.into(),
            percentage,
        }
    }
}

/// Stream chunk for streaming capability output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// The data chunk.
    pub data: serde_json::Value,
    /// Whether this is the final chunk.
    pub done: bool,
    /// Optional sequence number.
    pub sequence: Option<u64>,
}

/// Execution result from a capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityResult_output {
    /// The output data.
    pub output: serde_json::Value,
    /// Whether execution succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
    /// Execution duration in milliseconds.
    pub duration_ms: u64,
    /// Resources consumed.
    pub resources_used: ResourceRequirements,
    /// Any warnings produced during execution.
    pub warnings: Vec<String>,
}

impl CapabilityResult_output {
    /// Create a successful result.
    pub fn success(output: serde_json::Value, duration_ms: u64) -> Self {
        Self {
            output,
            success: true,
            error: None,
            duration_ms,
            resources_used: ResourceRequirements::default(),
            warnings: Vec::new(),
        }
    }

    /// Create a failed result.
    pub fn failure(error: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            output: serde_json::Value::Null,
            success: false,
            error: Some(error.into()),
            duration_ms,
            resources_used: ResourceRequirements::default(),
            warnings: Vec::new(),
        }
    }
}

/// The core Capability trait. Every capability in Neo must implement this.
#[async_trait]
pub trait Capability: Send + Sync {
    /// Returns the metadata for this capability.
    fn metadata(&self) -> &CapabilityMetadata;

    /// Returns a mutable reference to the metadata.
    fn metadata_mut(&mut self) -> &mut CapabilityMetadata;

    /// Validate inputs before execution.
    fn validate_input(&self, input: &serde_json::Value) -> CapabilityResult<()> {
        for input_schema in &self.metadata().inputs {
            input_schema.validate(input)?;
        }
        Ok(())
    }

    /// Execute the capability with the given input.
    async fn execute(
        &self,
        input: serde_json::Value,
        context: ExecutionContext,
    ) -> CapabilityResult<CapabilityResult_output>;

    /// Called when the capability is registered.
    async fn on_register(&mut self) -> CapabilityResult<()> {
        Ok(())
    }

    /// Called when the capability is enabled.
    async fn on_enable(&mut self) -> CapabilityResult<()> {
        Ok(())
    }

    /// Called when the capability is disabled.
    async fn on_disable(&mut self) -> CapabilityResult<()> {
        Ok(())
    }

    /// Called when the capability is revoked.
    async fn on_revoke(&mut self) -> CapabilityResult<()> {
        Ok(())
    }

    /// Check if this capability can execute with the given context.
    fn can_execute(&self, context: &ExecutionContext) -> bool {
        context.has_required_permissions(&self.metadata().required_permissions)
    }

    /// Get the estimated execution time in milliseconds.
    fn estimated_duration_ms(&self) -> Option<u64> {
        None
    }
}

/// Execution context passed to capabilities during execution.
#[derive(Clone)]
pub struct ExecutionContext {
    /// Unique execution ID.
    pub execution_id: Uuid,
    /// The capability being executed.
    pub capability_id: CapabilityId,
    /// Current permissions.
    pub permissions: Vec<String>,
    /// Execution environment variables.
    pub environment: HashMap<String, String>,
    /// Maximum execution timeout in milliseconds.
    pub timeout_ms: Option<u64>,
    /// Cancellation token.
    pub cancel_token: CancellationToken,
    /// Progress callback.
    pub progress_callback: Option<Arc<dyn Fn(ProgressUpdate) + Send + Sync>>,
}

impl ExecutionContext {
    /// Create a new execution context.
    pub fn new(capability_id: CapabilityId) -> Self {
        Self {
            execution_id: Uuid::new_v4(),
            capability_id,
            permissions: Vec::new(),
            environment: HashMap::new(),
            timeout_ms: None,
            cancel_token: CancellationToken::new(),
            progress_callback: None,
        }
    }

    /// Set the timeout.
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    /// Add a permission.
    pub fn with_permission(mut self, perm: impl Into<String>) -> Self {
        self.permissions.push(perm.into());
        self
    }

    /// Add multiple permissions.
    pub fn with_permissions(mut self, perms: Vec<String>) -> Self {
        self.permissions.extend(perms);
        self
    }

    /// Set environment variable.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.environment.insert(key.into(), value.into());
        self
    }

    /// Set the progress callback.
    pub fn with_progress_callback(
        mut self,
        callback: impl Fn(ProgressUpdate) + Send + Sync + 'static,
    ) -> Self {
        self.progress_callback = Some(Arc::new(callback));
        self
    }

    /// Check if required permissions are present.
    pub fn has_required_permissions(&self, required: &[String]) -> bool {
        required
            .iter()
            .all(|p| self.permissions.iter().any(|ep| ep == p || ep == "admin"))
    }

    /// Check if execution has been cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }

    /// Report progress.
    pub fn report_progress(&self, update: ProgressUpdate) {
        if let Some(callback) = &self.progress_callback {
            callback(update);
        }
    }
}

/// Simple cancellation token for capability execution.
#[derive(Debug, Clone)]
pub struct CancellationToken {
    cancelled: Arc<std::sync::atomic::AtomicBool>,
}

impl CancellationToken {
    /// Create a new cancellation token.
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Cancel the token.
    pub fn cancel(&self) {
        self.cancelled.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Check if cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// A registered capability entry in the registry.
#[derive(Debug, Clone)]
pub struct CapabilityEntry {
    /// The metadata.
    pub metadata: CapabilityMetadata,
    /// Current state.
    pub state: CapabilityState,
    /// When it was registered.
    pub registered_at: DateTime<Utc>,
    /// When it was last executed.
    pub last_executed_at: Option<DateTime<Utc>>,
    /// Number of times executed.
    pub execution_count: u64,
    /// Whether it's currently executing.
    pub is_executing: bool,
}

impl CapabilityEntry {
    /// Create a new entry from metadata.
    pub fn new(metadata: CapabilityMetadata) -> Self {
        Self {
            metadata,
            state: CapabilityState::Defined,
            registered_at: Utc::now(),
            last_executed_at: None,
            execution_count: 0,
            is_executing: false,
        }
    }

    /// Transition to a new state.
    pub fn transition(&mut self, target: CapabilityState) -> CapabilityResult<()> {
        if !self.state.can_transition_to(target) {
            return Err(CapabilityError::invalid_state(format!(
                "cannot transition capability '{}' from {} to {}",
                self.metadata.name, self.state, target
            )));
        }
        self.state = target;
        Ok(())
    }
}

/// A typed wrapper for capability search results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitySummary {
    pub id: CapabilityId,
    pub name: String,
    pub version: CapabilityVersion,
    pub category: CapabilityCategory,
    pub namespace: CapabilityNamespace,
    pub description: String,
    pub state: CapabilityState,
    pub tags: Vec<String>,
    pub execution_count: u64,
    pub last_executed_at: Option<DateTime<Utc>>,
}

impl From<&CapabilityEntry> for CapabilitySummary {
    fn from(entry: &CapabilityEntry) -> Self {
        Self {
            id: entry.metadata.id,
            name: entry.metadata.name.clone(),
            version: entry.metadata.version.clone(),
            category: entry.metadata.category.clone(),
            namespace: entry.metadata.namespace.clone(),
            description: entry.metadata.description.clone(),
            state: entry.state,
            tags: entry.metadata.tags.0.iter().cloned().collect(),
            execution_count: entry.execution_count,
            last_executed_at: entry.last_executed_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_id_creation() {
        let id1 = CapabilityId::new();
        let id2 = CapabilityId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn capability_id_display() {
        let id = CapabilityId::new();
        let s = format!("{}", id);
        assert_eq!(s.len(), 36);
    }

    #[test]
    fn version_ordering() {
        let v1 = CapabilityVersion::new(1, 0, 0);
        let v2 = CapabilityVersion::new(1, 1, 0);
        let v3 = CapabilityVersion::new(2, 0, 0);
        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v1 < v3);
    }

    #[test]
    fn version_compatibility() {
        let v1 = CapabilityVersion::new(1, 0, 0);
        let v1_1 = CapabilityVersion::new(1, 1, 0);
        let v2 = CapabilityVersion::new(2, 0, 0);

        assert!(v1_1.is_compatible_with(&v1));
        assert!(!v2.is_compatible_with(&v1));
        assert!(v1.is_compatible_with(&v1));
    }

    #[test]
    fn version_bump() {
        let mut v = CapabilityVersion::new(1, 2, 3);
        v.bump_patch();
        assert_eq!(v, CapabilityVersion::new(1, 2, 4));
        v.bump_minor();
        assert_eq!(v, CapabilityVersion::new(1, 3, 0));
        v.bump_major();
        assert_eq!(v, CapabilityVersion::new(2, 0, 0));
    }

    #[test]
    fn version_parse() {
        let v: CapabilityVersion = "1.2.3".parse().unwrap();
        assert_eq!(v, CapabilityVersion::new(1, 2, 3));
        assert!("1.2".parse::<CapabilityVersion>().is_err());
    }

    #[test]
    fn namespace_operations() {
        let ns = CapabilityNamespace::core();
        assert_eq!(ns.as_str(), "neo.core");

        let ns = CapabilityNamespace::inference();
        assert_eq!(ns.as_str(), "neo.inference");
    }

    #[test]
    fn tags_operations() {
        let mut tags = CapabilityTags::empty();
        tags.add("ai");
        tags.add("reasoning");
        assert!(tags.contains("ai"));
        assert!(!tags.contains("web"));
        assert!(tags.contains_all(&["ai", "reasoning"]));
        assert!(!tags.contains_all(&["ai", "web"]));
        assert!(tags.contains_any(&["ai", "web"]));
    }

    #[test]
    fn aliases_operations() {
        let mut aliases = CapabilityAliases::empty();
        aliases.add("SummarizeText");
        aliases.add("summarize");
        assert!(aliases.matches("SummarizeText"));
        assert!(aliases.matches("summarize"));
        assert!(aliases.matches("SUMMARIZE"));
        assert!(!aliases.matches("other"));
    }

    #[test]
    fn state_transitions() {
        assert!(CapabilityState::Defined.can_transition_to(CapabilityState::Registered));
        assert!(CapabilityState::Registered.can_transition_to(CapabilityState::Enabled));
        assert!(CapabilityState::Enabled.can_transition_to(CapabilityState::Executing));
        assert!(CapabilityState::Executing.can_transition_to(CapabilityState::Enabled));
        assert!(!CapabilityState::Revoked.can_transition_to(CapabilityState::Enabled));
        assert!(CapabilityState::Failed.can_transition_to(CapabilityState::Registered));
    }

    #[test]
    fn state_is_terminal() {
        assert!(CapabilityState::Revoked.is_terminal());
        assert!(CapabilityState::Failed.is_terminal());
        assert!(!CapabilityState::Enabled.is_terminal());
    }

    #[test]
    fn metadata_validation() {
        let meta = CapabilityMetadata::new(
            "test",
            CapabilityVersion::initial(),
            "A test capability",
            CapabilityCategory::System,
        );
        assert!(meta.validate().is_ok());

        let bad = CapabilityMetadata {
            name: String::new(),
            ..CapabilityMetadata::new(
                "x",
                CapabilityVersion::initial(),
                "desc",
                CapabilityCategory::System,
            )
        };
        assert!(bad.validate().is_err());
    }

    #[test]
    fn resource_requirements() {
        let none = ResourceRequirements::none();
        assert_eq!(none.cpu_units, 0.0);

        let heavy = ResourceRequirements::heavy();
        assert!(heavy.gpu_units > 0.0);
        assert!(heavy.memory_bytes > 0);
    }

    #[test]
    fn input_schema_validation() {
        let schema = InputSchema::new(
            serde_json::json!({"type": "object", "required": ["text"]}),
            "text input",
        );

        assert!(schema.validate(&serde_json::json!({"text": "hello"})).is_ok());
        assert!(schema.validate(&serde_json::json!({"other": "hello"})).is_err());

        let optional = schema.optional();
        assert!(optional.validate(&serde_json::Value::Null).is_ok());
    }

    #[test]
    fn progress_update() {
        let p = ProgressUpdate::new(3, 10, "step 3");
        assert_eq!(p.current, 3);
        assert_eq!(p.total, 10);
        assert!((p.percentage - 30.0).abs() < 0.01);
    }

    #[test]
    fn capability_result_output() {
        let ok = CapabilityResult_output::success(serde_json::json!({"done": true}), 100);
        assert!(ok.success);
        assert!(ok.error.is_none());

        let err = CapabilityResult_output::failure("bad", 50);
        assert!(!err.success);
        assert!(err.error.is_some());
    }

    #[test]
    fn execution_context_permissions() {
        let ctx = ExecutionContext::new(CapabilityId::new())
            .with_permission("read")
            .with_permission("write");

        assert!(ctx.has_required_permissions(&["read".to_string()]));
        assert!(ctx.has_required_permissions(&["read".to_string(), "write".to_string()]));
        assert!(!ctx.has_required_permissions(&["admin".to_string()]));
        assert!(ctx.has_required_permissions(&[]));
    }

    #[test]
    fn cancellation_token() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn capability_entry_transition() {
        let meta = CapabilityMetadata::new(
            "test",
            CapabilityVersion::initial(),
            "desc",
            CapabilityCategory::System,
        );
        let mut entry = CapabilityEntry::new(meta);
        assert_eq!(entry.state, CapabilityState::Defined);

        entry.transition(CapabilityState::Registered).unwrap();
        assert_eq!(entry.state, CapabilityState::Registered);

        entry.transition(CapabilityState::Enabled).unwrap();
        assert_eq!(entry.state, CapabilityState::Enabled);
    }

    #[test]
    fn capability_summary_from_entry() {
        let meta = CapabilityMetadata::new(
            "test-cap",
            CapabilityVersion::new(1, 0, 0),
            "A test",
            CapabilityCategory::Tool,
        )
        .with_tag("testing");

        let mut entry = CapabilityEntry::new(meta);
        entry.execution_count = 42;
        entry.transition(CapabilityState::Registered).unwrap();

        let summary = CapabilitySummary::from(&entry);
        assert_eq!(summary.name, "test-cap");
        assert_eq!(summary.execution_count, 42);
        assert!(summary.tags.contains(&"testing".to_string()));
    }
}
