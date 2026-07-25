use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::{
    Capability, CapabilityId, CancellationToken, CapabilityResult_output, ExecutionContext,
    ProgressUpdate, ResourceRequirements, StreamChunk,
};
use crate::error::{CapabilityError, CapabilityResult};

/// Execution mode for a pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Execute capabilities one after another.
    Sequential,
    /// Execute capabilities concurrently.
    Parallel,
    /// Execute capabilities in a pipeline (output of one feeds into the next).
    Pipeline,
}

impl Default for ExecutionMode {
    fn default() -> Self {
        Self::Sequential
    }
}

/// Configuration for retry behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retries.
    pub max_retries: u32,
    /// Base delay in milliseconds.
    pub base_delay_ms: u64,
    /// Maximum delay in milliseconds.
    pub max_delay_ms: u64,
    /// Backoff multiplier.
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 100,
            max_delay_ms: 30_000,
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// No retries.
    pub fn none() -> Self {
        Self {
            max_retries: 0,
            base_delay_ms: 0,
            max_delay_ms: 0,
            backoff_multiplier: 1.0,
        }
    }

    /// Calculate delay for a given attempt.
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::from_millis(self.base_delay_ms);
        }
        let delay = (self.base_delay_ms as f64) * self.backoff_multiplier.powi(attempt as i32 - 1);
        Duration::from_millis((delay as u64).min(self.max_delay_ms))
    }
}

/// Timeout configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// Timeout in milliseconds.
    pub timeout_ms: u64,
    /// Whether timeout is strict (returns error on timeout).
    pub strict: bool,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 60_000,
            strict: true,
        }
    }
}

impl TimeoutConfig {
    /// Create a strict timeout.
    pub fn strict_ms(ms: u64) -> Self {
        Self {
            timeout_ms: ms,
            strict: true,
        }
    }

    /// Create a lenient timeout.
    pub fn lenient_ms(ms: u64) -> Self {
        Self {
            timeout_ms: ms,
            strict: false,
        }
    }
}

/// A request to execute a capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRequest {
    /// The capability to execute.
    pub capability_id: CapabilityId,
    /// Input data.
    pub input: serde_json::Value,
    /// Execution mode.
    pub mode: ExecutionMode,
    /// Retry configuration.
    pub retry_config: RetryConfig,
    /// Timeout configuration.
    pub timeout_config: TimeoutConfig,
    /// Priority (0-255).
    pub priority: u8,
    /// Additional metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ExecutionRequest {
    /// Create a new execution request.
    pub fn new(capability_id: CapabilityId, input: serde_json::Value) -> Self {
        Self {
            capability_id,
            input,
            mode: ExecutionMode::Sequential,
            retry_config: RetryConfig::default(),
            timeout_config: TimeoutConfig::default(),
            priority: 128,
            metadata: HashMap::new(),
        }
    }

    /// Set retry config.
    pub fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    /// Set timeout config.
    pub fn with_timeout_config(mut self, config: TimeoutConfig) -> Self {
        self.timeout_config = config;
        self
    }

    /// Set priority.
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }
}

/// Record of a capability execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    /// Unique execution ID.
    pub id: Uuid,
    /// The request.
    pub request: ExecutionRequest,
    /// Result if completed.
    pub result: Option<CapabilityResult_output>,
    /// When execution started.
    pub started_at: DateTime<Utc>,
    /// When execution completed.
    pub completed_at: Option<DateTime<Utc>>,
    /// Worker that executed this.
    pub worker_id: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
    /// Current retry count.
    pub retry_count: u32,
    /// Status.
    pub status: ExecutionStatus,
}

/// Status of an execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

impl std::fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::TimedOut => write!(f, "timed_out"),
        }
    }
}

/// Streaming output manager for capability execution.
pub struct StreamingOutput {
    /// Sender for chunks.
    tx: tokio::sync::mpsc::UnboundedSender<StreamChunk>,
    /// Sequence counter.
    sequence: AtomicU64,
}

