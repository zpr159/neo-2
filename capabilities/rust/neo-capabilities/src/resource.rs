use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::CapabilityId;

// ---------------------------------------------------------------------------
// ResourceError / ResourceResult
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum ResourceError {
    InsufficientBudget {
        resource_type: ResourceType,
        requested: u64,
        available: u64,
    },
    BudgetNotFound(ResourceType),
    QuotaExceeded {
        capability_id: CapabilityId,
    },
}

impl fmt::Display for ResourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InsufficientBudget {
                resource_type,
                requested,
                available,
            } => write!(
                f,
                "{}: requested {} but only {} available",
                resource_type, requested, available
            ),
            Self::BudgetNotFound(rt) => write!(f, "no budget registered for {}", rt),
            Self::QuotaExceeded { capability_id } => {
                write!(f, "capability {} exceeded execution quota", capability_id)
            }
        }
    }
}

impl std::error::Error for ResourceError {}

pub type ResourceResult<T> = Result<T, ResourceError>;

// ---------------------------------------------------------------------------
// ResourceType
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    Cpu,
    Gpu,
    Memory,
    InferenceTokens,
    Disk,
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cpu => write!(f, "cpu"),
            Self::Gpu => write!(f, "gpu"),
            Self::Memory => write!(f, "memory"),
            Self::InferenceTokens => write!(f, "inference_tokens"),
            Self::Disk => write!(f, "disk"),
        }
    }
}

impl Default for ResourceType {
    fn default() -> Self {
        Self::Cpu
    }
}

impl ResourceType {
    pub const ALL: &'static [ResourceType] = &[
        ResourceType::Cpu,
        ResourceType::Gpu,
        ResourceType::Memory,
        ResourceType::InferenceTokens,
        ResourceType::Disk,
    ];
}

// ---------------------------------------------------------------------------
// ResourceRequirements
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub cpu_units: f64,
    pub gpu_units: f64,
    pub memory_bytes: u64,
    pub inference_tokens: u32,
    pub disk_bytes: u64,
}

impl ResourceRequirements {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn minimal() -> Self {
        Self {
            cpu_units: 0.01,
            gpu_units: 0.0,
            memory_bytes: 1024 * 1024,
            inference_tokens: 0,
            disk_bytes: 0,
        }
    }

    pub fn moderate() -> Self {
        Self {
            cpu_units: 0.25,
            gpu_units: 0.0,
            memory_bytes: 256 * 1024 * 1024,
            inference_tokens: 1024,
            disk_bytes: 1024 * 1024,
        }
    }

    pub fn heavy() -> Self {
        Self {
            cpu_units: 0.5,
            gpu_units: 0.5,
            memory_bytes: 1024 * 1024 * 1024,
            inference_tokens: 8192,
            disk_bytes: 100 * 1024 * 1024,
        }
    }
}

// ---------------------------------------------------------------------------
// ResourceBudget
// ---------------------------------------------------------------------------

pub struct ResourceBudget {
    pub resource_type: ResourceType,
    total: u64,
    allocated: AtomicU64,
    reserved: AtomicU64,
}

impl ResourceBudget {
    pub fn new(resource_type: ResourceType, total: u64) -> Self {
        Self {
            resource_type,
            total,
            allocated: AtomicU64::new(0),
            reserved: AtomicU64::new(0),
        }
    }

    pub fn total(&self) -> u64 {
        self.total
    }

    pub fn allocated(&self) -> u64 {
        self.allocated.load(Ordering::SeqCst)
    }

    pub fn reserved(&self) -> u64 {
        self.reserved.load(Ordering::SeqCst)
    }

    pub fn available(&self) -> u64 {
        let alloc = self.allocated.load(Ordering::SeqCst);
        if alloc >= self.total {
            0
        } else {
            self.total - alloc
        }
    }

    pub fn allocate(&self, amount: u64) -> ResourceResult<()> {
        let prev = self.allocated.fetch_add(amount, Ordering::SeqCst);
        if prev + amount > self.total {
            self.allocated.fetch_sub(amount, Ordering::SeqCst);
            return Err(ResourceError::InsufficientBudget {
                resource_type: self.resource_type,
                requested: amount,
                available: self.total - prev,
            });
        }
        Ok(())
    }

    pub fn release(&self, amount: u64) {
        let prev = self.allocated.load(Ordering::SeqCst);
        let to_release = amount.min(prev);
        if to_release > 0 {
            self.allocated.fetch_sub(to_release, Ordering::SeqCst);
        }
    }

    pub fn set_total(&mut self, total: u64) {
        self.total = total;
    }

    pub fn utilization_percentage(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        (self.allocated.load(Ordering::SeqCst) as f64 / self.total as f64) * 100.0
    }
}

// ---------------------------------------------------------------------------
// ExecutionQuota
// ---------------------------------------------------------------------------

pub struct ExecutionQuota {
    pub capability_id: CapabilityId,
    pub max_executions_per_hour: u32,
    pub max_executions_per_day: u32,
    pub max_concurrent: u32,
    current_hour_count: AtomicU32,
    current_day_count: AtomicU32,
    last_reset: RwLock<DateTime<Utc>>,
    active_count: AtomicU32,
}

impl ExecutionQuota {
    pub fn new(
        capability_id: CapabilityId,
        max_executions_per_hour: u32,
        max_executions_per_day: u32,
        max_concurrent: u32,
    ) -> Self {
        Self {
            capability_id,
            max_executions_per_hour,
            max_executions_per_day,
            max_concurrent,
            current_hour_count: AtomicU32::new(0),
            current_day_count: AtomicU32::new(0),
            last_reset: RwLock::new(Utc::now()),
            active_count: AtomicU32::new(0),
        }
    }

