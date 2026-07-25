use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use dashmap::DashMap;

use crate::config::EvolutionConfiguration;
use crate::error::{EvolutionError, EvolutionResult};
use crate::experiment::experiment::{Experiment, ExperimentConfig};
use crate::experiment::result::ExperimentResult;
use crate::types::{EvolutionId, EvolutionStatus, SubsystemTarget};

/// Central manager for creating, running, and tracking experiments.
pub struct ExperimentManager {
    experiments: DashMap<EvolutionId, Experiment>,
    results: DashMap<EvolutionId, ExperimentResult>,
    config: EvolutionConfiguration,
    active_count: AtomicUsize,
}

impl ExperimentManager {
    pub fn new(config: EvolutionConfiguration) -> Self {
        Self {
            experiments: DashMap::new(),
            results: DashMap::new(),
            config,
            active_count: AtomicUsize::new(0),
        }
    }

    pub fn create_experiment(&self, config: ExperimentConfig) -> EvolutionResult<EvolutionId> {
        let max = self.config.max_concurrent_cycles;
        let active = self.active_count.load(Ordering::Relaxed);
        if active >= max {
            return Err(EvolutionError::ResourceExhausted(format!(
                "max concurrent experiments ({max}) reached"
            )));
        }
        let id = config.id;
        let experiment = Experiment::new(config);
        self.experiments.insert(id, experiment);
        Ok(id)
    }

    pub fn start_experiment(&self, id: EvolutionId) -> EvolutionResult<()> {
        let mut exp = self
            .experiments
            .get_mut(&id)
            .ok_or_else(|| EvolutionError::NotFound(format!("experiment {id}")))?;
        exp.start();
        self.active_count.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    pub fn complete_experiment(
        &self,
        id: EvolutionId,
        result: ExperimentResult,
    ) -> EvolutionResult<()> {
        {
            let mut exp = self
                .experiments
                .get_mut(&id)
                .ok_or_else(|| EvolutionError::NotFound(format!("experiment {id}")))?;
            exp.complete();
        }
        self.active_count.fetch_sub(1, Ordering::Relaxed);
        self.results.insert(id, result);
        Ok(())
    }

    pub fn fail_experiment(&self, id: EvolutionId, errors: Vec<String>) -> EvolutionResult<()> {
        {
            let mut exp = self
                .experiments
                .get_mut(&id)
                .ok_or_else(|| EvolutionError::NotFound(format!("experiment {id}")))?;
            exp.fail();
        }
        self.active_count.fetch_sub(1, Ordering::Relaxed);
        let result = ExperimentResult {
            experiment_id: id,
            success: false,
            metrics: Default::default(),
            baseline_metrics: None,
            comparison: None,
            errors,
            output_data: HashMap::new(),
            duration_ms: 0,
            completed_at: chrono::Utc::now(),
        };
        self.results.insert(id, result);
        Ok(())
    }

    pub fn cancel_experiment(&self, id: EvolutionId) -> EvolutionResult<()> {
        let mut exp = self
            .experiments
            .get_mut(&id)
            .ok_or_else(|| EvolutionError::NotFound(format!("experiment {id}")))?;
        if exp.is_running() {
            self.active_count.fetch_sub(1, Ordering::Relaxed);
        }
        exp.cancel();
        Ok(())
    }

    pub fn get_experiment(&self, id: EvolutionId) -> Option<Experiment> {
        self.experiments.get(&id).map(|e| e.clone())
    }

    pub fn list_experiments(&self) -> Vec<Experiment> {
        self.experiments.iter().map(|e| e.value().clone()).collect()
    }

    pub fn list_by_status(&self, status: EvolutionStatus) -> Vec<Experiment> {
        self.experiments
            .iter()
            .filter(|e| e.status == status)
            .map(|e| e.value().clone())
            .collect()
    }

    pub fn get_active_count(&self) -> usize {
        self.active_count.load(Ordering::Relaxed)
    }

    pub fn get_result(&self, id: EvolutionId) -> Option<ExperimentResult> {
        self.results.get(&id).map(|r| r.clone())
    }
}
