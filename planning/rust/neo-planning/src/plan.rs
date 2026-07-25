//! Plan system for the Neo Planning engine.
//!
//! Provides the core `Plan` type and supporting structures for defining,
//! validating, executing, and tracking hierarchical task plans. The plan
//! lifecycle is driven by an explicit state machine (`PlanState`) and
//! each mutation is checkpointed for auditability.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::error::{PlanningError, PlanningErrorCode, PlanningResult};
use crate::id::{
    PlanCheckpointId, PlanId, PlanningGoalId, PlanningNodeId, PlanningSessionId, StrategyId,
};
use crate::types::{
    AlgorithmType, ExecutionBudget, PlanMetadata, PlanMetrics, PlanStatistics, PlanTaskType,
    PlanVersion, PlanningConfiguration, ResourceRequirements, TaskStatus,
};

// ---------------------------------------------------------------------------
// PlanState — state machine
// ---------------------------------------------------------------------------

/// Lifecycle state of a plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlanState {
    Created,
    Validated,
    Generating,
    Generated,
    Optimizing,
    Optimized,
    Executing,
    Paused,
    Completed,
    Failed,
    Cancelled,
    Archived,
}

impl PlanState {
    /// Returns `true` if no further transitions are possible from this state.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            PlanState::Completed | PlanState::Failed | PlanState::Cancelled | PlanState::Archived
        )
    }

    /// List of states this state can transition to.
    pub fn valid_transitions(self) -> &'static [PlanState] {
        match self {
            Self::Created => &[Self::Validated, Self::Failed, Self::Cancelled],
            Self::Validated => &[Self::Generating, Self::Failed, Self::Cancelled],
            Self::Generating => &[Self::Generated, Self::Failed, Self::Cancelled],
            Self::Generated => &[
                Self::Optimizing,
                Self::Executing,
                Self::Failed,
                Self::Cancelled,
            ],
            Self::Optimizing => &[Self::Optimized, Self::Failed, Self::Cancelled],
            Self::Optimized => &[Self::Executing, Self::Failed, Self::Cancelled],
            Self::Executing => &[Self::Paused, Self::Completed, Self::Failed, Self::Cancelled],
            Self::Paused => &[Self::Executing, Self::Failed, Self::Cancelled],
            Self::Completed => &[Self::Archived],
            Self::Failed => &[],
            Self::Cancelled => &[],
            Self::Archived => &[],
        }
    }

    /// Check whether transitioning from `self` to `target` is allowed.
    pub fn can_transition_to(self, target: PlanState) -> bool {
        self.valid_transitions().contains(&target)
    }
}

// ---------------------------------------------------------------------------
// PlanTask
// ---------------------------------------------------------------------------

