use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::cmp::Ordering as CmpOrdering;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::goal::{GoalId, GoalManager};
use crate::task::{Task, TaskId, TaskManager, TaskPriority, TaskState};
use crate::priority::{PriorityEngine, PriorityScore};
use crate::resource_coordination::{ResourceCoordinator, ResourceType};
use crate::error::{ExecutiveError, ExecutiveResult};

/// Unique identifier for a scheduled execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScheduleId(pub Uuid);

impl ScheduleId {
    /// Create a new schedule identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ScheduleId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ScheduleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A scheduled execution unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledExecution {
    pub id: ScheduleId,
    pub task_id: TaskId,
    pub priority_score: PriorityScore,
    pub scheduled_at: DateTime<Utc>,
    pub estimated_duration_ms: Option<u64>,
    pub resource_requirements: HashMap<ResourceType, u64>,
    pub preemptible: bool,
}

impl PartialEq for ScheduledExecution {
    fn eq(&self, other: &Self) -> bool {
        self.priority_score.total == other.priority_score.total
    }
}

impl Eq for ScheduledExecution {}

impl PartialOrd for ScheduledExecution {
    fn partial_cmp(&self, other: &Self) -> Option<CmpOrdering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScheduledExecution {
    fn cmp(&self, other: &Self) -> CmpOrdering {
        self.priority_score
            .total
            .partial_cmp(&other.priority_score.total)
            .unwrap_or(CmpOrdering::Equal)
    }
}

/// Scheduler statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchedulerStats {
    pub total_scheduled: u64,
    pub total_completed: u64,
    pub total_preempted: u64,
    pub total_failed: u64,
    pub avg_latency_ms: f64,
    pub max_latency_ms: f64,
    pub current_queue_depth: usize,
    pub active_executions: usize,
}

/// Scheduling policy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingPolicy {
    pub max_parallel: usize,
    pub enable_preemption: bool,
    pub enable_resource_awareness: bool,
    pub deadline_weight: f64,
    pub starvation_threshold_secs: u64,
}

impl Default for SchedulingPolicy {
    fn default() -> Self {
        Self {
            max_parallel: 8,
            enable_preemption: true,
            enable_resource_awareness: true,
            deadline_weight: 0.7,
            starvation_threshold_secs: 300,
        }
    }
}

/// The executive scheduler manages task scheduling, parallel execution, dependency scheduling, resource-aware scheduling, and preemption.
#[derive(Clone)]
pub struct ExecutiveScheduler {
    inner: Arc<SchedulerInner>,
}

struct SchedulerInner {
    queue: RwLock<BinaryHeap<ScheduledExecution>>,
    active: RwLock<HashMap<ScheduleId, ScheduledExecution>>,
    completed: RwLock<Vec<ScheduledExecution>>,
    dependency_graph: RwLock<HashMap<TaskId, HashSet<TaskId>>>,
    reverse_dependency_graph: RwLock<HashMap<TaskId, HashSet<TaskId>>>,
    policy: RwLock<SchedulingPolicy>,
    stats: RwLock<SchedulerStats>,
    preemption_log: RwLock<Vec<PreemptionEvent>>,
    start_time: std::time::Instant,
}

/// A preemption event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreemptionEvent {
    pub timestamp: DateTime<Utc>,
    pub preempted_task: TaskId,
    pub preempted_by: TaskId,
    pub reason: String,
}

impl ExecutiveScheduler {
    /// Create a new executive scheduler.
    pub fn new(policy: SchedulingPolicy) -> Self {
        Self {
            inner: Arc::new(SchedulerInner {
                queue: RwLock::new(BinaryHeap::new()),
                active: RwLock::new(HashMap::new()),
                completed: RwLock::new(Vec::new()),
                dependency_graph: RwLock::new(HashMap::new()),
                reverse_dependency_graph: RwLock::new(HashMap::new()),
                policy: RwLock::new(policy),
                stats: RwLock::new(SchedulerStats::default()),
                preemption_log: RwLock::new(Vec::new()),
                start_time: std::time::Instant::now(),
            }),
        }
    }

    /// Schedule a task for execution.
    pub fn schedule_task(
        &self,
        task: &Task,
        priority_engine: &PriorityEngine,
    ) -> ExecutiveResult<ScheduleId> {
        let policy = self.inner.policy.read();

        let active_count = self.inner.active.read().len();
        if active_count >= policy.max_parallel && self.inner.queue.read().len() >= policy.max_parallel * 2 {
            return Err(ExecutiveError::new(
                crate::error::ExecutiveErrorCode::SchedulerFull,
                "scheduler queue is full",
            ));
        }

        let score = priority_engine.score_task(
            task.deadline,
            task.priority,
            &[],
            task.created_at,
        );

        let execution = ScheduledExecution {
            id: ScheduleId::new(),
            task_id: task.id,
            priority_score: score,
            scheduled_at: Utc::now(),
            estimated_duration_ms: task.timeout_ms,
            resource_requirements: HashMap::new(),
            preemptible: policy.enable_preemption && task.priority != TaskPriority::Critical,
        };

        self.inner.queue.write().push(execution.clone());
        self.inner.stats.write().total_scheduled += 1;

        tracing::info!(
            schedule_id = %execution.id,
            task_id = %task.id,
            priority = score.total,
            "task scheduled"
        );

        Ok(execution.id)
    }

