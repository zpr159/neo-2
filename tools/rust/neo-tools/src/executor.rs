//! Tool execution engine with queue, scheduling, retries, and streaming.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Semaphore};

use crate::error::{ToolError, ToolResult};
use crate::registry::ToolRegistry;
use crate::types::{ExecutionId, ToolRequest, ToolResponse};

// ---------------------------------------------------------------------------
// ExecutionContext — runtime state for an execution
// ---------------------------------------------------------------------------

/// Runtime state for a single tool execution.
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub execution_id: ExecutionId,
    pub tool_name: String,
    pub operation: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub cancelled: Arc<AtomicBool>,
}

impl ExecutionContext {
    pub fn new(tool_name: impl Into<String>, operation: impl Into<String>) -> Self {
        Self {
            execution_id: ExecutionId::new(),
            tool_name: tool_name.into(),
            operation: operation.into(),
            started_at: chrono::Utc::now(),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

// ---------------------------------------------------------------------------
// ExecutionQueue — priority queue for pending executions
// ---------------------------------------------------------------------------

/// Priority-based execution queue.
pub struct ExecutionQueue {
    queue: RwLock<Vec<(u32, ToolRequest)>>,
    max_size: usize,
}

impl std::fmt::Debug for ExecutionQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutionQueue")
            .field("max_size", &self.max_size)
            .finish()
    }
}

impl ExecutionQueue {
    pub fn new(max_size: usize) -> Self {
        Self {
            queue: RwLock::new(Vec::new()),
            max_size,
        }
    }

    pub async fn enqueue(&self, priority: u32, request: ToolRequest) -> ToolResult<()> {
        let mut q = self.queue.write().await;
        if q.len() >= self.max_size {
            return Err(ToolError::resource_exhausted("execution queue is full"));
        }
        q.push((priority, request));
        q.sort_by(|a, b| b.0.cmp(&a.0));
        Ok(())
    }

    pub async fn dequeue(&self) -> Option<(u32, ToolRequest)> {
        let mut q = self.queue.write().await;
        q.pop()
    }

    pub async fn len(&self) -> usize {
        self.queue.read().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.queue.read().await.is_empty()
    }
}

// ---------------------------------------------------------------------------
// ToolExecutor — the execution engine
// ---------------------------------------------------------------------------

/// Execution engine that runs tool requests against the registry.
pub struct ToolExecutor {
    registry: Arc<ToolRegistry>,
    semaphore: Arc<Semaphore>,
    max_concurrent: usize,
    active_count: AtomicUsize,
    completed_count: AtomicUsize,
    failed_count: AtomicUsize,
}

impl std::fmt::Debug for ToolExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolExecutor")
            .field("max_concurrent", &self.max_concurrent)
            .field("active", &self.active_count.load(Ordering::Relaxed))
            .field("completed", &self.completed_count.load(Ordering::Relaxed))
            .field("failed", &self.failed_count.load(Ordering::Relaxed))
            .finish()
    }
}

impl ToolExecutor {
    pub fn new(registry: Arc<ToolRegistry>, max_concurrent: usize) -> Self {
        Self {
            registry,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_concurrent,
            active_count: AtomicUsize::new(0),
            completed_count: AtomicUsize::new(0),
            failed_count: AtomicUsize::new(0),
        }
    }

    /// Execute a tool request.
    pub async fn execute(&self, request: ToolRequest) -> ToolResult<ToolResponse> {
        let exec_ctx = ExecutionContext::new(request.tool_name(), &request.operation);

        let _permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| ToolError::resource_exhausted("execution semaphore closed"))?;

        self.active_count.fetch_add(1, Ordering::SeqCst);

        let start = std::time::Instant::now();
        let result = self.execute_inner(&request, &exec_ctx).await;
        let duration_ms = start.elapsed().as_millis() as u64;

        self.active_count.fetch_sub(1, Ordering::SeqCst);

        match &result {
            Ok(_) => self.completed_count.fetch_add(1, Ordering::SeqCst),
            Err(_) => self.failed_count.fetch_add(1, Ordering::SeqCst),
        };

