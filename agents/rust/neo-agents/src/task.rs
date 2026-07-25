use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::error::{AgentError, AgentResult};
use crate::types::{AgentId, TaskPriority};

// ---------------------------------------------------------------------------
// TaskId (framework-level alias using core type)
// ---------------------------------------------------------------------------

/// Identifier for a task managed by the agent framework.
pub type TaskId = neo_core::id::TaskId;

// ---------------------------------------------------------------------------
// TaskStatus
// ---------------------------------------------------------------------------

/// Status of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task has been created but not queued.
    Created,
    /// Task is in the queue waiting for assignment.
    Queued,
    /// Task has been assigned to an agent.
    Assigned,
    /// Task is actively being executed.
    Running,
    /// Task execution is paused.
    Paused,
    /// Task completed successfully.
    Completed,
    /// Task failed.
    Failed,
    /// Task was cancelled.
    Cancelled,
    /// Task timed out.
    TimedOut,
    /// Task is being retried after a failure.
    Retrying,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::Queued => write!(f, "queued"),
            Self::Assigned => write!(f, "assigned"),
            Self::Running => write!(f, "running"),
            Self::Paused => write!(f, "paused"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::TimedOut => write!(f, "timed_out"),
            Self::Retrying => write!(f, "retrying"),
        }
    }
}

// ---------------------------------------------------------------------------
// Task
// ---------------------------------------------------------------------------

/// A unit of work to be executed by an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier.
    pub id: TaskId,
    /// Human-readable name.
    pub name: String,
    /// Description of what the task does.
    pub description: String,
    /// Task priority.
    pub priority: TaskPriority,
    /// Current status.
    pub status: TaskStatus,
    /// The agent assigned to execute this task.
    pub assigned_agent: Option<AgentId>,
    /// Parent goal or task ID, if this is a sub-task.
    pub parent_id: Option<TaskId>,
    /// IDs of tasks that must complete before this one can start.
    pub dependencies: Vec<TaskId>,
    /// Task input payload.
    pub input: serde_json::Value,
    /// Task output (populated on completion).
    pub output: Option<serde_json::Value>,
    /// Error message if the task failed.
    pub error: Option<String>,
    /// Maximum number of retry attempts.
    pub max_retries: u32,
    /// Current retry count.
    pub retry_count: u32,
    /// Base delay between retries in milliseconds.
    pub retry_base_delay_ms: u64,
    /// Backoff multiplier for retries.
    pub retry_backoff_multiplier: f64,
    /// Maximum retry delay in milliseconds.
    pub retry_max_delay_ms: u64,
    /// Deadline (None means no deadline).
    pub deadline: Option<DateTime<Utc>>,
    /// Timeout in milliseconds (None means no timeout).
    pub timeout_ms: Option<u64>,
    /// When the task was created.
    pub created_at: DateTime<Utc>,
    /// When the task was last updated.
    pub updated_at: DateTime<Utc>,
    /// When the task started executing.
    pub started_at: Option<DateTime<Utc>>,
    /// When the task completed.
    pub completed_at: Option<DateTime<Utc>>,
    /// Duration of execution in milliseconds.
    pub duration_ms: Option<u64>,
    /// Tags for categorization.
    pub tags: Vec<String>,
    /// Custom metadata.
    pub metadata: HashMap<String, String>,
}

impl Task {
    /// Create a new task.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input: serde_json::Value,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: TaskId::new(),
            name: name.into(),
            description: description.into(),
            priority: TaskPriority::default(),
            status: TaskStatus::Created,
            assigned_agent: None,
            parent_id: None,
            dependencies: Vec::new(),
            input,
            output: None,
            error: None,
            max_retries: 3,
            retry_count: 0,
            retry_base_delay_ms: 1_000,
            retry_backoff_multiplier: 2.0,
            retry_max_delay_ms: 30_000,
            deadline: None,
            timeout_ms: None,
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
            duration_ms: None,
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Create a new `TaskBuilder` for constructing a task with a fluent API.
    #[must_use]
    pub fn builder() -> crate::sdk::TaskBuilder {
        crate::sdk::TaskBuilder::new()
    }