impl StreamingOutput {
    /// Create a new streaming output.
    pub fn new() -> (Self, tokio::sync::mpsc::UnboundedReceiver<StreamChunk>) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        (
            Self {
                tx,
                sequence: AtomicU64::new(0),
            },
            rx,
        )
    }

    /// Send a data chunk.
    pub fn send(&self, data: serde_json::Value) {
        let seq = self.sequence.fetch_add(1, Ordering::SeqCst);
        let _ = self.tx.send(StreamChunk {
            data,
            done: false,
            sequence: Some(seq),
        });
    }

    /// Send the final chunk.
    pub fn send_done(&self, data: serde_json::Value) {
        let seq = self.sequence.load(Ordering::SeqCst);
        let _ = self.tx.send(StreamChunk {
            data,
            done: true,
            sequence: Some(seq),
        });
    }

    /// Send an error and close.
    pub fn send_error(&self, error: String) {
        let _ = self.tx.send(StreamChunk {
            data: serde_json::json!({"error": error}),
            done: true,
            sequence: None,
        });
    }
}

impl Default for StreamingOutput {
    fn default() -> Self {
        let (out, _) = Self::new();
        out
    }
}

/// An execution pipeline that chains capabilities together.
pub struct ExecutionPipeline {
    /// Steps in the pipeline.
    steps: Vec<PipelineStep>,
    /// Pipeline-level timeout.
    timeout: TimeoutConfig,
    /// Pipeline-level retry config.
    retry: RetryConfig,
}

/// A single step in a pipeline.
#[derive(Clone)]
pub struct PipelineStep {
    /// Human-readable step name.
    pub name: String,
    /// The input template (supports placeholders like $prev.output).
    pub input_template: serde_json::Value,
}

impl ExecutionPipeline {
    /// Create a new empty pipeline.
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            timeout: TimeoutConfig::default(),
            retry: RetryConfig::none(),
        }
    }

    /// Add a step to the pipeline.
    pub fn add_step(&mut self, name: impl Into<String>, input_template: serde_json::Value) {
        self.steps.push(PipelineStep {
            name: name.into(),
            input_template,
        });
    }

    /// Set the pipeline timeout.
    pub fn with_timeout(mut self, config: TimeoutConfig) -> Self {
        self.timeout = config;
        self
    }

    /// Execute the pipeline with a sequence of capabilities.
    pub async fn execute(
        &self,
        capabilities: &[Arc<RwLock<dyn Capability>>],
        initial_input: serde_json::Value,
        mut context: ExecutionContext,
    ) -> CapabilityResult<CapabilityResult_output> {
        if capabilities.is_empty() {
            return Err(CapabilityError::composition_failed(
                "pipeline has no capabilities",
            ));
        }

        let start = Utc::now();
        let mut current_input = initial_input;
        let mut last_output = None;

        for (i, cap) in capabilities.iter().enumerate() {
            if context.is_cancelled() {
                return Err(CapabilityError::cancelled("pipeline cancelled"));
            }

            let cap_guard = cap.read();
            let cap_id = cap_guard.metadata().id;

            context.report_progress(ProgressUpdate::new(
                i as u32,
                capabilities.len() as u32,
                format!("executing step {}: {}", i, {
                    let meta = cap_guard.metadata();
                    meta.name.clone()
                }),
            ));

            let result = execute_with_timeout(
                &*cap_guard,
                current_input.clone(),
                context.clone(),
                self.timeout.timeout_ms,
            )
            .await?;

            if !result.success {
                return Err(CapabilityError::execution_failed(
                    result.error.unwrap_or_else(|| "pipeline step failed".to_string()),
                ));
            }

            current_input = result.output.clone();
            last_output = Some(result);
        }

        let total_duration = Utc::now()
            .signed_duration_since(start)
            .num_milliseconds() as u64;

        Ok(last_output.unwrap_or_else(|| {
            CapabilityResult_output::success(serde_json::Value::Null, total_duration)
        }))
    }

    /// Get the number of steps.
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }
}