    /// Dequeue the next task for execution.
    pub fn dequeue_next(&self) -> Option<ScheduledExecution> {
        let execution = self.inner.queue.write().pop()?;
        let id = execution.id;
        self.inner.active.write().insert(id, execution.clone());
        Some(execution)
    }

    /// Complete a scheduled execution.
    pub fn complete_execution(&self, id: ScheduleId) -> ExecutiveResult<()> {
        let execution = self
            .inner
            .active
            .write()
            .remove(&id)
            .ok_or_else(|| ExecutiveError::internal("execution not found"))?;

        let latency = Utc::now()
            .signed_duration_since(execution.scheduled_at)
            .num_milliseconds() as f64;

        let mut stats = self.inner.stats.write();
        stats.total_completed += 1;
        let n = stats.total_completed as f64;
        stats.avg_latency_ms = (stats.avg_latency_ms * (n - 1.0) + latency) / n;
        stats.max_latency_ms = stats.max_latency_ms.max(latency);

        self.inner.completed.write().push(execution);
        Ok(())
    }

    /// Preempt a running execution.
    pub fn preempt_execution(
        &self,
        id: ScheduleId,
        reason: String,
        preempted_by: TaskId,
    ) -> ExecutiveResult<ScheduledExecution> {
        let policy = self.inner.policy.read();
        if !policy.enable_preemption {
            return Err(ExecutiveError::new(
                crate::error::ExecutiveErrorCode::PreemptionDenied,
                "preemption is disabled",
            ));
        }

        let execution = self
            .inner
            .active
            .write()
            .remove(&id)
            .ok_or_else(|| ExecutiveError::internal("execution not found"))?;

        if !execution.preemptible {
            self.inner.active.write().insert(id, execution.clone());
            return Err(ExecutiveError::new(
                crate::error::ExecutiveErrorCode::PreemptionDenied,
                "execution is not preemptible",
            ));
        }

        self.inner.preemption_log.write().push(PreemptionEvent {
            timestamp: Utc::now(),
            preempted_task: execution.task_id,
            preempted_by,
            reason: reason.clone(),
        });

        self.inner.stats.write().total_preempted += 1;

        let mut requeued = execution.clone();
        requeued.scheduled_at = Utc::now();
        self.inner.queue.write().push(requeued);

        tracing::warn!(
            preempted = %execution.task_id,
            by = %preempted_by,
            reason = %reason,
            "execution preempted"
        );

        Ok(execution)
    }

    /// Add a dependency between tasks in the scheduler.
    pub fn add_dependency(&self, task_id: TaskId, depends_on: TaskId) {
        self.inner
            .dependency_graph
            .write()
            .entry(task_id)
            .or_default()
            .insert(depends_on);
        self.inner
            .reverse_dependency_graph
            .write()
            .entry(depends_on)
            .or_default()
            .insert(task_id);
    }

    /// Get tasks whose dependencies are all satisfied.
    pub fn executable_tasks(&self, task_manager: &TaskManager) -> Vec<Task> {
        let deps = self.inner.dependency_graph.read();
        let tasks = task_manager.all_tasks();

        tasks
            .into_iter()
            .filter(|t| {
                t.state == TaskState::Queued
                    && deps.get(&t.id).map_or(true, |d| {
                        d.iter().all(|dep_id| {
                            task_manager
                                .get_task(*dep_id)
                                .map_or(false, |dt| dt.state == TaskState::Completed)
                        })
                    })
            })
            .collect()
    }

    /// Check if a task can be scheduled given resource constraints.
    pub fn can_schedule_with_resources(
        &self,
        requirements: &HashMap<ResourceType, u64>,
        resource_coordinator: &ResourceCoordinator,
    ) -> bool {
        if !self.inner.policy.read().enable_resource_awareness {
            return true;
        }
        resource_coordinator.can_satisfy(requirements)
    }

    /// Get the queue depth.
    pub fn queue_depth(&self) -> usize {
        self.inner.queue.read().len()
    }

    /// Get the number of active executions.
    pub fn active_count(&self) -> usize {
        self.inner.active.read().len()
    }

