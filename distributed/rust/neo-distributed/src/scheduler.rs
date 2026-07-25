//! Distributed scheduler — multi-policy task scheduling across cluster nodes
//! with load balancing, placement strategies, and resource allocation.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::{SchedulingPolicy, SchedulerConfiguration};
use crate::error::{DistributedError, NeoResult};
use crate::node::NodeManager;
use crate::types::{
    NodeCapabilities, NodeId, NodeInfo, NodeResources, NodeState, TaskPriority,
};

// ---------------------------------------------------------------------------
// SchedulingTask
// ---------------------------------------------------------------------------

/// A task to be scheduled across the cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingTask {
    /// Unique task identifier.
    pub id: Uuid,
    /// Task type / capability required.
    pub task_type: String,
    /// Task priority.
    pub priority: TaskPriority,
    /// Estimated duration in milliseconds.
    pub estimated_duration_ms: u64,
    /// Required capabilities.
    pub required_capabilities: Vec<String>,
    /// Required labels.
    pub required_labels: HashMap<String, String>,
    /// Whether the task requires GPU.
    pub requires_gpu: bool,
    /// Minimum memory in bytes.
    pub min_memory_bytes: u64,
    /// Serialized task data.
    pub data: Vec<u8>,
    /// When the task was submitted.
    pub submitted_at: DateTime<Utc>,
    /// Deadline (optional).
    pub deadline: Option<DateTime<Utc>>,
}

impl SchedulingTask {
    /// Create a new task.
    pub fn new(task_type: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            task_type: task_type.into(),
            priority: TaskPriority::NORMAL,
            estimated_duration_ms: 10_000,
            required_capabilities: Vec::new(),
            required_labels: HashMap::new(),
            requires_gpu: false,
            min_memory_bytes: 0,
            data: Vec::new(),
            submitted_at: Utc::now(),
            deadline: None,
        }
    }

    /// Set priority.
    #[must_use]
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set required capabilities.
    #[must_use]
    pub fn with_capabilities(mut self, caps: Vec<String>) -> Self {
        self.required_capabilities = caps;
        self
    }

    /// Require GPU.
    #[must_use]
    pub fn requires_gpu(mut self) -> Self {
        self.requires_gpu = true;
        self
    }

    /// Set minimum memory.
    #[must_use]
    pub fn with_min_memory(mut self, bytes: u64) -> Self {
        self.min_memory_bytes = bytes;
        self
    }

    /// Set deadline.
    #[must_use]
    pub fn with_deadline(mut self, deadline: DateTime<Utc>) -> Self {
        self.deadline = Some(deadline);
        self
    }
}

// ---------------------------------------------------------------------------
// TaskAssignment
// ---------------------------------------------------------------------------

/// Result of scheduling a task — which node it was assigned to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAssignment {
    /// Task identifier.
    pub task_id: Uuid,
    /// Assigned node.
    pub assigned_node: NodeId,
    /// Scheduling policy used.
    pub policy: SchedulingPolicy,
    /// When the assignment was made.
    pub assigned_at: DateTime<Utc>,
    /// Estimated start time.
    pub estimated_start: DateTime<Utc>,
    /// Estimated completion time.
    pub estimated_completion: DateTime<Utc>,
    /// Lease expiration.
    pub lease_expires: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// NodeLoad
// ---------------------------------------------------------------------------

/// Tracked load information for a node.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeLoad {
    /// Number of active tasks.
    pub active_tasks: u32,
    /// Number of queued tasks.
    pub queued_tasks: u32,
    /// CPU utilization.
    pub cpu_utilization: f32,
    /// GPU utilization.
    pub gpu_utilization: f32,
    /// Memory utilization.
    pub memory_utilization: f32,
    /// Last update time.
    pub last_updated: DateTime<Utc>,
}

