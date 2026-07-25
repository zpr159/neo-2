use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::config::EvolutionConfiguration;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlanningMetrics {
    pub decomposition_score: f64,
    pub scheduling_efficiency: f64,
    pub parallelism_utilization: f64,
    pub resource_allocation_score: f64,
    pub plan_repair_success_rate: f64,
}

pub struct PlanningEvolution {
    metrics: RwLock<PlanningMetrics>,
    #[allow(dead_code)]
    config: EvolutionConfiguration,
}

impl PlanningEvolution {
    pub fn new(config: EvolutionConfiguration) -> Arc<Self> {
        Arc::new(Self {
            metrics: RwLock::new(PlanningMetrics::default()),
            config,
        })
    }

    pub fn improve_decomposition(&self, delta: f64) {
        let mut m = self.metrics.write();
        m.decomposition_score = (m.decomposition_score + delta).clamp(0.0, 1.0);
    }

    pub fn improve_scheduling(&self, delta: f64) {
        let mut m = self.metrics.write();
        m.scheduling_efficiency = (m.scheduling_efficiency + delta).clamp(0.0, 1.0);
    }

    pub fn improve_parallelism(&self, delta: f64) {
        let mut m = self.metrics.write();
        m.parallelism_utilization = (m.parallelism_utilization + delta).clamp(0.0, 1.0);
    }

    pub fn improve_resource_allocation(&self, delta: f64) {
        let mut m = self.metrics.write();
        m.resource_allocation_score = (m.resource_allocation_score + delta).clamp(0.0, 1.0);
    }

    pub fn improve_plan_repair(&self, delta: f64) {
        let mut m = self.metrics.write();
        m.plan_repair_success_rate = (m.plan_repair_success_rate + delta).clamp(0.0, 1.0);
    }

    pub fn get_metrics(&self) -> PlanningMetrics {
        self.metrics.read().clone()
    }

    pub fn record_improvement(&self, field: &str, delta: f64) {
        match field {
            "decomposition" => self.improve_decomposition(delta),
            "scheduling" => self.improve_scheduling(delta),
            "parallelism" => self.improve_parallelism(delta),
            "resource_allocation" => self.improve_resource_allocation(delta),
            "plan_repair" => self.improve_plan_repair(delta),
            _ => {}
        }
    }
}
