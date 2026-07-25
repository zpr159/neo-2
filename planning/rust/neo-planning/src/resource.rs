//! Resource management, allocation, and budget tracking for the Neo Planning System.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::error::{PlanningError, PlanningErrorCode, PlanningResult};
use crate::id::ResourceAllocationId;
use crate::types::{ExecutionBudget, ResourceRequirements, ResourceType, SchedulingPolicy};

// ---------------------------------------------------------------------------
// ResourceAllocation
// ---------------------------------------------------------------------------

/// A single resource allocation record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    pub id: ResourceAllocationId,
    pub resource_type: ResourceType,
    pub amount: f64,
    pub allocated_to: Option<String>,
    pub plan_id: Option<uuid::Uuid>,
    pub created_at: DateTime<Utc>,
    pub released_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

// ---------------------------------------------------------------------------
// ResourceAvailability
// ---------------------------------------------------------------------------

/// Tracks the availability of a specific resource type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAvailability {
    pub resource_type: ResourceType,
    pub total: f64,
    pub available: f64,
    pub allocated: f64,
    pub pending: f64,
}

impl ResourceAvailability {
    /// Check whether `amount` can be allocated from the available pool.
    pub fn can_allocate(&self, amount: f64) -> bool {
        self.available >= amount
    }

    /// Return the utilization percentage (0.0 – 100.0).
    pub fn utilization_pct(&self) -> f64 {
        if self.total <= 0.0 {
            return 0.0;
        }
        (self.allocated / self.total) * 100.0
    }
}

// ---------------------------------------------------------------------------
// ResourcePlanner
// ---------------------------------------------------------------------------

/// Manages the global resource pool and individual allocations.
#[derive(Debug, Clone)]
pub struct ResourcePlanner {
    resources: Arc<DashMap<ResourceType, ResourceAvailability>>,
    allocations: Arc<DashMap<ResourceAllocationId, ResourceAllocation>>,
}

impl ResourcePlanner {
    /// Create an empty planner.
    pub fn new() -> Self {
        Self {
            resources: Arc::new(DashMap::new()),
            allocations: Arc::new(DashMap::new()),
        }
    }

    /// Register a resource type with a total capacity.
    pub fn register_resource(&self, resource_type: ResourceType, total: f64) {
        self.resources.insert(
            resource_type.clone(),
            ResourceAvailability {
                resource_type,
                total,
                available: total,
                allocated: 0.0,
                pending: 0.0,
            },
        );
    }

    /// Allocate `amount` of a resource type.
    pub fn allocate(
        &self,
        resource_type: ResourceType,
        amount: f64,
        allocated_to: String,
    ) -> PlanningResult<ResourceAllocation> {
        {
            let mut entry = self.resources.get_mut(&resource_type).ok_or_else(|| {
                PlanningError::new(
                    PlanningErrorCode::ResourceAllocationFailed,
                    format!("resource type '{}' not registered", resource_type),
                )
            })?;

            if !entry.can_allocate(amount) {
                return Err(PlanningError::resource_exhausted(format!(
                    "insufficient '{}' resources: need {}, available {}",
                    resource_type, amount, entry.available
                )));
            }

            entry.available -= amount;
            entry.allocated += amount;
        }

        let alloc = ResourceAllocation {
            id: ResourceAllocationId::new(),
            resource_type,
            amount,
            allocated_to: Some(allocated_to),
            plan_id: None,
            created_at: Utc::now(),
            released_at: None,
            is_active: true,
        };

        self.allocations.insert(alloc.id, alloc.clone());
        Ok(alloc)
    }

    /// Release a previously made allocation.
    pub fn release(&self, allocation_id: ResourceAllocationId) -> PlanningResult<()> {
        let alloc = self.allocations.get(&allocation_id).ok_or_else(|| {
            PlanningError::new(
                PlanningErrorCode::ResourceAllocationFailed,
                format!("allocation '{}' not found", allocation_id),
            )
        })?;

        if !alloc.is_active {
            return Err(PlanningError::new(
                PlanningErrorCode::ResourceAllocationFailed,
                format!("allocation '{}' already released", allocation_id),
            ));
        }

        let resource_type = alloc.resource_type.clone();
        let amount = alloc.amount;
        drop(alloc);

        if let Some(mut avail) = self.resources.get_mut(&resource_type) {
            avail.available += amount;
            avail.allocated -= amount;
        }

        let mut alloc_mut = self.allocations.get_mut(&allocation_id).ok_or_else(|| {
            PlanningError::new(
                PlanningErrorCode::ResourceAllocationFailed,
                "allocation disappeared during release",
            )
        })?;
        alloc_mut.is_active = false;
        alloc_mut.released_at = Some(Utc::now());

        Ok(())
    }

