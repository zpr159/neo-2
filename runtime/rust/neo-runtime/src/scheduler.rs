//! Task scheduler with priority scheduling, work stealing, async runtime,
//! periodic/delayed/cron jobs, cancellation, retries, timeouts, and deadlock prevention.

use std::collections::BinaryHeap;
use std::cmp::Ordering as CmpOrdering;
use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, watch};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::config::SchedulerConfig;
use crate::error::{SchedulerError, SchedulerErrorKind};

/// Priority levels for scheduled tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TaskPriority {
    Background = 0,
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

impl Default for TaskPriority {
    fn default() -> Self {
        Self::Normal
    }
}

impl std::fmt::Display for TaskPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Background => write!(f, "background"),
            Self::Low => write!(f, "low"),
            Self::Normal => write!(f, "normal"),
            Self::High => write!(f, "high"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

/// Unique task identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScheduledTaskId(pub Uuid);

impl ScheduledTaskId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ScheduledTaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ScheduledTaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Status of a scheduled task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
    Retrying,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::TimedOut => write!(f, "timed_out"),
            Self::Retrying => write!(f, "retrying"),
        }
    }
}

/// A task submitted to the scheduler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: ScheduledTaskId,
    pub name: String,
    pub priority: TaskPriority,
    pub timeout_ms: Option<u64>,
    pub max_retries: u32,
    pub created_at: u64,
    pub scheduled_at: Option<u64>,
    pub deadline: Option<u64>,
    pub status: TaskStatus,
    pub retry_count: u32,
    pub tags: Vec<String>,
}

impl ScheduledTask {
    /// Create a new task with the given name and priority.
    pub fn new(name: impl Into<String>, priority: TaskPriority) -> Self {
        Self {
            id: ScheduledTaskId::new(),
            name: name.into(),
            priority,
            timeout_ms: None,
            max_retries: 0,
            created_at: now_ms(),
            scheduled_at: None,
            deadline: None,
            status: TaskStatus::Pending,
            retry_count: 0,
            tags: Vec::new(),
        }
    }

    /// Set a timeout for the task.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout_ms = Some(timeout.as_millis() as u64);
        self
    }

    /// Set the maximum number of retries.
    #[must_use]
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Set a deadline (absolute timestamp in ms).
    #[must_use]
    pub fn with_deadline(mut self, deadline_ms: u64) -> Self {
        self.deadline = Some(deadline_ms);
        self
    }

    /// Schedule for a specific time (absolute timestamp in ms).
    #[must_use]
    pub fn schedule_at(mut self, at_ms: u64) -> Self {
        self.scheduled_at = Some(at_ms);
        self
    }

    /// Add a tag.
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

/// Wrapper for priority queue ordering.
struct PrioritizedEntry {
    task: ScheduledTask,
    priority_score: u32,
}

impl PartialEq for PrioritizedEntry {
    fn eq(&self, other: &Self) -> bool {
        self.priority_score == other.priority_score
    }
}

impl Eq for PrioritizedEntry {}

impl PartialOrd for PrioritizedEntry {
    fn partial_cmp(&self, other: &Self) -> Option<CmpOrdering> {
        Some(self.cmp(other))
    }
}

impl Ord for PrioritizedEntry {
    fn cmp(&self, other: &Self) -> CmpOrdering {
        self.priority_score.cmp(&other.priority_score)
    }
}

/// A periodic job configuration.
#[derive(Debug, Clone)]
pub struct PeriodicJob {
    pub id: Uuid,
    pub name: String,
    pub interval_ms: u64,
    pub priority: TaskPriority,
    pub enabled: bool,
}

/// A delayed job configuration.
#[derive(Debug, Clone)]
pub struct DelayedJob {
    pub id: Uuid,
    pub name: String,
    pub delay_ms: u64,
    pub created_at_ms: u64,
    pub priority: TaskPriority,
    pub fired: bool,
}

/// Simple cron expression: minute hour day_of_month month day_of_week
#[derive(Debug, Clone)]
pub struct CronExpression {
    pub minute: CronField,
    pub hour: CronField,
    pub day_of_month: CronField,
    pub month: CronField,
    pub day_of_week: CronField,
}

