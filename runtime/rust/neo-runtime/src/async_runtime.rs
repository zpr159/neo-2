//! Async runtime integrating tokio with cancellation tokens, structured
//! concurrency, and backpressure.

use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use futures::future::join_all;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinSet;
use uuid::Uuid;

use crate::error::{RuntimeError, RuntimeErrorKind};

/// A simple cancellation token built on tokio primitives.
#[derive(Debug, Clone)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
    notify: Arc<tokio::sync::Notify>,
}

impl CancellationToken {
    /// Create a new cancellation token.
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            notify: Arc::new(tokio::sync::Notify::new()),
        }
    }

    /// Create a child token derived from a parent.
    /// Cancelling the parent also cancels the child.
    pub fn child_token(&self) -> Self {
        let child = Self::new();
        let child_cancelled = child.cancelled.clone();
        let child_notify = child.notify.clone();
        let parent_cancelled = self.cancelled.clone();
        let parent_notify = self.notify.clone();

        if parent_cancelled.load(Ordering::SeqCst) {
            child_cancelled.store(true, Ordering::SeqCst);
            child_notify.notify_waiters();
            return child;
        }

        let parent_notify_clone = parent_notify.clone();
        tokio::spawn(async move {
            loop {
                parent_notify_clone.notified().await;
                if parent_cancelled.load(Ordering::SeqCst) {
                    child_cancelled.store(true, Ordering::SeqCst);
                    child_notify.notify_waiters();
                    break;
                }
            }
        });

        child
    }

    /// Request cancellation.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    /// Check whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Wait until cancellation is requested.
    pub async fn cancelled(&self) {
        if self.is_cancelled() {
            return;
        }
        self.notify.notified().await;
    }

    /// Reset the token to non-cancelled state.
    pub fn reset(&self) {
        self.cancelled.store(false, Ordering::SeqCst);
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle for cancelling a spawned task.
#[derive(Debug, Clone)]
pub struct TaskHandle {
    pub id: Uuid,
    pub cancellation_token: CancellationToken,
}

impl TaskHandle {
    /// Request cancellation of this task.
    pub fn cancel(&self) {
        self.cancellation_token.cancel();
    }

    /// Check whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancellation_token.is_cancelled()
    }
}

/// Backpressure controller that limits concurrent work.
pub struct Backpressure {
    semaphore: Arc<Semaphore>,
    max_permits: usize,
    active: Arc<AtomicUsize>,
}

/// An RAII permit from the backpressure controller.
pub struct BackpressurePermit {
    _permit: OwnedSemaphorePermit,
    active: Arc<AtomicUsize>,
}

impl Drop for BackpressurePermit {
    fn drop(&mut self) {
        self.active.fetch_sub(1, Ordering::Relaxed);
    }
}

impl Backpressure {
    /// Create a new backpressure controller with the given concurrency limit.
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_permits: max_concurrent,
            active: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Acquire a permit. Blocks until one is available.
    pub async fn acquire(&self) -> Result<BackpressurePermit, RuntimeError> {
        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| {
                RuntimeError::new(
                    RuntimeErrorKind::Unknown,
                    format!("semaphore closed: {}", e),
                )
            })?;
        self.active.fetch_add(1, Ordering::Relaxed);
        Ok(BackpressurePermit {
            _permit: permit,
            active: Arc::clone(&self.active),
        })
    }

    /// Try to acquire a permit without blocking.
    pub fn try_acquire(&self) -> Result<BackpressurePermit, RuntimeError> {
        let permit = self.semaphore.clone().try_acquire_owned().map_err(|e| {
            RuntimeError::new(
                RuntimeErrorKind::Scheduler(crate::error::SchedulerErrorKind::QueueFull),
                format!("backpressure limit reached: {}", e),
            )
        })?;
        self.active.fetch_add(1, Ordering::Relaxed);
        Ok(BackpressurePermit {
            _permit: permit,
            active: Arc::clone(&self.active),
        })
    }

    /// Get the number of active permits.
    pub fn active_count(&self) -> usize {
        self.active.load(Ordering::Relaxed)
    }

    /// Get the maximum concurrency.
    pub fn max_concurrency(&self) -> usize {
        self.max_permits
    }
}

/// A structured concurrency scope that joins all spawned tasks before completing.
pub struct StructuredScope {
    id: Uuid,
    cancellation_token: CancellationToken,
    tasks: JoinSet<Result<(), RuntimeError>>,
}

impl StructuredScope {
    /// Create a new structured scope.
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            cancellation_token: CancellationToken::new(),
            tasks: JoinSet::new(),
        }
    }

    /// Create a child cancellation token derived from this scope.
    pub fn child_token(&self) -> CancellationToken {
        self.cancellation_token.child_token()
    }

    /// Spawn a future within this scope.
    pub fn spawn<F, Fut>(&mut self, f: F)
    where
        F: FnOnce(CancellationToken) -> Fut + Send + 'static,
        Fut: Future<Output = Result<(), RuntimeError>> + Send + 'static,
    {
        let token = self.cancellation_token.child_token();
        self.tasks.spawn(async move { f(token).await });
    }

    /// Spawn a simple future (no cancellation token) within this scope.
    pub fn spawn_simple<F, Fut>(&mut self, f: F)
    where
        F: Future<Output = Result<(), RuntimeError>> + Send + 'static,
    {
        self.tasks.spawn(f);
    }

    /// Wait for all tasks in the scope to complete.
    pub async fn join_all(self) -> Vec<Result<(), RuntimeError>> {
        let mut results = Vec::new();
        let mut tasks = self.tasks;
        while let Some(result) = tasks.join_next().await {
            match result {
                Ok(r) => results.push(r),
                Err(e) => {
                    results.push(Err(RuntimeError::new(
                        RuntimeErrorKind::Unknown,
                        format!("task join error: {}", e),
                    )));
                }
            }
        }
        results
    }

    /// Cancel all tasks in the scope.
    pub fn cancel_all(&self) {
        self.cancellation_token.cancel();
    }

    /// Get the scope ID.
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get the number of spawned tasks.
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }
}