    /// Check if the scheduler can accept more tasks.
    pub fn can_accept(&self) -> bool {
        let policy = self.inner.policy.read();
        let active = self.inner.active.read().len();
        let queued = self.inner.queue.read().len();
        active < policy.max_parallel || queued < policy.max_parallel * 2
    }

    /// Get scheduler statistics.
    pub fn statistics(&self) -> SchedulerStats {
        let mut stats = self.inner.stats.write().clone();
        stats.current_queue_depth = self.inner.queue.read().len();
        stats.active_executions = self.inner.active.read().len();
        stats
    }

    /// Get preemption log.
    pub fn preemption_log(&self) -> Vec<PreemptionEvent> {
        self.inner.preemption_log.read().clone()
    }

    /// Get the scheduling policy.
    pub fn policy(&self) -> SchedulingPolicy {
        self.inner.policy.read().clone()
    }

    /// Update the scheduling policy.
    pub fn set_policy(&self, policy: SchedulingPolicy) {
        *self.inner.policy.write() = policy;
    }

    /// Get all active executions.
    pub fn active_executions(&self) -> Vec<ScheduledExecution> {
        self.inner.active.read().values().cloned().collect()
    }

    /// Get all queued executions.
    pub fn queued_executions(&self) -> Vec<ScheduledExecution> {
        self.inner.queue.read().iter().cloned().collect()
    }

    /// Clear completed executions.
    pub fn clear_completed(&self) {
        self.inner.completed.write().clear();
    }

    /// Check for starved tasks (waiting too long).
    pub fn starved_tasks(&self) -> Vec<ScheduledExecution> {
        let threshold = self.inner.policy.read().starvation_threshold_secs;
        let now = Utc::now();

        self.inner
            .queue
            .read()
            .iter()
            .filter(|e| {
                now.signed_duration_since(e.scheduled_at)
                    .num_seconds()
                    as u64
                    > threshold
            })
            .cloned()
            .collect()
    }
}

impl Default for ExecutiveScheduler {
    fn default() -> Self {
        Self::new(SchedulingPolicy::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduler_creation() {
        let sched = ExecutiveScheduler::new(SchedulingPolicy::default());
        assert_eq!(sched.queue_depth(), 0);
        assert_eq!(sched.active_count(), 0);
    }

    #[test]
    fn schedule_and_dequeue() {
        let sched = ExecutiveScheduler::new(SchedulingPolicy::default());
        let priority_engine = PriorityEngine::new();
        let task = Task::new("test".to_string());

        sched.schedule_task(&task, &priority_engine).unwrap();
        assert_eq!(sched.queue_depth(), 1);

        let exec = sched.dequeue_next().unwrap();
        assert_eq!(exec.task_id, task.id);
        assert_eq!(sched.active_count(), 1);
    }

    #[test]
    fn complete_execution() {
        let sched = ExecutiveScheduler::new(SchedulingPolicy::default());
        let priority_engine = PriorityEngine::new();
        let task = Task::new("done".to_string());

        let exec_id = sched.schedule_task(&task, &priority_engine).unwrap();
        sched.dequeue_next().unwrap();
        sched.complete_execution(exec_id).unwrap();

        assert_eq!(sched.active_count(), 0);
        let stats = sched.statistics();
        assert_eq!(stats.total_completed, 1);
    }

    #[test]
    fn dependency_tracking() {
        let sched = ExecutiveScheduler::new(SchedulingPolicy::default());
        let t1 = TaskId::new();
        let t2 = TaskId::new();
        sched.add_dependency(t2, t1);
    }

    #[test]
    fn preemption() {
        let sched = ExecutiveScheduler::new(SchedulingPolicy {
            enable_preemption: true,
            ..SchedulingPolicy::default()
        });
        let priority_engine = PriorityEngine::new();
        let task = Task::new("preemptible".to_string());
        let exec_id = sched.schedule_task(&task, &priority_engine).unwrap();
        sched.dequeue_next().unwrap();

        let preemptor = TaskId::new();
        sched
            .preempt_execution(exec_id, "higher priority".to_string(), preemptor)
            .unwrap();

        assert_eq!(sched.active_count(), 0);
        assert_eq!(sched.queue_depth(), 1);
        assert_eq!(sched.statistics().total_preempted, 1);
    }

    #[test]
    fn scheduler_can_accept() {
        let sched = ExecutiveScheduler::new(SchedulingPolicy {
            max_parallel: 2,
            ..SchedulingPolicy::default()
        });
        assert!(sched.can_accept());
    }

    #[test]
    fn scheduler_stats() {
        let sched = ExecutiveScheduler::new(SchedulingPolicy::default());
        let stats = sched.statistics();
        assert_eq!(stats.total_scheduled, 0);
        assert_eq!(stats.total_completed, 0);
    }
}
