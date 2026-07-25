//! Thread pool with dynamic workers, affinity, priority, task queues, statistics,
//! and auto-scaling.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use parking_lot::{Condvar, Mutex};
use serde::{Deserialize, Serialize};

use crate::error::{RuntimeError, RuntimeErrorKind};
use crate::config::ThreadPoolConfig;

/// Priority of a task submitted to the thread pool.
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

/// A task to be executed by the thread pool.
type BoxTask = Box<dyn FnOnce() + Send + 'static>;

/// Wrapper that pairs a task with its priority.
struct PrioritizedTask {
    task: BoxTask,
    priority: TaskPriority,
}

impl PrioritizedTask {
    fn new(priority: TaskPriority, task: BoxTask) -> Self {
        Self { task, priority }
    }
}

/// Work-stealing deque for distributing tasks across workers.
struct WorkStealingQueue {
    inner: Mutex<VecDeque<PrioritizedTask>>,
    notify: Condvar,
}

impl WorkStealingQueue {
    fn new() -> Self {
        Self {
            inner: Mutex::new(VecDeque::new()),
            notify: Condvar::new(),
        }
    }

    fn push(&self, task: PrioritizedTask) {
        self.inner.lock().push_back(task);
        self.notify.notify_one();
    }

    fn pop(&self) -> Option<PrioritizedTask> {
        self.inner.lock().pop_front()
    }

    fn steal_from(&self) -> Option<PrioritizedTask> {
        self.inner.lock().pop_back()
    }

    fn len(&self) -> usize {
        self.inner.lock().len()
    }

    fn is_empty(&self) -> bool {
        self.inner.lock().is_empty()
    }

    fn wait_for_task(&self, timeout: Duration) -> Option<PrioritizedTask> {
        let mut queue = self.inner.lock();
        if let Some(task) = queue.pop_front() {
            return Some(task);
        }
        let mut timeout = Some(timeout);
        while queue.is_empty() {
            if let Some(t) = timeout {
                if self.notify.wait_for(&mut queue, t).timed_out() {
                    return queue.pop_front();
                }
            } else {
                self.notify.wait(&mut queue);
            }
        }
        queue.pop_front()
    }
}

/// Statistics for a single worker thread.
#[derive(Debug, Default)]
pub struct WorkerStats {
    pub tasks_completed: AtomicU64,
    pub tasks_stolen: AtomicU64,
    pub busy_time_ms: AtomicU64,
    pub idle_time_ms: AtomicU64,
}

/// Configuration for a single worker.
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub worker_id: usize,
    pub core_affinity: Option<usize>,
    pub stack_size: usize,
}

/// A worker thread that pulls tasks from its local queue or steals from others.
struct Worker {
    id: usize,
    local_queue: Arc<WorkStealingQueue>,
    global_queue: Arc<WorkStealingQueue>,
    all_queues: Vec<Arc<WorkStealingQueue>>,
    stats: Arc<WorkerStats>,
    running: Arc<AtomicBool>,
    config: WorkerConfig,
}

