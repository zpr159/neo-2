use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{ExecutiveError, ExecutiveResult};
use crate::goal::GoalId;

/// Unique identifier for a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub Uuid);

impl TaskId {
    /// Create a new task identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Get the inner UUID as a string.
    pub fn as_str(&self) -> String {
        self.0.to_string()
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Priority level for tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TaskPriority {
    Critical = 4,
    High = 3,
    Normal = 2,
    Low = 1,
    Background = 0,
}

impl Default for TaskPriority {
    fn default() -> Self {
        Self::Normal
    }
}

impl TaskPriority {
    /// Convert to a numeric score.
    pub fn score(self) -> u32 {
        self as u32
    }
}

/// State of a task in its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskState {
    Pending,
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
    Retrying,
    TimedOut,
}

impl TaskState {
    /// Check if the state is terminal.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            TaskState::Completed | TaskState::Failed | TaskState::Cancelled | TaskState::TimedOut
        )
    }

    /// Valid transitions from this state.
    pub fn valid_transitions(self) -> &'static [TaskState] {
        match self {
            Self::Pending => &[Self::Queued, Self::Cancelled],
            Self::Queued => &[Self::Running, Self::Cancelled],
            Self::Running => &[Self::Paused, Self::Completed, Self::Failed, Self::Cancelled, Self::TimedOut],
            Self::Paused => &[Self::Running, Self::Cancelled],
            Self::Completed => &[],
            Self::Failed => &[Self::Retrying],
            Self::Cancelled => &[],
            Self::Retrying => &[Self::Queued, Self::Failed],
            Self::TimedOut => &[Self::Retrying],
        }
    }

    /// Check if a transition to the target state is valid.
    pub fn can_transition_to(self, target: TaskState) -> bool {
        self.valid_transitions().contains(&target)
    }
}

/// Retry policy for a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 1000,
            max_delay_ms: 30_000,
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryPolicy {
    /// Calculate the delay for a given retry attempt.
    pub fn delay_for_attempt(&self, attempt: u32) -> u64 {
        let delay =
            (self.base_delay_ms as f64) * self.backoff_multiplier.powi(attempt as i32);
        (delay as u64).min(self.max_delay_ms)
    }
}

/// A task represents a unit of work to be executed by the executive system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub name: String,
    pub description: String,
    pub priority: TaskPriority,
    pub state: TaskState,
    pub goal_id: Option<GoalId>,
    pub owner: Option<String>,
    pub dependencies: Vec<TaskId>,
    pub dependents: Vec<TaskId>,
    pub retry_policy: RetryPolicy,
    pub retry_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub deadline: Option<DateTime<Utc>>,
    pub timeout_ms: Option<u64>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub context: HashMap<String, serde_json::Value>,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Task {
    /// Create a new task.
    pub fn new(name: String) -> Self {
        let now = Utc::now();
        Self {
            id: TaskId::new(),
            name,
            description: String::new(),
            priority: TaskPriority::Normal,
            state: TaskState::Pending,
            goal_id: None,
            owner: None,
            dependencies: Vec::new(),
            dependents: Vec::new(),
            retry_policy: RetryPolicy::default(),
            retry_count: 0,
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
            deadline: None,
            timeout_ms: None,
            result: None,
            error: None,
            context: HashMap::new(),
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Set the task description.
    pub fn with_description(mut self, desc: String) -> Self {
        self.description = desc;
        self
    }

    /// Set the priority.
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set the goal this task belongs to.
    pub fn with_goal(mut self, goal_id: GoalId) -> Self {
        self.goal_id = Some(goal_id);
        self
    }

    /// Set a deadline.
    pub fn with_deadline(mut self, deadline: DateTime<Utc>) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// Set a timeout.
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    /// Set retry policy.
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = policy;
        self
    }

    /// Add a dependency.
    pub fn with_dependency(mut self, dep_id: TaskId) -> Self {
        if !self.dependencies.contains(&dep_id) {
            self.dependencies.push(dep_id);
        }
        self
    }

    /// Add a tag.
    pub fn with_tag(mut self, tag: String) -> Self {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
        self
    }

    /// Set context.
    pub fn with_context(mut self, key: String, value: serde_json::Value) -> Self {
        self.context.insert(key, value);
        self
    }

    /// Transition to a new state.
    pub fn transition(&mut self, target: TaskState) -> ExecutiveResult<()> {
        if !self.state.can_transition_to(target) {
            return Err(ExecutiveError::internal(format!(
                "cannot transition task '{}' from {:?} to {:?}",
                self.name, self.state, target
            )));
        }
        self.state = target;
        self.updated_at = Utc::now();

        match target {
            TaskState::Running => {
                self.started_at = Some(Utc::now());
            }
            TaskState::Completed => {
                self.completed_at = Some(Utc::now());
            }
            _ => {}
        }

        Ok(())
    }

    /// Check if the task has exceeded its deadline.
    pub fn is_overdue(&self) -> bool {
        self.deadline
            .map(|d| Utc::now() > d)
            .unwrap_or(false)
    }

    /// Check if the task can be retried.
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.retry_policy.max_retries
    }

    /// Record a retry attempt.
    pub fn record_retry(&mut self) {
        self.retry_count += 1;
    }
}

