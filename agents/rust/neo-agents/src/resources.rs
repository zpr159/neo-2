use std::sync::Arc;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::error::{AgentError, AgentResult};
use crate::types::AgentId;

// ---------------------------------------------------------------------------
// ResourceType
// ---------------------------------------------------------------------------

/// Types of resources that can be managed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    /// CPU units.
    Cpu,
    /// GPU units.
    Gpu,
    /// Memory in bytes.
    Memory,
    /// Disk in bytes.
    Disk,
    /// Network bandwidth.
    Network,
    /// Concurrency slots.
    Concurrency,
    /// Inference tokens.
    InferenceTokens,
    /// Custom resource.
    Custom(String),
}

// ---------------------------------------------------------------------------
// AgentQuota
// ---------------------------------------------------------------------------

/// Resource quota for an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentQuota {
    /// The agent this quota applies to.
    pub agent_id: AgentId,
    /// Maximum CPU units.
    pub max_cpu_units: f64,
    /// Maximum memory in bytes.
    pub max_memory_bytes: u64,
    /// Maximum concurrent tasks.
    pub max_concurrent_tasks: usize,
    /// Maximum inference tokens per minute.
    pub max_inference_tokens_per_minute: u64,
    /// Maximum messages per second.
    pub max_messages_per_second: u64,
}

impl Default for AgentQuota {
    fn default() -> Self {
        Self {
            agent_id: AgentId::new(),
            max_cpu_units: 1.0,
            max_memory_bytes: 256 * 1024 * 1024,
            max_concurrent_tasks: 4,
            max_inference_tokens_per_minute: 10_000,
            max_messages_per_second: 100,
        }
    }
}

// ---------------------------------------------------------------------------
// AgentLimits
// ---------------------------------------------------------------------------

/// System-wide limits for agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLimits {
    /// Maximum total CPU units across all agents.
    pub total_cpu_units: f64,
    /// Maximum total memory across all agents in bytes.
    pub total_memory_bytes: u64,
    /// Maximum total concurrent tasks across all agents.
    pub total_concurrent_tasks: usize,
    /// Maximum number of agents.
    pub max_agents: usize,
    /// Maximum total inference tokens per minute.
    pub total_inference_tokens_per_minute: u64,
}

impl Default for AgentLimits {
    fn default() -> Self {
        Self {
            total_cpu_units: 16.0,
            total_memory_bytes: 8 * 1024 * 1024 * 1024,
            total_concurrent_tasks: 256,
            max_agents: 1024,
            total_inference_tokens_per_minute: 1_000_000,
        }
    }
}

// ---------------------------------------------------------------------------
// ResourceReservation
// ---------------------------------------------------------------------------

/// A reservation of resources for a specific agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceReservation {
    /// The agent holding this reservation.
    pub agent_id: AgentId,
    /// CPU units reserved.
    pub cpu_units: f64,
    /// Memory reserved in bytes.
    pub memory_bytes: u64,
    /// Concurrency slots reserved.
    pub concurrency_slots: usize,
    /// When the reservation was made.
    pub reserved_at: chrono::DateTime<chrono::Utc>,
}

// ---------------------------------------------------------------------------
// ResourceManager
// ---------------------------------------------------------------------------

/// Manages resource allocation and quotas for agents.
pub struct ResourceManager {
    /// Active reservations per agent.
    reservations: DashMap<AgentId, ResourceReservation>,
    /// Agent quotas.
    quotas: DashMap<AgentId, AgentQuota>,
    /// System-wide limits.
    limits: AgentLimits,
    /// Current total CPU usage.
    current_cpu_usage: Arc<RwLock<f64>>,
    /// Current total memory usage.
    current_memory_usage: Arc<RwLock<u64>>,
    /// Current total concurrent tasks.
    #[allow(dead_code)]
    current_concurrent_tasks: Arc<RwLock<usize>>,
}

impl ResourceManager {
    /// Create a new resource manager with the given limits.
    #[must_use]
    pub fn new(limits: AgentLimits) -> Self {
        Self {
            reservations: DashMap::new(),
            quotas: DashMap::new(),
            limits,
            current_cpu_usage: Arc::new(RwLock::new(0.0)),
            current_memory_usage: Arc::new(RwLock::new(0)),
            current_concurrent_tasks: Arc::new(RwLock::new(0)),
        }
    }

    /// Set a quota for an agent.
    pub fn set_quota(&self, quota: AgentQuota) {
        self.quotas.insert(quota.agent_id, quota);
    }

    /// Get the quota for an agent.
    pub fn get_quota(&self, agent_id: &AgentId) -> Option<AgentQuota> {
        self.quotas.get(agent_id).map(|q| q.clone())
    }