impl Default for ExecutionPipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// Execute a capability with a timeout.
async fn execute_with_timeout(
    capability: &dyn Capability,
    input: serde_json::Value,
    context: ExecutionContext,
    timeout_ms: u64,
) -> CapabilityResult<CapabilityResult_output> {
    let result = tokio::time::timeout(
        Duration::from_millis(timeout_ms),
        capability.execute(input, context),
    )
    .await;

    match result {
        Ok(inner) => inner,
        Err(_elapsed) => Err(CapabilityError::timeout(format!(
            "capability '{}' timed out after {}ms",
            capability.metadata().name, timeout_ms
        ))),
    }
}

/// Main capability executor that manages execution lifecycle.
pub struct CapabilityExecutor {
    /// Execution history.
    records: RwLock<HashMap<Uuid, ExecutionRecord>>,
    /// Active executions.
    active: RwLock<HashMap<Uuid, CancellationToken>>,
    /// Sequence counter for execution IDs.
    next_id: AtomicU64,
}

impl CapabilityExecutor {
    /// Create a new capability executor.
    pub fn new() -> Self {
        Self {
            records: RwLock::new(HashMap::new()),
            active: RwLock::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        }
    }

    /// Execute a capability.
    pub async fn execute_capability(
        &self,
        capability: &dyn Capability,
        mut context: ExecutionContext,
        input: serde_json::Value,
    ) -> CapabilityResult<CapabilityResult_output> {
        let request_id = Uuid::new_v4();
        let start = Utc::now();

        let record = ExecutionRecord {
            id: request_id,
            request: ExecutionRequest {
                capability_id: context.capability_id,
                input: input.clone(),
                mode: ExecutionMode::Sequential,
                retry_config: RetryConfig::default(),
                timeout_config: TimeoutConfig::default(),
                priority: 128,
                metadata: HashMap::new(),
            },
            result: None,
            started_at: start,
            completed_at: None,
            worker_id: None,
            error: None,
            retry_count: 0,
            status: ExecutionStatus::Running,
        };

        self.records.write().insert(request_id, record);
        self.active
            .write()
            .insert(request_id, context.cancel_token.clone());

        let result = capability.execute(input, context.clone()).await;

        self.active.write().remove(&request_id);

        let duration = Utc::now()
            .signed_duration_since(start)
            .num_milliseconds() as u64;

        let mut records = self.records.write();
        if let Some(record) = records.get_mut(&request_id) {
            record.completed_at = Some(Utc::now());
            match &result {
                Ok(output) => {
                    record.result = Some(output.clone());
                    record.status = if output.success {
                        ExecutionStatus::Completed
                    } else {
                        ExecutionStatus::Failed
                    };
                }
                Err(e) => {
                    record.error = Some(e.to_string());
                    record.status = ExecutionStatus::Failed;
                }
            }
        }

        result
    }

    /// Execute with retry logic.
    pub async fn execute_with_retry(
        &self,
        capability: &dyn Capability,
        context: ExecutionContext,
        input: serde_json::Value,
        retry_config: RetryConfig,
    ) -> CapabilityResult<CapabilityResult_output> {
        let mut last_error = None;

        for attempt in 0..=retry_config.max_retries {
            if context.is_cancelled() {
                return Err(CapabilityError::cancelled("execution cancelled"));
            }

            let ctx = ExecutionContext {
                execution_id: Uuid::new_v4(),
                capability_id: context.capability_id,
                permissions: context.permissions.clone(),
                environment: context.environment.clone(),
                timeout_ms: context.timeout_ms,
                cancel_token: context.cancel_token.clone(),
                progress_callback: context.progress_callback.clone(),
            };

            match capability.execute(input.clone(), ctx).await {
                Ok(result) => {
                    if result.success {
                        return Ok(result);
                    }
                    last_error = result.error.clone();
                }
                Err(e) => {
                    last_error = Some(e.to_string());
                }
            }

            if attempt < retry_config.max_retries {
                let delay = retry_config.delay_for_attempt(attempt);
                tokio::time::sleep(delay).await;
            }
        }

        Err(CapabilityError::execution_failed(format!(
            "failed after {} retries: {}",
            retry_config.max_retries,
            last_error.unwrap_or_else(|| "unknown error".to_string())
        )))
    }

