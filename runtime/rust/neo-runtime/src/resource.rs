//! Resource manager for CPU, GPU, RAM, disk, network, thread pools, memory pools,
//! quotas, and monitoring.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{ResourceError, ResourceErrorKind};

/// Unique identifier for a resource consumer (service, plugin, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConsumerId(pub Uuid);

impl ConsumerId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ConsumerId {
    fn default() -> Self {
        Self::new()
    }
}

/// Types of hardware and logical resources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    Cpu,
    Gpu,
    Ram,
    Disk,
    Network,
    ThreadPool,
    MemoryPool,
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cpu => write!(f, "cpu"),
            Self::Gpu => write!(f, "gpu"),
            Self::Ram => write!(f, "ram"),
            Self::Disk => write!(f, "disk"),
            Self::Network => write!(f, "network"),
            Self::ThreadPool => write!(f, "thread_pool"),
            Self::MemoryPool => write!(f, "memory_pool"),
        }
    }
}

/// A handle returned on allocation, used for release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceHandle {
    pub id: Uuid,
    pub resource_type: ResourceType,
    pub amount: u64,
    pub consumer: ConsumerId,
}

impl ResourceHandle {
    pub fn new(resource_type: ResourceType, amount: u64, consumer: ConsumerId) -> Self {
        Self {
            id: Uuid::new_v4(),
            resource_type,
            amount,
            consumer,
        }
    }
}

/// Resource quota for a consumer.
#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceQuota {
    pub consumer: ConsumerId,
    pub limits: HashMap<ResourceType, u64>,
    pub usage: HashMap<ResourceType, AtomicU64>,
}

impl Clone for ResourceQuota {
    fn clone(&self) -> Self {
        let usage = self
            .usage
            .iter()
            .map(|(&k, v)| (k, AtomicU64::new(v.load(Ordering::Relaxed))))
            .collect();
        Self {
            consumer: self.consumer,
            limits: self.limits.clone(),
            usage,
        }
    }
}

impl ResourceQuota {
    pub fn new(consumer: ConsumerId, limits: HashMap<ResourceType, u64>) -> Self {
        let usage = limits.keys().map(|&k| (k, AtomicU64::new(0))).collect();
        Self {
            consumer,
            limits,
            usage,
        }
    }

    /// Check whether allocating `amount` of the given resource would exceed the quota.
    pub fn would_exceed(&self, resource: ResourceType, amount: u64) -> bool {
        if let Some(limit) = self.limits.get(&resource) {
            let current = self
                .usage
                .get(&resource)
                .map_or(0, |u| u.load(Ordering::Relaxed));
            current + amount > *limit
        } else {
            false
        }
    }

    /// Record usage of a resource.
    pub fn record_usage(&self, resource: ResourceType, amount: u64) {
        if let Some(usage) = self.usage.get(&resource) {
            usage.fetch_add(amount, Ordering::Relaxed);
        }
    }

    /// Release usage of a resource.
    pub fn release_usage(&self, resource: ResourceType, amount: u64) {
        if let Some(usage) = self.usage.get(&resource) {
            let current = usage.load(Ordering::Relaxed);
            let new_val = current.saturating_sub(amount);
            usage.store(new_val, Ordering::Relaxed);
        }
    }

    /// Get current usage of a resource.
    pub fn current_usage(&self, resource: ResourceType) -> u64 {
        self.usage
            .get(&resource)
            .map_or(0, |u| u.load(Ordering::Relaxed))
    }

    /// Get the limit for a resource.
    pub fn limit(&self, resource: ResourceType) -> u64 {
        self.limits.get(&resource).copied().unwrap_or(0)
    }
}

/// Atomic counters for a single resource pool.
struct ResourcePoolEntry {
    total: AtomicU64,
    available: AtomicU64,
    allocated: AtomicU64,
}

impl ResourcePoolEntry {
    fn new(total: u64) -> Self {
        Self {
            total: AtomicU64::new(total),
            available: AtomicU64::new(total),
            allocated: AtomicU64::new(0),
        }
    }
}

