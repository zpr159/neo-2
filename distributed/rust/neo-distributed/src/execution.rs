//! Distributed execution engine — remote capability execution, workflow
//! execution, planning, inference, and multimodal processing across nodes.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::ExecutionConfiguration;
use crate::error::{DistributedError, NeoResult};
use crate::types::{NodeId, TaskPriority};

// ---------------------------------------------------------------------------
// ExecutionRequest
// ---------------------------------------------------------------------------

/// A request to execute work on a remote node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRequest {
    /// Unique request identifier.
    pub id: Uuid,
    /// Type of execution (capability, workflow, inference, etc.).
    pub execution_type: ExecutionType,
    /// Task priority.
    pub priority: TaskPriority,
    /// Required capabilities.
    pub required_capabilities: Vec<String>,
    /// Serialized payload.
    pub payload: Vec<u8>,
    /// Request timeout.
    pub timeout: Duration,
    /// Deadline.
    pub deadline: Option<DateTime<Utc>>,
    /// Whether to allow migration on failure.
    pub allow_migration: bool,
    /// Maximum retry count.
    pub max_retries: u32,
}

impl ExecutionRequest {
    /// Create a new execution request.
    pub fn new(execution_type: ExecutionType) -> Self {
        Self {
            id: Uuid::new_v4(),
            execution_type,
            priority: TaskPriority::NORMAL,
            required_capabilities: Vec::new(),
            payload: Vec::new(),
            timeout: Duration::from_secs(60),
            deadline: None,
            allow_migration: true,
            max_retries: 3,
        }
    }

    /// Set priority.
    #[must_use]
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set capabilities.
    #[must_use]
    pub fn with_capabilities(mut self, caps: Vec<String>) -> Self {
        self.required_capabilities = caps;
        self
    }

    /// Set payload.
    #[must_use]
    pub fn with_payload(mut self, payload: Vec<u8>) -> Self {
        self.payload = payload;
        self
    }

    /// Set timeout.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

// ---------------------------------------------------------------------------
// ExecutionType
// ---------------------------------------------------------------------------

/// Types of distributed execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExecutionType {
    /// Remote capability execution.
    Capability,
    /// Remote workflow execution.
    Workflow,
    /// Remote planning.
    Planning,
    /// Remote inference.
    Inference,
    /// Remote multimodal processing.
    Multimodal,
    /// Generic task.
    Task,
}

impl std::fmt::Display for ExecutionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Capability => write!(f, "capability"),
            Self::Workflow => write!(f, "workflow"),
            Self::Planning => write!(f, "planning"),
            Self::Inference => write!(f, "inference"),
            Self::Multimodal => write!(f, "multimodal"),
            Self::Task => write!(f, "task"),
        }
    }
}

// ---------------------------------------------------------------------------
// ExecutionResponse
// ---------------------------------------------------------------------------

/// Response from a remote execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResponse {
    /// Request ID this response corresponds to.
    pub request_id: Uuid,
    /// Whether the execution succeeded.
    pub success: bool,
    /// Serialized result payload.
    pub result: Vec<u8>,
    /// Error message if failed.
    pub error: Option<String>,
    /// Node that executed the request.
    pub executed_by: NodeId,
    /// Execution duration.
    pub duration: Duration,
    /// Whether the result was migrated from another node.
    pub migrated: bool,
}

// ---------------------------------------------------------------------------
// ExecutionLease
// ---------------------------------------------------------------------------

/// A lease granting exclusive execution rights for a time window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLease {
    /// Unique lease ID.
    pub id: Uuid,
    /// Task ID the lease is for.
    pub task_id: Uuid,
    /// Node that holds the lease.
    pub holder: NodeId,
    /// When the lease was created.
    pub created_at: DateTime<Utc>,
    /// When the lease expires.
    pub expires_at: DateTime<Utc>,
    /// Lease version for renewal.
    pub version: u64,
}

impl ExecutionLease {
    /// Check if the lease is still valid.
    pub fn is_valid(&self) -> bool {
        Utc::now() < self.expires_at
    }

    /// Check if the lease is expiring soon (within the given duration).
    pub fn is_expiring_soon(&self, within: Duration) -> bool {
        let remaining = self.expires_at.signed_duration_since(Utc::now());
        remaining.to_std().unwrap_or(Duration::ZERO) < within
    }
}

// ---------------------------------------------------------------------------
// ExecutionQueue
// ---------------------------------------------------------------------------

/// Queue for pending execution requests.
pub struct ExecutionQueue {
    /// Pending requests.
    queue: RwLock<VecDeque<ExecutionRequest>>,
    /// Maximum queue size.
    max_size: usize,
    /// Request counter.
    total_submitted: std::sync::atomic::AtomicU64,
    /// Total processed.
    total_processed: std::sync::atomic::AtomicU64,
}