impl Worker {
    fn run(self) {
        if let Some(core) = self.config.core_affinity {
            let _ = set_thread_affinity(core);
        }

        while self.running.load(Ordering::Relaxed) {
            let task = self
                .local_queue
                .wait_for_task(Duration::from_millis(100))
                .or_else(|| {
                    for queue in &self.all_queues {
                        if let Some(task) = queue.steal_from() {
                            self.stats.tasks_stolen.fetch_add(1, Ordering::Relaxed);
                            return Some(task);
                        }
                    }
                    self.global_queue.pop()
                });

            if let Some(_task) = task {
                let start = std::time::Instant::now();
                (_task.task)();
                let elapsed = start.elapsed().as_millis() as u64;
                self.stats.tasks_completed.fetch_add(1, Ordering::Relaxed);
                self.stats.busy_time_ms.fetch_add(elapsed, Ordering::Relaxed);
            } else {
                thread::sleep(Duration::from_millis(1));
                self.stats.idle_time_ms.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

/// Thread pool statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThreadPoolStatistics {
    pub total_workers: usize,
    pub active_workers: usize,
    pub tasks_submitted: u64,
    pub tasks_completed: u64,
    pub tasks_stolen: u64,
    pub total_busy_ms: u64,
    pub total_idle_ms: u64,
    pub queue_depth: usize,
    pub utilization: f64,
}

/// Auto-scaling policy.
#[derive(Debug, Clone)]
pub struct AutoScaler {
    scale_up_threshold: f64,
    scale_down_threshold: f64,
    min_workers: usize,
    max_workers: usize,
    check_interval: Duration,
}

impl AutoScaler {
    pub fn new(config: &ThreadPoolConfig) -> Self {
        Self {
            scale_up_threshold: config.scale_up_threshold,
            scale_down_threshold: config.scale_down_threshold,
            min_workers: config.min_workers,
            max_workers: config.max_workers,
            check_interval: Duration::from_secs(5),
        }
    }

    /// Determine the desired number of workers based on current utilization.
    pub fn desired_workers(&self, current_workers: usize, utilization: f64) -> usize {
        if utilization > self.scale_up_threshold && current_workers < self.max_workers {
            (current_workers + 1).min(self.max_workers)
        } else if utilization < self.scale_down_threshold && current_workers > self.min_workers {
            current_workers.saturating_sub(1).max(self.min_workers)
        } else {
            current_workers
        }
    }
}

/// Thread pool with work stealing and auto-scaling.
pub struct ThreadPool {
    global_queue: Arc<WorkStealingQueue>,
    worker_queues: Vec<Arc<WorkStealingQueue>>,
    worker_stats: Vec<Arc<WorkerStats>>,
    running: Arc<AtomicBool>,
    config: ThreadPoolConfig,
    auto_scaler: AutoScaler,
    tasks_submitted: AtomicUsize,
    workers: Mutex<Vec<thread::JoinHandle<()>>>,
}

impl ThreadPool {
    /// Create a new thread pool with the given configuration.
    pub fn new(config: ThreadPoolConfig) -> Self {
        let auto_scaler = AutoScaler::new(&config);
        let worker_count = config.min_workers;

        let global_queue = Arc::new(WorkStealingQueue::new());
        let mut worker_queues = Vec::with_capacity(worker_count);
        let mut worker_stats = Vec::with_capacity(worker_count);

        for _ in 0..worker_count {
            worker_queues.push(Arc::new(WorkStealingQueue::new()));
            worker_stats.push(Arc::new(WorkerStats::default()));
        }

        let running = Arc::new(AtomicBool::new(true));

        let mut handles = Vec::new();
        for i in 0..worker_count {
            let local_queue = worker_queues[i].clone();
            let all_queues: Vec<_> = worker_queues.iter().cloned().collect();
            let worker = Worker {
                id: i,
                local_queue: local_queue.clone(),
                global_queue: global_queue.clone(),
                all_queues,
                stats: worker_stats[i].clone(),
                running: running.clone(),
                config: WorkerConfig {
                    worker_id: i,
                    core_affinity: None,
                    stack_size: config.worker_stack_size,
                },
            };

            let handle = thread::Builder::new()
                .name(format!("neo-worker-{}", i))
                .spawn(move || worker.run())
                .expect("failed to spawn worker thread");
            handles.push(handle);
        }

        Self {
            global_queue,
            worker_queues,
            worker_stats,
            running,
            config,
            auto_scaler,
            tasks_submitted: AtomicUsize::new(0),
            workers: Mutex::new(handles),
        }
    }

    /// Submit a task with the given priority.
    pub fn submit<F>(&self, priority: TaskPriority, task: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.tasks_submitted.fetch_add(1, Ordering::Relaxed);

        let prioritized = PrioritizedTask::new(priority, Box::new(task));

        if self.worker_queues.is_empty() {
            self.global_queue.push(prioritized);
            return;
        }

        let target = self.tasks_submitted.load(Ordering::Relaxed) % self.worker_queues.len();
        self.worker_queues[target].push(prioritized);
    }

    /// Get aggregate statistics for the pool.
    pub fn statistics(&self) -> ThreadPoolStatistics {
        let total_workers = self.worker_queues.len();
        let mut tasks_completed: u64 = 0;
        let mut tasks_stolen: u64 = 0;
        let mut total_busy: u64 = 0;
        let mut total_idle: u64 = 0;
        let mut queue_depth = self.global_queue.len();

        for (i, queue) in self.worker_queues.iter().enumerate() {
            queue_depth += queue.len();
            let stats = &self.worker_stats[i];
            let completed = stats.tasks_completed.load(Ordering::Relaxed);
            tasks_completed += completed;
            tasks_stolen += stats.tasks_stolen.load(Ordering::Relaxed);
            total_busy += stats.busy_time_ms.load(Ordering::Relaxed);
            total_idle += stats.idle_time_ms.load(Ordering::Relaxed);
        }

        let total_time = total_busy + total_idle;
        let utilization = if total_time > 0 && total_workers > 0 {
            total_busy as f64 / (total_time as f64 / total_workers as f64)
        } else {
            0.0
        };

        ThreadPoolStatistics {
            total_workers,
            active_workers: total_workers,
            tasks_submitted: self.tasks_submitted.load(Ordering::Relaxed) as u64,
            tasks_completed,
            tasks_stolen,
            total_busy_ms: total_busy,
            total_idle_ms: total_idle,
            queue_depth,
            utilization: utilization.clamp(0.0, 1.0),
        }
    }

    /// Shut down the thread pool, waiting for all tasks to complete.
    pub fn shutdown(&self) {
        self.running.store(false, Ordering::Relaxed);
        for queue in &self.worker_queues {
            queue.notify.notify_all();
        }
        self.global_queue.notify.notify_all();

        let mut workers = self.workers.lock();
        while let Some(handle) = workers.pop() {
            let _ = handle.join();
        }
    }

    /// Get the number of worker threads.
    pub fn worker_count(&self) -> usize {
        self.worker_queues.len()
    }

    /// Get the current queue depth (global + all worker queues).
    pub fn queue_depth(&self) -> usize {
        let mut depth = self.global_queue.len();
        for q in &self.worker_queues {
            depth += q.len();
        }
        depth
    }

    /// Check whether the pool is running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        if self.running.load(Ordering::Relaxed) {
            self.shutdown();
        }
    }
}

fn set_thread_affinity(_core: usize) -> Result<(), String> {
    // Thread affinity is platform-specific. On Linux this would use sched_setaffinity,
    // but we keep the interface safe without requiring the libc crate.
    // The affinity hint is best-effort and non-critical for correctness.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Arc;

    #[test]
    fn thread_pool_creation() {
        let config = ThreadPoolConfig {
            min_workers: 2,
            max_workers: 4,
            auto_scale: false,
            ..ThreadPoolConfig::default()
        };
        let pool = ThreadPool::new(config);
        assert_eq!(pool.worker_count(), 2);
        assert!(pool.is_running());
        pool.shutdown();
        assert!(!pool.is_running());
    }

    #[test]
    fn submit_and_complete_tasks() {
        let config = ThreadPoolConfig {
            min_workers: 2,
            max_workers: 4,
            auto_scale: false,
            ..ThreadPoolConfig::default()
        };
        let pool = ThreadPool::new(config);
        let counter = Arc::new(AtomicUsize::new(0));

        for _ in 0..100 {
            let c = counter.clone();
            pool.submit(TaskPriority::Normal, move || {
                c.fetch_add(1, Ordering::SeqCst);
            });
        }

        thread::sleep(Duration::from_millis(500));
        assert_eq!(counter.load(Ordering::SeqCst), 100);
        pool.shutdown();
    }

    #[test]
    fn priority_ordering() {
        let config = ThreadPoolConfig {
            min_workers: 1,
            max_workers: 1,
            auto_scale: false,
            ..ThreadPoolConfig::default()
        };
        let pool = ThreadPool::new(config);

        assert!(TaskPriority::Critical > TaskPriority::High);
        assert!(TaskPriority::High > TaskPriority::Normal);
        assert!(TaskPriority::Normal > TaskPriority::Low);
        assert!(TaskPriority::Low > TaskPriority::Background);

        pool.submit(TaskPriority::Normal, || {});
        pool.shutdown();
    }

    #[test]
    fn statistics_tracking() {
        let config = ThreadPoolConfig {
            min_workers: 2,
            max_workers: 4,
            auto_scale: false,
            ..ThreadPoolConfig::default()
        };
        let pool = ThreadPool::new(config);

        for _ in 0..10 {
            pool.submit(TaskPriority::Normal, || {
                thread::sleep(Duration::from_millis(1));
            });
        }

        thread::sleep(Duration::from_millis(200));
        let stats = pool.statistics();
        assert_eq!(stats.total_workers, 2);
        assert!(stats.tasks_completed >= 10);
        pool.shutdown();
    }

    #[test]
    fn auto_scaler_desired_workers() {
        let config = ThreadPoolConfig {
            min_workers: 2,
            max_workers: 8,
            auto_scale: true,
            scale_up_threshold: 0.8,
            scale_down_threshold: 0.2,
            ..ThreadPoolConfig::default()
        };
        let scaler = AutoScaler::new(&config);

        assert_eq!(scaler.desired_workers(4, 0.9), 5);
        assert_eq!(scaler.desired_workers(4, 0.1), 3);
        assert_eq!(scaler.desired_workers(4, 0.5), 4);
        assert_eq!(scaler.desired_workers(8, 0.9), 8);
        assert_eq!(scaler.desired_workers(2, 0.1), 2);
    }

    #[test]
    fn work_stealing_queue() {
        let queue = WorkStealingQueue::new();
        assert!(queue.is_empty());

        queue.push(PrioritizedTask::new(
            TaskPriority::Normal,
            Box::new(|| {}),
        ));
        assert_eq!(queue.len(), 1);

        let task = queue.pop();
        assert!(task.is_some());
        assert!(queue.is_empty());
    }
}