    /// Get the current availability for a resource type.
    pub fn availability(&self, resource_type: &ResourceType) -> Option<ResourceAvailability> {
        self.resources.get(resource_type).map(|r| r.clone())
    }

    /// Check whether the given requirements can be satisfied.
    pub fn check_feasibility(&self, requirements: &ResourceRequirements) -> bool {
        let checks: Vec<(ResourceType, f64)> = vec![
            (ResourceType::Agent, requirements.agents as f64),
            (ResourceType::Cpu, requirements.cpu_units as f64),
            (ResourceType::Memory, requirements.memory_mb as f64),
            (ResourceType::Storage, requirements.storage_mb as f64),
        ];

        for (rt, needed) in checks {
            if needed > 0.0 {
                match self.availability(&rt) {
                    Some(avail) if avail.can_allocate(needed) => {}
                    _ => return false,
                }
            }
        }

        for tool in &requirements.tool_requirements {
            let rt = ResourceType::Custom(tool.clone());
            match self.availability(&rt) {
                Some(avail) if avail.can_allocate(1.0) => {}
                _ => return false,
            }
        }

        for cap in &requirements.capability_requirements {
            let rt = ResourceType::Custom(cap.clone());
            match self.availability(&rt) {
                Some(avail) if avail.can_allocate(1.0) => {}
                _ => return false,
            }
        }

        true
    }
}

impl Default for ResourcePlanner {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// ResourceAllocator
// ---------------------------------------------------------------------------

/// Higher-level allocator that manages resource allocations per plan.
#[derive(Debug, Clone)]
pub struct ResourceAllocator {
    planner: ResourcePlanner,
    plan_allocations: Arc<DashMap<uuid::Uuid, Vec<ResourceAllocationId>>>,
}

impl ResourceAllocator {
    /// Create a new allocator wrapping a fresh planner.
    pub fn new() -> Self {
        Self {
            planner: ResourcePlanner::new(),
            plan_allocations: Arc::new(DashMap::new()),
        }
    }

    /// Allocate all resources required by `requirements` for the given plan.
    pub fn allocate_for_plan(
        &self,
        requirements: &ResourceRequirements,
        plan_id: uuid::Uuid,
    ) -> PlanningResult<Vec<ResourceAllocation>> {
        let mut allocations = Vec::new();
        let label = format!("plan-{}", plan_id);

        let resource_map: Vec<(ResourceType, f64)> = vec![
            (ResourceType::Agent, requirements.agents as f64),
            (ResourceType::Cpu, requirements.cpu_units as f64),
            (ResourceType::Memory, requirements.memory_mb as f64),
            (ResourceType::Storage, requirements.storage_mb as f64),
        ];

        for (rt, amount) in resource_map {
            if amount > 0.0 {
                let mut alloc = self.planner.allocate(rt, amount, label.clone())?;
                alloc.plan_id = Some(plan_id);
                allocations.push(alloc);
            }
        }

        for tool in &requirements.tool_requirements {
            let rt = ResourceType::Custom(tool.clone());
            let mut alloc = self.planner.allocate(rt, 1.0, label.clone())?;
            alloc.plan_id = Some(plan_id);
            allocations.push(alloc);
        }

        for cap in &requirements.capability_requirements {
            let rt = ResourceType::Custom(cap.clone());
            let mut alloc = self.planner.allocate(rt, 1.0, label.clone())?;
            alloc.plan_id = Some(plan_id);
            allocations.push(alloc);
        }

        let ids: Vec<ResourceAllocationId> = allocations.iter().map(|a| a.id).collect();
        self.plan_allocations.insert(plan_id, ids);

        Ok(allocations)
    }