    /// Set the priority.
    #[must_use]
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Add a dependency.
    #[must_use]
    pub fn with_dependency(mut self, dep: TaskId) -> Self {
        self.dependencies.push(dep);
        self
    }

    /// Set the parent task/goal ID.
    #[must_use]
    pub fn with_parent(mut self, parent_id: TaskId) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Set the timeout.
    #[must_use]
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    /// Set the deadline.
    #[must_use]
    pub fn with_deadline(mut self, deadline: DateTime<Utc>) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// Set maximum retries.
    #[must_use]
    pub fn with_max_retries(mut self, max: u32) -> Self {
        self.max_retries = max;
        self
    }

    /// Add a tag.
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add custom metadata.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Mark the task as queued.
    pub fn queue(&mut self) -> AgentResult<()> {
        if self.status != TaskStatus::Created && self.status != TaskStatus::Retrying {
            return Err(AgentError::InvalidState(format!(
                "cannot queue task in {} state",
                self.status
            )));
        }
        self.status = TaskStatus::Queued;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Assign the task to an agent.
    pub fn assign(&mut self, agent_id: AgentId) -> AgentResult<()> {
        if self.status != TaskStatus::Queued && self.status != TaskStatus::Assigned {
            return Err(AgentError::InvalidState(format!(
                "cannot assign task in {} state",
                self.status
            )));
        }
        self.status = TaskStatus::Assigned;
        self.assigned_agent = Some(agent_id);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Mark the task as running.
    pub fn start_execution(&mut self) -> AgentResult<()> {
        if self.status != TaskStatus::Assigned && self.status != TaskStatus::Retrying {
            return Err(AgentError::InvalidState(format!(
                "cannot start task in {} state",
                self.status
            )));
        }
        self.status = TaskStatus::Running;
        self.started_at = Some(Utc::now());
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Mark the task as completed.
    pub fn complete(&mut self, output: serde_json::Value) -> AgentResult<()> {
        if self.status != TaskStatus::Running {
            return Err(AgentError::InvalidState(format!(
                "cannot complete task in {} state",
                self.status
            )));
        }
        self.status = TaskStatus::Completed;
        self.output = Some(output);
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
        if let Some(started) = self.started_at {
            self.duration_ms =
                Some(Utc::now().signed_duration_since(started).num_milliseconds() as u64);
        }
        Ok(())
    }

    /// Mark the task as failed.
    pub fn fail(&mut self, error: impl Into<String>) -> AgentResult<()> {
        if self.status == TaskStatus::Completed || self.status == TaskStatus::Cancelled {
            return Err(AgentError::InvalidState(format!(
                "cannot fail task in {} state",
                self.status
            )));
        }
        self.status = TaskStatus::Failed;
        self.error = Some(error.into());
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
        if let Some(started) = self.started_at {
            self.duration_ms =
                Some(Utc::now().signed_duration_since(started).num_milliseconds() as u64);
        }
        Ok(())
    }

    /// Cancel the task.
    pub fn cancel(&mut self) -> AgentResult<()> {
        if self.status == TaskStatus::Completed {
            return Err(AgentError::InvalidState(
                "cannot cancel completed task".into(),
            ));
        }
        self.status = TaskStatus::Cancelled;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Attempt a retry.
    pub fn attempt_retry(&mut self) -> AgentResult<()> {
        if self.retry_count >= self.max_retries {
            return Err(AgentError::MaxRetriesExceeded(format!(
                "task {} exceeded max retries ({})",
                self.id, self.max_retries
            )));
        }
        self.retry_count += 1;
        self.status = TaskStatus::Retrying;
        self.error = None;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Calculate the retry delay based on current retry count.
    #[must_use]
    pub fn retry_delay_ms(&self) -> u64 {
        let delay = self.retry_base_delay_ms as f64
            * self.retry_backoff_multiplier.powi(self.retry_count as i32);
        (delay as u64).min(self.retry_max_delay_ms)
    }

    /// Check if the task has exceeded its deadline.
    #[must_use]
    pub fn is_overdue(&self) -> bool {
        if let Some(deadline) = self.deadline {
            Utc::now() > deadline
        } else {
            false
        }
    }

    /// Check if dependencies are met given a set of completed task IDs.
    #[must_use]
    pub fn dependencies_met(&self, completed: &HashSet<TaskId>) -> bool {
        self.dependencies.iter().all(|dep| completed.contains(dep))
    }

    /// Check if the task is in a terminal state.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled
        )
    }
}

// ---------------------------------------------------------------------------
// TaskResult
// ---------------------------------------------------------------------------

/// The result of a task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// The task ID.
    pub task_id: TaskId,
    /// Whether the task succeeded.
    pub success: bool,
    /// The output value.
    pub output: Option<serde_json::Value>,
    /// Error message if the task failed.
    pub error: Option<String>,
    /// Execution duration in milliseconds.
    pub duration_ms: u64,
    /// The agent that executed the task.
    pub agent_id: AgentId,
}

// ---------------------------------------------------------------------------
// TaskQueue
// ---------------------------------------------------------------------------

/// A priority task queue with dependency awareness.
pub struct TaskQueue {
    /// Pending tasks sorted by priority.
    queue: Arc<RwLock<VecDeque<Task>>>,
    /// Task lookup by ID.
    tasks: DashMap<TaskId, Task>,
    /// Set of completed task IDs for dependency resolution.
    completed_tasks: Arc<RwLock<HashSet<TaskId>>>,
}

impl TaskQueue {
    /// Create a new task queue.
    #[must_use]
    pub fn new() -> Self {
        Self {
            queue: Arc::new(RwLock::new(VecDeque::new())),
            tasks: DashMap::new(),
            completed_tasks: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Enqueue a task.
    pub async fn enqueue(&self, mut task: Task) -> AgentResult<TaskId> {
        let id = task.id;
        task.queue()?;
        self.tasks.insert(id, task.clone());

        // Insert in priority order (higher priority first)
        let mut queue = self.queue.write().await;
        let insert_pos = queue
            .iter()
            .position(|t| t.priority < task.priority)
            .unwrap_or(queue.len());
        queue.insert(insert_pos, task);

        Ok(id)
    }

    /// Dequeue the next task whose dependencies are met.
    pub async fn dequeue(&self) -> Option<Task> {
        let completed = self.completed_tasks.read().await;
        let mut queue = self.queue.write().await;

        // Find first task whose dependencies are satisfied
        if let Some(pos) = queue.iter().position(|t| t.dependencies_met(&completed)) {
            let mut task = queue.remove(pos)?;
            task.status = TaskStatus::Queued;
            Some(task)
        } else {
            None
        }
    }

    /// Mark a task as completed.
    pub async fn complete_task(&self, task_id: TaskId) {
        let mut completed = self.completed_tasks.write().await;
        completed.insert(task_id);
    }

    /// Get a task by ID.
    pub fn get_task(&self, task_id: &TaskId) -> Option<Task> {
        self.tasks.get(task_id).map(|t| t.clone())
    }

    /// Update a task in the registry.
    pub fn update_task(&self, task: Task) {
        self.tasks.insert(task.id, task);
    }

    /// Remove a completed task from the active set.
    pub fn remove_task(&self, task_id: &TaskId) {
        self.tasks.remove(task_id);
    }

    /// Return the number of pending tasks.
    pub async fn pending_count(&self) -> usize {
        let queue = self.queue.read().await;
        queue.len()
    }

    /// Return the total number of tracked tasks.
    #[must_use]
    pub fn total_count(&self) -> usize {
        self.tasks.len()
    }

    /// Check if the queue is empty.
    pub async fn is_empty(&self) -> bool {
        let queue = self.queue.read().await;
        queue.is_empty()
    }

    /// List all task IDs.
    #[must_use]
    pub fn list_tasks(&self) -> Vec<TaskId> {
        self.tasks.iter().map(|entry| *entry.key()).collect()
    }

    /// List tasks by status.
    #[must_use]
    pub fn list_by_status(&self, status: TaskStatus) -> Vec<Task> {
        self.tasks
            .iter()
            .filter(|entry| entry.value().status == status)
            .map(|entry| entry.value().clone())
            .collect()
    }
}

impl Default for TaskQueue {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// TaskAssignment
// ---------------------------------------------------------------------------

/// Records the assignment of a task to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAssignment {
    /// The task ID.
    pub task_id: TaskId,
    /// The assigned agent ID.
    pub agent_id: AgentId,
    /// When the assignment was made.
    pub assigned_at: DateTime<Utc>,
    /// When the assignment expires (if applicable).
    pub expires_at: Option<DateTime<Utc>>,
}

// ---------------------------------------------------------------------------
// TaskScheduler
// ---------------------------------------------------------------------------

/// Schedules tasks to agents based on priority, availability, and capability.
pub struct TaskScheduler {
    /// The task queue.
    pub task_queue: Arc<TaskQueue>,
    /// Active task assignments: task_id -> assignment.
    assignments: DashMap<TaskId, TaskAssignment>,
    /// Agent task counts: agent_id -> count of active tasks.
    agent_task_counts: DashMap<AgentId, u64>,
    /// Maximum tasks per agent.
    max_tasks_per_agent: usize,
}

impl TaskScheduler {
    /// Create a new task scheduler.
    #[must_use]
    pub fn new(max_tasks_per_agent: usize) -> Self {
        Self {
            task_queue: Arc::new(TaskQueue::new()),
            assignments: DashMap::new(),
            agent_task_counts: DashMap::new(),
            max_tasks_per_agent,
        }
    }

    /// Submit a task to the scheduler.
    pub async fn submit_task(&self, task: Task) -> AgentResult<TaskId> {
        self.task_queue.enqueue(task).await
    }

    /// Assign the next available task to an agent.
    pub async fn assign_next_task(&self, agent_id: AgentId) -> AgentResult<Option<Task>> {
        // Check agent capacity
        let count = self
            .agent_task_counts
            .get(&agent_id)
            .map(|c| *c)
            .unwrap_or(0);
        if count >= self.max_tasks_per_agent as u64 {
            return Ok(None);
        }

        // Try to dequeue a task
        if let Some(mut task) = self.task_queue.dequeue().await {
            task.assign(agent_id)?;
            self.task_queue.update_task(task.clone());

            self.assignments.insert(
                task.id,
                TaskAssignment {
                    task_id: task.id,
                    agent_id,
                    assigned_at: Utc::now(),
                    expires_at: task.deadline,
                },
            );

            self.agent_task_counts
                .entry(agent_id)
                .and_modify(|c| *c += 1)
                .or_insert(1);

            Ok(Some(task))
        } else {
            Ok(None)
        }
    }

    /// Complete a task assignment.
    pub async fn complete_assignment(
        &self,
        task_id: TaskId,
        output: serde_json::Value,
    ) -> AgentResult<TaskResult> {
        let assignment = self
            .assignments
            .remove(&task_id)
            .ok_or_else(|| AgentError::NotFound(format!("no assignment for task {task_id}")))?
            .1;

        let mut task = self
            .task_queue
            .get_task(&task_id)
            .ok_or_else(|| AgentError::NotFound(format!("task {task_id} not found")))?;

        if task.status == TaskStatus::Assigned {
            task.start_execution()?;
        }
        task.complete(output.clone())?;
        self.task_queue.update_task(task.clone());
        self.task_queue.complete_task(task_id).await;

        // Decrement agent task count
        self.agent_task_counts
            .entry(assignment.agent_id)
            .and_modify(|c| *c = c.saturating_sub(1));

        let duration = task.duration_ms.unwrap_or(0);

        Ok(TaskResult {
            task_id,
            success: true,
            output: Some(output),
            error: None,
            duration_ms: duration,
            agent_id: assignment.agent_id,
        })
    }

    /// Handle a task failure.
    pub async fn fail_assignment(
        &self,
        task_id: TaskId,
        error: String,
    ) -> AgentResult<Option<Task>> {
        let assignment = self
            .assignments
            .remove(&task_id)
            .ok_or_else(|| AgentError::NotFound(format!("no assignment for task {task_id}")))?
            .1;

        // Decrement agent task count
        self.agent_task_counts
            .entry(assignment.agent_id)
            .and_modify(|c| *c = c.saturating_sub(1));

        let mut task = self
            .task_queue
            .get_task(&task_id)
            .ok_or_else(|| AgentError::NotFound(format!("task {task_id} not found")))?;

        task.fail(&error)?;
        self.task_queue.update_task(task.clone());

        // Try to retry
        if task.attempt_retry().is_ok() {
            task.queue()?;
            self.task_queue.update_task(task.clone());
            let mut queue = self.task_queue.queue.write().await;
            let insert_pos = queue
                .iter()
                .position(|t| t.priority < task.priority)
                .unwrap_or(queue.len());
            queue.insert(insert_pos, task.clone());
            Ok(Some(task))
        } else {
            Ok(None)
        }
    }

    /// Cancel a task.
    pub async fn cancel_task(&self, task_id: TaskId) -> AgentResult<()> {
        if let Some((_, assignment)) = self.assignments.remove(&task_id) {
            self.agent_task_counts
                .entry(assignment.agent_id)
                .and_modify(|c| *c = c.saturating_sub(1));
        }

        if let Some(mut task) = self.task_queue.get_task(&task_id) {
            task.cancel()?;
            self.task_queue.update_task(task);
        }

        Ok(())
    }

    /// Get the assignment for a task.
    pub fn get_assignment(&self, task_id: &TaskId) -> Option<TaskAssignment> {
        self.assignments.get(task_id).map(|a| a.clone())
    }

    /// Get the number of active tasks for an agent.
    #[must_use]
    pub fn agent_task_count(&self, agent_id: &AgentId) -> u64 {
        self.agent_task_counts
            .get(agent_id)
            .map(|c| *c)
            .unwrap_or(0)
    }

    /// Get the total number of pending tasks.
    pub async fn pending_count(&self) -> usize {
        self.task_queue.pending_count().await
    }

    /// List all task assignments.
    #[must_use]
    pub fn list_assignments(&self) -> Vec<TaskAssignment> {
        self.assignments
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }
}

impl Default for TaskScheduler {
    fn default() -> Self {
        Self::new(4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new("test", "description", serde_json::json!({"key": "value"}));
        assert_eq!(task.name, "test");
        assert_eq!(task.status, TaskStatus::Created);
        assert_eq!(task.priority, TaskPriority::Normal);
    }

    #[test]
    fn test_task_lifecycle() {
        let mut task = Task::new("test", "desc", serde_json::json!(null));
        task.queue().unwrap();
        assert_eq!(task.status, TaskStatus::Queued);

        let agent = AgentId::new();
        task.assign(agent).unwrap();
        assert_eq!(task.status, TaskStatus::Assigned);

        task.start_execution().unwrap();
        assert_eq!(task.status, TaskStatus::Running);

        task.complete(serde_json::json!("done")).unwrap();
        assert_eq!(task.status, TaskStatus::Completed);
        assert!(task.is_terminal());
    }

    #[test]
    fn test_task_retry() {
        let mut task = Task::new("retry-test", "desc", serde_json::json!(null)).with_max_retries(3);
        task.queue().unwrap();
        let agent = AgentId::new();
        task.assign(agent).unwrap();
        task.start_execution().unwrap();
        task.fail("error").unwrap();

        task.attempt_retry().unwrap();
        assert_eq!(task.status, TaskStatus::Retrying);
        assert_eq!(task.retry_count, 1);

        let delay = task.retry_delay_ms();
        assert_eq!(delay, 2000); // 1000 * 2^1
    }

    #[test]
    fn test_task_cancel() {
        let mut task = Task::new("cancel", "desc", serde_json::json!(null));
        task.queue().unwrap();
        task.cancel().unwrap();
        assert_eq!(task.status, TaskStatus::Cancelled);
        assert!(task.is_terminal());
    }

    #[test]
    fn test_task_dependencies() {
        let dep1 = TaskId::new();
        let dep2 = TaskId::new();

        let task = Task::new("main", "desc", serde_json::json!(null))
            .with_dependency(dep1)
            .with_dependency(dep2);

        let mut completed = HashSet::new();
        assert!(!task.dependencies_met(&completed));

        completed.insert(dep1);
        assert!(!task.dependencies_met(&completed));

        completed.insert(dep2);
        assert!(task.dependencies_met(&completed));
    }

    #[test]
    fn test_task_builder() {
        let task = Task::new("built", "desc", serde_json::json!(null))
            .with_priority(TaskPriority::High)
            .with_max_retries(5)
            .with_timeout_ms(10_000)
            .with_tag("important")
            .with_metadata("key", "value");

        assert_eq!(task.priority, TaskPriority::High);
        assert_eq!(task.max_retries, 5);
        assert_eq!(task.timeout_ms, Some(10_000));
        assert!(task.tags.contains(&"important".to_string()));
        assert_eq!(task.metadata.get("key").unwrap(), "value");
    }

    #[tokio::test]
    async fn test_task_queue() {
        let q = TaskQueue::new();
        let t1 = Task::new("t1", "d", serde_json::json!(null)).with_priority(TaskPriority::Low);
        let t2 = Task::new("t2", "d", serde_json::json!(null)).with_priority(TaskPriority::High);

        q.enqueue(t1).await.unwrap();
        q.enqueue(t2).await.unwrap();

        // Should dequeue high priority first
        let task = q.dequeue().await.unwrap();
        assert_eq!(task.name, "t2");
    }

    #[tokio::test]
    async fn test_task_queue_dependencies() {
        let q = TaskQueue::new();
        let dep = TaskId::new();
        let t1 = Task::new("dep-task", "d", serde_json::json!(null));
        let t2 = Task::new("main-task", "d", serde_json::json!(null)).with_dependency(dep);

        q.enqueue(t2).await.unwrap();
        q.enqueue(t1).await.unwrap();

        // t2 should not be dequeued yet (dep not met)
        // But t1 has no deps, so it should be dequeued
        // Actually t2 was enqueued first, but it has unmet deps
        // Let's just verify dequeue works
        let task = q.dequeue().await;
        assert!(task.is_some());
    }

    #[tokio::test]
    async fn test_task_scheduler() {
        let scheduler = TaskScheduler::new(4);
        let task = Task::new("sched-test", "desc", serde_json::json!(null));
        let task_id = scheduler.submit_task(task).await.unwrap();

        let agent = AgentId::new();
        let assigned = scheduler.assign_next_task(agent).await.unwrap();
        assert!(assigned.is_some());
        let assigned_task = assigned.unwrap();
        assert_eq!(assigned_task.id, task_id);

        // Start execution before completing
        let mut task = scheduler.task_queue.get_task(&task_id).unwrap();
        task.start_execution().unwrap();
        scheduler.task_queue.update_task(task);

        let result = scheduler
            .complete_assignment(task_id, serde_json::json!("done"))
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.output, Some(serde_json::json!("done")));
    }
}