    /// Reserve resources for an agent.
    pub async fn reserve(
        &self,
        agent_id: AgentId,
        cpu_units: f64,
        memory_bytes: u64,
        concurrency_slots: usize,
    ) -> AgentResult<ResourceReservation> {
        // Check against agent quota
        if let Some(quota) = self.quotas.get(&agent_id) {
            if cpu_units > quota.max_cpu_units {
                return Err(AgentError::ResourceReservationFailed(format!(
                    "requested {cpu_units} CPU units exceeds quota {}",
                    quota.max_cpu_units
                )));
            }
            if memory_bytes > quota.max_memory_bytes {
                return Err(AgentError::ResourceReservationFailed(format!(
                    "requested {memory_bytes} bytes exceeds quota {}",
                    quota.max_memory_bytes
                )));
            }
        }

        // Check against system limits
        {
            let cpu = self.current_cpu_usage.read().await;
            if *cpu + cpu_units > self.limits.total_cpu_units {
                return Err(AgentError::ResourceReservationFailed(
                    "system CPU limit reached".into(),
                ));
            }
        }
        {
            let mem = self.current_memory_usage.read().await;
            if *mem + memory_bytes > self.limits.total_memory_bytes {
                return Err(AgentError::ResourceReservationFailed(
                    "system memory limit reached".into(),
                ));
            }
        }

        // Apply reservation
        {
            let mut cpu = self.current_cpu_usage.write().await;
            *cpu += cpu_units;
        }
        {
            let mut mem = self.current_memory_usage.write().await;
            *mem += memory_bytes;
        }

        let reservation = ResourceReservation {
            agent_id,
            cpu_units,
            memory_bytes,
            concurrency_slots,
            reserved_at: chrono::Utc::now(),
        };

        self.reservations.insert(agent_id, reservation.clone());
        Ok(reservation)
    }

    /// Release resources for an agent.
    pub async fn release(&self, agent_id: &AgentId) -> AgentResult<()> {
        if let Some((_, reservation)) = self.reservations.remove(agent_id) {
            let mut cpu = self.current_cpu_usage.write().await;
            *cpu = (*cpu - reservation.cpu_units).max(0.0);

            let mut mem = self.current_memory_usage.write().await;
            *mem = mem.saturating_sub(reservation.memory_bytes);
        }
        Ok(())
    }

    /// Get current CPU usage.
    #[must_use]
    pub async fn cpu_usage(&self) -> f64 {
        *self.current_cpu_usage.read().await
    }

    /// Get current memory usage.
    #[must_use]
    pub async fn memory_usage(&self) -> u64 {
        *self.current_memory_usage.read().await
    }

    /// Get system limits.
    #[must_use]
    pub fn limits(&self) -> &AgentLimits {
        &self.limits
    }

    /// Check if an agent has exceeded its quota.
    #[must_use]
    pub fn check_quota(&self, agent_id: &AgentId) -> bool {
        if let Some(reservation) = self.reservations.get(agent_id) {
            if let Some(quota) = self.quotas.get(agent_id) {
                return reservation.cpu_units <= quota.max_cpu_units
                    && reservation.memory_bytes <= quota.max_memory_bytes;
            }
        }
        true
    }

    /// Get the reservation for an agent.
    pub fn get_reservation(&self, agent_id: &AgentId) -> Option<ResourceReservation> {
        self.reservations.get(agent_id).map(|r| r.clone())
    }
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new(AgentLimits::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_reserve_and_release() {
        let rm = ResourceManager::new(AgentLimits::default());
        let agent = AgentId::new();

        let reservation = rm.reserve(agent, 0.5, 1024 * 1024, 2).await.unwrap();
        assert_eq!(reservation.cpu_units, 0.5);

        assert!((rm.cpu_usage().await - 0.5).abs() < f64::EPSILON);

        rm.release(&agent).await.unwrap();
        assert!((rm.cpu_usage().await).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_quota_enforcement() {
        let rm = ResourceManager::new(AgentLimits::default());
        let agent = AgentId::new();

        rm.set_quota(AgentQuota {
            agent_id: agent,
            max_cpu_units: 1.0,
            max_memory_bytes: 1024,
            max_concurrent_tasks: 2,
            max_inference_tokens_per_minute: 100,
            max_messages_per_second: 10,
        });

        // Should succeed
        rm.reserve(agent, 0.5, 512, 1).await.unwrap();

        // Release and try over quota
        rm.release(&agent).await.unwrap();
        let result = rm.reserve(agent, 2.0, 512, 1).await;
        assert!(result.is_err());
    }
}