/// Statistics for a resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceStats {
    pub total: u64,
    pub available: u64,
    pub allocated: u64,
    pub utilization: f64,
}

/// Thread-safe resource manager.
pub struct ResourceManager {
    pools: DashMap<ResourceType, Arc<ResourcePoolEntry>>,
    quotas: DashMap<ConsumerId, Arc<ResourceQuota>>,
    handles: DashMap<Uuid, ResourceHandle>,
}

impl ResourceManager {
    /// Create a new resource manager with default resource pools.
    pub fn new() -> Self {
        Self {
            pools: DashMap::new(),
            quotas: DashMap::new(),
            handles: DashMap::new(),
        }
    }

    /// Register a resource pool with the given total capacity.
    pub fn register_pool(&self, resource: ResourceType, total: u64) {
        self.pools
            .insert(resource, Arc::new(ResourcePoolEntry::new(total)));
    }

    /// Allocate resources for a consumer.
    pub fn allocate(
        &self,
        resource: ResourceType,
        amount: u64,
        consumer: ConsumerId,
    ) -> Result<ResourceHandle, ResourceError> {
        if amount == 0 {
            return Err(ResourceError::new(
                ResourceErrorKind::AllocationFailed,
                "allocation amount must be > 0",
            ));
        }

        if let Some(quota) = self.quotas.get(&consumer) {
            if quota.would_exceed(resource, amount) {
                return Err(ResourceError::new(
                    ResourceErrorKind::QuotaExceeded,
                    format!(
                        "consumer {} would exceed quota for {}",
                        consumer.0, resource
                    ),
                ));
            }
        }

        let pool = self
            .pools
            .get(&resource)
            .ok_or_else(|| ResourceError::new(
                ResourceErrorKind::NotFound,
                format!("resource pool '{}' not registered", resource),
            ))?;

        let prev = pool.available.fetch_sub(amount, Ordering::SeqCst);
        if prev < amount {
            pool.available.fetch_add(amount, Ordering::SeqCst);
            return Err(ResourceError::new(
                ResourceErrorKind::Exhausted,
                format!(
                    "not enough {} available: requested {}, have {}",
                    resource, amount, prev
                ),
            ));
        }

        pool.allocated.fetch_add(amount, Ordering::SeqCst);

        if let Some(quota) = self.quotas.get(&consumer) {
            quota.record_usage(resource, amount);
        }

        let handle = ResourceHandle::new(resource, amount, consumer);
        self.handles.insert(handle.id, handle);

        Ok(handle)
    }

    /// Release a previously allocated resource handle.
    pub fn release(&self, handle: ResourceHandle) -> Result<(), ResourceError> {
        self.handles.remove(&handle.id);

        if let Some(pool) = self.pools.get(&handle.resource_type) {
            pool.available.fetch_add(handle.amount, Ordering::SeqCst);
            pool.allocated.fetch_sub(handle.amount, Ordering::SeqCst);
        }

        if let Some(quota) = self.quotas.get(&handle.consumer) {
            quota.release_usage(handle.resource_type, handle.amount);
        }

        Ok(())
    }

    /// Register a consumer quota.
    pub fn register_quota(&self, quota: ResourceQuota) {
        self.quotas.insert(quota.consumer, Arc::new(quota));
    }

    /// Get statistics for a resource.
    pub fn stats(&self, resource: ResourceType) -> Option<ResourceStats> {
        self.pools.get(&resource).map(|pool| {
            let total = pool.total.load(Ordering::Relaxed);
            let available = pool.available.load(Ordering::Relaxed);
            let allocated = pool.allocated.load(Ordering::Relaxed);
            let utilization = if total > 0 {
                allocated as f64 / total as f64
            } else {
                0.0
            };
            ResourceStats {
                total,
                available,
                allocated,
                utilization,
            }
        })
    }

    /// Get available amount for a resource.
    pub fn available(&self, resource: ResourceType) -> u64 {
        self.pools
            .get(&resource)
            .map_or(0, |p| p.available.load(Ordering::Relaxed))
    }

    /// Get total capacity for a resource.
    pub fn total(&self, resource: ResourceType) -> u64 {
        self.pools
            .get(&resource)
            .map_or(0, |p| p.total.load(Ordering::Relaxed))
    }