/// A single task within a plan definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanTask {
    pub id: PlanningNodeId,
    pub name: String,
    pub description: String,
    pub task_type: PlanTaskType,
    pub status: TaskStatus,
    pub dependencies: Vec<PlanningNodeId>,
    pub cost_estimate: f64,
    pub duration_estimate_secs: u64,
    pub resource_requirements: ResourceRequirements,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PlanTask {
    /// Create a new task with the given name and type.
    pub fn new(name: impl Into<String>, task_type: PlanTaskType) -> Self {
        let now = Utc::now();
        Self {
            id: PlanningNodeId::new(),
            name: name.into(),
            description: String::new(),
            task_type,
            status: TaskStatus::Pending,
            dependencies: Vec::new(),
            cost_estimate: 0.0,
            duration_estimate_secs: 0,
            resource_requirements: ResourceRequirements::default(),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Set the description.
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Add a dependency on another task.
    #[must_use]
    pub fn with_dependency(mut self, dep: PlanningNodeId) -> Self {
        self.dependencies.push(dep);
        self
    }

    /// Set the cost estimate.
    #[must_use]
    pub fn with_cost_estimate(mut self, cost: f64) -> Self {
        self.cost_estimate = cost;
        self
    }

    /// Set the duration estimate in seconds.
    #[must_use]
    pub fn with_duration_estimate(mut self, secs: u64) -> Self {
        self.duration_estimate_secs = secs;
        self
    }

    /// Set the resource requirements.
    #[must_use]
    pub fn with_resources(mut self, resources: ResourceRequirements) -> Self {
        self.resource_requirements = resources;
        self
    }

    /// Insert a metadata entry.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Transition the task to a new status.
    pub fn transition(&mut self, target: TaskStatus) {
        self.status = target;
        self.updated_at = Utc::now();
    }
}

// ---------------------------------------------------------------------------
// PlanCheckpoint
// ---------------------------------------------------------------------------

/// A point-in-time checkpoint capturing the state of a plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanCheckpoint {
    pub id: PlanCheckpointId,
    pub plan_id: PlanId,
    pub version: PlanVersion,
    pub state: PlanState,
    pub task_statuses: HashMap<PlanningNodeId, TaskStatus>,
    pub created_at: DateTime<Utc>,
}

impl PlanCheckpoint {
    /// Create a new checkpoint.
    pub fn new(plan_id: PlanId, version: PlanVersion, state: PlanState) -> Self {
        Self {
            id: PlanCheckpointId::new(),
            plan_id,
            version,
            state,
            task_statuses: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    /// Set the task statuses snapshot.
    #[must_use]
    pub fn with_task_statuses(mut self, statuses: HashMap<PlanningNodeId, TaskStatus>) -> Self {
        self.task_statuses = statuses;
        self
    }
}

// ---------------------------------------------------------------------------
// PlanDefinition
// ---------------------------------------------------------------------------

/// Structural definition of a plan: the tasks, their ordering constraints,
/// and the algorithm used to generate the plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanDefinition {
    pub tasks: Vec<PlanTask>,
    pub goal_id: PlanningGoalId,
    pub budget: ExecutionBudget,
    pub algorithm: AlgorithmType,
    pub allow_parallelism: bool,
}

impl PlanDefinition {
    /// Create a new plan definition.
    pub fn new(goal_id: PlanningGoalId, algorithm: AlgorithmType) -> Self {
        Self {
            tasks: Vec::new(),
            goal_id,
            budget: ExecutionBudget::default(),
            algorithm,
            allow_parallelism: false,
        }
    }

    /// Set the execution budget.
    #[must_use]
    pub fn with_budget(mut self, budget: ExecutionBudget) -> Self {
        self.budget = budget;
        self
    }

    /// Enable or disable parallel task execution.
    #[must_use]
    pub fn with_parallelism(mut self, allow: bool) -> Self {
        self.allow_parallelism = allow;
        self
    }

    /// Add a task.
    #[must_use]
    pub fn with_task(mut self, task: PlanTask) -> Self {
        self.tasks.push(task);
        self
    }

    /// Add multiple tasks.
    #[must_use]
    pub fn with_tasks(mut self, tasks: Vec<PlanTask>) -> Self {
        self.tasks.extend(tasks);
        self
    }
}

// ---------------------------------------------------------------------------
// PlanContext
// ---------------------------------------------------------------------------

/// Execution context that is available to the planner and the executor.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlanContext {
    pub goal_descriptions: Vec<String>,
    pub constraints: HashMap<String, serde_json::Value>,
    pub available_resources: ResourceRequirements,
    pub environment: HashMap<String, serde_json::Value>,
}

impl PlanContext {
    /// Create an empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a goal description.
    #[must_use]
    pub fn with_goal_description(mut self, desc: impl Into<String>) -> Self {
        self.goal_descriptions.push(desc.into());
        self
    }

    /// Add a constraint.
    #[must_use]
    pub fn with_constraint(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.constraints.insert(key.into(), value);
        self
    }

    /// Set the available resources.
    #[must_use]
    pub fn with_resources(mut self, resources: ResourceRequirements) -> Self {
        self.available_resources = resources;
        self
    }

    /// Add an environment variable.
    #[must_use]
    pub fn with_environment(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.environment.insert(key.into(), value);
        self
    }
}

// ---------------------------------------------------------------------------
// PlanSnapshot
// ---------------------------------------------------------------------------

/// A read-only snapshot of a plan at a specific point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSnapshot {
    pub plan_id: PlanId,
    pub state: PlanState,
    pub statistics: PlanStatistics,
    pub metrics: PlanMetrics,
    pub timestamp: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// PlanResult
// ---------------------------------------------------------------------------

/// The outcome of a completed plan execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanResult {
    pub plan_id: PlanId,
    pub success: bool,
    pub statistics: PlanStatistics,
    pub metrics: PlanMetrics,
    pub error: Option<String>,
    pub completed_at: DateTime<Utc>,
}

impl PlanResult {
    /// Create a successful result.
    pub fn success(plan_id: PlanId, statistics: PlanStatistics, metrics: PlanMetrics) -> Self {
        Self {
            plan_id,
            success: true,
            statistics,
            metrics,
            error: None,
            completed_at: Utc::now(),
        }
    }

    /// Create a failed result.
    pub fn failure(
        plan_id: PlanId,
        statistics: PlanStatistics,
        metrics: PlanMetrics,
        error: impl Into<String>,
    ) -> Self {
        Self {
            plan_id,
            success: false,
            statistics,
            metrics,
            error: Some(error.into()),
            completed_at: Utc::now(),
        }
    }
}

// ---------------------------------------------------------------------------
// PlanExecution
// ---------------------------------------------------------------------------

/// Tracks the live execution state of a plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanExecution {
    pub plan_id: PlanId,
    pub current_task: Option<PlanningNodeId>,
    pub completed_tasks: HashSet<PlanningNodeId>,
    pub failed_tasks: HashSet<PlanningNodeId>,
    pub started_at: DateTime<Utc>,
    pub elapsed_secs: u64,
}

impl PlanExecution {
    /// Create a new execution tracker.
    pub fn new(plan_id: PlanId) -> Self {
        Self {
            plan_id,
            current_task: None,
            completed_tasks: HashSet::new(),
            failed_tasks: HashSet::new(),
            started_at: Utc::now(),
            elapsed_secs: 0,
        }
    }

