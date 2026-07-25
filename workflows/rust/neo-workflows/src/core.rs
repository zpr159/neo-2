use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{WorkflowError, WorkflowResult};

// ---------------------------------------------------------------------------
// ID Types
// ---------------------------------------------------------------------------

macro_rules! define_workflow_id {
    ($name:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub Uuid);

        impl $name {
            /// Create a new random ID.
            #[must_use]
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple(stringify!($name)).field(&self.0).finish()
            }
        }

        impl std::str::FromStr for $name {
            type Err = uuid::Error;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(Uuid::parse_str(s)?))
            }
        }

        impl From<Uuid> for $name {
            fn from(uuid: Uuid) -> Self {
                Self(uuid)
            }
        }

        impl From<$name> for Uuid {
            fn from(id: $name) -> Self {
                id.0
            }
        }
    };
}

define_workflow_id!(WorkflowId);
define_workflow_id!(NodeId);
define_workflow_id!(ExecutionId);
define_workflow_id!(CheckpointId);
define_workflow_id!(ScheduleId);

// ---------------------------------------------------------------------------
// Workflow State Machine
// ---------------------------------------------------------------------------

/// Lifecycle state of a workflow execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, strum::Display)]
#[strum(serialize_all = "PascalCase")]
pub enum WorkflowState {
    /// Definition created but not yet queued.
    Created,
    /// Queued and waiting for resources.
    Queued,
    /// Actively executing nodes.
    Running,
    /// Waiting for an external event or approval.
    Waiting,
    /// Paused by user or policy.
    Paused,
    /// Successfully completed all nodes.
    Completed,
    /// Execution failed with an unrecoverable error.
    Failed,
    /// Cancelled by user or system.
    Cancelled,
    /// Execution exceeded its timeout.
    TimedOut,
    /// Executing compensation actions in reverse order.
    RollingBack,
}

impl WorkflowState {
    /// Returns `true` if the workflow can accept a `run` command.
    #[must_use]
    pub fn can_run(self) -> bool {
        matches!(self, Self::Created | Self::Queued | Self::Paused)
    }

    /// Returns `true` if the workflow can be paused.
    #[must_use]
    pub fn can_pause(self) -> bool {
        matches!(self, Self::Running | Self::Waiting)
    }

    /// Returns `true` if the workflow can be resumed.
    #[must_use]
    pub fn can_resume(self) -> bool {
        matches!(self, Self::Paused | Self::Waiting)
    }

    /// Returns `true` if the workflow can be cancelled.
    #[must_use]
    pub fn can_cancel(self) -> bool {
        !matches!(self, Self::Completed | Self::Cancelled | Self::Failed)
    }

    /// Returns `true` if the workflow is in a terminal state.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Cancelled | Self::TimedOut
        )
    }

    /// Valid transitions from this state.
    #[must_use]
    pub fn valid_transitions(self) -> &'static [WorkflowState] {
        match self {
            Self::Created => &[Self::Queued, Self::Cancelled],
            Self::Queued => &[Self::Running, Self::Cancelled],
            Self::Running => &[
                Self::Waiting,
                Self::Paused,
                Self::Completed,
                Self::Failed,
                Self::Cancelled,
                Self::TimedOut,
                Self::RollingBack,
            ],
            Self::Waiting => &[Self::Running, Self::Paused, Self::Cancelled, Self::TimedOut],
            Self::Paused => &[Self::Running, Self::Cancelled],
            Self::RollingBack => &[Self::Completed, Self::Failed, Self::Cancelled],
            Self::Completed => &[],
            Self::Failed => &[Self::Created],
            Self::Cancelled => &[Self::Created],
            Self::TimedOut => &[Self::Created],
        }
    }

    /// Attempt a state transition, returning an error if invalid.
    pub fn try_transition(self, target: WorkflowState) -> WorkflowResult<WorkflowState> {
        if self.valid_transitions().contains(&target) {
            Ok(target)
        } else {
            Err(WorkflowError::invalid_state(
                self,
                "transition",
                format!("{:?}", self.valid_transitions()),
            ))
        }
    }
}

impl Default for WorkflowState {
    fn default() -> Self {
        Self::Created
    }
}

// ---------------------------------------------------------------------------
// Node State
// ---------------------------------------------------------------------------

/// State of an individual node within a workflow execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, strum::Display)]
#[strum(serialize_all = "PascalCase")]
pub enum NodeState {
    /// Not yet ready to execute (dependencies not met).
    Pending,
    /// All dependencies met, ready to be scheduled.
    Ready,
    /// Currently executing.
    Running,
    /// Completed successfully.
    Completed,
    /// Failed with an error.
    Failed,
    /// Skipped due to conditional logic.
    Skipped,
    /// Waiting for human approval.
    WaitingForApproval,
    /// Compensation action in progress.
    Compensating,
    /// Compensation action completed.
    Compensated,
}