/// Async runtime wrapping tokio.
pub struct NeoAsyncRuntime {
    runtime: tokio::runtime::Runtime,
    backpressure: Arc<Backpressure>,
    active_tasks: AtomicU64,
}

impl NeoAsyncRuntime {
    /// Create a new async runtime with the given concurrency limit.
    pub fn new(max_concurrent: usize) -> Result<Self, RuntimeError> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("neo-async")
            .build()
            .map_err(|e| {
                RuntimeError::new(
                    RuntimeErrorKind::Unknown,
                    format!("failed to build tokio runtime: {}", e),
                )
            })?;

        Ok(Self {
            runtime,
            backpressure: Arc::new(Backpressure::new(max_concurrent)),
            active_tasks: AtomicU64::new(0),
        })
    }

    /// Spawn a future on the runtime with a cancellation token.
    pub fn spawn_with_cancel<F, Fut>(&self, f: F) -> TaskHandle
    where
        F: FnOnce(CancellationToken) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let token = CancellationToken::new();
        let token_clone = token.clone();
        let bp = self.backpressure.clone();
        let active = Arc::new(AtomicU64::new(self.active_tasks.load(Ordering::Relaxed)));

        self.runtime.spawn(async move {
            let _permit = bp.acquire().await;
            active.fetch_add(1, Ordering::Relaxed);
            f(token_clone).await;
            active.fetch_sub(1, Ordering::Relaxed);
        });

        TaskHandle {
            id: Uuid::new_v4(),
            cancellation_token: token,
        }
    }

    /// Spawn a simple future on the runtime.
    pub fn spawn<F>(&self, f: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let bp = self.backpressure.clone();
        let active = Arc::new(AtomicU64::new(self.active_tasks.load(Ordering::Relaxed)));

        self.runtime.spawn(async move {
            let _permit = bp.acquire().await;
            active.fetch_add(1, Ordering::Relaxed);
            f.await;
            active.fetch_sub(1, Ordering::Relaxed);
        });
    }

    /// Spawn a future and return a JoinHandle.
    pub fn spawn_tracked<F>(&self, f: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.runtime.spawn(f)
    }

    /// Block on a future.
    pub fn block_on<F: Future>(&self, f: F) -> F::Output {
        self.runtime.block_on(f)
    }

    /// Run all futures concurrently, returning all results.
    pub async fn join_all<F>(&self, futures: Vec<F>) -> Vec<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.runtime
            .spawn(async move { join_all(futures).await })
            .await
            .unwrap_or_default()
    }

    /// Get a handle to the underlying tokio runtime.
    pub fn handle(&self) -> &tokio::runtime::Handle {
        self.runtime.handle()
    }

    /// Get the backpressure controller.
    pub fn backpressure(&self) -> &Backpressure {
        &self.backpressure
    }

    /// Get the number of active tasks.
    pub fn active_task_count(&self) -> u64 {
        self.active_tasks.load(Ordering::Relaxed)
    }

    /// Create a new structured scope.
    pub fn scope(&self) -> StructuredScope {
        StructuredScope::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn async_runtime_creation() {
        let rt = NeoAsyncRuntime::new(16).unwrap();
        assert_eq!(rt.active_task_count(), 0);
    }

    #[tokio::test]
    async fn spawn_and_complete() {
        let rt = NeoAsyncRuntime::new(16).unwrap();
        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = flag.clone();

        rt.spawn(async move {
            flag_clone.store(true, Ordering::SeqCst);
        });

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(flag.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn cancellation_token() {
        let rt = NeoAsyncRuntime::new(16).unwrap();
        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = flag.clone();

        let handle = rt.spawn_with_cancel(move |token| async move {
            loop {
                if token.is_cancelled() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            flag_clone.store(true, Ordering::SeqCst);
        });

        tokio::time::sleep(Duration::from_millis(50)).await;
        handle.cancel();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(flag.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn backpressure_limits_concurrency() {
        let bp = Backpressure::new(2);
        assert_eq!(bp.max_concurrency(), 2);
        assert_eq!(bp.active_count(), 0);

        let _p1 = bp.acquire().await.unwrap();
        let _p2 = bp.acquire().await.unwrap();
        assert_eq!(bp.active_count(), 2);
    }

    #[tokio::test]
    async fn structured_scope() {
        let mut scope = StructuredScope::new();
        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = flag.clone();

        scope.spawn_simple(async move {
            flag_clone.store(true, Ordering::SeqCst);
            Ok(())
        });

        let results = scope.join_all().await;
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
        assert!(flag.load(Ordering::SeqCst));
    }

    #[test]
    fn backpressure_try_acquire() {
        let bp = Backpressure::new(1);
        let _p1 = bp.try_acquire().unwrap();
        assert!(bp.try_acquire().is_err());
    }

    #[tokio::test]
    async fn cancellation_token_child() {
        let parent = CancellationToken::new();
        let child = parent.child_token();
        assert!(!child.is_cancelled());

        parent.cancel();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(child.is_cancelled());
    }

    #[tokio::test]
    async fn cancellation_token_wait() {
        let token = CancellationToken::new();
        let token_clone = token.clone();

        let handle = tokio::spawn(async move {
            token_clone.cancelled().await;
            true
        });

        tokio::time::sleep(Duration::from_millis(50)).await;
        token.cancel();
        let result = handle.await.unwrap();
        assert!(result);
    }
}