    /// Mark a task as complete.
    pub fn mark_complete(&mut self, task_id: PlanningNodeId) {
        self.completed_tasks.insert(task_id);
        if self.current_task == Some(task_id) {
            self.current_task = None;
        }
        self.elapsed_secs = (Utc::now() - self.started_at).num_seconds() as u64;
    }

    /// Mark a task as failed.
    pub fn mark_failed(&mut self, task_id: PlanningNodeId) {
        self.failed_tasks.insert(task_id);
        if self.current_task == Some(task_id) {
            self.current_task = None;
        }
        self.elapsed_secs = (Utc::now() - self.started_at).num_seconds() as u64;
    }

    /// Set the currently executing task.
    pub fn set_current_task(&mut self, task_id: Option<PlanningNodeId>) {
        self.current_task = task_id;
    }

    /// Return the IDs of tasks whose dependencies are all completed and which
    /// have not themselves been completed, failed, or cancelled.
    pub fn next_ready_tasks(&self, tasks: &[PlanTask]) -> Vec<PlanningNodeId> {
        tasks
            .iter()
            .filter(|t| {
                t.status == TaskStatus::Pending
                    && !self.completed_tasks.contains(&t.id)
                    && !self.failed_tasks.contains(&t.id)
                    && t.dependencies
                        .iter()
                        .all(|dep| self.completed_tasks.contains(dep))
            })
            .map(|t| t.id)
            .collect()
    }

    /// Compute the progress as a fraction in `[0.0, 1.0]`.
    pub fn progress(&self, total_tasks: usize) -> f64 {
        if total_tasks == 0 {
            return 1.0;
        }
        self.completed_tasks.len() as f64 / total_tasks as f64
    }

    /// Refresh the elapsed time counter.
    pub fn refresh_elapsed(&mut self) {
        self.elapsed_secs = (Utc::now() - self.started_at).num_seconds() as u64;
    }

    /// Check whether all non-failed tasks are completed.
    pub fn is_done(&self, total_tasks: usize) -> bool {
        self.completed_tasks.len() + self.failed_tasks.len() >= total_tasks
    }
}

// ---------------------------------------------------------------------------
// Plan — the core struct
// ---------------------------------------------------------------------------

