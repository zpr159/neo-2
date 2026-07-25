use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::error::{ExecutiveError, ExecutiveResult};
use crate::context::ExecutiveContext;

/// Types of hardware and logical resources managed by the executive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    Cpu,
    Gpu,
    Ram,
    Disk,
    NetworkBandwidth,
    ModelSlot,
    InferenceBudget,
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cpu => write!(f, "cpu"),
            Self::Gpu => write!(f, "gpu"),
            Self::Ram => write!(f, "ram"),
            Self::Disk => write!(f, "disk"),
            Self::NetworkBandwidth => write!(f, "network_bandwidth"),
            Self::ModelSlot => write!(f, "model_slot"),
            Self::InferenceBudget => write!(f, "inference_budget"),
        }
    }
}

/// Resource allocation record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    pub resource_type: ResourceType,
    pub amount: u64,
    pub owner: String,
    pub allocated_at: chrono::DateTime<chrono::Utc>,
    pub ttl_secs: Option<u64>,
}

/// Resource pool status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePoolStatus {
    pub resource_type: ResourceType,
    pub total: u64,
    pub available: u64,
    pub allocated: u64,
    pub utilization: f64,
}

/// Model allocation record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelAllocation {
    pub model_id: String,
    pub gpu_count: u32,
    pub ram_mb: u64,
    pub owner: String,
    pub allocated_at: chrono::DateTime<chrono::Utc>,
}

/// Inference budget tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceBudget {
    pub total_tokens: u64,
    pub consumed_tokens: u64,
    pub reserved_tokens: u64,
    pub period: BudgetPeriod,
}

impl InferenceBudget {
    /// Create a new inference budget.
    pub fn new(total_tokens: u64, period: BudgetPeriod) -> Self {
        Self {
            total_tokens,
            consumed_tokens: 0,
            reserved_tokens: 0,
            period,
        }
    }

    /// Check if budget allows consumption.
    pub fn can_consume(&self, tokens: u64) -> bool {
        self.consumed_tokens + self.reserved_tokens + tokens <= self.total_tokens
    }

    /// Get remaining budget.
    pub fn remaining(&self) -> u64 {
        self.total_tokens
            .saturating_sub(self.consumed_tokens)
            .saturating_sub(self.reserved_tokens)
    }
}

/// Budget period.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BudgetPeriod {
    PerSecond,
    PerMinute,
    PerHour,
    PerDay,
    Unlimited,
}

/// Resource coordinator manages CPU, GPU, RAM, model allocation, and inference budget for the executive system.
#[derive(Clone)]
pub struct ResourceCoordinator {
    inner: Arc<ResourceCoordinatorInner>,
}

struct ResourceCoordinatorInner {
    pools: RwLock<HashMap<ResourceType, ResourcePool>>,
    allocations: RwLock<Vec<ResourceAllocation>>,
    model_allocations: RwLock<Vec<ModelAllocation>>,
    inference_budget: RwLock<InferenceBudget>,
}

struct ResourcePool {
    total: u64,
    available: u64,
}

impl ResourceCoordinator {
    /// Create a new resource coordinator.
    pub fn new() -> Self {
        let mut pools = HashMap::new();
        pools.insert(ResourceType::Cpu, ResourcePool { total: 8, available: 8 });
        pools.insert(ResourceType::Gpu, ResourcePool { total: 4, available: 4 });
        pools.insert(ResourceType::Ram, ResourcePool { total: 32768, available: 32768 });
        pools.insert(ResourceType::Disk, ResourcePool { total: 1024000, available: 1024000 });
        pools.insert(ResourceType::NetworkBandwidth, ResourcePool { total: 1000, available: 1000 });
        pools.insert(ResourceType::ModelSlot, ResourcePool { total: 4, available: 4 });
        pools.insert(ResourceType::InferenceBudget, ResourcePool { total: 1000000, available: 1000000 });

        Self {
            inner: Arc::new(ResourceCoordinatorInner {
                pools: RwLock::new(pools),
                allocations: RwLock::new(Vec::new()),
                model_allocations: RwLock::new(Vec::new()),
                inference_budget: RwLock::new(InferenceBudget::new(1000000, BudgetPeriod::PerDay)),
            }),
        }
    }