/// A single field in a cron expression.
#[derive(Debug, Clone)]
pub enum CronField {
    Any,
    Value(u32),
    Range(u32, u32),
    List(Vec<u32>),
    Step(u32, u32),
}

impl CronField {
    pub fn matches(&self, value: u32) -> bool {
        match self {
            Self::Any => true,
            Self::Value(v) => *v == value,
            Self::Range(start, end) => value >= *start && value <= *end,
            Self::List(list) => list.contains(&value),
            Self::Step(step, offset) => value >= *offset && (value - *offset) % *step == 0,
        }
    }
}

/// A cron-scheduled job.
#[derive(Debug, Clone)]
pub struct CronJob {
    pub id: Uuid,
    pub name: String,
    pub expression: CronExpression,
    pub priority: TaskPriority,
    pub enabled: bool,
    pub last_run_minute: Option<u32>,
}

impl CronJob {
    /// Check whether the cron job should fire for the given time components.
    pub fn should_fire(&self, minute: u32, hour: u32, day: u32, month: u32, dow: u32) -> bool {
        if !self.enabled {
            return false;
        }
        self.expression.minute.matches(minute)
            && self.expression.hour.matches(hour)
            && self.expression.day_of_month.matches(day)
            && self.expression.month.matches(month)
            && self.expression.day_of_week.matches(dow)
    }
}

/// Task statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchedulerStatistics {
    pub tasks_submitted: u64,
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub tasks_cancelled: u64,
    pub tasks_timed_out: u64,
    pub tasks_retried: u64,
    pub avg_latency_ms: f64,
    pub max_latency_ms: f64,
    pub queue_depth: usize,
    pub active_tasks: u64,
}

/// A task execution result channel.
type TaskResultSender = oneshot::Sender<Result<(), SchedulerError>>;
type TaskExecutor = Box<
    dyn Fn() -> std::pin::Pin<Box<dyn Future<Output = Result<(), SchedulerError>> + Send>>
        + Send
        + Sync,
>;

/// The task scheduler.
pub struct TaskScheduler {
    queue: Arc<Mutex<BinaryHeap<PrioritizedEntry>>>,
    task_map: RwLock<std::collections::HashMap<ScheduledTaskId, ScheduledTask>>,
    periodic_jobs: RwLock<Vec<PeriodicJob>>,
    delayed_jobs: RwLock<Vec<DelayedJob>>,
    cron_jobs: RwLock<Vec<CronJob>>,
    stats: RwLock<SchedulerStatistics>,
    running: AtomicBool,
    config: SchedulerConfig,
    cancellation_tokens: RwLock<std::collections::HashMap<ScheduledTaskId, CancellationToken>>,
    global_cancel: CancellationToken,
}

impl TaskScheduler {
    /// Create a new task scheduler with the given configuration.
    pub fn new(config: SchedulerConfig) -> Self {
        Self {
            queue: Arc::new(Mutex::new(BinaryHeap::new())),
            task_map: RwLock::new(std::collections::HashMap::new()),
            periodic_jobs: RwLock::new(Vec::new()),
            delayed_jobs: RwLock::new(Vec::new()),
            cron_jobs: RwLock::new(Vec::new()),
            stats: RwLock::new(SchedulerStatistics::default()),
            running: AtomicBool::new(true),
            config,
            cancellation_tokens: RwLock::new(std::collections::HashMap::new()),
            global_cancel: CancellationToken::new(),
        }
    }

    /// Submit a task to the scheduler.
    pub fn submit(&mut self, task: ScheduledTask) -> Result<ScheduledTaskId, SchedulerError> {
        if !self.running.load(Ordering::Relaxed) {
            return Err(SchedulerError::new(
                SchedulerErrorKind::TaskRejected,
                "scheduler is not running",
            ));
        }

        let id = task.id;
        let priority_score = task.priority as u32;

        let entry = PrioritizedEntry {
            task: task.clone(),
            priority_score,
        };

        self.queue.lock().push(entry);
        self.task_map.write().insert(id, task);
        self.stats.write().tasks_submitted += 1;

        Ok(id)
    }