impl NodeState {
    /// Returns `true` if the node is in a terminal state.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Skipped | Self::Compensated
        )
    }

    /// Returns `true` if the node can be retried.
    #[must_use]
    pub fn is_retryable(self) -> bool {
        matches!(self, Self::Failed | Self::Skipped)
    }
}

impl Default for NodeState {
    fn default() -> Self {
        Self::Pending
    }
}

// ---------------------------------------------------------------------------
// Workflow Metadata
// ---------------------------------------------------------------------------

/// Metadata associated with a workflow definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetadata {
    /// Human-readable name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Author.
    pub author: String,
    /// Tags for categorization.
    pub tags: Vec<String>,
    /// Custom key-value metadata.
    pub properties: HashMap<String, serde_json::Value>,
    /// When the workflow was created.
    pub created_at: DateTime<Utc>,
    /// When the workflow was last modified.
    pub modified_at: DateTime<Utc>,
}

impl WorkflowMetadata {
    /// Create new metadata with the given name.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            name: name.into(),
            description: String::new(),
            author: String::new(),
            tags: Vec::new(),
            properties: HashMap::new(),
            created_at: now,
            modified_at: now,
        }
    }
}

// ---------------------------------------------------------------------------
// Workflow Configuration
// ---------------------------------------------------------------------------

/// Configuration for workflow execution behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    /// Maximum number of retry attempts per node.
    pub max_retries: u32,
    /// Overall timeout in milliseconds (0 = no timeout).
    pub timeout_ms: u64,
    /// Enable parallel node execution.
    pub enable_parallel: bool,
    /// Maximum concurrent nodes for parallel execution.
    pub max_concurrency: u32,
    /// Enable automatic checkpointing.
    pub enable_checkpoints: bool,
    /// Checkpoint interval in milliseconds.
    pub checkpoint_interval_ms: u64,
    /// Enable automatic rollback on failure.
    pub enable_rollback: bool,
    /// Whether to continue execution if a non-critical node fails.
    pub continue_on_non_critical_failure: bool,
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            timeout_ms: 3_600_000,
            enable_parallel: true,
            max_concurrency: 10,
            enable_checkpoints: true,
            checkpoint_interval_ms: 30_000,
            enable_rollback: true,
            continue_on_non_critical_failure: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Workflow Version
// ---------------------------------------------------------------------------

/// Semantic version for workflow definitions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowVersion {
    /// Major version (breaking changes).
    pub major: u32,
    /// Minor version (new features, backwards compatible).
    pub minor: u32,
    /// Patch version (bug fixes).
    pub patch: u32,
}

impl WorkflowVersion {
    /// Create a new version.
    #[must_use]
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Initial version (1.0.0).
    #[must_use]
    pub fn initial() -> Self {
        Self::new(1, 0, 0)
    }

    /// Bump the major version.
    pub fn bump_major(&mut self) {
        self.major += 1;
        self.minor = 0;
        self.patch = 0;
    }

    /// Bump the minor version.
    pub fn bump_minor(&mut self) {
        self.minor += 1;
        self.patch = 0;
    }

    /// Bump the patch version.
    pub fn bump_patch(&mut self) {
        self.patch += 1;
    }

    /// Check if this version is compatible with (>=) the required version.
    #[must_use]
    pub fn is_compatible_with(&self, required: &WorkflowVersion) -> bool {
        if self.major != required.major {
            return false;
        }
        self.minor >= required.minor
            && (self.minor > required.minor || self.patch >= required.patch)
    }
}

impl fmt::Display for WorkflowVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Default for WorkflowVersion {
    fn default() -> Self {
        Self::initial()
    }
}

// ---------------------------------------------------------------------------
// Workflow Context
// ---------------------------------------------------------------------------

/// Runtime context passed through workflow execution.
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowContext {
    /// Execution-level variables.
    #[serde(skip)]
    pub variables: RwLock<HashMap<String, serde_json::Value>>,
    /// Parent execution ID (for sub-workflows).
    pub parent_execution_id: Option<ExecutionId>,
    /// Environment variables.
    pub environment: HashMap<String, String>,
    /// Custom metadata for this execution.
    pub metadata: HashMap<String, serde_json::Value>,
    /// Cancellation flag.
    #[serde(skip)]
    pub cancelled: Arc<AtomicBool>,
}

impl Clone for WorkflowContext {
    fn clone(&self) -> Self {
        Self {
            variables: RwLock::new(self.variables.read().clone()),
            parent_execution_id: self.parent_execution_id,
            environment: self.environment.clone(),
            metadata: self.metadata.clone(),
            cancelled: Arc::new(AtomicBool::new(self.cancelled.load(Ordering::SeqCst))),
        }
    }
}