/// Thread-safe task manager responsible for task lifecycle, queue, ownership, cancellation, retry, and deadlines.
#[derive(Clone)]
pub struct TaskManager {
    inner: Arc<TaskManagerInner>,
}

struct TaskManagerInner {
    tasks: RwLock<HashMap<TaskId, Task>>,
    queue: RwLock<Vec<TaskId>>,
    dependency_graph: RwLock<HashMap<TaskId, HashSet<TaskId>>>,
    reverse_dependency_graph: RwLock<HashMap<TaskId, HashSet<TaskId>>>,
    ownership: RwLock<HashMap<String, Vec<TaskId>>>,
}

impl TaskManager {
    /// Create a new task manager.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(TaskManagerInner {
                tasks: RwLock::new(HashMap::new()),
                queue: RwLock::new(Vec::new()),
                dependency_graph: RwLock::new(HashMap::new()),
                reverse_dependency_graph: RwLock::new(HashMap::new()),
                ownership: RwLock::new(HashMap::new()),
            }),
        }
    }

    /// Create and register a new task.
    pub fn create_task(&self, name: String) -> Task {
        let task = Task::new(name);
        let id = task.id;
        self.inner.tasks.write().insert(id, task.clone());
        self.inner
            .dependency_graph
            .write()
            .entry(id)
            .or_default();
        tracing::info!(task_id = %id, task_name = %task.name, "task created");
        task
    }

    /// Submit a task to the queue.
    pub fn submit_task(&self, task: Task) -> ExecutiveResult<TaskId> {
        let id = task.id;
        let mut task = task;
        task.transition(TaskState::Queued)?;

        self.inner.tasks.write().insert(id, task.clone());
        self.inner.queue.write().push(id);

        tracing::info!(task_id = %id, task_name = %task.name, "task submitted");
        Ok(id)
    }

    /// Get a task by ID.
    pub fn get_task(&self, id: TaskId) -> ExecutiveResult<Task> {
        self.inner
            .tasks
            .read()
            .get(&id)
            .cloned()
            .ok_or_else(|| ExecutiveError::task_not_found(&id.as_str()))
    }

    /// Update a task.
    pub fn update_task(&self, task: Task) -> ExecutiveResult<()> {
        if !self.inner.tasks.read().contains_key(&task.id) {
            return Err(ExecutiveError::task_not_found(&task.id.as_str()));
        }
        self.inner.tasks.write().insert(task.id, task);
        Ok(())
    }

    /// Start executing a task.
    pub fn start_task(&self, id: TaskId, owner: String) -> ExecutiveResult<()> {
        let mut task = self.get_task(id)?;

        if let Some(current_owner) = &task.owner {
            if current_owner != &owner {
                return Err(ExecutiveError::new(
                    crate::error::ExecutiveErrorCode::TaskOwnershipConflict,
                    format!(
                        "task '{}' is owned by '{}', cannot be claimed by '{}'",
                        task.name, current_owner, owner
                    ),
                ));
            }
        }

        task.owner = Some(owner.clone());
        task.transition(TaskState::Running)?;

        self.inner.tasks.write().insert(id, task);
        self.inner
            .ownership
            .write()
            .entry(owner.clone())
            .or_default()
            .push(id);

        tracing::info!(task_id = %id, owner = %owner, "task started");
        Ok(())
    }

    /// Complete a task.
    pub fn complete_task(&self, id: TaskId, result: serde_json::Value) -> ExecutiveResult<()> {
        let mut task = self.get_task(id)?;
        task.transition(TaskState::Completed)?;
        task.result = Some(result);
        self.inner.tasks.write().insert(id, task);
        tracing::info!(task_id = %id, "task completed");
        Ok(())
    }

    /// Fail a task.
    pub fn fail_task(&self, id: TaskId, error: String) -> ExecutiveResult<bool> {
        let mut task = self.get_task(id)?;
        task.transition(TaskState::Failed)?;
        task.error = Some(error.clone());
        self.inner.tasks.write().insert(id, task.clone());

        let should_retry = task.can_retry();
        if should_retry {
            let mut task = task;
            task.record_retry();
            task.transition(TaskState::Retrying)?;
            let retry_count = task.retry_count;
            self.inner.tasks.write().insert(id, task);
            tracing::warn!(task_id = %id, error = %error, retry = retry_count, "task failed, retrying");
        } else {
            self.inner.queue.write().retain(|tid| *tid != id);
            tracing::warn!(task_id = %id, error = %error, "task failed permanently");
        }

        Ok(should_retry)
    }

    /// Cancel a task.
    pub fn cancel_task(&self, id: TaskId) -> ExecutiveResult<()> {
        let mut task = self.get_task(id)?;
        task.transition(TaskState::Cancelled)?;
        self.inner.tasks.write().insert(id, task);

        self.inner.queue.write().retain(|tid| *tid != id);
        tracing::info!(task_id = %id, "task cancelled");
        Ok(())
    }

    /// Pause a task.
    pub fn pause_task(&self, id: TaskId) -> ExecutiveResult<()> {
        let mut task = self.get_task(id)?;
        task.transition(TaskState::Paused)?;
        self.inner.tasks.write().insert(id, task);
        Ok(())
    }

    /// Resume a paused task.
    pub fn resume_task(&self, id: TaskId) -> ExecutiveResult<()> {
        let mut task = self.get_task(id)?;
        task.transition(TaskState::Running)?;
        task.started_at = Some(Utc::now());
        self.inner.tasks.write().insert(id, task);
        Ok(())
    }

    /// Add a dependency between tasks.
    pub fn add_dependency(&self, task_id: TaskId, depends_on: TaskId) -> ExecutiveResult<()> {
        if task_id == depends_on {
            return Err(ExecutiveError::internal("task cannot depend on itself"));
        }

        {
            let mut deps = self.inner.dependency_graph.write();
            deps.entry(task_id).or_default().insert(depends_on);
        }
        {
            let mut rev_deps = self.inner.reverse_dependency_graph.write();
            rev_deps.entry(depends_on).or_default().insert(task_id);
        }

        let mut task = self.get_task(task_id)?;
        if !task.dependencies.contains(&depends_on) {
            task.dependencies.push(depends_on);
            task.updated_at = Utc::now();
            self.inner.tasks.write().insert(task_id, task);
        }

        Ok(())
    }

    /// Get tasks ready for execution (all dependencies satisfied, queued).
    pub fn ready_tasks(&self) -> Vec<Task> {
        let tasks = self.inner.tasks.read();
        let deps = self.inner.dependency_graph.read();

        tasks
            .values()
            .filter(|t| {
                t.state == TaskState::Queued
                    && deps
                        .get(&t.id)
                        .map_or(true, |d| {
                            d.iter().all(|dep_id| {
                                tasks.get(dep_id).map_or(false, |dt| {
                                    dt.state == TaskState::Completed
                                })
                            })
                        })
            })
            .cloned()
            .collect()
    }

    /// Get tasks sorted by priority.
    pub fn tasks_by_priority(&self) -> Vec<Task> {
        let mut tasks: Vec<Task> = self
            .inner
            .tasks
            .read()
            .values()
            .filter(|t| !t.state.is_terminal())
            .cloned()
            .collect();
        tasks.sort_by(|a, b| b.priority.cmp(&a.priority));
        tasks
    }

    /// Get overdue tasks.
    pub fn overdue_tasks(&self) -> Vec<Task> {
        self.inner
            .tasks
            .read()
            .values()
            .filter(|t| !t.state.is_terminal() && t.is_overdue())
            .cloned()
            .collect()
    }

    /// Get tasks by state.
    pub fn tasks_by_state(&self, state: TaskState) -> Vec<Task> {
        self.inner
            .tasks
            .read()
            .values()
            .filter(|t| t.state == state)
            .cloned()
            .collect()
    }

    /// Get tasks by owner.
    pub fn tasks_by_owner(&self, owner: &str) -> Vec<Task> {
        self.inner
            .tasks
            .read()
            .values()
            .filter(|t| t.owner.as_deref() == Some(owner))
            .cloned()
            .collect()
    }

    /// Get tasks for a goal.
    pub fn tasks_for_goal(&self, goal_id: GoalId) -> Vec<Task> {
        self.inner
            .tasks
            .read()
            .values()
            .filter(|t| t.goal_id == Some(goal_id))
            .cloned()
            .collect()
    }

    /// Get the task count.
    pub fn task_count(&self) -> usize {
        self.inner.tasks.read().len()
    }

    /// Get the queue depth.
    pub fn queue_depth(&self) -> usize {
        self.inner.queue.read().len()
    }

    /// Get all tasks.
    pub fn all_tasks(&self) -> Vec<Task> {
        self.inner.tasks.read().values().cloned().collect()
    }

    /// Check for deadline-exceeded tasks and mark them as timed out.
    pub fn check_deadlines(&self) -> Vec<TaskId> {
        let mut timed_out = Vec::new();
        let mut tasks = self.inner.tasks.write();

        for task in tasks.values_mut() {
            if task.state == TaskState::Running && task.is_overdue() {
                let _ = task.transition(TaskState::TimedOut);
                timed_out.push(task.id);
            }
        }

        timed_out
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_creation() {
        let mgr = TaskManager::new();
        let task = mgr.create_task("test".to_string());
        assert_eq!(task.state, TaskState::Pending);
        assert_eq!(mgr.task_count(), 1);
    }

    #[test]
    fn task_submit_and_start() {
        let mgr = TaskManager::new();
        let task = mgr.create_task("submit".to_string());
        let id = task.id;

        mgr.submit_task(task).unwrap();
        assert_eq!(mgr.get_task(id).unwrap().state, TaskState::Queued);

        mgr.start_task(id, "worker-1".to_string()).unwrap();
        assert_eq!(mgr.get_task(id).unwrap().state, TaskState::Running);
    }

    #[test]
    fn task_complete() {
        let mgr = TaskManager::new();
        let task = mgr.create_task("done".to_string());
        let id = task.id;

        mgr.submit_task(task).unwrap();
        mgr.start_task(id, "w".to_string()).unwrap();
        mgr.complete_task(id, serde_json::json!({"ok": true}))
            .unwrap();

        let task = mgr.get_task(id).unwrap();
        assert!(task.state.is_terminal());
        assert!(task.result.is_some());
    }

    #[test]
    fn task_cancel() {
        let mgr = TaskManager::new();
        let task = mgr.create_task("cancel".to_string());
        let id = task.id;

        mgr.submit_task(task).unwrap();
        mgr.cancel_task(id).unwrap();
        assert!(mgr.get_task(id).unwrap().state.is_terminal());
    }

    #[test]
    fn task_retry_on_failure() {
        let mgr = TaskManager::new();
        let task = mgr.create_task("retry".to_string());
        let id = task.id;

        mgr.submit_task(task).unwrap();
        mgr.start_task(id, "w".to_string()).unwrap();
        let retried = mgr.fail_task(id, "oops".to_string()).unwrap();
        assert!(retried);
        assert_eq!(mgr.get_task(id).unwrap().retry_count, 1);
    }

    #[test]
    fn task_ownership_conflict() {
        let mgr = TaskManager::new();
        let task = mgr.create_task("own".to_string());
        let id = task.id;

        mgr.submit_task(task).unwrap();
        mgr.start_task(id, "worker-1".to_string()).unwrap();
        let result = mgr.start_task(id, "worker-2".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn task_dependencies() {
        let mgr = TaskManager::new();
        let t1 = mgr.create_task("dep1".to_string());
        let t2 = mgr.create_task("dep2".to_string());

        mgr.add_dependency(t2.id, t1.id).unwrap();
        let task2 = mgr.get_task(t2.id).unwrap();
        assert!(task2.dependencies.contains(&t1.id));
    }

    #[test]
    fn task_priority_ordering() {
        let mgr = TaskManager::new();
        let t1 = mgr.create_task("low".to_string()).with_priority(TaskPriority::Low);
        let t2 = mgr.create_task("high".to_string()).with_priority(TaskPriority::High);
        let t3 = mgr.create_task("crit".to_string()).with_priority(TaskPriority::Critical);

        mgr.submit_task(t1).unwrap();
        mgr.submit_task(t2).unwrap();
        mgr.submit_task(t3).unwrap();

        let sorted = mgr.tasks_by_priority();
        assert_eq!(sorted[0].priority, TaskPriority::Critical);
        assert_eq!(sorted[1].priority, TaskPriority::High);
        assert_eq!(sorted[2].priority, TaskPriority::Low);
    }

    #[test]
    fn retry_policy_delay() {
        let policy = RetryPolicy {
            max_retries: 5,
            base_delay_ms: 100,
            max_delay_ms: 10_000,
            backoff_multiplier: 2.0,
        };

        assert_eq!(policy.delay_for_attempt(0), 100);
        assert_eq!(policy.delay_for_attempt(1), 200);
        assert_eq!(policy.delay_for_attempt(2), 400);
    }

    #[test]
    fn task_overdue() {
        let mut task = Task::new("overdue".to_string());
        task.deadline = Some(Utc::now() - chrono::Duration::hours(1));
        assert!(task.is_overdue());
    }

    #[test]
    fn task_not_found() {
        let mgr = TaskManager::new();
        let result = mgr.get_task(TaskId::new());
        assert!(result.is_err());
    }
}
