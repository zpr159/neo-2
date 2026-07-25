use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkerState {
    Connected,
    Busy,
    Disconnected,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteWorker {
    pub id: Uuid,
    pub addr: String,
    pub port: u16,
    pub state: WorkerState,
    pub capabilities: Vec<String>,
    pub gpu_count: u32,
    pub memory_bytes: u64,
    pub cpu_cores: u32,
    pub connected_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub average_latency_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedConfig {
    pub worker_timeout_secs: u64,
    pub heartbeat_interval_secs: u64,
    pub max_retries: usize,
    pub retry_delay_ms: u64,
    pub enable_fault_tolerance: bool,
    pub auto_discover_workers: bool,
    pub discovery_addr: String,
    pub max_workers: usize,
    pub load_balance_strategy: LoadBalanceStrategy,
}

impl Default for DistributedConfig {
    fn default() -> Self {
        Self {
            worker_timeout_secs: 30,
            heartbeat_interval_secs: 10,
            max_retries: 3,
            retry_delay_ms: 1000,
            enable_fault_tolerance: true,
            auto_discover_workers: false,
            discovery_addr: "0.0.0.0:7946".to_string(),
            max_workers: 256,
            load_balance_strategy: LoadBalanceStrategy::LeastLoaded,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LoadBalanceStrategy {
    RoundRobin,
    LeastLoaded,
    LeastLatency,
    WeightedRandom,
    ConsistentHash,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterNode {
    pub id: Uuid,
    pub addr: String,
    pub port: u16,
    pub role: NodeRole,
    pub state: WorkerState,
    pub metadata: HashMap<String, serde_json::Value>,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeRole {
    Leader,
    Follower,
    Worker,
}

pub struct DistributedInferenceManager {
    config: DistributedConfig,
    workers: parking_lot::RwLock<HashMap<Uuid, RemoteWorker>>,
    cluster_nodes: parking_lot::RwLock<HashMap<Uuid, ClusterNode>>,
    round_robin_index: AtomicU64,
    is_leader: AtomicBool,
    local_node_id: Uuid,
}

impl std::fmt::Debug for DistributedInferenceManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DistributedInferenceManager")
            .field("worker_count", &self.worker_count())
            .field("is_leader", &self.is_leader.load(Ordering::Relaxed))
            .finish()
    }
}

impl DistributedInferenceManager {
    pub fn new(config: DistributedConfig) -> Self {
        Self {
            config,
            workers: parking_lot::RwLock::new(HashMap::new()),
            cluster_nodes: parking_lot::RwLock::new(HashMap::new()),
            round_robin_index: AtomicU64::new(0),
            is_leader: AtomicBool::new(false),
            local_node_id: Uuid::new_v4(),
        }
    }

    pub fn register_worker(&self, worker: RemoteWorker) {
        self.workers.write().insert(worker.id, worker);
    }

    pub fn unregister_worker(&self, worker_id: Uuid) {
        self.workers.write().remove(&worker_id);
    }

    pub fn get_worker(&self, worker_id: Uuid) -> Option<RemoteWorker> {
        self.workers.read().get(&worker_id).cloned()
    }

    #[must_use]
    pub fn worker_count(&self) -> usize {
        self.workers.read().len()
    }

    #[must_use]
    pub fn active_worker_count(&self) -> usize {
        self.workers
            .read()
            .values()
            .filter(|w| w.state == WorkerState::Connected || w.state == WorkerState::Busy)
            .count()
    }

    pub fn select_worker(&self) -> Option<RemoteWorker> {
        let workers = self.workers.read();
        let active: Vec<&RemoteWorker> = workers
            .values()
            .filter(|w| w.state == WorkerState::Connected)
            .collect();
        if active.is_empty() {
            return None;
        }
        match self.config.load_balance_strategy {
            LoadBalanceStrategy::RoundRobin => {
                let idx = self.round_robin_index.fetch_add(1, Ordering::Relaxed) as usize % active.len();
                Some(active[idx].clone())
            }
            LoadBalanceStrategy::LeastLoaded => {
                active
                    .iter()
                    .min_by_key(|w| w.tasks_completed)
                    .map(|w| (*w).clone())
            }
            LoadBalanceStrategy::LeastLatency => {
                active
                    .iter()
                    .min_by(|a, b| a.average_latency_ms.partial_cmp(&b.average_latency_ms).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|w| (*w).clone())
            }
            LoadBalanceStrategy::WeightedRandom => {
                let idx = self.round_robin_index.fetch_add(1, Ordering::Relaxed) as usize % active.len();
                Some(active[idx].clone())
            }
            LoadBalanceStrategy::ConsistentHash => {
                let idx = self.round_robin_index.fetch_add(1, Ordering::Relaxed) as usize % active.len();
                Some(active[idx].clone())
            }
        }
    }

    pub fn add_cluster_node(&self, node: ClusterNode) {
        self.cluster_nodes.write().insert(node.id, node);
    }

    pub fn remove_cluster_node(&self, node_id: Uuid) {
        self.cluster_nodes.write().remove(&node_id);
    }

    #[must_use]
    pub fn is_leader(&self) -> bool {
        self.is_leader.load(Ordering::Relaxed)
    }

    pub fn set_leader(&self, is_leader: bool) {
        self.is_leader.store(is_leader, Ordering::SeqCst);
    }

    #[must_use]
    pub fn local_node_id(&self) -> Uuid {
        self.local_node_id
    }

    pub fn retry_with_fallback<F, T>(&self, mut op: F) -> Option<T>
    where
        F: FnMut() -> Option<T>,
    {
        if !self.config.enable_fault_tolerance {
            return op();
        }
        for attempt in 0..=self.config.max_retries {
            if let Some(result) = op() {
                return Some(result);
            }
            if attempt < self.config.max_retries {
                std::thread::sleep(std::time::Duration::from_millis(
                    self.config.retry_delay_ms,
                ));
            }
        }
        None
    }
}