impl WorkflowContext {
    /// Create a new empty context.
    #[must_use]
    pub fn new() -> Self {
        Self {
            variables: RwLock::new(HashMap::new()),
            parent_execution_id: None,
            environment: HashMap::new(),
            metadata: HashMap::new(),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Set a variable.
    pub fn set_variable(&self, key: impl Into<String>, value: serde_json::Value) {
        self.variables.write().insert(key.into(), value);
    }

    /// Get a variable.
    #[must_use]
    pub fn get_variable(&self, key: &str) -> Option<serde_json::Value> {
        self.variables.read().get(key).cloned()
    }

    /// Get all variables as a snapshot.
    #[must_use]
    pub fn snapshot_variables(&self) -> HashMap<String, serde_json::Value> {
        self.variables.read().clone()
    }

    /// Set multiple variables from a map.
    pub fn set_variables(&self, vars: HashMap<String, serde_json::Value>) {
        self.variables.write().extend(vars);
    }

    /// Request cancellation.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Check if cancelled.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

impl Default for WorkflowContext {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Workflow Result
// ---------------------------------------------------------------------------

/// Result produced by a completed workflow execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResultOutput {
    /// Whether the workflow completed successfully.
    pub success: bool,
    /// Output value from the final node.
    pub output: serde_json::Value,
    /// Error message if failed.
    pub error: Option<String>,
    /// Total execution duration in milliseconds.
    pub duration_ms: u64,
    /// Number of nodes that executed.
    pub nodes_executed: u32,
    /// Number of retries performed.
    pub retries: u32,
}

impl WorkflowResultOutput {
    /// Create a successful result.
    #[must_use]
    pub fn success(output: serde_json::Value, duration_ms: u64, nodes_executed: u32) -> Self {
        Self {
            success: true,
            output,
            error: None,
            duration_ms,
            nodes_executed,
            retries: 0,
        }
    }

    /// Create a failure result.
    #[must_use]
    pub fn failure(error: String, duration_ms: u64, nodes_executed: u32) -> Self {
        Self {
            success: false,
            output: serde_json::Value::Null,
            error: Some(error),
            duration_ms,
            nodes_executed,
            retries: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Workflow Statistics
// ---------------------------------------------------------------------------

/// Aggregated statistics for a workflow.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkflowStatistics {
    /// Total number of executions.
    pub total_executions: u64,
    /// Number of successful executions.
    pub successful_executions: u64,
    /// Number of failed executions.
    pub failed_executions: u64,
    /// Number of cancelled executions.
    pub cancelled_executions: u64,
    /// Average execution duration in milliseconds.
    pub avg_duration_ms: f64,
    /// Average node latency in milliseconds.
    pub avg_node_latency_ms: f64,
    /// Total retry count across all executions.
    pub total_retries: u64,
    /// Total rollback count.
    pub total_rollbacks: u64,
    /// Success rate (0.0 to 1.0).
    pub success_rate: f64,
    /// Timestamp of the last execution.
    pub last_executed_at: Option<DateTime<Utc>>,
}

impl WorkflowStatistics {
    /// Update statistics with a new execution result.
    pub fn record_execution(&mut self, result: &WorkflowResultOutput) {
        self.total_executions += 1;
        if result.success {
            self.successful_executions += 1;
        } else {
            self.failed_executions += 1;
        }
        self.total_retries += u64::from(result.retries);
        self.last_executed_at = Some(Utc::now());

        let n = self.total_executions as f64;
        self.avg_duration_ms = (self.avg_duration_ms * (n - 1.0) + (result.duration_ms as f64)) / n;
        self.success_rate = self.successful_executions as f64 / n;
    }
}

// ---------------------------------------------------------------------------
// Workflow Snapshot (for persistence)
// ---------------------------------------------------------------------------

/// Serializable snapshot of a workflow definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSnapshot {
    /// The workflow ID.
    pub workflow_id: WorkflowId,
    /// Version.
    pub version: WorkflowVersion,
    /// Serialized definition.
    pub definition_json: serde_json::Value,
    /// Serialized config.
    pub config: WorkflowConfig,
    /// Metadata.
    pub metadata: WorkflowMetadata,
    /// Snapshot timestamp.
    pub created_at: DateTime<Utc>,
    /// Checksum for integrity.
    pub checksum: String,
}

impl WorkflowSnapshot {
    /// Compute SHA-256 checksum of the definition.
    #[must_use]
    pub fn compute_checksum(definition_json: &serde_json::Value) -> String {
        use sha2::{Digest, Sha256};
        let bytes = serde_json::to_vec(definition_json).unwrap_or_default();
        let hash = Sha256::digest(&bytes);
        format!("{hash:x}")
    }
}

// Re-export Arc for WorkflowContext
use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_id_roundtrip() {
        let id = WorkflowId::new();
        let s = id.to_string();
        let parsed: WorkflowId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn state_transitions() {
        assert!(WorkflowState::Created.can_run());
        assert!(!WorkflowState::Running.can_run());
        assert!(WorkflowState::Running.can_pause());
        assert!(!WorkflowState::Completed.can_pause());
        assert!(WorkflowState::Running.can_cancel());
        assert!(!WorkflowState::Completed.can_cancel());
        assert!(WorkflowState::Completed.is_terminal());
        assert!(!WorkflowState::Running.is_terminal());
    }

    #[test]
    fn state_try_transition() {
        let result = WorkflowState::Created.try_transition(WorkflowState::Running);
        assert!(result.is_err());
        let result = WorkflowState::Created.try_transition(WorkflowState::Queued);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), WorkflowState::Queued);
    }

    #[test]
    fn node_state_properties() {
        assert!(NodeState::Completed.is_terminal());
        assert!(!NodeState::Running.is_terminal());
        assert!(NodeState::Failed.is_retryable());
        assert!(!NodeState::Completed.is_retryable());
    }

    #[test]
    fn version_comparison() {
        let v1 = WorkflowVersion::new(1, 0, 0);
        let v2 = WorkflowVersion::new(1, 1, 0);
        let v3 = WorkflowVersion::new(2, 0, 0);
        assert!(v2.is_compatible_with(&v1));
        assert!(!v1.is_compatible_with(&v2));
        assert!(!v3.is_compatible_with(&v1));
    }

    #[test]
    fn version_display() {
        let v = WorkflowVersion::new(2, 3, 4);
        assert_eq!(v.to_string(), "2.3.4");
    }

    #[test]
    fn version_bump() {
        let mut v = WorkflowVersion::new(1, 2, 3);
        v.bump_patch();
        assert_eq!(v, WorkflowVersion::new(1, 2, 4));
        v.bump_minor();
        assert_eq!(v, WorkflowVersion::new(1, 3, 0));
        v.bump_major();
        assert_eq!(v, WorkflowVersion::new(2, 0, 0));
    }

    #[test]
    fn context_variables() {
        let ctx = WorkflowContext::new();
        ctx.set_variable("x".to_string(), serde_json::json!(42));
        assert_eq!(ctx.get_variable("x"), Some(serde_json::json!(42)));
        assert!(ctx.get_variable("y").is_none());
        let snapshot = ctx.snapshot_variables();
        assert_eq!(snapshot.len(), 1);
    }

    #[test]
    fn context_cancellation() {
        let ctx = WorkflowContext::new();
        assert!(!ctx.is_cancelled());
        ctx.cancel();
        assert!(ctx.is_cancelled());
    }

    #[test]
    fn result_output_constructors() {
        let ok = WorkflowResultOutput::success(serde_json::json!("done"), 100, 5);
        assert!(ok.success);
        assert_eq!(ok.duration_ms, 100);

        let err = WorkflowResultOutput::failure("boom".into(), 200, 3);
        assert!(!err.success);
        assert_eq!(err.error.unwrap(), "boom");
    }

    #[test]
    fn statistics_recording() {
        let mut stats = WorkflowStatistics::default();
        let r1 = WorkflowResultOutput::success(serde_json::json!(null), 100, 5);
        let r2 = WorkflowResultOutput::failure("err".into(), 200, 3);
        stats.record_execution(&r1);
        stats.record_execution(&r2);
        assert_eq!(stats.total_executions, 2);
        assert_eq!(stats.successful_executions, 1);
        assert_eq!(stats.failed_executions, 1);
        assert!((stats.success_rate - 0.5).abs() < f64::EPSILON);
        assert!(stats.last_executed_at.is_some());
    }

    #[test]
    fn snapshot_checksum() {
        let v1 = serde_json::json!({"a": 1});
        let v2 = serde_json::json!({"a": 1});
        let v3 = serde_json::json!({"a": 2});
        assert_eq!(
            WorkflowSnapshot::compute_checksum(&v1),
            WorkflowSnapshot::compute_checksum(&v2)
        );
        assert_ne!(
            WorkflowSnapshot::compute_checksum(&v1),
            WorkflowSnapshot::compute_checksum(&v3)
        );
    }

    #[test]
    fn default_impls() {
        let _ = WorkflowId::default();
        let _ = NodeId::default();
        let _ = WorkflowState::default();
        let _ = NodeState::default();
        let _ = WorkflowConfig::default();
        let _ = WorkflowVersion::default();
        let _ = WorkflowContext::default();
    }

    #[test]
    fn workflow_metadata_creation() {
        let meta = WorkflowMetadata::new("test-workflow");
        assert_eq!(meta.name, "test-workflow");
        assert!(meta.description.is_empty());
        assert!(meta.tags.is_empty());
    }
}
