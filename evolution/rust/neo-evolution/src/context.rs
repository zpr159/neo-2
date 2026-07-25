use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::config::EvolutionConfiguration;
use crate::state::EvolutionState;
use crate::types::{EvolutionId, SubsystemTarget};

/// Per-subsystem metrics collected during analysis.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubsystemMetrics {
    pub analyzed_count: u64,
    pub improvement_count: u64,
    pub experiment_count: u64,
    pub success_rate: f64,
}

/// Shared context threaded through evolution operations.
pub struct EvolutionContext {
    pub config: EvolutionConfiguration,
    state: RwLock<EvolutionState>,
    pub subsystem_metrics: dashmap::DashMap<SubsystemTarget, SubsystemMetrics>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl EvolutionContext {
    pub fn new(config: EvolutionConfiguration) -> Arc<Self> {
        Arc::new(Self {
            config,
            state: RwLock::new(EvolutionState::default()),
            subsystem_metrics: dashmap::DashMap::new(),
            created_at: chrono::Utc::now(),
        })
    }

    pub fn get_state(&self) -> EvolutionState {
        self.state.read().clone()
    }

    pub fn transition_to(&self, new_status: crate::types::EvolutionStatus) {
        let mut s = self.state.write();
        s.status = new_status;
    }

    pub fn record_analysis(&self, target: SubsystemTarget) {
        self.subsystem_metrics
            .entry(target)
            .or_default()
            .analyzed_count += 1;
    }

    pub fn record_experiment(&self, target: SubsystemTarget) {
        self.subsystem_metrics
            .entry(target)
            .or_default()
            .experiment_count += 1;
    }

    pub fn get_metrics(&self) -> Vec<(SubsystemTarget, SubsystemMetrics)> {
        self.subsystem_metrics
            .iter()
            .map(|e| (*e.key(), e.value().clone()))
            .collect()
    }

    pub fn snapshot(&self) -> crate::state::EvolutionSnapshot {
        crate::state::EvolutionSnapshot::capture(&self.get_state())
    }
}