    /// Release all resources previously allocated for a plan.
    pub fn release_plan_resources(&self, plan_id: uuid::Uuid) -> PlanningResult<()> {
        let ids = self
            .plan_allocations
            .remove(&plan_id)
            .ok_or_else(|| {
                PlanningError::new(
                    PlanningErrorCode::ResourceAllocationFailed,
                    format!("no allocations found for plan '{}'", plan_id),
                )
            })?
            .1;

        for id in ids {
            self.planner.release(id)?;
        }

        Ok(())
    }

    /// Merge small allocations of the same resource type.
    pub fn optimize_allocation(
        &self,
        allocations: &[ResourceAllocation],
    ) -> Vec<ResourceAllocation> {
        let mut groups: HashMap<ResourceType, Vec<&ResourceAllocation>> = HashMap::new();
        for alloc in allocations {
            groups
                .entry(alloc.resource_type.clone())
                .or_default()
                .push(alloc);
        }

        let mut result: Vec<ResourceAllocation> = Vec::new();

        for (resource_type, group) in groups {
            if group.len() <= 1 {
                result.extend(group.into_iter().cloned());
                continue;
            }

            let total: f64 = group.iter().map(|a| a.amount).sum();
            let threshold = total * 0.1;

            let mut small_amount = 0.0;
            let mut small_count = 0u32;

            for alloc in &group {
                if alloc.amount < threshold && alloc.amount > 0.0 {
                    small_amount += alloc.amount;
                    small_count += 1;
                } else {
                    result.push((*alloc).clone());
                }
            }

            if small_count > 1 {
                let representative = group
                    .iter()
                    .find(|a| a.amount < threshold && a.amount > 0.0)
                    .unwrap();
                result.push(ResourceAllocation {
                    id: ResourceAllocationId::new(),
                    resource_type,
                    amount: small_amount,
                    allocated_to: representative.allocated_to.clone(),
                    plan_id: representative.plan_id,
                    created_at: Utc::now(),
                    released_at: None,
                    is_active: true,
                });
            } else if small_count == 1 {
                for alloc in &group {
                    if alloc.amount < threshold && alloc.amount > 0.0 {
                        result.push((*alloc).clone());
                    }
                }
            }
        }

        result
    }

    /// Access the underlying planner.
    pub fn planner(&self) -> &ResourcePlanner {
        &self.planner
    }
}

impl Default for ResourceAllocator {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// ExecutionBudgetTracker
// ---------------------------------------------------------------------------

/// Tracks consumption against an execution budget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionBudgetTracker {
    budget: ExecutionBudget,
    consumed_cpu: u64,
    consumed_memory: u64,
    consumed_cost: f64,
    consumed_tokens: u64,
}

impl ExecutionBudgetTracker {
    /// Create a tracker with the given budget.
    pub fn new(budget: ExecutionBudget) -> Self {
        Self {
            budget,
            consumed_cpu: 0,
            consumed_memory: 0,
            consumed_cost: 0.0,
            consumed_tokens: 0,
        }
    }

    /// Consume CPU units, returning an error if the budget is exceeded.
    pub fn consume_cpu(&mut self, units: u64) -> PlanningResult<()> {
        let new_total = self.consumed_cpu + units;
        if new_total > self.budget.max_cpu_units as u64 {
            return Err(PlanningError::budget_exceeded(format!(
                "CPU budget exceeded: consumed {} + {} > max {}",
                self.consumed_cpu, units, self.budget.max_cpu_units
            )));
        }
        self.consumed_cpu = new_total;
        Ok(())
    }

    /// Consume memory (MB), returning an error if the budget is exceeded.
    pub fn consume_memory(&mut self, mb: u64) -> PlanningResult<()> {
        let new_total = self.consumed_memory + mb;
        if new_total > self.budget.max_memory_mb {
            return Err(PlanningError::budget_exceeded(format!(
                "memory budget exceeded: consumed {} + {} MB > max {} MB",
                self.consumed_memory, mb, self.budget.max_memory_mb
            )));
        }
        self.consumed_memory = new_total;
        Ok(())
    }