    /// Allocate resources.
    pub fn allocate(
        &self,
        resource_type: ResourceType,
        amount: u64,
        owner: String,
    ) -> ExecutiveResult<ResourceAllocation> {
        let mut pools = self.inner.pools.write();
        let pool = pools
            .get_mut(&resource_type)
            .ok_or_else(|| ExecutiveError::new(
                crate::error::ExecutiveErrorCode::ResourceAllocationFailed,
                format!("resource pool '{}' not found", resource_type),
            ))?;

        if pool.available < amount {
            return Err(ExecutiveError::new(
                crate::error::ExecutiveErrorCode::ResourceExhausted,
                format!(
                    "insufficient {}: requested {}, available {}",
                    resource_type, amount, pool.available
                ),
            ));
        }

        pool.available -= amount;

        let allocation = ResourceAllocation {
            resource_type,
            amount,
            owner,
            allocated_at: chrono::Utc::now(),
            ttl_secs: None,
        };

        self.inner.allocations.write().push(allocation.clone());
        Ok(allocation)
    }

    /// Release resources.
    pub fn release(&self, allocation: &ResourceAllocation) -> ExecutiveResult<()> {
        let mut pools = self.inner.pools.write();
        if let Some(pool) = pools.get_mut(&allocation.resource_type) {
            pool.available += allocation.amount;
        }

        self.inner.allocations.write().retain(|a| {
            a.resource_type != allocation.resource_type
                || a.owner != allocation.owner
                || a.amount != allocation.amount
        });

        Ok(())
    }

    /// Get available amount for a resource.
    pub fn available(&self, resource_type: ResourceType) -> u64 {
        self.inner
            .pools
            .read()
            .get(&resource_type)
            .map_or(0, |p| p.available)
    }

    /// Get total capacity for a resource.
    pub fn total(&self, resource_type: ResourceType) -> u64 {
        self.inner
            .pools
            .read()
            .get(&resource_type)
            .map_or(0, |p| p.total)
    }

    /// Get utilization for a resource.
    pub fn utilization(&self, resource_type: ResourceType) -> f64 {
        let pools = self.inner.pools.read();
        pools.get(&resource_type).map_or(0.0, |p| {
            if p.total == 0 {
                0.0
            } else {
                (p.total - p.available) as f64 / p.total as f64
            }
        })
    }

    /// Check if resource requirements can be satisfied.
    pub fn can_satisfy(&self, requirements: &HashMap<ResourceType, u64>) -> bool {
        let pools = self.inner.pools.read();
        requirements.iter().all(|(&rtype, &amount)| {
            pools.get(&rtype).map_or(false, |p| p.available >= amount)
        })
    }

    /// Get all pool statuses.
    pub fn pool_statuses(&self) -> Vec<ResourcePoolStatus> {
        self.inner
            .pools
            .read()
            .iter()
            .map(|(&rtype, pool)| ResourcePoolStatus {
                resource_type: rtype,
                total: pool.total,
                available: pool.available,
                allocated: pool.total - pool.available,
                utilization: if pool.total == 0 {
                    0.0
                } else {
                    (pool.total - pool.available) as f64 / pool.total as f64
                },
            })
            .collect()
    }

    /// Get all active allocations.
    pub fn active_allocations(&self) -> Vec<ResourceAllocation> {
        self.inner.allocations.read().clone()
    }

    /// Allocate a model slot.
    pub fn allocate_model(
        &self,
        model_id: String,
        gpu_count: u32,
        ram_mb: u64,
        owner: String,
    ) -> ExecutiveResult<ModelAllocation> {
        let gpu_result = self.allocate(ResourceType::Gpu, gpu_count as u64, owner.clone());
        if gpu_result.is_err() {
            return Err(ExecutiveError::new(
                crate::error::ExecutiveErrorCode::ModelAllocationFailed,
                "insufficient GPU resources",
            ));
        }

        let ram_result = self.allocate(ResourceType::Ram, ram_mb, owner.clone());
        if ram_result.is_err() {
            self.release(&gpu_result?).ok();
            return Err(ExecutiveError::new(
                crate::error::ExecutiveErrorCode::ModelAllocationFailed,
                "insufficient RAM for model",
            ));
        }

        let slot_result = self.allocate(ResourceType::ModelSlot, 1, owner.clone());
        if slot_result.is_err() {
            self.release(&gpu_result?).ok();
            self.release(&ram_result?).ok();
            return Err(ExecutiveError::new(
                crate::error::ExecutiveErrorCode::ModelAllocationFailed,
                "no model slots available",
            ));
        }

        let allocation = ModelAllocation {
            model_id,
            gpu_count,
            ram_mb,
            owner,
            allocated_at: chrono::Utc::now(),
        };

        self.inner.model_allocations.write().push(allocation.clone());
        Ok(allocation)
    }