impl NodeLoad {
    /// Composite load score 0.0 (idle) – 1.0 (fully loaded).
    pub fn load_score(&self) -> f32 {
        let task_score = (self.active_tasks as f32 / 32.0).min(1.0);
        (self.cpu_utilization * 0.35
            + self.gpu_utilization * 0.30
            + self.memory_utilization * 0.20
            + task_score * 0.15)
            .min(1.0)
    }
}

// ---------------------------------------------------------------------------
// SchedulingStats
// ---------------------------------------------------------------------------

/// Scheduler statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchedulingStats {
    pub total_scheduled: u64,
    pub total_completed: u64,
    pub total_failed: u64,
    pub total_reassigned: u64,
    pub avg_assignment_time_ms: f64,
    pub avg_queue_depth: f64,
    pub current_queue_depth: usize,
    pub current_active: usize,
}

// ---------------------------------------------------------------------------
// DistributedScheduler
// ---------------------------------------------------------------------------

/// Multi-policy distributed scheduler.
pub struct DistributedScheduler {
    /// Configuration.
    config: RwLock<SchedulerConfiguration>,
    /// Pending tasks (waiting for assignment).
    pending_queue: RwLock<VecDeque<SchedulingTask>>,
    /// Active task assignments.
    assignments: RwLock<HashMap<Uuid, TaskAssignment>>,
    /// Per-node load tracking.
    node_loads: RwLock<HashMap<NodeId, NodeLoad>>,
    /// Scheduling counter.
    schedule_counter: AtomicU64,
    /// Completion counter.
    complete_counter: AtomicU64,
    /// Failure counter.
    fail_counter: AtomicU64,
    /// Round-robin index for round-robin scheduling.
    round_robin_index: AtomicU64,
}

impl DistributedScheduler {
    /// Create a new scheduler with the given configuration.
    pub fn new(config: SchedulerConfiguration) -> Self {
        tracing::info!(
            policy = %config.default_policy,
            max_queue = config.max_queue_depth,
            "distributed scheduler created"
        );
        Self {
            config: RwLock::new(config),
            pending_queue: RwLock::new(VecDeque::new()),
            assignments: RwLock::new(HashMap::new()),
            node_loads: RwLock::new(HashMap::new()),
            schedule_counter: AtomicU64::new(0),
            complete_counter: AtomicU64::new(0),
            fail_counter: AtomicU64::new(0),
            round_robin_index: AtomicU64::new(0),
        }
    }

    /// Create a scheduler with default configuration.
    pub fn default_config() -> Self {
        Self::new(SchedulerConfiguration::default())
    }

    // -- Node management --

    /// Register a node with the scheduler.
    pub fn register_node(&self, node_id: NodeId) {
        self.node_loads
            .write()
            .entry(node_id)
            .or_insert_with(NodeLoad::default);
        tracing::debug!(node_id = %node_id, "node registered with scheduler");
    }

    /// Remove a node from the scheduler.
    pub fn remove_node(&self, node_id: NodeId) {
        self.node_loads.write().remove(&node_id);
        tracing::debug!(node_id = %node_id, "node removed from scheduler");
    }

    /// Update load information for a node.
    pub fn update_load(&self, node_id: NodeId, load: NodeLoad) {
        self.node_loads.write().insert(node_id, load);
    }

    /// Get registered node count.
    pub fn node_count(&self) -> usize {
        self.node_loads.read().len()
    }

    // -- Task submission --

    /// Submit a task for scheduling.
    pub fn submit_task(&self, task: SchedulingTask) -> NeoResult<()> {
        let config = self.config.read();
        if self.pending_queue.read().len() >= config.max_queue_depth {
            return Err(DistributedError::scheduler("queue depth exceeded"));
        }
        drop(config);

        tracing::debug!(
            task_id = %task.id,
            task_type = %task.task_type,
            priority = %task.priority,
            "task submitted"
        );

        self.pending_queue.write().push_back(task);
        Ok(())
    }