/// The central plan structure tying together definition, context, state,
/// metadata, statistics, and metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: PlanId,
    pub version: PlanVersion,
    pub state: PlanState,
    pub definition: PlanDefinition,
    pub context: PlanContext,
    pub metadata: PlanMetadata,
    pub statistics: PlanStatistics,
    pub metrics: PlanMetrics,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Plan {
    /// Create a new plan in the `Created` state.
    pub fn new(definition: PlanDefinition, metadata: PlanMetadata) -> Self {
        let now = Utc::now();
        Self {
            id: PlanId::new(),
            version: PlanVersion::initial(),
            state: PlanState::Created,
            definition,
            context: PlanContext::new(),
            metadata,
            statistics: PlanStatistics::default(),
            metrics: PlanMetrics::default(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Set the execution context.
    #[must_use]
    pub fn with_context(mut self, context: PlanContext) -> Self {
        self.context = context;
        self
    }

    /// Transition the plan to a new state.
    pub fn transition(&mut self, target: PlanState) -> PlanningResult<()> {
        if !self.state.can_transition_to(target) {
            return Err(PlanningError::new(
                PlanningErrorCode::PlanInvalidState,
                format!(
                    "cannot transition plan '{}' from {:?} to {:?}",
                    self.id, self.state, target
                ),
            ));
        }
        self.state = target;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Run basic structural validation on the plan definition.
    ///
    /// Checks:
    /// - at least one task is present
    /// - no task depends on itself
    /// - every dependency references an existing task
    pub fn validate(&self) -> PlanningResult<()> {
        if self.definition.tasks.is_empty() {
            return Err(PlanningError::validation(
                "plan must contain at least one task",
            ));
        }

        let task_ids: HashSet<PlanningNodeId> =
            self.definition.tasks.iter().map(|t| t.id).collect();

        for task in &self.definition.tasks {
            if task.dependencies.contains(&task.id) {
                return Err(PlanningError::new(
                    PlanningErrorCode::PlanGraphCycleDetected,
                    format!("task '{}' depends on itself", task.name),
                ));
            }
            for dep in &task.dependencies {
                if !task_ids.contains(dep) {
                    return Err(PlanningError::new(
                        PlanningErrorCode::PlanValidationFailed,
                        format!("task '{}' depends on unknown task {}", task.name, dep),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Create a point-in-time snapshot.
    pub fn snapshot(&self) -> PlanSnapshot {
        PlanSnapshot {
            plan_id: self.id,
            state: self.state,
            statistics: self.statistics.clone(),
            metrics: self.metrics.clone(),
            timestamp: Utc::now(),
        }
    }

    /// Build a checkpoint from the current state.
    pub fn checkpoint(&self) -> PlanCheckpoint {
        let statuses: HashMap<PlanningNodeId, TaskStatus> = self
            .definition
            .tasks
            .iter()
            .map(|t| (t.id, t.status))
            .collect();

        PlanCheckpoint::new(self.id, self.version, self.state).with_task_statuses(statuses)
    }

    /// Advance the version (patch bump).
    pub fn bump_version(&mut self) {
        self.version = self.version.bump_patch();
        self.updated_at = Utc::now();
    }

    /// Collect the current statistics from the task list.
    pub fn compute_statistics(&mut self) {
        let mut stats = PlanStatistics {
            total_tasks: self.definition.tasks.len(),
            ..Default::default()
        };
        for task in &self.definition.tasks {
            match task.status {
                TaskStatus::Completed => stats.completed_tasks += 1,
                TaskStatus::Failed => stats.failed_tasks += 1,
                TaskStatus::Cancelled => stats.cancelled_tasks += 1,
                TaskStatus::Skipped => stats.skipped_tasks += 1,
                TaskStatus::Pending | TaskStatus::Ready => stats.pending_tasks += 1,
                TaskStatus::Running | TaskStatus::Paused | TaskStatus::Retrying => {
                    stats.running_tasks += 1
                }
            }
        }
        stats.total_cost = self.definition.tasks.iter().map(|t| t.cost_estimate).sum();
        stats.total_duration_ms = self
            .definition
            .tasks
            .iter()
            .map(|t| t.duration_estimate_secs * 1000)
            .sum();
        self.statistics = stats;
    }
}

// ---------------------------------------------------------------------------
// PlanningSession
// ---------------------------------------------------------------------------

/// A session that manages a single planning operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningSession {
    pub id: PlanningSessionId,
    pub plan_id: Option<PlanId>,
    pub state: PlanState,
    pub configuration: PlanningConfiguration,
    pub started_at: DateTime<Utc>,
    pub timeout_secs: u64,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl PlanningSession {
    /// Create a new session.
    pub fn new(configuration: PlanningConfiguration) -> Self {
        Self {
            id: PlanningSessionId::new(),
            plan_id: None,
            state: PlanState::Created,
            configuration,
            started_at: Utc::now(),
            timeout_secs: 300,
            metadata: HashMap::new(),
        }
    }

    /// Associate a plan with this session.
    #[must_use]
    pub fn with_plan_id(mut self, plan_id: PlanId) -> Self {
        self.plan_id = Some(plan_id);
        self
    }

    /// Set the timeout.
    #[must_use]
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    /// Insert metadata.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Check whether the session has exceeded its timeout.
    pub fn is_expired(&self) -> bool {
        let elapsed = (Utc::now() - self.started_at).num_seconds() as u64;
        elapsed >= self.timeout_secs
    }

    /// Return the time remaining before the session expires, in seconds.
    pub fn time_remaining_secs(&self) -> i64 {
        let elapsed = (Utc::now() - self.started_at).num_seconds() as i64;
        let remaining = self.timeout_secs as i64 - elapsed;
        remaining.max(0)
    }

    /// Transition the session state.
    pub fn transition(&mut self, target: PlanState) -> PlanningResult<()> {
        if !self.state.can_transition_to(target) {
            return Err(PlanningError::new(
                PlanningErrorCode::PlanInvalidState,
                format!(
                    "cannot transition session '{}' from {:?} to {:?}",
                    self.id, self.state, target
                ),
            ));
        }
        self.state = target;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Thread-safe PlanStore (convenience)
// ---------------------------------------------------------------------------

/// Thread-safe store for plans, keyed by `PlanId`.
#[derive(Clone)]
pub struct PlanStore {
    inner: Arc<PlanStoreInner>,
}

struct PlanStoreInner {
    plans: RwLock<HashMap<PlanId, Plan>>,
}

impl PlanStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(PlanStoreInner {
                plans: RwLock::new(HashMap::new()),
            }),
        }
    }

    /// Insert a plan.
    pub fn insert(&self, plan: Plan) -> PlanId {
        let id = plan.id;
        self.inner.plans.write().insert(id, plan);
        id
    }

    /// Retrieve a plan by id.
    pub fn get(&self, id: PlanId) -> PlanningResult<Plan> {
        self.inner
            .plans
            .read()
            .get(&id)
            .cloned()
            .ok_or_else(|| PlanningError::plan_not_found(&id.as_str()))
    }

    /// Remove a plan.
    pub fn remove(&self, id: PlanId) -> PlanningResult<Plan> {
        self.inner
            .plans
            .write()
            .remove(&id)
            .ok_or_else(|| PlanningError::plan_not_found(&id.as_str()))
    }

    /// Return the number of stored plans.
    pub fn len(&self) -> usize {
        self.inner.plans.read().len()
    }

    /// Check if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.plans.read().is_empty()
    }

    /// Return all plan IDs.
    pub fn plan_ids(&self) -> Vec<PlanId> {
        self.inner.plans.read().keys().copied().collect()
    }

    /// Return all stored plans.
    pub fn list(&self) -> Vec<Plan> {
        self.inner.plans.read().values().cloned().collect()
    }
}

impl Default for PlanStore {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_task(name: &str) -> PlanTask {
        PlanTask::new(name, PlanTaskType::Atomic)
    }

    fn make_task_with_deps(name: &str, deps: Vec<PlanningNodeId>) -> PlanTask {
        let mut t = make_task(name);
        t.dependencies = deps;
        t
    }

    fn make_plan_with_tasks(tasks: Vec<PlanTask>) -> Plan {
        let goal_id = PlanningGoalId::new();
        let def = PlanDefinition {
            tasks,
            goal_id,
            budget: ExecutionBudget::default(),
            algorithm: AlgorithmType::HierarchicalTaskNetwork,
            allow_parallelism: false,
        };
        Plan::new(def, PlanMetadata::new("test-plan"))
    }

    // ---- PlanState ----

    #[test]
    fn plan_state_created_is_not_terminal() {
        assert!(!PlanState::Created.is_terminal());
    }

    #[test]
    fn plan_state_completed_is_terminal() {
        assert!(PlanState::Completed.is_terminal());
    }

    #[test]
    fn plan_state_failed_is_terminal() {
        assert!(PlanState::Failed.is_terminal());
    }

    #[test]
    fn plan_state_cancelled_is_terminal() {
        assert!(PlanState::Cancelled.is_terminal());
    }

    #[test]
    fn plan_state_archived_is_terminal() {
        assert!(PlanState::Archived.is_terminal());
    }

    #[test]
    fn plan_state_paused_is_not_terminal() {
        assert!(!PlanState::Paused.is_terminal());
    }

    #[test]
    fn plan_state_valid_transition() {
        assert!(PlanState::Created.can_transition_to(PlanState::Validated));
        assert!(PlanState::Created.can_transition_to(PlanState::Failed));
        assert!(PlanState::Created.can_transition_to(PlanState::Cancelled));
        assert!(!PlanState::Created.can_transition_to(PlanState::Executing));
    }

    #[test]
    fn plan_state_executing_to_paused() {
        assert!(PlanState::Executing.can_transition_to(PlanState::Paused));
        assert!(PlanState::Executing.can_transition_to(PlanState::Completed));
        assert!(!PlanState::Executing.can_transition_to(PlanState::Created));
    }

    #[test]
    fn plan_state_terminal_no_transitions() {
        assert!(PlanState::Completed.valid_transitions().is_empty());
        assert!(PlanState::Failed.valid_transitions().is_empty());
        assert!(PlanState::Cancelled.valid_transitions().is_empty());
        assert!(PlanState::Archived.valid_transitions().is_empty());
    }

    #[test]
    fn plan_state_full_lifecycle() {
        let mut state = PlanState::Created;
        let transitions = [
            PlanState::Validated,
            PlanState::Generating,
            PlanState::Generated,
            PlanState::Executing,
            PlanState::Completed,
            PlanState::Archived,
        ];
        for &target in &transitions {
            assert!(
                state.can_transition_to(target),
                "failed {:?} -> {:?}",
                state,
                target
            );
            state = target;
        }
        assert!(state.is_terminal());
    }

    #[test]
    fn plan_state_generated_to_optimizing() {
        assert!(PlanState::Generated.can_transition_to(PlanState::Optimizing));
        assert!(PlanState::Generated.can_transition_to(PlanState::Executing));
    }

    // ---- PlanTask ----

    #[test]
    fn plan_task_creation() {
        let t = make_task("alpha");
        assert_eq!(t.name, "alpha");
        assert_eq!(t.task_type, PlanTaskType::Atomic);
        assert_eq!(t.status, TaskStatus::Pending);
        assert!(t.dependencies.is_empty());
    }

    #[test]
    fn plan_task_builder() {
        let dep_id = PlanningNodeId::new();
        let t = PlanTask::new("built", PlanTaskType::Composite)
            .with_description("desc")
            .with_dependency(dep_id)
            .with_cost_estimate(42.0)
            .with_duration_estimate(100)
            .with_metadata("k", serde_json::json!("v"));

        assert_eq!(t.description, "desc");
        assert_eq!(t.dependencies, vec![dep_id]);
        assert_eq!(t.cost_estimate, 42.0);
        assert_eq!(t.duration_estimate_secs, 100);
        assert_eq!(t.metadata.get("k").unwrap(), "v");
    }

    #[test]
    fn plan_task_transition() {
        let mut t = make_task("x");
        t.transition(TaskStatus::Running);
        assert_eq!(t.status, TaskStatus::Running);
    }

    // ---- PlanCheckpoint ----

    #[test]
    fn checkpoint_creation() {
        let plan_id = PlanId::new();
        let cp = PlanCheckpoint::new(plan_id, PlanVersion::initial(), PlanState::Created);
        assert_eq!(cp.plan_id, plan_id);
        assert_eq!(cp.state, PlanState::Created);
        assert!(cp.task_statuses.is_empty());
    }

    #[test]
    fn checkpoint_with_task_statuses() {
        let plan_id = PlanId::new();
        let node_id = PlanningNodeId::new();
        let mut statuses = HashMap::new();
        statuses.insert(node_id, TaskStatus::Completed);

        let cp = PlanCheckpoint::new(plan_id, PlanVersion::initial(), PlanState::Executing)
            .with_task_statuses(statuses);
        assert_eq!(
            cp.task_statuses.get(&node_id).unwrap(),
            &TaskStatus::Completed
        );
    }

    // ---- PlanDefinition ----

    #[test]
    fn plan_definition_creation() {
        let goal_id = PlanningGoalId::new();
        let def = PlanDefinition::new(goal_id, AlgorithmType::AStar);
        assert!(def.tasks.is_empty());
        assert_eq!(def.goal_id, goal_id);
        assert!(!def.allow_parallelism);
    }

    #[test]
    fn plan_definition_builder() {
        let goal_id = PlanningGoalId::new();
        let t1 = make_task("t1");
        let t2 = make_task("t2");
        let def = PlanDefinition::new(goal_id, AlgorithmType::AStar)
            .with_budget(ExecutionBudget {
                max_cpu_units: 8,
                ..ExecutionBudget::default()
            })
            .with_parallelism(true)
            .with_task(t1)
            .with_tasks(vec![t2]);
        assert_eq!(def.tasks.len(), 2);
        assert!(def.allow_parallelism);
        assert_eq!(def.budget.max_cpu_units, 8);
    }

    // ---- PlanContext ----

    #[test]
    fn plan_context_default() {
        let ctx = PlanContext::default();
        assert!(ctx.goal_descriptions.is_empty());
        assert!(ctx.constraints.is_empty());
        assert!(ctx.environment.is_empty());
    }

    #[test]
    fn plan_context_builder() {
        let ctx = PlanContext::new()
            .with_goal_description("g1")
            .with_goal_description("g2")
            .with_constraint("k", serde_json::json!(42))
            .with_environment("env", serde_json::json!("prod"))
            .with_resources(ResourceRequirements {
                agents: 2,
                ..Default::default()
            });

        assert_eq!(ctx.goal_descriptions.len(), 2);
        assert_eq!(ctx.constraints.get("k").unwrap(), 42);
        assert_eq!(ctx.environment.get("env").unwrap(), "prod");
        assert_eq!(ctx.available_resources.agents, 2);
    }

    // ---- PlanResult ----

    #[test]
    fn plan_result_success() {
        let plan_id = PlanId::new();
        let r = PlanResult::success(plan_id, PlanStatistics::default(), PlanMetrics::default());
        assert!(r.success);
        assert!(r.error.is_none());
        assert_eq!(r.plan_id, plan_id);
    }

    #[test]
    fn plan_result_failure() {
        let plan_id = PlanId::new();
        let r = PlanResult::failure(
            plan_id,
            PlanStatistics::default(),
            PlanMetrics::default(),
            "boom",
        );
        assert!(!r.success);
        assert_eq!(r.error.unwrap(), "boom");
    }

    // ---- PlanExecution ----

    #[test]
    fn plan_execution_mark_complete() {
        let mut ex = PlanExecution::new(PlanId::new());
        let task = PlanningNodeId::new();
        ex.current_task = Some(task);
        ex.mark_complete(task);
        assert!(ex.completed_tasks.contains(&task));
        assert!(ex.current_task.is_none());
    }

    #[test]
    fn plan_execution_mark_failed() {
        let mut ex = PlanExecution::new(PlanId::new());
        let task = PlanningNodeId::new();
        ex.current_task = Some(task);
        ex.mark_failed(task);
        assert!(ex.failed_tasks.contains(&task));
        assert!(ex.current_task.is_none());
    }

    #[test]
    fn plan_execution_next_ready_tasks() {
        let mut ex = PlanExecution::new(PlanId::new());

        let t1 = make_task("t1");
        let t2_id = PlanningNodeId::new();
        let mut t2 = PlanTask::new("t2", PlanTaskType::Atomic);
        t2.id = t2_id;
        t2.dependencies = vec![t1.id];
        let t3 = make_task("t3");

        let tasks = vec![t1.clone(), t2, t3.clone()];

        // Initially, t1 and t3 are ready (no deps), t2 is not.
        let ready = ex.next_ready_tasks(&tasks);
        assert_eq!(ready.len(), 2);
        assert!(ready.contains(&t1.id));
        assert!(ready.contains(&t3.id));
        assert!(!ready.contains(&t2_id));

        // Complete t1.
        ex.mark_complete(t1.id);
        let ready = ex.next_ready_tasks(&tasks);
        assert_eq!(ready.len(), 2);
        assert!(ready.contains(&t2_id));
        assert!(ready.contains(&t3.id));
    }

    #[test]
    fn plan_execution_progress() {
        let ex = PlanExecution::new(PlanId::new());
        assert_eq!(ex.progress(0), 1.0);
        assert_eq!(ex.progress(10), 0.0);
    }

    #[test]
    fn plan_execution_is_done() {
        let mut ex = PlanExecution::new(PlanId::new());
        assert!(!ex.is_done(2));
        ex.mark_complete(PlanningNodeId::new());
        assert!(!ex.is_done(2));
        ex.mark_complete(PlanningNodeId::new());
        assert!(ex.is_done(2));
    }

    #[test]
    fn plan_execution_is_done_with_failures() {
        let mut ex = PlanExecution::new(PlanId::new());
        ex.mark_complete(PlanningNodeId::new());
        ex.mark_failed(PlanningNodeId::new());
        assert!(ex.is_done(2));
    }

    // ---- Plan ----

    #[test]
    fn plan_creation() {
        let t = make_task("t");
        let mut plan = make_plan_with_tasks(vec![t]);
        assert_eq!(plan.state, PlanState::Created);
        assert_eq!(plan.version, PlanVersion::initial());
    }

    #[test]
    fn plan_transition_ok() {
        let t = make_task("t");
        let mut plan = make_plan_with_tasks(vec![t]);
        plan.transition(PlanState::Validated).unwrap();
        assert_eq!(plan.state, PlanState::Validated);
    }

    #[test]
    fn plan_transition_invalid() {
        let t = make_task("t");
        let mut plan = make_plan_with_tasks(vec![t]);
        let result = plan.transition(PlanState::Executing);
        assert!(result.is_err());
        assert_eq!(plan.state, PlanState::Created);
    }

    #[test]
    fn plan_validate_empty_tasks() {
        let plan = make_plan_with_tasks(vec![]);
        assert!(plan.validate().is_err());
    }

    #[test]
    fn plan_validate_self_dependency() {
        let mut t = make_task("self-dep");
        t.dependencies = vec![t.id];
        let plan = make_plan_with_tasks(vec![t]);
        assert!(plan.validate().is_err());
    }

    #[test]
    fn plan_validate_missing_dependency() {
        let missing = PlanningNodeId::new();
        let t = make_task_with_deps("orphan", vec![missing]);
        let plan = make_plan_with_tasks(vec![t]);
        assert!(plan.validate().is_err());
    }

    #[test]
    fn plan_validate_ok() {
        let t1 = make_task("t1");
        let t2 = make_task_with_deps("t2", vec![t1.id]);
        let plan = make_plan_with_tasks(vec![t1, t2]);
        assert!(plan.validate().is_ok());
    }

    #[test]
    fn plan_snapshot() {
        let t = make_task("t");
        let plan = make_plan_with_tasks(vec![t]);
        let snap = plan.snapshot();
        assert_eq!(snap.plan_id, plan.id);
        assert_eq!(snap.state, PlanState::Created);
    }

    #[test]
    fn plan_checkpoint() {
        let t = make_task("t");
        let plan = make_plan_with_tasks(vec![t]);
        let cp = plan.checkpoint();
        assert_eq!(cp.plan_id, plan.id);
        assert_eq!(cp.version, plan.version);
    }

    #[test]
    fn plan_bump_version() {
        let t = make_task("t");
        let mut plan = make_plan_with_tasks(vec![t]);
        plan.bump_version();
        assert_eq!(plan.version.patch, 1);
    }

    #[test]
    fn plan_compute_statistics() {
        let mut t1 = make_task("t1");
        t1.status = TaskStatus::Completed;
        let mut t2 = make_task("t2");
        t2.status = TaskStatus::Pending;
        t2.cost_estimate = 10.0;

        let mut plan = make_plan_with_tasks(vec![t1, t2]);
        plan.compute_statistics();

        assert_eq!(plan.statistics.total_tasks, 2);
        assert_eq!(plan.statistics.completed_tasks, 1);
        assert_eq!(plan.statistics.pending_tasks, 1);
        assert!((plan.statistics.total_cost - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn plan_with_context() {
        let t = make_task("t");
        let plan = make_plan_with_tasks(vec![t]).with_context(
            PlanContext::new()
                .with_goal_description("g")
                .with_constraint("x", serde_json::json!(1)),
        );
        assert_eq!(plan.context.goal_descriptions.len(), 1);
    }

    #[test]
    fn plan_full_lifecycle() {
        let mut t1 = make_task("t1");
        let mut t2 = make_task("t2");
        t2.dependencies = vec![t1.id];

        let mut plan = make_plan_with_tasks(vec![t1, t2]);
        plan.validate().unwrap();

        let lifecycle = [
            PlanState::Validated,
            PlanState::Generating,
            PlanState::Generated,
            PlanState::Optimizing,
            PlanState::Optimized,
            PlanState::Executing,
            PlanState::Completed,
        ];
        for &target in &lifecycle {
            plan.transition(target).unwrap();
        }
        assert!(plan.state.is_terminal());
    }

    // ---- PlanningSession ----

    #[test]
    fn session_creation() {
        let s = PlanningSession::new(PlanningConfiguration::default());
        assert_eq!(s.state, PlanState::Created);
        assert!(s.plan_id.is_none());
    }

    #[test]
    fn session_builder() {
        let plan_id = PlanId::new();
        let s = PlanningSession::new(PlanningConfiguration::default())
            .with_plan_id(plan_id)
            .with_timeout(60)
            .with_metadata("m", serde_json::json!("v"));
        assert_eq!(s.plan_id, Some(plan_id));
        assert_eq!(s.timeout_secs, 60);
        assert_eq!(s.metadata.get("m").unwrap(), "v");
    }

    #[test]
    fn session_not_expired_immediately() {
        let s = PlanningSession::new(PlanningConfiguration::default());
        assert!(!s.is_expired());
    }

    #[test]
    fn session_time_remaining() {
        let s = PlanningSession::new(PlanningConfiguration::default()).with_timeout(100);
        let remaining = s.time_remaining_secs();
        assert!(remaining > 0 && remaining <= 100);
    }

    #[test]
    fn session_transition() {
        let mut s = PlanningSession::new(PlanningConfiguration::default());
        s.transition(PlanState::Validated).unwrap();
        assert_eq!(s.state, PlanState::Validated);
    }

    #[test]
    fn session_transition_invalid() {
        let mut s = PlanningSession::new(PlanningConfiguration::default());
        let result = s.transition(PlanState::Executing);
        assert!(result.is_err());
        assert_eq!(s.state, PlanState::Created);
    }

    #[test]
    fn session_expired_when_timeout_zero() {
        let mut s = PlanningSession::new(PlanningConfiguration::default());
        s.timeout_secs = 0;
        assert!(s.is_expired());
    }

    #[test]
    fn session_time_remaining_zero_when_expired() {
        let mut s = PlanningSession::new(PlanningConfiguration::default());
        s.timeout_secs = 0;
        assert_eq!(s.time_remaining_secs(), 0);
    }

    // ---- PlanSnapshot ----

    #[test]
    fn snapshot_fields() {
        let t = make_task("t");
        let plan = make_plan_with_tasks(vec![t]);
        let snap = plan.snapshot();
        assert_eq!(snap.plan_id, plan.id);
        assert_eq!(snap.state, PlanState::Created);
        assert!(snap.timestamp <= Utc::now());
    }

    // ---- PlanStore ----

    #[test]
    fn store_insert_and_get() {
        let store = PlanStore::new();
        let t = make_task("t");
        let plan = make_plan_with_tasks(vec![t]);
        let id = plan.id;
        store.insert(plan);
        let retrieved = store.get(id).unwrap();
        assert_eq!(retrieved.id, id);
    }

    #[test]
    fn store_get_missing() {
        let store = PlanStore::new();
        let result = store.get(PlanId::new());
        assert!(result.is_err());
    }

    #[test]
    fn store_remove() {
        let store = PlanStore::new();
        let t = make_task("t");
        let plan = make_plan_with_tasks(vec![t]);
        let id = plan.id;
        store.insert(plan);
        let removed = store.remove(id).unwrap();
        assert_eq!(removed.id, id);
        assert!(store.get(id).is_err());
    }

    #[test]
    fn store_len() {
        let store = PlanStore::new();
        assert!(store.is_empty());
        store.insert(make_plan_with_tasks(vec![make_task("a")]));
        assert_eq!(store.len(), 1);
        store.insert(make_plan_with_tasks(vec![make_task("b")]));
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn store_plan_ids() {
        let store = PlanStore::new();
        let p1 = make_plan_with_tasks(vec![make_task("a")]);
        let p2 = make_plan_with_tasks(vec![make_task("b")]);
        let id1 = p1.id;
        let id2 = p2.id;
        store.insert(p1);
        store.insert(p2);
        let ids = store.plan_ids();
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
    }

    // ---- Serialization roundtrip ----

    #[test]
    fn plan_serialize_roundtrip() {
        let t1 = make_task("t1");
        let t2 = make_task_with_deps("t2", vec![t1.id]);
        let plan = make_plan_with_tasks(vec![t1, t2]);

        let json = serde_json::to_string(&plan).unwrap();
        let deserialized: Plan = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, plan.id);
        assert_eq!(deserialized.state, plan.state);
        assert_eq!(deserialized.definition.tasks.len(), 2);
    }

    #[test]
    fn checkpoint_serialize_roundtrip() {
        let plan_id = PlanId::new();
        let cp = PlanCheckpoint::new(plan_id, PlanVersion::initial(), PlanState::Created);
        let json = serde_json::to_string(&cp).unwrap();
        let deserialized: PlanCheckpoint = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.plan_id, plan_id);
    }

    #[test]
    fn execution_serialize_roundtrip() {
        let mut ex = PlanExecution::new(PlanId::new());
        ex.mark_complete(PlanningNodeId::new());
        ex.mark_failed(PlanningNodeId::new());
        let json = serde_json::to_string(&ex).unwrap();
        let deserialized: PlanExecution = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.completed_tasks.len(), 1);
        assert_eq!(deserialized.failed_tasks.len(), 1);
    }
}