    /// Release a model allocation.
    pub fn release_model(&self, allocation: &ModelAllocation) -> ExecutiveResult<()> {
        self.inner
            .model_allocations
            .write()
            .retain(|a| a.model_id != allocation.model_id || a.owner != allocation.owner);
        Ok(())
    }

    /// Get active model allocations.
    pub fn model_allocations(&self) -> Vec<ModelAllocation> {
        self.inner.model_allocations.read().clone()
    }

    /// Get the inference budget.
    pub fn inference_budget(&self) -> InferenceBudget {
        self.inner.inference_budget.read().clone()
    }

    /// Consume inference budget.
    pub fn consume_inference_budget(&self, tokens: u64) -> ExecutiveResult<()> {
        let mut budget = self.inner.inference_budget.write();
        if !budget.can_consume(tokens) {
            return Err(ExecutiveError::new(
                crate::error::ExecutiveErrorCode::InferenceBudgetExceeded,
                format!(
                    "inference budget exceeded: requested {} tokens, remaining {}",
                    tokens,
                    budget.remaining()
                ),
            ));
        }
        budget.consumed_tokens += tokens;
        Ok(())
    }

    /// Reset inference budget.
    pub fn reset_inference_budget(&self, total_tokens: u64) {
        let mut budget = self.inner.inference_budget.write();
        budget.total_tokens = total_tokens;
        budget.consumed_tokens = 0;
        budget.reserved_tokens = 0;
    }

    /// Get total active resource allocation count.
    pub fn allocation_count(&self) -> usize {
        self.inner.allocations.read().len()
    }
}

impl Default for ResourceCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_allocation() {
        let coordinator = ResourceCoordinator::new();
        let alloc = coordinator
            .allocate(ResourceType::Cpu, 2, "test".to_string())
            .unwrap();
        assert_eq!(alloc.amount, 2);
        assert_eq!(coordinator.available(ResourceType::Cpu), 6);
    }

    #[test]
    fn resource_exhaustion() {
        let coordinator = ResourceCoordinator::new();
        let result = coordinator.allocate(ResourceType::Cpu, 100, "test".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn resource_release() {
        let coordinator = ResourceCoordinator::new();
        let alloc = coordinator
            .allocate(ResourceType::Ram, 1024, "test".to_string())
            .unwrap();
        coordinator.release(&alloc).unwrap();
        assert_eq!(coordinator.available(ResourceType::Ram), 32768);
    }

    #[test]
    fn can_satisfy() {
        let coordinator = ResourceCoordinator::new();
        let mut reqs = HashMap::new();
        reqs.insert(ResourceType::Cpu, 4);
        reqs.insert(ResourceType::Ram, 1024);
        assert!(coordinator.can_satisfy(&reqs));

        let mut reqs2 = HashMap::new();
        reqs2.insert(ResourceType::Cpu, 100);
        assert!(!coordinator.can_satisfy(&reqs2));
    }

    #[test]
    fn model_allocation() {
        let coordinator = ResourceCoordinator::new();
        let alloc = coordinator
            .allocate_model("llama-7b".to_string(), 1, 4096, "inference".to_string())
            .unwrap();
        assert_eq!(alloc.gpu_count, 1);
        assert_eq!(coordinator.model_allocations().len(), 1);
    }

    #[test]
    fn inference_budget() {
        let coordinator = ResourceCoordinator::new();
        coordinator.consume_inference_budget(500).unwrap();
        let budget = coordinator.inference_budget();
        assert_eq!(budget.consumed_tokens, 500);
    }

    #[test]
    fn inference_budget_exceeded() {
        let coordinator = ResourceCoordinator::new();
        let result = coordinator.consume_inference_budget(2000000);
        assert!(result.is_err());
    }

    #[test]
    fn pool_statuses() {
        let coordinator = ResourceCoordinator::new();
        let statuses = coordinator.pool_statuses();
        assert_eq!(statuses.len(), 7);
    }
}