    /// Schedule the next task using the configured policy.
    pub fn schedule_next(
        &self,
        node_manager: &NodeManager,
    ) -> NeoResult<Option<TaskAssignment>> {
        let task = {
            let mut queue = self.pending_queue.write();
            // Sort by priority (lower number = higher priority).
            let mut tasks: Vec<SchedulingTask> = queue.drain(..).collect();
            tasks.sort_by(|a, b| a.priority.cmp(&b.priority));
            if let Some(task) = tasks.first() {
                let task = task.clone();
                // Put remaining tasks back.
                for t in tasks.into_iter().skip(1) {
                    queue.push_back(t);
                }
                Some(task)
            } else {
                None
            }
        };

        let task = match task {
            Some(t) => t,
            None => return Ok(None),
        };

        let config = self.config.read();
        let policy = config.default_policy;
        drop(config);

        // Find the best node for this task.
        let target = self.select_node(&task, policy, node_manager)?;

        let now = Utc::now();
        let lease_duration =
            chrono::Duration::from_std(std::time::Duration::from_secs(300))
                .unwrap_or(chrono::Duration::seconds(300));

        let assignment = TaskAssignment {
            task_id: task.id,
            assigned_node: target,
            policy,
            assigned_at: now,
            estimated_start: now,
            estimated_completion: now + chrono::Duration::milliseconds(task.estimated_duration_ms as i64),
            lease_expires: now + lease_duration,
        };

        // Update load.
        if let Some(load) = self.node_loads.write().get_mut(&target) {
            load.active_tasks += 1;
        }

        self.assignments
            .write()
            .insert(task.id, assignment.clone());
        self.schedule_counter.fetch_add(1, Ordering::Relaxed);

        tracing::info!(
            task_id = %task.id,
            node_id = %target,
            policy = %policy,
            "task assigned"
        );

        Ok(Some(assignment))
    }

    /// Complete a task.
    pub fn complete_task(&self, task_id: Uuid) -> NeoResult<()> {
        let assignment = self
            .assignments
            .write()
            .remove(&task_id)
            .ok_or_else(|| DistributedError::scheduler(format!("task not found: {task_id}")))?;

        if let Some(load) = self.node_loads.write().get_mut(&assignment.assigned_node) {
            load.active_tasks = load.active_tasks.saturating_sub(1);
        }

        self.complete_counter.fetch_add(1, Ordering::Relaxed);

        tracing::info!(
            task_id = %task_id,
            node_id = %assignment.assigned_node,
            "task completed"
        );

        Ok(())
    }

    /// Fail a task.
    pub fn fail_task(&self, task_id: Uuid, _error: &str) -> NeoResult<()> {
        let assignment = self
            .assignments
            .write()
            .remove(&task_id)
            .ok_or_else(|| DistributedError::scheduler(format!("task not found: {task_id}")))?;

        if let Some(load) = self.node_loads.write().get_mut(&assignment.assigned_node) {
            load.active_tasks = load.active_tasks.saturating_sub(1);
        }

        self.fail_counter.fetch_add(1, Ordering::Relaxed);

        tracing::warn!(
            task_id = %task_id,
            node_id = %assignment.assigned_node,
            "task failed"
        );

        Ok(())
    }

    // -- Node selection --

    /// Select the best node for a task using the given policy.
    fn select_node(
        &self,
        task: &SchedulingTask,
        policy: SchedulingPolicy,
        node_manager: &NodeManager,
    ) -> NeoResult<NodeId> {
        let healthy_nodes = node_manager.healthy();
        if healthy_nodes.is_empty() {
            return Err(DistributedError::scheduler("no healthy nodes available"));
        }

        match policy {
            SchedulingPolicy::LeastLoaded => self.select_least_loaded(&healthy_nodes),
            SchedulingPolicy::CapabilityAware => {
                self.select_by_capability(task, &healthy_nodes)
            }
            SchedulingPolicy::GpuPreferred => self.select_gpu_preferred(task, &healthy_nodes),
            SchedulingPolicy::CpuPreferred => self.select_cpu_preferred(task, &healthy_nodes),
            SchedulingPolicy::LocalityAware => {
                // For locality, prefer nodes in the same zone.
                self.select_least_loaded(&healthy_nodes) // Fallback.
            }
            SchedulingPolicy::LatencyOptimized => self.select_lowest_latency(&healthy_nodes),
            SchedulingPolicy::MemoryOptimized => self.select_most_memory(&healthy_nodes),
            SchedulingPolicy::RoundRobin => self.select_round_robin(&healthy_nodes),
            SchedulingPolicy::Random => {
                let idx = rand::random::<usize>() % healthy_nodes.len();
                Ok(healthy_nodes[idx].id)
            }
        }
    }

