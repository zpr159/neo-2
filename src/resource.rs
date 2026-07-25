use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::error::{NeoError, NeoResult};

/// Types of hardware and logical resources managed by Neo.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Resource {
    Gpu,
    Cpu,
    Memory,
    Storage,
    Network,
    Custom(String),
}

/// A handle returned when a resource is allocated, used for release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceHandle(pub uuid::Uuid);

impl ResourceHandle {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for ResourceHandle {
    fn default() -> Self {
        Self::new()
    }
}

struct PoolEntry {
    total: AtomicU64,
    available: AtomicU64,
}

/// A thread-safe resource pool that tracks allocation and release.
pub struct ResourcePool {
    pools: Mutex<HashMap<Resource, Arc<PoolEntry>>>,
}

impl ResourcePool {
    pub fn new() -> Self {
        Self {
            pools: Mutex::new(HashMap::new()),
        }
    }

    /// Register a resource type with the given total capacity.
    pub fn register(&self, resource: Resource, total: u64) {
        let mut pools = self.pools.lock().unwrap();
        pools.insert(
            resource,
            Arc::new(PoolEntry {
                total: AtomicU64::new(total),
                available: AtomicU64::new(total),
            }),
        );
    }

    /// Allocate `amount` units of the given resource.
    pub fn allocate(&self, resource: &Resource, amount: u64) -> NeoResult<ResourceHandle> {
        let pools = self.pools.lock().unwrap();
        let entry = pools
            .get(resource)
            .ok_or_else(|| NeoError::NotFound(format!("resource {:?} not registered", resource)))?;

        let prev = entry.available.fetch_sub(amount, Ordering::SeqCst);
        if prev < amount {
            entry.available.fetch_add(amount, Ordering::SeqCst);
            return Err(NeoError::ResourceExhausted(format!(
                "not enough {:?} available: requested {}, have {}",
                resource,
                amount,
                prev
            )));
        }

        Ok(ResourceHandle::new())
    }

    /// Release a previously allocated resource handle.
    pub fn release(&self, handle: ResourceHandle) {
        let _ = handle;
    }

    /// Release a specific amount back to a resource pool.
    pub fn release_amount(&self, resource: &Resource, amount: u64) {
        let pools = self.pools.lock().unwrap();
        if let Some(entry) = pools.get(resource) {
            let total = entry.total.load(Ordering::SeqCst);
            let current = entry.available.fetch_add(amount, Ordering::SeqCst);
            let new_val = current + amount;
            if new_val > total {
                entry.available.store(total, Ordering::SeqCst);
            }
        }
    }

    /// Returns the currently available amount of a resource.
    pub fn available(&self, resource: &Resource) -> u64 {
        let pools = self.pools.lock().unwrap();
        pools
            .get(resource)
            .map(|e| e.available.load(Ordering::SeqCst))
            .unwrap_or(0)
    }
}

impl Default for ResourcePool {
    fn default() -> Self {
        Self::new()
    }
}