    /// Execute with a timeout.
    pub async fn execute_with_timeout(
        &self,
        capability: &dyn Capability,
        context: ExecutionContext,
        input: serde_json::Value,
        timeout_ms: u64,
    ) -> CapabilityResult<CapabilityResult_output> {
        execute_with_timeout(capability, input, context, timeout_ms).await
    }

    /// Cancel an execution.
    pub fn cancel_execution(&self, execution_id: &Uuid) -> bool {
        if let Some(token) = self.active.read().get(execution_id) {
            token.cancel();
            if let Some(record) = self.records.write().get_mut(execution_id) {
                record.status = ExecutionStatus::Cancelled;
                record.completed_at = Some(Utc::now());
            }
            true
        } else {
            false
        }
    }

    /// Get an execution record.
    pub fn get_record(&self, id: &Uuid) -> Option<ExecutionRecord> {
        self.records.read().get(id).cloned()
    }

    /// List all execution records.
    pub fn list_executions(&self) -> Vec<ExecutionRecord> {
        self.records.read().values().cloned().collect()
    }

    /// Get active execution IDs.
    pub fn active_executions(&self) -> Vec<Uuid> {
        self.active.read().keys().cloned().collect()
    }

    /// Get execution count.
    pub fn execution_count(&self) -> usize {
        self.records.read().len()
    }
}