    /// Get all resource statistics.
    pub fn all_stats(&self) -> Vec<(ResourceType, ResourceStats)> {
        self.pools
            .iter()
            .map(|entry| {
                let resource = *entry.key();
                let pool = entry.value();
                let total = pool.total.load(Ordering::Relaxed);
                let available = pool.available.load(Ordering::Relaxed);
                let allocated = pool.allocated.load(Ordering::Relaxed);
                let utilization = if total > 0 {
                    allocated as f64 / total as f64
                } else {
                    0.0
                };
                (
                    resource,
                    ResourceStats {
                        total,
                        available,
                        allocated,
                        utilization,
                    },
                )
            })
            .collect()
    }

    /// Get the number of active handles.
    pub fn active_handle_count(&self) -> usize {
        self.handles.len()
    }
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory pool that manages pre-allocated memory blocks.
pub struct MemoryPool {
    block_size: usize,
    total_blocks: usize,
    available: parking_lot::Mutex<Vec<usize>>,
    allocated: DashMap<usize, usize>,
}

impl MemoryPool {
    /// Create a new memory pool with the given block size and count.
    pub fn new(block_size: usize, total_blocks: usize) -> Self {
        let available: Vec<usize> = (0..total_blocks).collect();
        Self {
            block_size,
            total_blocks,
            available: parking_lot::Mutex::new(available),
            allocated: DashMap::new(),
        }
    }

    /// Allocate a memory block. Returns the block index.
    pub fn allocate(&self) -> Result<usize, ResourceError> {
        let mut avail = self.available.lock();
        let block_id = avail.pop().ok_or_else(|| {
            ResourceError::new(
                ResourceErrorKind::Exhausted,
                "memory pool exhausted",
            )
        })?;
        drop(avail);
        self.allocated.insert(block_id, self.block_size);
        Ok(block_id)
    }

    /// Release a previously allocated memory block.
    pub fn release(&self, block_id: usize) -> Result<(), ResourceError> {
        self.allocated
            .remove(&block_id)
            .ok_or_else(|| ResourceError::new(
                ResourceErrorKind::DeallocationFailed,
                format!("block {} not allocated", block_id),
            ))?;
        self.available.lock().push(block_id);
        Ok(())
    }

    /// Get the number of available blocks.
    pub fn available_blocks(&self) -> usize {
        self.available.lock().len()
    }

    /// Get the number of allocated blocks.
    pub fn allocated_blocks(&self) -> usize {
        self.allocated.len()
    }

    /// Get the total number of blocks.
    pub fn total_blocks(&self) -> usize {
        self.total_blocks
    }

    /// Get the block size in bytes.
    pub fn block_size(&self) -> usize {
        self.block_size
    }

    /// Get utilization as a ratio.
    pub fn utilization(&self) -> f64 {
        if self.total_blocks == 0 {
            return 0.0;
        }
        self.allocated_blocks() as f64 / self.total_blocks as f64
    }
}

/// Monitor that periodically samples resource usage.
pub struct ResourceMonitor {
    manager: ResourceManager,
    sampling_interval: Duration,
}

impl ResourceMonitor {
    pub fn new(manager: ResourceManager, sampling_interval: Duration) -> Self {
        Self {
            manager,
            sampling_interval,
        }
    }

    /// Get the current snapshot of all resource stats.
    pub fn snapshot(&self) -> Vec<(ResourceType, ResourceStats)> {
        self.manager.all_stats()
    }

    /// Get the sampling interval.
    pub fn sampling_interval(&self) -> Duration {
        self.sampling_interval
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_allocate() {
        let mgr = ResourceManager::new();
        mgr.register_pool(ResourceType::Cpu, 8);
        let consumer = ConsumerId::new();
        let handle = mgr.allocate(ResourceType::Cpu, 2, consumer).unwrap();
        assert_eq!(handle.amount, 2);
        assert_eq!(mgr.available(ResourceType::Cpu), 6);
    }

    #[test]
    fn allocate_exceeds_available() {
        let mgr = ResourceManager::new();
        mgr.register_pool(ResourceType::Ram, 100);
        let consumer = ConsumerId::new();
        let result = mgr.allocate(ResourceType::Ram, 200, consumer);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err().kind, ResourceErrorKind::Exhausted));
    }