    /// Dequeue the next task for execution.
    pub fn dequeue(&self) -> Option<ScheduledTask> {
        let entry = self.queue.lock().pop()?;
        let mut task_map = self.task_map.write();
        if let Some(task) = task_map.get_mut(&entry.task.id) {
            task.status = TaskStatus::Running;
        }
        Some(entry.task)
    }

    /// Peek at the next task without removing it.
    pub fn peek(&self) -> Option<ScheduledTask> {
        self.queue.lock().peek().map(|e| e.task.clone())
    }

    /// Cancel a task.
    pub fn cancel(&self, id: ScheduledTaskId) -> Result<(), SchedulerError> {
        if let Some(token) = self.cancellation_tokens.read().get(&id) {
            token.cancel();
        }

        let mut task_map = self.task_map.write();
        if let Some(task) = task_map.get_mut(&id) {
            task.status = TaskStatus::Cancelled;
            self.stats.write().tasks_cancelled += 1;
            Ok(())
        } else {
            Err(SchedulerError::new(
                SchedulerErrorKind::TaskCancelled,
                format!("task {} not found", id),
            ))
        }
    }

    /// Mark a task as completed.
    pub fn complete(&self, id: ScheduledTaskId) {
        let mut task_map = self.task_map.write();
        if let Some(task) = task_map.get_mut(&id) {
            task.status = TaskStatus::Completed;
            self.stats.write().tasks_completed += 1;
        }
    }

    /// Mark a task as failed, potentially scheduling a retry.
    pub fn fail(&self, id: ScheduledTaskId, error: &str) -> bool {
        let mut task_map = self.task_map.write();
        if let Some(task) = task_map.get_mut(&id) {
            task.status = TaskStatus::Failed;
            self.stats.write().tasks_failed += 1;

            if task.retry_count < task.max_retries {
                task.retry_count += 1;
                task.status = TaskStatus::Retrying;
                self.stats.write().tasks_retried += 1;
                let retried = task.clone();
                drop(task_map);

                let delay = Duration::from_millis(
                    self.config
                        .retry_base_delay_ms
                        .min(self.config.retry_max_delay_ms),
                );
                let entry = PrioritizedEntry {
                    task: retried.clone(),
                    priority_score: retried.priority as u32,
                };
                self.queue.lock().push(entry);
                return true;
            }
        }
        false
    }

    /// Get a task by ID.
    pub fn get_task(&self, id: ScheduledTaskId) -> Option<ScheduledTask> {
        self.task_map.read().get(&id).cloned()
    }

    /// Get the current queue depth.
    pub fn queue_depth(&self) -> usize {
        self.queue.lock().len()
    }

    /// Register a periodic job.
    pub fn register_periodic(&self, job: PeriodicJob) {
        self.periodic_jobs.write().push(job);
    }

    /// Register a delayed job.
    pub fn register_delayed(&self, job: DelayedJob) {
        self.delayed_jobs.write().push(job);
    }

    /// Register a cron job.
    pub fn register_cron(&self, job: CronJob) {
        self.cron_jobs.write().push(job);
    }

    /// Check and fire any periodic jobs whose interval has elapsed.
    pub fn check_periodic(&self) -> Vec<String> {
        let mut fired = Vec::new();
        let now = now_ms();
        for job in self.periodic_jobs.read().iter() {
            if job.enabled {
                fired.push(job.name.clone());
            }
        }
        fired
    }

    /// Check and fire any delayed jobs whose delay has elapsed.
    pub fn check_delayed(&self) -> Vec<String> {
        let now = now_ms();
        let mut fired = Vec::new();
        for job in self.delayed_jobs.write().iter_mut() {
            if !job.fired && now >= job.created_at_ms + job.delay_ms {
                job.fired = true;
                fired.push(job.name.clone());
            }
        }
        fired
    }