impl Default for CapabilityExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{CapabilityCategory, CapabilityMetadata, CapabilityNamespace, CapabilityVersion};

    #[test]
    fn retry_config_delay() {
        let config = RetryConfig {
            max_retries: 3,
            base_delay_ms: 100,
            max_delay_ms: 10_000,
            backoff_multiplier: 2.0,
        };
        assert_eq!(config.delay_for_attempt(0), Duration::from_millis(100));
        assert_eq!(config.delay_for_attempt(1), Duration::from_millis(100));
        assert_eq!(config.delay_for_attempt(2), Duration::from_millis(200));
    }

    #[test]
    fn retry_config_none() {
        let config = RetryConfig::none();
        assert_eq!(config.max_retries, 0);
    }

    #[test]
    fn timeout_config() {
        let strict = TimeoutConfig::strict_ms(5000);
        assert!(strict.strict);
        assert_eq!(strict.timeout_ms, 5000);

        let lenient = TimeoutConfig::lenient_ms(10000);
        assert!(!lenient.strict);
    }

    #[test]
    fn execution_request_builder() {
        let req = ExecutionRequest::new(CapabilityId::new(), serde_json::json!({"x": 1}))
            .with_priority(200)
            .with_retry_config(RetryConfig::none())
            .with_timeout_config(TimeoutConfig::strict_ms(1000));

        assert_eq!(req.priority, 200);
        assert_eq!(req.retry_config.max_retries, 0);
        assert_eq!(req.timeout_config.timeout_ms, 1000);
    }

    #[test]
    fn execution_record_status() {
        let status = ExecutionStatus::Running;
        assert_eq!(format!("{}", status), "running");

        let status = ExecutionStatus::Completed;
        assert_eq!(format!("{}", status), "completed");
    }

    #[test]
    fn streaming_output() {
        let (output, mut rx) = StreamingOutput::new();
        output.send(serde_json::json!({"data": 1}));
        output.send_done(serde_json::json!({"done": true}));

        let chunk1 = rx.try_recv().unwrap();
        assert!(!chunk1.done);
        assert_eq!(chunk1.sequence, Some(0));

        let chunk2 = rx.try_recv().unwrap();
        assert!(chunk2.done);
    }

    #[test]
    fn pipeline_step_count() {
        let mut pipeline = ExecutionPipeline::new();
        assert_eq!(pipeline.step_count(), 0);
        pipeline.add_step("step1", serde_json::json!({}));
        pipeline.add_step("step2", serde_json::json!({}));
        assert_eq!(pipeline.step_count(), 2);
    }

    #[test]
    fn executor_creation() {
        let executor = CapabilityExecutor::new();
        assert_eq!(executor.execution_count(), 0);
        assert!(executor.active_executions().is_empty());
    }

    #[test]
    fn cancel_nonexistent() {
        let executor = CapabilityExecutor::new();
        assert!(!executor.cancel_execution(&Uuid::new_v4()));
    }

    #[tokio::test]
    async fn execute_capability() {
        use crate::core::Capability;

        struct TestCap;

        #[async_trait]
        impl Capability for TestCap {
            fn metadata(&self) -> &CapabilityMetadata {
                use std::sync::OnceLock;
                static META: OnceLock<CapabilityMetadata> = OnceLock::new();
                static INSTANCE: TestCap = TestCap;
                META.get_or_init(|| {
                    CapabilityMetadata::new(
                        "test-exec",
                        CapabilityVersion::initial(),
                        "test",
                        CapabilityCategory::System,
                    )
                });
                META.get_or_init(|| {
                    CapabilityMetadata::new(
                        "test-exec",
                        CapabilityVersion::initial(),
                        "test",
                        CapabilityCategory::System,
                    )
                })
            }

            fn metadata_mut(&mut self) -> &mut CapabilityMetadata {
                unimplemented!("test")
            }

            async fn execute(
                &self,
                input: serde_json::Value,
                _ctx: ExecutionContext,
            ) -> CapabilityResult<CapabilityResult_output> {
                Ok(CapabilityResult_output::success(
                    serde_json::json!({"result": "ok"}),
                    10,
                ))
            }
        }

        let executor = CapabilityExecutor::new();
        let cap = TestCap;
        let ctx = ExecutionContext::new(cap.metadata().id);
        let result = executor
            .execute_capability(&cap, ctx, serde_json::json!({}))
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(executor.execution_count(), 1);
    }

    #[tokio::test]
    async fn execute_with_retry_succeeds() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::OnceLock;

        static ATTEMPTS: OnceLock<AtomicU32> = OnceLock::new();

        struct FlippyCap;

        #[async_trait]
        impl Capability for FlippyCap {
            fn metadata(&self) -> &CapabilityMetadata {
                use std::sync::OnceLock;
                static META: OnceLock<CapabilityMetadata> = OnceLock::new();
                META.get_or_init(|| {
                    CapabilityMetadata::new(
                        "flippy",
                        CapabilityVersion::initial(),
                        "fails twice",
                        CapabilityCategory::System,
                    )
                })
            }

            fn metadata_mut(&mut self) -> &mut CapabilityMetadata {
                unimplemented!()
            }

            async fn execute(
                &self,
                _input: serde_json::Value,
                _ctx: ExecutionContext,
            ) -> CapabilityResult<CapabilityResult_output> {
                let attempts = ATTEMPTS.get_or_init(|| AtomicU32::new(0));
                let n = attempts.fetch_add(1, Ordering::SeqCst);
                if n < 2 {
                    Err(CapabilityError::execution_failed("flip"))
                } else {
                    Ok(CapabilityResult_output::success(
                        serde_json::json!({"ok": true}),
                        5,
                    ))
                }
            }
        }

        let executor = CapabilityExecutor::new();
        let cap = FlippyCap;
        let ctx = ExecutionContext::new(cap.metadata().id);
        let retry = RetryConfig {
            max_retries: 3,
            base_delay_ms: 1,
            max_delay_ms: 10,
            backoff_multiplier: 2.0,
        };

        let result = executor
            .execute_with_retry(&cap, ctx, serde_json::json!({}), retry)
            .await
            .unwrap();
        assert!(result.success);
    }
}
