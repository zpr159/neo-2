use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::config::EvolutionConfiguration;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LearningMetrics {
    pub experience_selection_quality: f64,
    pub reflection_quality: f64,
    pub pattern_extraction_rate: f64,
    pub consolidation_effectiveness: f64,
    pub skill_generation_rate: f64,
}

pub struct LearningEvolution {
    metrics: RwLock<LearningMetrics>,
    #[allow(dead_code)]
    config: EvolutionConfiguration,
}

impl LearningEvolution {
    pub fn new(config: EvolutionConfiguration) -> Arc<Self> {
        Arc::new(Self {
            metrics: RwLock::new(LearningMetrics::default()),
            config,
        })
    }

    pub fn improve_experience_selection(&self, delta: f64) {
        let mut m = self.metrics.write();
        m.experience_selection_quality = (m.experience_selection_quality + delta).clamp(0.0, 1.0);
    }

    pub fn improve_reflection(&self, delta: f64) {
        let mut m = self.metrics.write();
        m.reflection_quality = (m.reflection_quality + delta).clamp(0.0, 1.0);
    }

    pub fn improve_pattern_extraction(&self, delta: f64) {
        let mut m = self.metrics.write();
        m.pattern_extraction_rate = (m.pattern_extraction_rate + delta).clamp(0.0, 1.0);
    }

    pub fn improve_consolidation(&self, delta: f64) {
        let mut m = self.metrics.write();
        m.consolidation_effectiveness = (m.consolidation_effectiveness + delta).clamp(0.0, 1.0);
    }

    pub fn improve_skill_generation(&self, delta: f64) {
        let mut m = self.metrics.write();
        m.skill_generation_rate = (m.skill_generation_rate + delta).clamp(0.0, 1.0);
    }

    pub fn get_metrics(&self) -> LearningMetrics {
        self.metrics.read().clone()
    }
}