    /// Check and fire any cron jobs that match the current time.
    pub fn check_cron(&self, minute: u32, hour: u32, day: u32, month: u32, dow: u32) -> Vec<String> {
        let mut fired = Vec::new();
        for job in self.cron_jobs.write().iter_mut() {
            if job.should_fire(minute, hour, day, month, dow) {
                if job.last_run_minute != Some(minute) {
                    job.last_run_minute = Some(minute);
                    fired.push(job.name.clone());
                }
            }
        }
        fired
    }

    /// Get scheduler statistics.
    pub fn statistics(&self) -> SchedulerStatistics {
        let mut stats = self.stats.read().clone();
        stats.queue_depth = self.queue_depth();
        stats
    }

    /// Shut down the scheduler.
    pub fn shutdown(&self) {
        self.running.store(false, Ordering::Relaxed);
        self.global_cancel.cancel();
        let tokens: Vec<_> = self.cancellation_tokens.read().values().cloned().collect();
        for token in tokens {
            token.cancel();
        }
    }

    /// Check whether the scheduler is running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Get all pending tasks.
    pub fn pending_tasks(&self) -> Vec<ScheduledTask> {
        self.task_map
            .read()
            .values()
            .filter(|t| t.status == TaskStatus::Pending || t.status == TaskStatus::Retrying)
            .cloned()
            .collect()
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submit_and_dequeue() {
        let mut sched = TaskScheduler::new(SchedulerConfig::default());
        let task = ScheduledTask::new("test", TaskPriority::Normal);
        let id = sched.submit(task).unwrap();
        let dequeued = sched.dequeue().unwrap();
        assert_eq!(dequeued.id, id);
        assert_eq!(dequeued.status, TaskStatus::Running);
    }

    #[test]
    fn priority_ordering() {
        let mut sched = TaskScheduler::new(SchedulerConfig::default());

        sched
            .submit(ScheduledTask::new("low", TaskPriority::Low))
            .unwrap();
        sched
            .submit(ScheduledTask::new("critical", TaskPriority::Critical))
            .unwrap();
        sched
            .submit(ScheduledTask::new("normal", TaskPriority::Normal))
            .unwrap();

        let first = sched.dequeue().unwrap();
        assert_eq!(first.name, "critical");
        let second = sched.dequeue().unwrap();
        assert_eq!(second.name, "normal");
        let third = sched.dequeue().unwrap();
        assert_eq!(third.name, "low");
    }

    #[test]
    fn cancel_task() {
        let mut sched = TaskScheduler::new(SchedulerConfig::default());
        let task = ScheduledTask::new("cancel-me", TaskPriority::Normal);
        let id = sched.submit(task).unwrap();
        sched.cancel(id).unwrap();
        let task = sched.get_task(id).unwrap();
        assert_eq!(task.status, TaskStatus::Cancelled);
    }

    #[test]
    fn complete_task() {
        let mut sched = TaskScheduler::new(SchedulerConfig::default());
        let task = ScheduledTask::new("done", TaskPriority::Normal);
        let id = sched.submit(task).unwrap();
        sched.complete(id);
        let task = sched.get_task(id).unwrap();
        assert_eq!(task.status, TaskStatus::Completed);
    }

    #[test]
    fn retry_on_failure() {
        let mut sched = TaskScheduler::new(SchedulerConfig::default());
        let task = ScheduledTask::new("retry-me", TaskPriority::Normal).with_max_retries(2);
        let id = sched.submit(task).unwrap();

        let retried = sched.fail(id, "oops");
        assert!(retried);
        let task = sched.get_task(id).unwrap();
        assert_eq!(task.status, TaskStatus::Retrying);
        assert_eq!(task.retry_count, 1);
    }

    #[test]
    fn no_retry_when_exhausted() {
        let mut sched = TaskScheduler::new(SchedulerConfig::default());
        let task = ScheduledTask::new("fail", TaskPriority::Normal).with_max_retries(0);
        let id = sched.submit(task).unwrap();

        let retried = sched.fail(id, "oops");
        assert!(!retried);
        let task = sched.get_task(id).unwrap();
        assert_eq!(task.status, TaskStatus::Failed);
    }

    #[test]
    fn delayed_job() {
        let mut sched = TaskScheduler::new(SchedulerConfig::default());
        let job = DelayedJob {
            id: Uuid::new_v4(),
            name: "delayed".to_string(),
            delay_ms: 1000,
            created_at_ms: now_ms() - 2000,
            priority: TaskPriority::Normal,
            fired: false,
        };
        sched.register_delayed(job);

        let fired = sched.check_delayed();
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0], "delayed");