        match result {
            Ok(output) => Ok(ToolResponse::success(
                exec_ctx.execution_id,
                request.tool_id,
                output,
                duration_ms,
            )),
            Err(err) => Ok(ToolResponse::failure(
                exec_ctx.execution_id,
                request.tool_id,
                err.to_string(),
                duration_ms,
            )),
        }
    }

    /// Execute with automatic retries.
    pub async fn execute_with_retries(
        &self,
        request: ToolRequest,
        max_retries: u32,
    ) -> ToolResult<ToolResponse> {
        let mut last_err = None;
        for attempt in 0..=max_retries {
            if attempt > 0 {
                tracing::debug!(
                    tool = %request.tool_name(),
                    attempt,
                    "retrying execution"
                );
                let delay = std::time::Duration::from_millis(100 * 2u64.pow(attempt - 1));
                tokio::time::sleep(delay).await;
            }
            match self.execute(request.clone()).await {
                Ok(response) => {
                    if response.success {
                        return Ok(response);
                    }
                    last_err = Some(response.error.unwrap_or_else(|| "unknown".into()));
                }
                Err(err) => {
                    if !err.is_retryable() {
                        return Err(err);
                    }
                    last_err = Some(err.to_string());
                }
            }
        }
        Err(ToolError::execution_failed(format!(
            "failed after {} retries: {}",
            max_retries,
            last_err.unwrap_or_else(|| "unknown".into())
        )))
    }

    /// Execute with a timeout.
    pub async fn execute_with_timeout(
        &self,
        request: ToolRequest,
        timeout_ms: u64,
    ) -> ToolResult<ToolResponse> {
        let fut = self.execute(request);
        match tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), fut).await {
            Ok(result) => result,
            Err(_) => Err(ToolError::timeout(format!(
                "execution timed out after {timeout_ms}ms"
            ))),
        }
    }

    /// Stream execution results via a channel.
    pub async fn execute_streaming(
        &self,
        requests: Vec<ToolRequest>,
    ) -> mpsc::Receiver<ToolResult<ToolResponse>> {
        let (tx, rx) = mpsc::channel(requests.len().max(1));
        for request in requests {
            let tx = tx.clone();
            let executor = self.clone_handle();
            tokio::spawn(async move {
                let result = executor.execute(request).await;
                let _ = tx.send(result).await;
            });
        }
        drop(tx);
        rx
    }

    fn clone_handle(&self) -> Self {
        Self {
            registry: Arc::clone(&self.registry),
            semaphore: Arc::clone(&self.semaphore),
            max_concurrent: self.max_concurrent,
            active_count: AtomicUsize::new(self.active_count.load(Ordering::SeqCst)),
            completed_count: AtomicUsize::new(self.completed_count.load(Ordering::SeqCst)),
            failed_count: AtomicUsize::new(self.failed_count.load(Ordering::SeqCst)),
        }
    }

    async fn execute_inner(
        &self,
        request: &ToolRequest,
        _exec_ctx: &ExecutionContext,
    ) -> ToolResult<serde_json::Value> {
        let tool_arc = self.registry.get(&request.tool_name()).await?;
        let tool = tool_arc.read().await;

        if !tool.is_executable() {
            return Err(ToolError::not_ready(format!(
                "tool '{}' is not in an executable state",
                request.tool_name()
            )));
        }

        if !tool.manifest.config.enabled {
            return Err(ToolError::disabled(format!(
                "tool '{}' is disabled",
                request.tool_name()
            )));
        }

        (tool.execute_fn)(request.parameters.clone(), request.context.clone()).await
    }

    pub fn active_count(&self) -> usize {
        self.active_count.load(Ordering::SeqCst)
    }

    pub fn completed_count(&self) -> usize {
        self.completed_count.load(Ordering::SeqCst)
    }

    pub fn failed_count(&self) -> usize {
        self.failed_count.load(Ordering::SeqCst)
    }

    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }
}

// ---------------------------------------------------------------------------
// ToolExecutorBuilder
// ---------------------------------------------------------------------------

/// Builder for constructing a `ToolExecutor`.
pub struct ToolExecutorBuilder {
    registry: Option<Arc<ToolRegistry>>,
    max_concurrent: usize,
}

impl std::fmt::Debug for ToolExecutorBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolExecutorBuilder")
            .field("has_registry", &self.registry.is_some())
            .field("max_concurrent", &self.max_concurrent)
            .finish()
    }
}

impl ToolExecutorBuilder {
    pub fn new() -> Self {
        Self {
            registry: None,
            max_concurrent: 10,
        }
    }

    pub fn registry(mut self, registry: Arc<ToolRegistry>) -> Self {
        self.registry = Some(registry);
        self
    }

    pub fn max_concurrent(mut self, val: usize) -> Self {
        self.max_concurrent = val;
        self
    }

    pub fn build(self) -> ToolResult<ToolExecutor> {
        let registry = self
            .registry
            .ok_or_else(|| ToolError::config("registry is required"))?;
        Ok(ToolExecutor::new(registry, self.max_concurrent))
    }
}

impl Default for ToolExecutorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::ToolRegistry;
    use crate::tool::ToolBuilder;
    use crate::types::{ToolCategory, ToolContext, ToolId, ToolType, ToolVersion};
    use std::sync::Arc;

    fn make_test_tool(name: &str) -> crate::tool::DynamicTool {
        ToolBuilder::new(
            name,
            ToolVersion::new(1, 0, 0),
            "Test tool",
            ToolType::Custom("test".into()),
            ToolCategory::Execute,
        )
        .on_execute(|params, _ctx| Box::pin(async move { Ok(params) }))
        .build()
        .unwrap()
    }

    #[tokio::test]
    async fn test_execute_tool() {
        let registry = Arc::new(ToolRegistry::new());
        registry.register(make_test_tool("echo")).await.unwrap();

        let executor = ToolExecutor::new(Arc::clone(&registry), 5);

        let tool_id = ToolId::new();
        let ctx = ToolContext::new("test", crate::types::CallerType::Internal);
        let request = ToolRequest::named(
            tool_id,
            "echo",
            "echo",
            serde_json::json!({"message": "hello"}),
            ctx,
        );

        let response = executor.execute(request).await.unwrap();
        assert!(response.success);
        assert_eq!(executor.completed_count(), 1);
    }

    #[tokio::test]
    async fn test_concurrent_limit() {
        let registry = Arc::new(ToolRegistry::new());
        registry.register(make_test_tool("slow")).await.unwrap();

        let executor = ToolExecutor::new(Arc::clone(&registry), 2);
        assert_eq!(executor.max_concurrent(), 2);
    }
}