    pub fn can_execute(&self) -> bool {
        self.reset_if_needed();
        let hour_ok =
            self.current_hour_count.load(Ordering::SeqCst) < self.max_executions_per_hour;
        let day_ok =
            self.current_day_count.load(Ordering::SeqCst) < self.max_executions_per_day;
        let conc_ok = self.active_count.load(Ordering::SeqCst) < self.max_concurrent;
        hour_ok && day_ok && conc_ok
    }

    pub fn record_execution(&self) -> ResourceResult<()> {
        if !self.can_execute() {
            return Err(ResourceError::QuotaExceeded {
                capability_id: self.capability_id,
            });
        }
        self.current_hour_count.fetch_add(1, Ordering::SeqCst);
        self.current_day_count.fetch_add(1, Ordering::SeqCst);
        self.active_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    pub fn release_execution(&self) {
        let prev = self.active_count.load(Ordering::SeqCst);
        if prev > 0 {
            self.active_count.fetch_sub(1, Ordering::SeqCst);
        }
    }

    pub fn reset_if_needed(&self) {
        let now = Utc::now();
        let mut last = self.last_reset.write();
        let elapsed = now - *last;

        if elapsed.num_hours() >= 1 {
            self.current_hour_count.store(0, Ordering::SeqCst);
            *last = now;
        }
        if elapsed.num_days() >= 1 {
            self.current_day_count.store(0, Ordering::SeqCst);
        }
    }

    pub fn current_usage(&self) -> QuotaUsage {
        self.reset_if_needed();
        QuotaUsage {
            hour_count: self.current_hour_count.load(Ordering::SeqCst),
            day_count: self.current_day_count.load(Ordering::SeqCst),
            active_count: self.active_count.load(Ordering::SeqCst),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaUsage {
    pub hour_count: u32,
    pub day_count: u32,
    pub active_count: u32,
}

// ---------------------------------------------------------------------------
// ResourcePool
// ---------------------------------------------------------------------------

pub struct ResourcePool {
    pub(crate) budgets: RwLock<HashMap<ResourceType, ResourceBudget>>,
    pub(crate) quotas: RwLock<HashMap<CapabilityId, ExecutionQuota>>,
}

impl ResourcePool {
    pub fn new() -> Self {
        Self {
            budgets: RwLock::new(HashMap::new()),
            quotas: RwLock::new(HashMap::new()),
        }
    }

    pub fn register_budget(&self, budget: ResourceBudget) {
        let mut budgets = self.budgets.write();
        budgets.insert(budget.resource_type, budget);
    }

    pub fn allocate(&self, resource_type: ResourceType, amount: u64) -> ResourceResult<()> {
        let budgets = self.budgets.read();
        let budget = budgets
            .get(&resource_type)
            .ok_or(ResourceError::BudgetNotFound(resource_type))?;
        budget.allocate(amount)
    }

    pub fn release(&self, resource_type: ResourceType, amount: u64) {
        let budgets = self.budgets.read();
        if let Some(budget) = budgets.get(&resource_type) {
            budget.release(amount);
        }
    }

    pub fn get_budget(&self, resource_type: ResourceType) -> Option<ResourceBudgetInfo> {
        let budgets = self.budgets.read();
        budgets.get(&resource_type).map(|b| ResourceBudgetInfo {
            resource_type: b.resource_type,
            total: b.total,
            allocated: b.allocated(),
            available: b.available(),
            utilization: b.utilization_percentage(),
        })
    }

    pub fn check_quota(&self, capability_id: &CapabilityId) -> bool {
        let quotas = self.quotas.read();
        match quotas.get(capability_id) {
            Some(quota) => quota.can_execute(),
            None => true,
        }
    }

    pub fn record_quota_usage(&self, capability_id: &CapabilityId) -> ResourceResult<()> {
        let quotas = self.quotas.read();
        match quotas.get(capability_id) {
            Some(quota) => quota.record_execution(),
            None => Ok(()),
        }
    }

    pub fn release_quota_usage(&self, capability_id: &CapabilityId) {
        let quotas = self.quotas.read();
        if let Some(quota) = quotas.get(capability_id) {
            quota.release_execution();
        }
    }

    pub fn register_quota(&self, quota: ExecutionQuota) {
        let mut quotas = self.quotas.write();
        quotas.insert(quota.capability_id, quota);
    }

    pub fn total_utilization(&self) -> HashMap<ResourceType, f64> {
        let budgets = self.budgets.read();
        budgets
            .iter()
            .map(|(rt, b)| (*rt, b.utilization_percentage()))
            .collect()
    }

    pub fn budget_status(&self) -> ResourcePoolStatus {
        let budgets = self.budgets.read();
        let mut status_budgets: Vec<BudgetStatus> = budgets
            .values()
            .map(|b| BudgetStatus {
                resource_type: b.resource_type,
                total: b.total,
                allocated: b.allocated(),
                available: b.available(),
                utilization: b.utilization_percentage(),
            })
            .collect();
        status_budgets.sort_by_key(|s| format!("{}", s.resource_type));

        let find = |rt: ResourceType| -> f64 {
            budgets
                .get(&rt)
                .map(|b| b.utilization_percentage())
                .unwrap_or(0.0)
        };

        ResourcePoolStatus {
            budgets: status_budgets,
            total_cpu_utilization: find(ResourceType::Cpu),
            total_gpu_utilization: find(ResourceType::Gpu),
            total_memory_utilization: find(ResourceType::Memory),
        }
    }
}

impl Default for ResourcePool {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// ResourcePoolStatus / BudgetStatus
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePoolStatus {
    pub budgets: Vec<BudgetStatus>,
    pub total_cpu_utilization: f64,
    pub total_gpu_utilization: f64,
    pub total_memory_utilization: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetStatus {
    pub resource_type: ResourceType,
    pub total: u64,
    pub allocated: u64,
    pub available: u64,
    pub utilization: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceBudgetInfo {
    pub resource_type: ResourceType,
    pub total: u64,
    pub allocated: u64,
    pub available: u64,
    pub utilization: f64,
}

// ---------------------------------------------------------------------------
// ResourceManager
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationRecord {
    pub id: Uuid,
    pub capability_id: CapabilityId,
    pub resource_type: ResourceType,
    pub amount: u64,
    pub allocated_at: DateTime<Utc>,
    pub released_at: Option<DateTime<Utc>>,
}

pub struct ResourceManager {
    pool: ResourcePool,
    allocation_history: RwLock<Vec<AllocationRecord>>,
}

impl ResourceManager {
    pub fn new(pool: ResourcePool) -> Self {
        Self {
            pool,
            allocation_history: RwLock::new(Vec::new()),
        }
    }

    pub fn create_default_pool() -> Self {
        let pool = ResourcePool::new();
        pool.register_budget(ResourceBudget::new(ResourceType::Cpu, 8));
        pool.register_budget(ResourceBudget::new(ResourceType::Gpu, 4));
        pool.register_budget(ResourceBudget::new(
            ResourceType::Memory,
            16 * 1024 * 1024 * 1024,
        ));
        pool.register_budget(ResourceBudget::new(ResourceType::InferenceTokens, 100_000));
        pool.register_budget(ResourceBudget::new(
            ResourceType::Disk,
            1024 * 1024 * 1024 * 1024,
        ));
        Self::new(pool)
    }

    pub fn request_resources(
        &self,
        requirements: &ResourceRequirements,
        capability_id: CapabilityId,
    ) -> ResourceResult<Vec<Uuid>> {
        let mut allocated: Vec<(ResourceType, u64)> = Vec::new();

        let checks: Vec<(ResourceType, u64)> = vec![
            (ResourceType::Cpu, float_to_units(requirements.cpu_units)),
            (ResourceType::Gpu, float_to_units(requirements.gpu_units)),
            (ResourceType::Memory, requirements.memory_bytes),
            (
                ResourceType::InferenceTokens,
                requirements.inference_tokens as u64,
            ),
            (ResourceType::Disk, requirements.disk_bytes),
        ];

        for (rt, amount) in &checks {
            if *amount == 0 {
                continue;
            }
            match self.pool.allocate(*rt, *amount) {
                Ok(()) => {
                    allocated.push((*rt, *amount));
                }
                Err(e) => {
                    for (prev_rt, prev_amount) in &allocated {
                        self.pool.release(*prev_rt, *prev_amount);
                    }
                    return Err(e);
                }
            }
        }

        let mut record_ids = Vec::new();
        let mut history = self.allocation_history.write();
        for (rt, amount) in &allocated {
            let record = AllocationRecord {
                id: Uuid::new_v4(),
                capability_id,
                resource_type: *rt,
                amount: *amount,
                allocated_at: Utc::now(),
                released_at: None,
            };
            record_ids.push(record.id);
            history.push(record);
        }

        Ok(record_ids)
    }

    pub fn release_resources(&self, capability_id: &CapabilityId) {
        let mut history = self.allocation_history.write();
        for record in history.iter_mut() {
            if record.capability_id == *capability_id && record.released_at.is_none() {
                self.pool.release(record.resource_type, record.amount);
                record.released_at = Some(Utc::now());
            }
        }
    }

    pub fn get_status(&self) -> ResourcePoolStatus {
        self.pool.budget_status()
    }

    pub fn enforce_budget(
        &self,
        capability_id: &CapabilityId,
        resource_type: ResourceType,
        amount: u64,
    ) -> ResourceResult<()> {
        if !self.pool.check_quota(capability_id) {
            return Err(ResourceError::QuotaExceeded {
                capability_id: *capability_id,
            });
        }
        self.pool.allocate(resource_type, amount)?;
        let mut history = self.allocation_history.write();
        history.push(AllocationRecord {
            id: Uuid::new_v4(),
            capability_id: *capability_id,
            resource_type,
            amount,
            allocated_at: Utc::now(),
            released_at: None,
        });
        Ok(())
    }

    pub fn reset_quota(&self, capability_id: &CapabilityId) {
        let quotas = self.pool.quotas.read();
        if let Some(quota) = quotas.get(capability_id) {
            quota.reset_if_needed();
        }
    }

    pub fn update_budget(&self, resource_type: ResourceType, new_total: u64) {
        let mut budgets = self.pool.budgets.write();
        if let Some(budget) = budgets.get_mut(&resource_type) {
            budget.set_total(new_total);
        }
    }

    pub fn pool(&self) -> &ResourcePool {
        &self.pool
    }

    pub fn active_allocations(&self, capability_id: &CapabilityId) -> Vec<AllocationRecord> {
        let history = self.allocation_history.read();
        history
            .iter()
            .filter(|r| r.capability_id == *capability_id && r.released_at.is_none())
            .cloned()
            .collect()
    }

    pub fn allocation_history(&self) -> Vec<AllocationRecord> {
        self.allocation_history.read().clone()
    }
}

fn float_to_units(value: f64) -> u64 {
    (value * 100.0).round() as u64
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn cap_id() -> CapabilityId {
        CapabilityId::new()
    }

    // -- ResourceType tests --

    #[test]
    fn resource_type_default_is_cpu() {
        assert_eq!(ResourceType::default(), ResourceType::Cpu);
    }

    #[test]
    fn resource_type_display() {
        assert_eq!(format!("{}", ResourceType::Cpu), "cpu");
        assert_eq!(format!("{}", ResourceType::Gpu), "gpu");
        assert_eq!(format!("{}", ResourceType::Memory), "memory");
        assert_eq!(
            format!("{}", ResourceType::InferenceTokens),
            "inference_tokens"
        );
        assert_eq!(format!("{}", ResourceType::Disk), "disk");
    }

    #[test]
    fn resource_type_all_count() {
        assert_eq!(ResourceType::ALL.len(), 5);
    }

    // -- ResourceRequirements tests --

    #[test]
    fn requirements_none_is_default() {
        let none = ResourceRequirements::none();
        assert_eq!(none.cpu_units, 0.0);
        assert_eq!(none.memory_bytes, 0);
    }

    #[test]
    fn requirements_minimal() {
        let m = ResourceRequirements::minimal();
        assert!(m.cpu_units > 0.0);
        assert!(m.memory_bytes > 0);
        assert_eq!(m.gpu_units, 0.0);
    }

    #[test]
    fn requirements_heavy() {
        let h = ResourceRequirements::heavy();
        assert!(h.gpu_units > 0.0);
        assert!(h.memory_bytes > 0);
        assert!(h.inference_tokens > 0);
    }

    // -- ResourceError tests --

    #[test]
    fn error_display_insufficient() {
        let err = ResourceError::InsufficientBudget {
            resource_type: ResourceType::Cpu,
            requested: 10,
            available: 5,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("cpu"));
        assert!(msg.contains("10"));
        assert!(msg.contains("5"));
    }

    #[test]
    fn error_display_budget_not_found() {
        let err = ResourceError::BudgetNotFound(ResourceType::Gpu);
        let msg = format!("{}", err);
        assert!(msg.contains("gpu"));
    }

    #[test]
    fn error_display_quota_exceeded() {
        let cid = cap_id();
        let err = ResourceError::QuotaExceeded {
            capability_id: cid,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("quota"));
    }

    #[test]
    fn error_is_std_error() {
        let err = ResourceError::BudgetNotFound(ResourceType::Cpu);
        let _: &dyn std::error::Error = &err;
    }

    // -- ResourceBudget tests --

    #[test]
    fn budget_new_starts_empty() {
        let b = ResourceBudget::new(ResourceType::Cpu, 8);
        assert_eq!(b.total(), 8);
        assert_eq!(b.allocated(), 0);
        assert_eq!(b.available(), 8);
    }

    #[test]
    fn budget_allocate_within_limit() {
        let b = ResourceBudget::new(ResourceType::Cpu, 8);
        assert!(b.allocate(3).is_ok());
        assert_eq!(b.allocated(), 3);
        assert_eq!(b.available(), 5);
    }

    #[test]
    fn budget_allocate_exact_limit() {
        let b = ResourceBudget::new(ResourceType::Cpu, 8);
        assert!(b.allocate(8).is_ok());
        assert_eq!(b.allocated(), 8);
        assert_eq!(b.available(), 0);
    }

    #[test]
    fn budget_allocate_exceeds_limit() {
        let b = ResourceBudget::new(ResourceType::Cpu, 8);
        assert!(b.allocate(9).is_err());
        assert_eq!(b.allocated(), 0);
        assert_eq!(b.available(), 8);
    }

    #[test]
    fn budget_allocate_returns_correct_error() {
        let b = ResourceBudget::new(ResourceType::Memory, 1024);
        b.allocate(600).unwrap();
        let err = b.allocate(500).unwrap_err();
        match err {
            ResourceError::InsufficientBudget {
                resource_type,
                requested,
                available,
            } => {
                assert_eq!(resource_type, ResourceType::Memory);
                assert_eq!(requested, 500);
                assert_eq!(available, 424);
            }
            _ => panic!("wrong error variant"),
        }
    }

    #[test]
    fn budget_release_reduces_allocated() {
        let b = ResourceBudget::new(ResourceType::Cpu, 8);
        b.allocate(5).unwrap();
        b.release(3);
        assert_eq!(b.allocated(), 2);
        assert_eq!(b.available(), 6);
    }

    #[test]
    fn budget_release_more_than_allocated_clamps_to_zero() {
        let b = ResourceBudget::new(ResourceType::Cpu, 8);
        b.allocate(2).unwrap();
        b.release(100);
        assert_eq!(b.allocated(), 0);
        assert_eq!(b.available(), 8);
    }

    #[test]
    fn budget_utilization_percentage() {
        let b = ResourceBudget::new(ResourceType::Cpu, 8);
        b.allocate(4).unwrap();
        assert!((b.utilization_percentage() - 50.0).abs() < 0.01);
    }

    #[test]
    fn budget_utilization_zero_total() {
        let b = ResourceBudget::new(ResourceType::Cpu, 0);
        assert_eq!(b.utilization_percentage(), 0.0);
    }

    #[test]
    fn budget_set_total() {
        let mut b = ResourceBudget::new(ResourceType::Cpu, 4);
        b.set_total(16);
        assert_eq!(b.total(), 16);
        assert_eq!(b.available(), 16);
    }

    #[test]
    fn budget_reserved_starts_zero() {
        let b = ResourceBudget::new(ResourceType::Cpu, 8);
        assert_eq!(b.reserved(), 0);
    }

    // -- ExecutionQuota tests --

    #[test]
    fn quota_starts_executable() {
        let q = ExecutionQuota::new(cap_id(), 10, 100, 3);
        assert!(q.can_execute());
    }

    #[test]
    fn quota_record_execution_increments() {
        let q = ExecutionQuota::new(cap_id(), 10, 100, 3);
        q.record_execution().unwrap();
        let usage = q.current_usage();
        assert_eq!(usage.hour_count, 1);
        assert_eq!(usage.day_count, 1);
        assert_eq!(usage.active_count, 1);
    }

    #[test]
    fn quota_release_execution_decrements_active() {
        let q = ExecutionQuota::new(cap_id(), 10, 100, 3);
        q.record_execution().unwrap();
        q.release_execution();
        let usage = q.current_usage();
        assert_eq!(usage.active_count, 0);
        assert_eq!(usage.hour_count, 1);
    }

    #[test]
    fn quota_max_concurrent_blocks() {
        let q = ExecutionQuota::new(cap_id(), 100, 100, 2);
        q.record_execution().unwrap();
        q.record_execution().unwrap();
        assert!(!q.can_execute());
        q.release_execution();
        assert!(q.can_execute());
    }

    #[test]
    fn quota_max_hourly_blocks() {
        let q = ExecutionQuota::new(cap_id(), 2, 100, 10);
        q.record_execution().unwrap();
        q.record_execution().unwrap();
        q.release_execution();
        assert!(!q.can_execute());
    }

    #[test]
    fn quota_max_daily_blocks() {
        let q = ExecutionQuota::new(cap_id(), 100, 2, 10);
        q.record_execution().unwrap();
        q.record_execution().unwrap();
        q.release_execution();
        assert!(!q.can_execute());
    }

    #[test]
    fn quota_record_when_exceeded_returns_error() {
        let q = ExecutionQuota::new(cap_id(), 1, 1, 1);
        q.record_execution().unwrap();
        assert!(q.record_execution().is_err());
    }

    #[test]
    fn quota_release_clamps_to_zero() {
        let q = ExecutionQuota::new(cap_id(), 10, 10, 10);
        q.release_execution();
        let usage = q.current_usage();
        assert_eq!(usage.active_count, 0);
    }

    #[test]
    fn quota_usage_clone() {
        let q = ExecutionQuota::new(cap_id(), 5, 5, 5);
        q.record_execution().unwrap();
        let usage = q.current_usage();
        let cloned = usage.clone();
        assert_eq!(usage.hour_count, cloned.hour_count);
        assert_eq!(usage.day_count, cloned.day_count);
        assert_eq!(usage.active_count, cloned.active_count);
    }

    // -- ResourcePool tests --

    #[test]
    fn pool_new_is_empty() {
        let pool = ResourcePool::new();
        assert!(pool.get_budget(ResourceType::Cpu).is_none());
    }

    #[test]
    fn pool_register_and_get_budget() {
        let pool = ResourcePool::new();
        pool.register_budget(ResourceBudget::new(ResourceType::Cpu, 8));
        let info = pool.get_budget(ResourceType::Cpu).unwrap();
        assert_eq!(info.total, 8);
        assert_eq!(info.available, 8);
    }

    #[test]
    fn pool_allocate_success() {
        let pool = ResourcePool::new();
        pool.register_budget(ResourceBudget::new(ResourceType::Cpu, 8));
        assert!(pool.allocate(ResourceType::Cpu, 3).is_ok());
        let info = pool.get_budget(ResourceType::Cpu).unwrap();
        assert_eq!(info.allocated, 3);
    }

    #[test]
    fn pool_allocate_unregistered_type_fails() {
        let pool = ResourcePool::new();
        assert!(pool.allocate(ResourceType::Cpu, 1).is_err());
    }

    #[test]
    fn pool_allocate_exceeds_fails_and_preserves_state() {
        let pool = ResourcePool::new();
        pool.register_budget(ResourceBudget::new(ResourceType::Cpu, 8));
        pool.allocate(ResourceType::Cpu, 6).unwrap();
        assert!(pool.allocate(ResourceType::Cpu, 3).is_err());
        let info = pool.get_budget(ResourceType::Cpu).unwrap();
        assert_eq!(info.allocated, 6);
    }

    #[test]
    fn pool_release_works() {
        let pool = ResourcePool::new();
        pool.register_budget(ResourceBudget::new(ResourceType::Cpu, 8));
        pool.allocate(ResourceType::Cpu, 5).unwrap();
        pool.release(ResourceType::Cpu, 2);
        let info = pool.get_budget(ResourceType::Cpu).unwrap();
        assert_eq!(info.allocated, 3);
    }

    #[test]
    fn pool_check_quota_no_quota_registered_returns_true() {
        let pool = ResourcePool::new();
        assert!(pool.check_quota(&cap_id()));
    }

    #[test]
    fn pool_budget_status() {
        let pool = ResourcePool::new();
        pool.register_budget(ResourceBudget::new(ResourceType::Cpu, 8));
        pool.register_budget(ResourceBudget::new(ResourceType::Gpu, 4));
        pool.allocate(ResourceType::Cpu, 4).unwrap();
        pool.allocate(ResourceType::Gpu, 2).unwrap();

        let status = pool.budget_status();
        assert_eq!(status.budgets.len(), 2);
        assert!((status.total_cpu_utilization - 50.0).abs() < 0.01);
        assert!((status.total_gpu_utilization - 50.0).abs() < 0.01);
        assert!((status.total_memory_utilization).abs() < 0.01);
    }

    #[test]
    fn pool_total_utilization() {
        let pool = ResourcePool::new();
        pool.register_budget(ResourceBudget::new(ResourceType::Cpu, 10));
        pool.allocate(ResourceType::Cpu, 7).unwrap();

        let util = pool.total_utilization();
        assert!((util[&ResourceType::Cpu] - 70.0).abs() < 0.01);
    }

    #[test]
    fn pool_register_quota() {
        let pool = ResourcePool::new();
        let cid = cap_id();
        pool.register_quota(ExecutionQuota::new(cid, 5, 50, 3));
        assert!(pool.check_quota(&cid));
    }

    #[test]
    fn pool_quota_blocks_after_limit() {
        let pool = ResourcePool::new();
        let cid = cap_id();
        pool.register_quota(ExecutionQuota::new(cid, 2, 10, 10));

        pool.record_quota_usage(&cid).unwrap();
        pool.record_quota_usage(&cid).unwrap();
        assert!(!pool.check_quota(&cid));
    }

    #[test]
    fn pool_quota_release_allows_more() {
        let pool = ResourcePool::new();
        let cid = cap_id();
        pool.register_quota(ExecutionQuota::new(cid, 2, 10, 1));

        pool.record_quota_usage(&cid).unwrap();
        assert!(!pool.check_quota(&cid));
        pool.release_quota_usage(&cid);
        assert!(pool.check_quota(&cid));
    }

    // -- ResourceManager tests --

    #[test]
    fn default_pool_has_all_resource_types() {
        let mgr = ResourceManager::create_default_pool();
        for rt in ResourceType::ALL {
            let info = mgr.pool().get_budget(*rt);
            assert!(info.is_some(), "missing budget for {}", rt);
        }
    }

    #[test]
    fn default_pool_cpu_has_8_units() {
        let mgr = ResourceManager::create_default_pool();
        let info = mgr.pool().get_budget(ResourceType::Cpu).unwrap();
        assert_eq!(info.total, 8);
    }

    #[test]
    fn default_pool_gpu_has_4_units() {
        let mgr = ResourceManager::create_default_pool();
        let info = mgr.pool().get_budget(ResourceType::Gpu).unwrap();
        assert_eq!(info.total, 4);
    }

    #[test]
    fn default_pool_memory_has_16gb() {
        let mgr = ResourceManager::create_default_pool();
        let info = mgr.pool().get_budget(ResourceType::Memory).unwrap();
        assert_eq!(info.total, 16 * 1024 * 1024 * 1024);
    }

    #[test]
    fn default_pool_inference_tokens_has_100k() {
        let mgr = ResourceManager::create_default_pool();
        let info = mgr
            .pool()
            .get_budget(ResourceType::InferenceTokens)
            .unwrap();
        assert_eq!(info.total, 100_000);
    }

    #[test]
    fn default_pool_disk_has_1tb() {
        let mgr = ResourceManager::create_default_pool();
        let info = mgr.pool().get_budget(ResourceType::Disk).unwrap();
        assert_eq!(info.total, 1024 * 1024 * 1024 * 1024);
    }

    #[test]
    fn request_resources_zero_requirements_succeeds() {
        let mgr = ResourceManager::create_default_pool();
        let reqs = ResourceRequirements::none();
        let ids = mgr.request_resources(&reqs, cap_id()).unwrap();
        assert!(ids.is_empty());
    }

    #[test]
    fn request_resources_success() {
        let mgr = ResourceManager::create_default_pool();
        let reqs = ResourceRequirements {
            cpu_units: 0.02,
            gpu_units: 0.01,
            memory_bytes: 1024 * 1024,
            inference_tokens: 100,
            disk_bytes: 2048,
        };
        let ids = mgr.request_resources(&reqs, cap_id()).unwrap();
        assert_eq!(ids.len(), 5);
    }

    #[test]
    fn request_resources_partial_failure_rolls_back_all() {
        let mgr = ResourceManager::create_default_pool();
        let cid = cap_id();

        let reqs = ResourceRequirements {
            cpu_units: 0.02,
            gpu_units: 0.0,
            memory_bytes: 16 * 1024 * 1024 * 1024 + 1,
            inference_tokens: 0,
            disk_bytes: 0,
        };
        let result = mgr.request_resources(&reqs, cid);
        assert!(result.is_err());

        let cpu_info = mgr.pool().get_budget(ResourceType::Cpu).unwrap();
        assert_eq!(cpu_info.allocated, 0);
        let mem_info = mgr.pool().get_budget(ResourceType::Memory).unwrap();
        assert_eq!(mem_info.allocated, 0);
    }

    #[test]
    fn request_resources_records_history() {
        let mgr = ResourceManager::create_default_pool();
        let cid = cap_id();
        let reqs = ResourceRequirements::minimal();
        let _ids = mgr.request_resources(&reqs, cid).unwrap();
        let history = mgr.allocation_history();
        assert!(!history.is_empty());
        for record in &history {
            assert_eq!(record.capability_id, cid);
            assert!(record.released_at.is_none());
        }
    }

    #[test]
    fn release_resources_marks_records_released() {
        let mgr = ResourceManager::create_default_pool();
        let cid = cap_id();
        let reqs = ResourceRequirements::minimal();
        mgr.request_resources(&reqs, cid).unwrap();

        mgr.release_resources(&cid);
        let history = mgr.allocation_history();
        for record in &history {
            if record.capability_id == cid {
                assert!(record.released_at.is_some());
            }
        }
    }

    #[test]
    fn release_resources_frees_pool_budget() {
        let mgr = ResourceManager::create_default_pool();
        let cid = cap_id();
        let reqs = ResourceRequirements::minimal();
        mgr.request_resources(&reqs, cid).unwrap();

        let before = mgr.pool().get_budget(ResourceType::Memory).unwrap();
        let allocated_before = before.allocated;

        mgr.release_resources(&cid);
        let after = mgr.pool().get_budget(ResourceType::Memory).unwrap();
        assert_eq!(after.allocated, 0);
        assert!(allocated_before > 0);
    }

    #[test]
    fn enforce_budget_success() {
        let mgr = ResourceManager::create_default_pool();
        let cid = cap_id();
        assert!(mgr.enforce_budget(&cid, ResourceType::Cpu, 4).is_ok());
        let info = mgr.pool().get_budget(ResourceType::Cpu).unwrap();
        assert_eq!(info.allocated, 4);
    }

    #[test]
    fn enforce_budget_exceeds_fails() {
        let mgr = ResourceManager::create_default_pool();
        let cid = cap_id();
        assert!(mgr.enforce_budget(&cid, ResourceType::Cpu, 10).is_err());
    }

    #[test]
    fn update_budget_changes_total() {
        let mgr = ResourceManager::create_default_pool();
        mgr.update_budget(ResourceType::Cpu, 32);
        let info = mgr.pool().get_budget(ResourceType::Cpu).unwrap();
        assert_eq!(info.total, 32);
    }

    #[test]
    fn update_budget_existing_allocation_not_affected() {
        let mgr = ResourceManager::create_default_pool();
        let cid = cap_id();
        mgr.enforce_budget(&cid, ResourceType::Cpu, 6).unwrap();
        mgr.update_budget(ResourceType::Cpu, 32);
        let info = mgr.pool().get_budget(ResourceType::Cpu).unwrap();
        assert_eq!(info.total, 32);
        assert_eq!(info.allocated, 6);
    }

    #[test]
    fn active_allocations_filters_correctly() {
        let mgr = ResourceManager::create_default_pool();
        let cid1 = cap_id();
        let cid2 = cap_id();

        let reqs = ResourceRequirements::minimal();
        mgr.request_resources(&reqs, cid1).unwrap();
        mgr.request_resources(&reqs, cid2).unwrap();

        let active1 = mgr.active_allocations(&cid1);
        let active2 = mgr.active_allocations(&cid2);
        assert_eq!(active1.len(), 2);
        assert_eq!(active2.len(), 2);

        mgr.release_resources(&cid1);
        let active1 = mgr.active_allocations(&cid1);
        assert_eq!(active1.len(), 0);
        let active2 = mgr.active_allocations(&cid2);
        assert_eq!(active2.len(), 2);
    }

    #[test]
    fn double_release_is_idempotent() {
        let mgr = ResourceManager::create_default_pool();
        let cid = cap_id();
        let reqs = ResourceRequirements::minimal();
        mgr.request_resources(&reqs, cid).unwrap();

        mgr.release_resources(&cid);
        mgr.release_resources(&cid);

        let info = mgr.pool().get_budget(ResourceType::Memory).unwrap();
        assert_eq!(info.allocated, 0);
    }

    #[test]
    fn request_resources_heavy_needs_enough_budget() {
        let pool = ResourcePool::new();
        pool.register_budget(ResourceBudget::new(ResourceType::Cpu, 100));
        pool.register_budget(ResourceBudget::new(ResourceType::Gpu, 100));
        pool.register_budget(ResourceBudget::new(ResourceType::Memory, 4 * 1024 * 1024 * 1024));
        pool.register_budget(ResourceBudget::new(ResourceType::InferenceTokens, 100_000));
        pool.register_budget(ResourceBudget::new(ResourceType::Disk, 1024 * 1024 * 1024));
        let mgr = ResourceManager::new(pool);
        let heavy = ResourceRequirements::heavy();
        let cid = cap_id();
        assert!(mgr.request_resources(&heavy, cid).is_ok());
    }

    #[test]
    fn get_status_returns_all_budgets() {
        let mgr = ResourceManager::create_default_pool();
        let status = mgr.get_status();
        assert_eq!(status.budgets.len(), 5);
        for budget in &status.budgets {
            assert!(budget.total > 0);
            assert_eq!(budget.allocated, 0);
        }
    }

    // -- Quota integration tests --

    #[test]
    fn pool_quota_integration_blocks_after_limit() {
        let pool = ResourcePool::new();
        pool.register_budget(ResourceBudget::new(ResourceType::Cpu, 100));

        let cid = cap_id();
        pool.register_quota(ExecutionQuota::new(cid, 2, 10, 10));

        assert!(pool.check_quota(&cid));
        pool.record_quota_usage(&cid).unwrap();
        pool.record_quota_usage(&cid).unwrap();
        assert!(!pool.check_quota(&cid));

        pool.release_quota_usage(&cid);
        let active = pool.quotas.read();
        let q = active.get(&cid).unwrap();
        let usage = q.current_usage();
        assert_eq!(usage.active_count, 1);
    }

    #[test]
    fn budget_status_sorted_by_type_name() {
        let pool = ResourcePool::new();
        pool.register_budget(ResourceBudget::new(ResourceType::Memory, 1024));
        pool.register_budget(ResourceBudget::new(ResourceType::Cpu, 8));
        pool.register_budget(ResourceBudget::new(ResourceType::Gpu, 4));

        let status = pool.budget_status();
        let names: Vec<String> = status
            .budgets
            .iter()
            .map(|b| format!("{}", b.resource_type))
            .collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }

    #[test]
    fn allocation_amounts_match_requirements_exactly() {
        let mgr = ResourceManager::create_default_pool();
        let reqs = ResourceRequirements {
            cpu_units: 0.05,
            gpu_units: 0.03,
            memory_bytes: 512 * 1024 * 1024,
            inference_tokens: 500,
            disk_bytes: 1024 * 1024,
        };
        let cid = cap_id();
        mgr.request_resources(&reqs, cid).unwrap();

        let history = mgr.allocation_history();
        let cpu_record = history
            .iter()
            .find(|r| r.resource_type == ResourceType::Cpu)
            .unwrap();
        assert_eq!(cpu_record.amount, float_to_units(0.05));

        let gpu_record = history
            .iter()
            .find(|r| r.resource_type == ResourceType::Gpu)
            .unwrap();
        assert_eq!(gpu_record.amount, float_to_units(0.03));

        let mem_record = history
            .iter()
            .find(|r| r.resource_type == ResourceType::Memory)
            .unwrap();
        assert_eq!(mem_record.amount, 512 * 1024 * 1024);
    }

    #[test]
    fn concurrent_allocate_release_stress() {
        use std::sync::Arc;

        let mgr = Arc::new(ResourceManager::create_default_pool());
        let mut handles = Vec::new();

        for _ in 0..20 {
            let mgr = Arc::clone(&mgr);
            handles.push(std::thread::spawn(move || {
                let cid = cap_id();
                let reqs = ResourceRequirements::minimal();
                if mgr.request_resources(&reqs, cid).is_ok() {
                    std::thread::yield_now();
                    mgr.release_resources(&cid);
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        for rt in ResourceType::ALL {
            let info = mgr.pool().get_budget(*rt).unwrap();
            assert_eq!(info.allocated, 0, "leak detected for {}", rt);
        }
    }

    #[test]
    fn allocation_history_clone() {
        let mgr = ResourceManager::create_default_pool();
        let cid = cap_id();
        let reqs = ResourceRequirements::minimal();
        mgr.request_resources(&reqs, cid).unwrap();

        let history = mgr.allocation_history();
        assert_eq!(history.len(), 2);
        let cloned = history.clone();
        assert_eq!(cloned.len(), history.len());
        for (a, b) in history.iter().zip(cloned.iter()) {
            assert_eq!(a.id, b.id);
            assert_eq!(a.capability_id, b.capability_id);
            assert_eq!(a.resource_type, b.resource_type);
            assert_eq!(a.amount, b.amount);
        }
    }

    #[test]
    fn budget_status_serialization() {
        let status = ResourcePoolStatus {
            budgets: vec![BudgetStatus {
                resource_type: ResourceType::Cpu,
                total: 8,
                allocated: 4,
                available: 4,
                utilization: 50.0,
            }],
            total_cpu_utilization: 50.0,
            total_gpu_utilization: 0.0,
            total_memory_utilization: 0.0,
        };
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: ResourcePoolStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.budgets.len(), 1);
        assert_eq!(deserialized.total_cpu_utilization, 50.0);
    }

    #[test]
    fn multiple_capabilities_separate_allocations() {
        let mgr = ResourceManager::create_default_pool();
        let cid1 = cap_id();
        let cid2 = cap_id();

        let reqs1 = ResourceRequirements {
            cpu_units: 0.03,
            gpu_units: 0.0,
            memory_bytes: 512 * 1024 * 1024,
            inference_tokens: 0,
            disk_bytes: 0,
        };
        let reqs2 = ResourceRequirements {
            cpu_units: 0.04,
            gpu_units: 0.0,
            memory_bytes: 1024 * 1024 * 1024,
            inference_tokens: 0,
            disk_bytes: 0,
        };

        mgr.request_resources(&reqs1, cid1).unwrap();
        mgr.request_resources(&reqs2, cid2).unwrap();

        let active1 = mgr.active_allocations(&cid1);
        let active2 = mgr.active_allocations(&cid2);
        assert_eq!(active1.len(), 2);
        assert_eq!(active2.len(), 2);

        let cpu_info = mgr.pool().get_budget(ResourceType::Cpu).unwrap();
        assert_eq!(cpu_info.allocated, 7);

        let mem_info = mgr.pool().get_budget(ResourceType::Memory).unwrap();
        assert_eq!(mem_info.allocated, 512 * 1024 * 1024 + 1024 * 1024 * 1024);

        mgr.release_resources(&cid1);

        let cpu_info = mgr.pool().get_budget(ResourceType::Cpu).unwrap();
        assert_eq!(cpu_info.allocated, 4);

        let mem_info = mgr.pool().get_budget(ResourceType::Memory).unwrap();
        assert_eq!(mem_info.allocated, 1024 * 1024 * 1024);
    }

    #[test]
    fn update_budget_too_small_for_existing_allocation() {
        let mgr = ResourceManager::create_default_pool();
        let cid = cap_id();
        mgr.enforce_budget(&cid, ResourceType::Cpu, 6).unwrap();
        mgr.update_budget(ResourceType::Cpu, 4);

        let info = mgr.pool().get_budget(ResourceType::Cpu).unwrap();
        assert_eq!(info.total, 4);
        assert_eq!(info.allocated, 6);
        assert!(info.available == 0 || info.allocated > info.total);
    }
}