    /// Consume cost, returning an error if the budget is exceeded.
    pub fn consume_cost(&mut self, cost: f64) -> PlanningResult<()> {
        let new_total = self.consumed_cost + cost;
        if new_total > self.budget.max_cost {
            return Err(PlanningError::budget_exceeded(format!(
                "cost budget exceeded: consumed {:.2} + {:.2} > max {:.2}",
                self.consumed_cost, cost, self.budget.max_cost
            )));
        }
        self.consumed_cost = new_total;
        Ok(())
    }

    /// Consume tokens, returning an error if the budget is exceeded.
    pub fn consume_tokens(&mut self, tokens: u64) -> PlanningResult<()> {
        let new_total = self.consumed_tokens + tokens;
        if new_total > self.budget.max_token_usage {
            return Err(PlanningError::budget_exceeded(format!(
                "token budget exceeded: consumed {} + {} > max {}",
                self.consumed_tokens, tokens, self.budget.max_token_usage
            )));
        }
        self.consumed_tokens = new_total;
        Ok(())
    }

    /// Return the remaining budget for each resource.
    pub fn remaining_budget(&self) -> ExecutionBudget {
        ExecutionBudget {
            max_cpu_units: self
                .budget
                .max_cpu_units
                .saturating_sub(self.consumed_cpu as u32),
            max_memory_mb: self
                .budget
                .max_memory_mb
                .saturating_sub(self.consumed_memory),
            max_storage_mb: self.budget.max_storage_mb,
            max_cost: (self.budget.max_cost - self.consumed_cost).max(0.0),
            max_duration_secs: self.budget.max_duration_secs,
            max_tool_invocations: self.budget.max_tool_invocations,
            max_token_usage: self
                .budget
                .max_token_usage
                .saturating_sub(self.consumed_tokens),
        }
    }

