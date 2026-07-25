use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::config::EvolutionConfiguration;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReasoningMetrics {
    pub inference_path_efficiency: f64,
    pub search_depth_optimization: f64,
    pub heuristic_ordering_score: f64,
    pub confidence_estimation_accuracy: f64,
    pub contradiction_handling_rate: f64,
}

pub struct ReasoningEvolution {
    metrics: RwLock<ReasoningMetrics>,
    #[allow(dead_code)]
    config: EvolutionConfiguration,
}

impl ReasoningEvolution {
    pub fn new(config: EvolutionConfiguration) -> Arc<Self> {
        Arc::new(Self {
            metrics: RwLock::new(ReasoningMetrics::default()),
            config,
        })
    }

    pub fn optimize_inference_paths(&self, delta: f64) {
        let mut m = self.metrics.write();
        m.inference_path_efficiency = (m.inference_path_efficiency + delta).clamp(0.0, 1.0);
    }

    pub fn optimize_search_depth(&self, delta: f64) {
        let mut m = self.metrics.write();
        m.search_depth_optimization = (m.search_depth_optimization + delta).clamp(0.0, 1.0);
    }

    pub fn optimize_heuristic_ordering(&self, delta: f64) {
        let mut m = self.metrics.write();
        m.heuristic_ordering_score = (m.heuristic_ordering_score + delta).clamp(0.0, 1.0);
    }

    pub fn optimize_confidence_estimation(&self, delta: f64) {
        let mut m = self.metrics.write();
        m.confidence_estimation_accuracy =
            (m.confidence_estimation_accuracy + delta).clamp(0.0, 1.0);
    }

    pub fn improve_contradiction_handling(&self, delta: f64) {
        let mut m = self.metrics.write();
        m.contradiction_handling_rate = (m.contradiction_handling_rate + delta).clamp(0.0, 1.0);
    }

    pub fn get_metrics(&self) -> ReasoningMetrics {
        self.metrics.read().clone()
    }
}