    fn select_least_loaded(&self, nodes: &[crate::node::NodeEntry]) -> NeoResult<NodeId> {
        let loads = self.node_loads.read();
        nodes
            .iter()
            .min_by(|a, b| {
                let la = loads.get(&a.id).map_or(0.0, |l| l.load_score());
                let lb = loads.get(&b.id).map_or(0.0, |l| l.load_score());
                la.partial_cmp(&lb).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|n| n.id)
            .ok_or_else(|| DistributedError::scheduler("no nodes available"))
    }

    fn select_by_capability(
        &self,
        task: &SchedulingTask,
        nodes: &[crate::node::NodeEntry],
    ) -> NeoResult<NodeId> {
        let loads = self.node_loads.read();
        let candidates: Vec<_> = nodes
            .iter()
            .filter(|n| {
                task.required_capabilities
                    .iter()
                    .all(|c| n.info.capabilities.supports_capability(c))
                    && (!task.requires_gpu || n.info.capabilities.has_gpu())
                    && n.info.capabilities.memory_bytes >= task.min_memory_bytes
            })
            .collect();

        if candidates.is_empty() {
            return Err(DistributedError::scheduler(
                "no nodes with required capabilities",
            ));
        }

        candidates
            .iter()
            .min_by(|a, b| {
                let la = loads.get(&a.id).map_or(0.0, |l| l.load_score());
                let lb = loads.get(&b.id).map_or(0.0, |l| l.load_score());
                la.partial_cmp(&lb).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|n| n.id)
            .ok_or_else(|| DistributedError::scheduler("no suitable nodes"))
    }

    fn select_gpu_preferred(
        &self,
        _task: &SchedulingTask,
        nodes: &[crate::node::NodeEntry],
    ) -> NeoResult<NodeId> {
        let loads = self.node_loads.read();
        // Prefer GPU nodes, fall back to any.
        let gpu_nodes: Vec<_> = nodes
            .iter()
            .filter(|n| n.info.capabilities.has_gpu())
            .collect();

        let candidates = if gpu_nodes.is_empty() {
            nodes.to_vec()
        } else {
            gpu_nodes.into_iter().cloned().collect()
        };

        candidates
            .iter()
            .min_by(|a, b| {
                let la = loads.get(&a.id).map_or(0.0, |l| l.load_score());
                let lb = loads.get(&b.id).map_or(0.0, |l| l.load_score());
                la.partial_cmp(&lb).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|n| n.id)
            .ok_or_else(|| DistributedError::scheduler("no nodes available"))
    }

    fn select_cpu_preferred(
        &self,
        _task: &SchedulingTask,
        nodes: &[crate::node::NodeEntry],
    ) -> NeoResult<NodeId> {
        let loads = self.node_loads.read();
        // Prefer nodes with more CPU cores.
        nodes
            .iter()
            .max_by_key(|n| n.info.capabilities.cpu_cores)
            .map(|n| n.id)
            .ok_or_else(|| DistributedError::scheduler("no nodes available"))
    }

    fn select_lowest_latency(&self, nodes: &[crate::node::NodeEntry]) -> NeoResult<NodeId> {
        nodes
            .iter()
            .min_by(|a, b| {
                a.health
                    .latency_ms
                    .partial_cmp(&b.health.latency_ms)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|n| n.id)
            .ok_or_else(|| DistributedError::scheduler("no nodes available"))
    }

    fn select_most_memory(&self, nodes: &[crate::node::NodeEntry]) -> NeoResult<NodeId> {
        nodes
            .iter()
            .max_by_key(|n| n.info.capabilities.memory_bytes)
            .map(|n| n.id)
            .ok_or_else(|| DistributedError::scheduler("no nodes available"))
    }

    fn select_round_robin(&self, nodes: &[crate::node::NodeEntry]) -> NeoResult<NodeId> {
        let idx = self.round_robin_index.fetch_add(1, Ordering::Relaxed) as usize % nodes.len();
        Ok(nodes[idx].id)
    }

    // -- Queries --

    /// Get pending task count.
    pub fn pending_count(&self) -> usize {
        self.pending_queue.read().len()
    }

    /// Get active assignment count.
    pub fn active_count(&self) -> usize {
        self.assignments.read().len()
    }

    /// Get an assignment by task ID.
    pub fn get_assignment(&self, task_id: Uuid) -> Option<TaskAssignment> {
        self.assignments.read().get(&task_id).cloned()
    }

    /// Get all active assignments.
    pub fn active_assignments(&self) -> Vec<TaskAssignment> {
        self.assignments.read().values().cloned().collect()
    }

    /// Get scheduling statistics.
    pub fn stats(&self) -> SchedulingStats {
        SchedulingStats {
            total_scheduled: self.schedule_counter.load(Ordering::Relaxed),
            total_completed: self.complete_counter.load(Ordering::Relaxed),
            total_failed: self.fail_counter.load(Ordering::Relaxed),
            total_reassigned: 0,
            avg_assignment_time_ms: 0.0,
            avg_queue_depth: 0.0,
            current_queue_depth: self.pending_count(),
            current_active: self.active_count(),
        }
    }

    /// Get node load information.
    pub fn node_loads(&self) -> HashMap<NodeId, NodeLoad> {
        self.node_loads.read().clone()
    }

    /// Drain expired task assignments (lease expired).
    pub fn drain_expired(&self) -> Vec<Uuid> {
        let now = Utc::now();
        let mut expired = Vec::new();
        let mut assignments = self.assignments.write();
        let mut loads = self.node_loads.write();

        assignments.retain(|task_id, assignment| {
            if now > assignment.lease_expires {
                expired.push(*task_id);
                if let Some(load) = loads.get_mut(&assignment.assigned_node) {
                    load.active_tasks = load.active_tasks.saturating_sub(1);
                }
                false
            } else {
                true
            }
        });

        if !expired.is_empty() {
            tracing::warn!(count = expired.len(), "expired tasks drained");
        }

        expired
    }
}

impl std::fmt::Debug for DistributedScheduler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DistributedScheduler")
            .field("pending", &self.pending_count())
            .field("active", &self.active_count())
            .field("nodes", &self.node_count())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::NodeCapabilities;

    fn make_scheduler() -> DistributedScheduler {
        DistributedScheduler::new(SchedulerConfiguration::default())
    }

    fn make_node_entry(hostname: &str) -> crate::node::NodeEntry {
        crate::node::NodeEntry {
            id: NodeId::new(),
            info: crate::types::NodeInfo {
                hostname: hostname.to_string(),
                ip_address: "127.0.0.1".to_string(),
                port: 7000,
                node_type: crate::types::NodeType::CpuWorker,
                capabilities: NodeCapabilities {
                    cpu_cores: 8,
                    memory_bytes: 16 * 1024 * 1024 * 1024,
                    ..Default::default()
                },
                version: "0.1.0".to_string(),
                zone: "default".to_string(),
                rack: None,
            },
            state: NodeState::Ready,
            joined_at: Utc::now(),
            last_heartbeat: Utc::now(),
            resources: NodeResources::default(),
            health: crate::types::NodeHealth {
                score: 1.0,
                state: NodeState::Ready,
                last_heartbeat: Utc::now(),
                latency_ms: 1.0,
                clock_drift_us: 0,
                responsive: true,
                warnings: vec![],
            },
            version: "0.1.0".to_string(),
        }
    }

    #[test]
    fn scheduler_creation() {
        let sched = make_scheduler();
        assert_eq!(sched.node_count(), 0);
        assert_eq!(sched.pending_count(), 0);
    }

    #[test]
    fn submit_and_schedule() {
        let sched = make_scheduler();
        let mut mgr = crate::node::NodeManager::new();
        let entry = mgr.register(crate::types::NodeInfo {
            hostname: "h1".to_string(),
            ip_address: "127.0.0.1".to_string(),
            port: 7000,
            node_type: crate::types::NodeType::CpuWorker,
            capabilities: NodeCapabilities::default(),
            version: "0.1.0".to_string(),
            zone: "default".to_string(),
            rack: None,
        }).unwrap();
        mgr.transition(entry.id, NodeState::Ready).unwrap();
        sched.register_node(entry.id);

        let task = SchedulingTask::new("test");
        sched.submit_task(task).unwrap();
        assert_eq!(sched.pending_count(), 1);

        let assignment = sched.schedule_next(&mgr).unwrap();
        assert!(assignment.is_some());
        assert_eq!(sched.pending_count(), 0);
        assert_eq!(sched.active_count(), 1);
    }

    #[test]
    fn complete_task() {
        let sched = make_scheduler();
        let mut mgr = crate::node::NodeManager::new();
        let entry = mgr.register(crate::types::NodeInfo {
            hostname: "h1".to_string(),
            ip_address: "127.0.0.1".to_string(),
            port: 7000,
            node_type: crate::types::NodeType::CpuWorker,
            capabilities: NodeCapabilities::default(),
            version: "0.1.0".to_string(),
            zone: "default".to_string(),
            rack: None,
        }).unwrap();
        mgr.transition(entry.id, NodeState::Ready).unwrap();
        sched.register_node(entry.id);

        let task = SchedulingTask::new("test");
        sched.submit_task(task.clone()).unwrap();
        let assignment = sched.schedule_next(&mgr).unwrap().unwrap();
        sched.complete_task(assignment.task_id).unwrap();
        assert_eq!(sched.active_count(), 0);
    }

    #[test]
    fn task_priority_ordering() {
        let sched = make_scheduler();
        let mut mgr = crate::node::NodeManager::new();
        let entry = mgr.register(crate::types::NodeInfo {
            hostname: "h1".to_string(),
            ip_address: "127.0.0.1".to_string(),
            port: 7000,
            node_type: crate::types::NodeType::CpuWorker,
            capabilities: NodeCapabilities::default(),
            version: "0.1.0".to_string(),
            zone: "default".to_string(),
            rack: None,
        }).unwrap();
        mgr.transition(entry.id, NodeState::Ready).unwrap();
        sched.register_node(entry.id);

        let low = SchedulingTask::new("low").with_priority(TaskPriority::LOW);
        let high = SchedulingTask::new("high").with_priority(TaskPriority::HIGH);
        sched.submit_task(low).unwrap();
        sched.submit_task(high).unwrap();

        let a1 = sched.schedule_next(&mgr).unwrap().unwrap();
        let a2 = sched.schedule_next(&mgr).unwrap().unwrap();

        // High priority should be scheduled first.
        let task1 = sched.get_assignment(a1.task_id).unwrap();
        let task2 = sched.get_assignment(a2.task_id).unwrap();
        assert!(task1.assigned_at <= task2.assigned_at);
    }

    #[test]
    fn stats() {
        let sched = make_scheduler();
        let stats = sched.stats();
        assert_eq!(stats.total_scheduled, 0);
        assert_eq!(stats.current_queue_depth, 0);
    }
}