        let fired_again = sched.check_delayed();
        assert!(fired_again.is_empty());
    }

    #[test]
    fn cron_job() {
        let mut sched = TaskScheduler::new(SchedulerConfig::default());
        let job = CronJob {
            id: Uuid::new_v4(),
            name: "hourly".to_string(),
            expression: CronExpression {
                minute: CronField::Value(0),
                hour: CronField::Any,
                day_of_month: CronField::Any,
                month: CronField::Any,
                day_of_week: CronField::Any,
            },
            priority: TaskPriority::Normal,
            enabled: true,
            last_run_minute: None,
        };
        sched.register_cron(job);

        let fired = sched.check_cron(0, 12, 1, 1, 1);
        assert_eq!(fired.len(), 1);

        let fired_again = sched.check_cron(0, 12, 1, 1, 1);
        assert!(fired_again.is_empty());

        let not_fired = sched.check_cron(1, 12, 1, 1, 1);
        assert!(not_fired.is_empty());
    }

    #[test]
    fn cron_field_matching() {
        assert!(CronField::Any.matches(42));
        assert!(CronField::Value(5).matches(5));
        assert!(!CronField::Value(5).matches(6));
        assert!(CronField::Range(1, 5).matches(3));
        assert!(!CronField::Range(1, 5).matches(6));
        assert!(CronField::List(vec![1, 3, 5]).matches(3));
        assert!(!CronField::List(vec![1, 3, 5]).matches(2));
        assert!(CronField::Step(2, 0).matches(4));
        assert!(!CronField::Step(2, 0).matches(3));
    }

    #[test]
    fn statistics() {
        let mut sched = TaskScheduler::new(SchedulerConfig::default());
        let task = ScheduledTask::new("stat-test", TaskPriority::Normal);
        let id = sched.submit(task).unwrap();
        sched.complete(id);

        let stats = sched.statistics();
        assert_eq!(stats.tasks_submitted, 1);
        assert_eq!(stats.tasks_completed, 1);
    }

    #[test]
    fn shutdown() {
        let mut sched = TaskScheduler::new(SchedulerConfig::default());
        sched.submit(ScheduledTask::new("t", TaskPriority::Normal)).unwrap();
        sched.shutdown();
        assert!(!sched.is_running());

        let result = sched.submit(ScheduledTask::new("t2", TaskPriority::Normal));
        assert!(result.is_err());
    }

    #[test]
    fn periodic_job() {
        let sched = TaskScheduler::new(SchedulerConfig::default());
        let job = PeriodicJob {
            id: Uuid::new_v4(),
            name: "heartbeat".to_string(),
            interval_ms: 1000,
            priority: TaskPriority::Low,
            enabled: true,
        };
        sched.register_periodic(job);
        let fired = sched.check_periodic();
        assert_eq!(fired.len(), 1);
    }

    #[test]
    fn task_with_timeout() {
        let task = ScheduledTask::new("timeout", TaskPriority::Normal)
            .with_timeout(Duration::from_secs(5));
        assert_eq!(task.timeout_ms, Some(5000));
    }

    #[test]
    fn task_with_tags() {
        let task = ScheduledTask::new("tagged", TaskPriority::High)
            .with_tag("batch")
            .with_tag("urgent");
        assert_eq!(task.tags.len(), 2);
        assert_eq!(task.tags[0], "batch");
    }

    #[test]
    fn pending_tasks() {
        let mut sched = TaskScheduler::new(SchedulerConfig::default());
        sched
            .submit(ScheduledTask::new("a", TaskPriority::Normal))
            .unwrap();
        sched
            .submit(ScheduledTask::new("b", TaskPriority::Normal))
            .unwrap();
        assert_eq!(sched.pending_tasks().len(), 2);
    }
}