    #[test]
    fn release_restores_available() {
        let mgr = ResourceManager::new();
        mgr.register_pool(ResourceType::Cpu, 8);
        let consumer = ConsumerId::new();
        let handle = mgr.allocate(ResourceType::Cpu, 4, consumer).unwrap();
        assert_eq!(mgr.available(ResourceType::Cpu), 4);

        mgr.release(handle).unwrap();
        assert_eq!(mgr.available(ResourceType::Cpu), 8);
    }

    #[test]
    fn quota_enforcement() {
        let mgr = ResourceManager::new();
        mgr.register_pool(ResourceType::Cpu, 16);

        let consumer = ConsumerId::new();
        let mut limits = HashMap::new();
        limits.insert(ResourceType::Cpu, 4);
        let quota = ResourceQuota::new(consumer, limits);
        mgr.register_quota(quota);

        let h1 = mgr.allocate(ResourceType::Cpu, 3, consumer).unwrap();
        assert_eq!(mgr.available(ResourceType::Cpu), 13);

        let result = mgr.allocate(ResourceType::Cpu, 2, consumer);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err().kind, ResourceErrorKind::QuotaExceeded));

        mgr.release(h1).unwrap();
        let h2 = mgr.allocate(ResourceType::Cpu, 2, consumer).unwrap();
        mgr.release(h2).unwrap();
    }

    #[test]
    fn zero_amount_rejected() {
        let mgr = ResourceManager::new();
        mgr.register_pool(ResourceType::Cpu, 8);
        let consumer = ConsumerId::new();
        let result = mgr.allocate(ResourceType::Cpu, 0, consumer);
        assert!(result.is_err());
    }

    #[test]
    fn unregistered_resource() {
        let mgr = ResourceManager::new();
        let consumer = ConsumerId::new();
        let result = mgr.allocate(ResourceType::Gpu, 1, consumer);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err().kind, ResourceErrorKind::NotFound));
    }

    #[test]
    fn stats_tracking() {
        let mgr = ResourceManager::new();
        mgr.register_pool(ResourceType::Ram, 1024);
        let consumer = ConsumerId::new();
        let h = mgr.allocate(ResourceType::Ram, 256, consumer).unwrap();

        let stats = mgr.stats(ResourceType::Ram).unwrap();
        assert_eq!(stats.total, 1024);
        assert_eq!(stats.allocated, 256);
        assert_eq!(stats.available, 768);
        assert!((stats.utilization - 0.25).abs() < f64::EPSILON);

        mgr.release(h).unwrap();
        let stats = mgr.stats(ResourceType::Ram).unwrap();
        assert_eq!(stats.allocated, 0);
    }

    #[test]
    fn all_stats() {
        let mgr = ResourceManager::new();
        mgr.register_pool(ResourceType::Cpu, 8);
        mgr.register_pool(ResourceType::Ram, 1024);
        let all = mgr.all_stats();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn memory_pool() {
        let pool = MemoryPool::new(1024, 4);
        assert_eq!(pool.available_blocks(), 4);
        assert_eq!(pool.allocated_blocks(), 0);

        let b0 = pool.allocate().unwrap();
        let b1 = pool.allocate().unwrap();
        assert_eq!(pool.available_blocks(), 2);
        assert_eq!(pool.allocated_blocks(), 2);

        pool.release(b0).unwrap();
        assert_eq!(pool.available_blocks(), 3);

        let _b2 = pool.allocate().unwrap();
        let _b3 = pool.allocate().unwrap();
        assert!(pool.allocate().is_err());
    }

    #[test]
    fn resource_monitor_snapshot() {
        let mgr = ResourceManager::new();
        mgr.register_pool(ResourceType::Cpu, 8);
        let monitor = ResourceMonitor::new(mgr, Duration::from_secs(1));
        let snap = monitor.snapshot();
        assert_eq!(snap.len(), 1);
    }
}