    /// Return overall utilization as 0.0–1.0, based on the most-constrained resource.
    pub fn utilization(&self) -> f64 {
        let cpu = if self.budget.max_cpu_units > 0 {
            self.consumed_cpu as f64 / self.budget.max_cpu_units as f64
        } else {
            0.0
        };
        let mem = if self.budget.max_memory_mb > 0 {
            self.consumed_memory as f64 / self.budget.max_memory_mb as f64
        } else {
            0.0
        };
        let cost = if self.budget.max_cost > 0.0 {
            self.consumed_cost / self.budget.max_cost
        } else {
            0.0
        };
        let tokens = if self.budget.max_token_usage > 0 {
            self.consumed_tokens as f64 / self.budget.max_token_usage as f64
        } else {
            0.0
        };

        cpu.max(mem).max(cost).max(tokens).min(1.0)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_budget() -> ExecutionBudget {
        ExecutionBudget {
            max_cpu_units: 8,
            max_memory_mb: 4096,
            max_storage_mb: 102400,
            max_cost: 100.0,
            max_duration_secs: 3600,
            max_tool_invocations: 500,
            max_token_usage: 500_000,
        }
    }

    // ---- ResourceAvailability ----

    #[test]
    fn availability_can_allocate() {
        let avail = ResourceAvailability {
            resource_type: ResourceType::Cpu,
            total: 10.0,
            available: 5.0,
            allocated: 5.0,
            pending: 0.0,
        };
        assert!(avail.can_allocate(5.0));
        assert!(!avail.can_allocate(6.0));
    }

    #[test]
    fn availability_utilization_pct() {
        let avail = ResourceAvailability {
            resource_type: ResourceType::Memory,
            total: 100.0,
            available: 30.0,
            allocated: 70.0,
            pending: 0.0,
        };
        assert!((avail.utilization_pct() - 70.0).abs() < f64::EPSILON);
    }

    #[test]
    fn availability_utilization_pct_zero_total() {
        let avail = ResourceAvailability {
            resource_type: ResourceType::Cpu,
            total: 0.0,
            available: 0.0,
            allocated: 0.0,
            pending: 0.0,
        };
        assert!((avail.utilization_pct() - 0.0).abs() < f64::EPSILON);
    }

    // ---- ResourcePlanner ----

    #[test]
    fn planner_register_and_availability() {
        let planner = ResourcePlanner::new();
        planner.register_resource(ResourceType::Cpu, 16.0);
        let avail = planner.availability(&ResourceType::Cpu).unwrap();
        assert_eq!(avail.total, 16.0);
        assert_eq!(avail.available, 16.0);
    }

    #[test]
    fn planner_allocate_success() {
        let planner = ResourcePlanner::new();
        planner.register_resource(ResourceType::Cpu, 16.0);
        let alloc = planner
            .allocate(ResourceType::Cpu, 4.0, "worker-1".to_string())
            .unwrap();
        assert!(alloc.is_active);
        assert_eq!(alloc.amount, 4.0);
        assert_eq!(alloc.allocated_to.as_deref(), Some("worker-1"));

        let avail = planner.availability(&ResourceType::Cpu).unwrap();
        assert_eq!(avail.available, 12.0);
        assert_eq!(avail.allocated, 4.0);
    }

    #[test]
    fn planner_allocate_insufficient() {
        let planner = ResourcePlanner::new();
        planner.register_resource(ResourceType::Cpu, 4.0);
        let result = planner.allocate(ResourceType::Cpu, 5.0, "w".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn planner_allocate_unregistered() {
        let planner = ResourcePlanner::new();
        let result = planner.allocate(ResourceType::Cpu, 1.0, "w".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn planner_release() {
        let planner = ResourcePlanner::new();
        planner.register_resource(ResourceType::Memory, 1024.0);
        let alloc = planner
            .allocate(ResourceType::Memory, 256.0, "task-1".to_string())
            .unwrap();
        let avail_before = planner.availability(&ResourceType::Memory).unwrap();
        assert_eq!(avail_before.available, 768.0);

        planner.release(alloc.id).unwrap();
        let avail_after = planner.availability(&ResourceType::Memory).unwrap();
        assert_eq!(avail_after.available, 1024.0);
        assert_eq!(avail_after.allocated, 0.0);
    }

    #[test]
    fn planner_release_already_released() {
        let planner = ResourcePlanner::new();
        planner.register_resource(ResourceType::Storage, 500.0);
        let alloc = planner
            .allocate(ResourceType::Storage, 100.0, "t".to_string())
            .unwrap();
        planner.release(alloc.id).unwrap();
        let result = planner.release(alloc.id);
        assert!(result.is_err());
    }

    #[test]
    fn planner_check_feasibility() {
        let planner = ResourcePlanner::new();
        planner.register_resource(ResourceType::Agent, 10.0);
        planner.register_resource(ResourceType::Cpu, 32.0);
        planner.register_resource(ResourceType::Memory, 16384.0);
        planner.register_resource(ResourceType::Storage, 102400.0);

        let req = ResourceRequirements {
            agents: 2,
            cpu_units: 8,
            memory_mb: 4096,
            ..Default::default()
        };
        assert!(planner.check_feasibility(&req));

        let req_big = ResourceRequirements {
            agents: 20,
            ..Default::default()
        };
        assert!(!planner.check_feasibility(&req_big));
    }

    #[test]
    fn planner_check_feasibility_custom_resources() {
        let planner = ResourcePlanner::new();
        planner.register_resource(ResourceType::Custom("calculator".to_string()), 5.0);

        let req = ResourceRequirements {
            tool_requirements: vec!["calculator".to_string()],
            ..Default::default()
        };
        assert!(planner.check_feasibility(&req));

        let req_missing = ResourceRequirements {
            tool_requirements: vec!["unknown_tool".to_string()],
            ..Default::default()
        };
        assert!(!planner.check_feasibility(&req_missing));
    }

    // ---- ResourceAllocator ----

    #[test]
    fn allocator_allocate_for_plan() {
        let allocator = ResourceAllocator::new();
        allocator
            .planner
            .register_resource(ResourceType::Agent, 20.0);
        allocator.planner.register_resource(ResourceType::Cpu, 64.0);
        allocator
            .planner
            .register_resource(ResourceType::Memory, 32768.0);
        allocator
            .planner
            .register_resource(ResourceType::Storage, 204800.0);

        let req = ResourceRequirements {
            agents: 4,
            cpu_units: 16,
            memory_mb: 8192,
            ..Default::default()
        };
        let plan_id = uuid::Uuid::new_v4();
        let allocs = allocator.allocate_for_plan(&req, plan_id).unwrap();
        assert_eq!(allocs.len(), 3); // Agent, Cpu, Memory
        for a in &allocs {
            assert_eq!(a.plan_id, Some(plan_id));
            assert!(a.is_active);
        }
    }

    #[test]
    fn allocator_release_plan_resources() {
        let allocator = ResourceAllocator::new();
        allocator.planner.register_resource(ResourceType::Cpu, 32.0);
        allocator
            .planner
            .register_resource(ResourceType::Memory, 8192.0);

        let req = ResourceRequirements {
            cpu_units: 8,
            memory_mb: 2048,
            ..Default::default()
        };
        let plan_id = uuid::Uuid::new_v4();
        allocator.allocate_for_plan(&req, plan_id).unwrap();

        let avail_before = allocator.planner.availability(&ResourceType::Cpu).unwrap();
        assert_eq!(avail_before.available, 24.0);

        allocator.release_plan_resources(plan_id).unwrap();

        let avail_after = allocator.planner.availability(&ResourceType::Cpu).unwrap();
        assert_eq!(avail_after.available, 32.0);
    }

    #[test]
    fn allocator_release_nonexistent_plan() {
        let allocator = ResourceAllocator::new();
        let result = allocator.release_plan_resources(uuid::Uuid::new_v4());
        assert!(result.is_err());
    }

    #[test]
    fn allocator_optimize_merge_small() {
        let allocator = ResourceAllocator::new();

        let allocations: Vec<ResourceAllocation> = (0..10)
            .map(|i| ResourceAllocation {
                id: ResourceAllocationId::new(),
                resource_type: ResourceType::Cpu,
                amount: 0.5, // all small
                allocated_to: Some(format!("worker-{}", i)),
                plan_id: None,
                created_at: Utc::now(),
                released_at: None,
                is_active: true,
            })
            .collect();

        let optimized = allocator.optimize_allocation(&allocations);
        // 10 small allocations of 0.5 should merge into 1 allocation of 5.0
        assert_eq!(optimized.len(), 1);
        assert!((optimized[0].amount - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn allocator_optimize_keeps_large() {
        let allocator = ResourceAllocator::new();

        let mut allocations: Vec<ResourceAllocation> = Vec::new();
        // One large allocation (60% of total)
        allocations.push(ResourceAllocation {
            id: ResourceAllocationId::new(),
            resource_type: ResourceType::Memory,
            amount: 600.0,
            allocated_to: Some("big-worker".to_string()),
            plan_id: None,
            created_at: Utc::now(),
            released_at: None,
            is_active: true,
        });
        // Four small allocations (10% each)
        for i in 0..4 {
            allocations.push(ResourceAllocation {
                id: ResourceAllocationId::new(),
                resource_type: ResourceType::Memory,
                amount: 100.0,
                allocated_to: Some(format!("small-{}", i)),
                plan_id: None,
                created_at: Utc::now(),
                released_at: None,
                is_active: true,
            });
        }

        let optimized = allocator.optimize_allocation(&allocations);
        // Large one kept + 4 small merged = 2 total
        assert_eq!(optimized.len(), 2);
        let total: f64 = optimized.iter().map(|a| a.amount).sum();
        assert!((total - 1000.0).abs() < f64::EPSILON);
    }

    // ---- ExecutionBudgetTracker ----

    #[test]
    fn tracker_consume_cpu() {
        let mut tracker = ExecutionBudgetTracker::new(default_budget());
        tracker.consume_cpu(4).unwrap();
        assert_eq!(tracker.consumed_cpu, 4);
    }

    #[test]
    fn tracker_consume_cpu_exceeded() {
        let mut tracker = ExecutionBudgetTracker::new(default_budget());
        tracker.consume_cpu(7).unwrap();
        let result = tracker.consume_cpu(2);
        assert!(result.is_err());
    }

    #[test]
    fn tracker_consume_memory() {
        let mut tracker = ExecutionBudgetTracker::new(default_budget());
        tracker.consume_memory(1024).unwrap();
        assert_eq!(tracker.consumed_memory, 1024);
    }

    #[test]
    fn tracker_consume_memory_exceeded() {
        let mut tracker = ExecutionBudgetTracker::new(default_budget());
        tracker.consume_memory(3500).unwrap();
        let result = tracker.consume_memory(600);
        assert!(result.is_err());
    }

    #[test]
    fn tracker_consume_cost() {
        let mut tracker = ExecutionBudgetTracker::new(default_budget());
        tracker.consume_cost(50.0).unwrap();
        assert!((tracker.consumed_cost - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn tracker_consume_cost_exceeded() {
        let mut tracker = ExecutionBudgetTracker::new(default_budget());
        tracker.consume_cost(90.0).unwrap();
        let result = tracker.consume_cost(15.0);
        assert!(result.is_err());
    }

    #[test]
    fn tracker_consume_tokens() {
        let mut tracker = ExecutionBudgetTracker::new(default_budget());
        tracker.consume_tokens(100_000).unwrap();
        assert_eq!(tracker.consumed_tokens, 100_000);
    }

    #[test]
    fn tracker_consume_tokens_exceeded() {
        let mut tracker = ExecutionBudgetTracker::new(default_budget());
        tracker.consume_tokens(450_000).unwrap();
        let result = tracker.consume_tokens(60_000);
        assert!(result.is_err());
    }

    #[test]
    fn tracker_remaining_budget() {
        let mut tracker = ExecutionBudgetTracker::new(default_budget());
        tracker.consume_cpu(3).unwrap();
        tracker.consume_memory(1024).unwrap();
        tracker.consume_cost(25.0).unwrap();
        tracker.consume_tokens(50_000).unwrap();

        let remaining = tracker.remaining_budget();
        assert_eq!(remaining.max_cpu_units, 5);
        assert_eq!(remaining.max_memory_mb, 3072);
        assert!((remaining.max_cost - 75.0).abs() < f64::EPSILON);
        assert_eq!(remaining.max_token_usage, 450_000);
    }

    #[test]
    fn tracker_utilization() {
        let mut tracker = ExecutionBudgetTracker::new(default_budget());
        tracker.consume_cpu(4).unwrap(); // 4/8 = 0.5
        tracker.consume_memory(1024).unwrap(); // 1024/4096 = 0.25
        tracker.consume_cost(25.0).unwrap(); // 25/100 = 0.25
        tracker.consume_tokens(100_000).unwrap(); // 100k/500k = 0.2

        let util = tracker.utilization();
        assert!((util - 0.5).abs() < f64::EPSILON); // most constrained = CPU
    }

    #[test]
    fn tracker_utilization_zero_budget() {
        let budget = ExecutionBudget {
            max_cpu_units: 0,
            max_memory_mb: 0,
            max_storage_mb: 0,
            max_cost: 0.0,
            max_duration_secs: 0,
            max_tool_invocations: 0,
            max_token_usage: 0,
        };
        let tracker = ExecutionBudgetTracker::new(budget);
        assert!((tracker.utilization() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn tracker_utilization_clamped() {
        let budget = ExecutionBudget {
            max_cpu_units: 4,
            max_memory_mb: 1024,
            ..default_budget()
        };
        let mut tracker = ExecutionBudgetTracker::new(budget);
        tracker.consume_cpu(4).unwrap();
        let util = tracker.utilization();
        assert!((util - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn tracker_serialization_roundtrip() {
        let mut tracker = ExecutionBudgetTracker::new(default_budget());
        tracker.consume_cpu(2).unwrap();
        tracker.consume_cost(10.0).unwrap();

        let json = serde_json::to_string(&tracker).unwrap();
        let restored: ExecutionBudgetTracker = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.consumed_cpu, 2);
        assert!((restored.consumed_cost - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn resource_allocation_serialization_roundtrip() {
        let alloc = ResourceAllocation {
            id: ResourceAllocationId::new(),
            resource_type: ResourceType::Cpu,
            amount: 8.0,
            allocated_to: Some("worker-1".to_string()),
            plan_id: Some(uuid::Uuid::new_v4()),
            created_at: Utc::now(),
            released_at: None,
            is_active: true,
        };
        let json = serde_json::to_string(&alloc).unwrap();
        let restored: ResourceAllocation = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, alloc.id);
        assert_eq!(restored.amount, 8.0);
        assert!(restored.is_active);
    }
}