impl ExecutionQueue {
    pub fn new(max_size: usize) -> Self {
        Self {
            queue: RwLock::new(VecDeque::new()),
            max_size,
            total_submitted: std::sync::atomic::AtomicU64::new(0),
            total_processed: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Enqueue an execution request.
    pub fn enqueue(&self, request: ExecutionRequest) -> NeoResult<()> {
        let mut queue = self.queue.write();
        if queue.len() >= self.max_size {
            return Err(DistributedError::execution("execution queue full"));
        }
        queue.push_back(request);
        self.total_submitted
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    /// Dequeue the highest-priority request.
    pub fn dequeue(&self) -> Option<ExecutionRequest> {
        let mut queue = self.queue.write();
        if queue.is_empty() {
            return None;
        }
        // Find the highest priority (lowest number).
        let min_idx = queue
            .iter()
            .enumerate()
            .min_by_key(|(_, r)| r.priority)
            .map(|(i, _)| i)?;
        let request = queue.remove(min_idx)?;
        self.total_processed
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Some(request)
    }

    /// Queue depth.
    pub fn depth(&self) -> usize {
        self.queue.read().len()
    }

    /// Total submitted.
    pub fn total_submitted(&self) -> u64 {
        self.total_submitted.load(std::sync::atomic::Ordering::Relaxed)
    }
}

// ---------------------------------------------------------------------------
// RemoteExecutionEngine
// ---------------------------------------------------------------------------

/// The main distributed execution engine.
pub struct RemoteExecutionEngine {
    /// Configuration.
    config: RwLock<ExecutionConfiguration>,
    /// Execution queue.
    queue: Arc<ExecutionQueue>,
    /// Active executions.
    active: RwLock<HashMap<Uuid, ExecutionLease>>,
    /// Completed executions.
    completed: RwLock<HashMap<Uuid, ExecutionResponse>>,
    /// Execution history.
    history: RwLock<Vec<ExecutionResponse>>,
}

impl RemoteExecutionEngine {
    /// Create a new execution engine.
    pub fn new(config: ExecutionConfiguration) -> Self {
        let queue = Arc::new(ExecutionQueue::new(config.queue_capacity));
        tracing::info!(
            max_concurrent = config.max_concurrent,
            queue_capacity = config.queue_capacity,
            "remote execution engine created"
        );
        Self {
            config: RwLock::new(config),
            queue,
            active: RwLock::new(HashMap::new()),
            completed: RwLock::new(HashMap::new()),
            history: RwLock::new(Vec::new()),
        }
    }

    /// Submit an execution request.
    pub fn submit(&self, request: ExecutionRequest) -> NeoResult<()> {
        self.queue.enqueue(request)
    }

    /// Acquire a lease for execution on a specific node.
    pub fn acquire_lease(
        &self,
        task_id: Uuid,
        node_id: NodeId,
    ) -> NeoResult<ExecutionLease> {
        let config = self.config.read();
        let lease = ExecutionLease {
            id: Uuid::new_v4(),
            task_id,
            holder: node_id,
            created_at: Utc::now(),
            expires_at: Utc::now()
                + chrono::Duration::from_std(config.lease_duration)
                    .unwrap_or(chrono::Duration::seconds(300)),
            version: 1,
        };

        self.active.write().insert(task_id, lease.clone());
        tracing::debug!(
            task_id = %task_id,
            node_id = %node_id,
            "execution lease acquired"
        );
        Ok(lease)
    }

    /// Release an execution lease.
    pub fn release_lease(&self, task_id: Uuid) -> NeoResult<()> {
        self.active.write().remove(&task_id);
        tracing::debug!(task_id = %task_id, "execution lease released");
        Ok(())
    }

    /// Record execution completion.
    pub fn complete(
        &self,
        task_id: Uuid,
        result: Vec<u8>,
        duration: Duration,
        node_id: NodeId,
    ) -> NeoResult<ExecutionResponse> {
        let response = ExecutionResponse {
            request_id: task_id,
            success: true,
            result,
            error: None,
            executed_by: node_id,
            duration,
            migrated: false,
        };

        self.completed.write().insert(task_id, response.clone());
        self.active.write().remove(&task_id);
        self.history.write().push(response.clone());

        tracing::info!(
            task_id = %task_id,
            node_id = %node_id,
            duration_ms = duration.as_millis() as u64,
            "execution completed"
        );

        Ok(response)
    }

    /// Record execution failure.
    pub fn fail(
        &self,
        task_id: Uuid,
        error: String,
        node_id: NodeId,
        duration: Duration,
    ) -> ExecutionResponse {
        let response = ExecutionResponse {
            request_id: task_id,
            success: false,
            result: Vec::new(),
            error: Some(error.clone()),
            executed_by: node_id,
            duration,
            migrated: false,
        };

        self.completed.write().insert(task_id, response.clone());
        self.active.write().remove(&task_id);
        self.history.write().push(response.clone());

        tracing::warn!(
            task_id = %task_id,
            node_id = %node_id,
            error = %error,
            "execution failed"
        );

        response
    }

    /// Check for expired leases and clean them up.
    pub fn cleanup_expired_leases(&self) -> Vec<Uuid> {
        let mut expired = Vec::new();
        let mut active = self.active.write();
        active.retain(|task_id, lease| {
            if !lease.is_valid() {
                expired.push(*task_id);
                false
            } else {
                true
            }
        });

        if !expired.is_empty() {
            tracing::warn!(count = expired.len(), "expired leases cleaned up");
        }
        expired
    }

    /// Get execution statistics.
    pub fn stats(&self) -> ExecutionStats {
        let history = self.history.read();
        let total = history.len();
        let succeeded = history.iter().filter(|r| r.success).count();
        let failed = total - succeeded;
        let avg_duration = if total > 0 {
            let total_ms: u64 = history
                .iter()
                .map(|r| r.duration.as_millis() as u64)
                .sum();
            total_ms as f64 / total as f64
        } else {
            0.0
        };

        ExecutionStats {
            total_executions: total,
            succeeded,
            failed,
            active_leases: self.active.read().len(),
            queue_depth: self.queue.depth(),
            avg_duration_ms: avg_duration,
        }
    }

    /// Get the execution queue.
    pub fn queue(&self) -> &Arc<ExecutionQueue> {
        &self.queue
    }

    /// Get pending requests from the queue.
    pub fn pending(&self) -> Vec<ExecutionRequest> {
        let queue = self.queue.queue.read();
        queue.iter().cloned().collect()
    }

    /// Get active leases.
    pub fn active_leases(&self) -> Vec<ExecutionLease> {
        self.active.read().values().cloned().collect()
    }
}

impl std::fmt::Debug for RemoteExecutionEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteExecutionEngine")
            .field("queue_depth", &self.queue.depth())
            .field("active_leases", &self.active.read().len())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// ExecutionStats
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStats {
    pub total_executions: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub active_leases: usize,
    pub queue_depth: usize,
    pub avg_duration_ms: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_creation() {
        let req = ExecutionRequest::new(ExecutionType::Inference)
            .with_priority(TaskPriority::HIGH)
            .with_timeout(Duration::from_secs(30));
        assert_eq!(req.execution_type, ExecutionType::Inference);
        assert_eq!(req.priority, TaskPriority::HIGH);
    }

    #[test]
    fn execution_queue() {
        let queue = ExecutionQueue::new(100);
        let req = ExecutionRequest::new(ExecutionType::Task);
        queue.enqueue(req).unwrap();
        assert_eq!(queue.depth(), 1);

        let dequeued = queue.dequeue().unwrap();
        assert_eq!(dequeued.execution_type, ExecutionType::Task);
        assert_eq!(queue.depth(), 0);
    }

    #[test]
    fn execution_queue_full() {
        let queue = ExecutionQueue::new(1);
        queue.enqueue(ExecutionRequest::new(ExecutionType::Task)).unwrap();
        let result = queue.enqueue(ExecutionRequest::new(ExecutionType::Task));
        assert!(result.is_err());
    }

    #[test]
    fn lease_validity() {
        let lease = ExecutionLease {
            id: Uuid::new_v4(),
            task_id: Uuid::new_v4(),
            holder: NodeId::new(),
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::seconds(60),
            version: 1,
        };
        assert!(lease.is_valid());
    }

    #[test]
    fn engine_stats() {
        let engine = RemoteExecutionEngine::new(ExecutionConfiguration::default());
        let stats = engine.stats();
        assert_eq!(stats.total_executions, 0);
        assert_eq!(stats.active_leases, 0);
    }

    #[test]
    fn engine_acquire_release_lease() {
        let engine = RemoteExecutionEngine::new(ExecutionConfiguration::default());
        let lease = engine.acquire_lease(Uuid::new_v4(), NodeId::new()).unwrap();
        assert!(lease.is_valid());
        engine.release_lease(lease.task_id).unwrap();
    }

    #[test]
    fn engine_complete_execution() {
        let engine = RemoteExecutionEngine::new(ExecutionConfiguration::default());
        let task_id = Uuid::new_v4();
        engine.acquire_lease(task_id, NodeId::new()).unwrap();
        let response = engine
            .complete(task_id, vec![1, 2, 3], Duration::from_millis(100), NodeId::new())
            .unwrap();
        assert!(response.success);
    }

    #[test]
    fn cleanup_expired_leases() {
        let engine = RemoteExecutionEngine::new(ExecutionConfiguration::default());
        let task_id = Uuid::new_v4();
        // Create a lease that's already expired.
        let lease = ExecutionLease {
            id: Uuid::new_v4(),
            task_id,
            holder: NodeId::new(),
            created_at: Utc::now() - chrono::Duration::seconds(120),
            expires_at: Utc::now() - chrono::Duration::seconds(60),
            version: 1,
        };
        engine.active.write().insert(task_id, lease);
        let expired = engine.cleanup_expired_leases();
        assert_eq!(expired.len(), 1);
    }
}
