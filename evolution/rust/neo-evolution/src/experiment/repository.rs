use dashmap::DashMap;

use crate::error::{EvolutionError, EvolutionResult};
use crate::experiment::experiment::Experiment;
use crate::types::{EvolutionId, EvolutionStatus, SubsystemTarget};

/// In-memory repository for experiment persistence.
pub struct ExperimentRepository {
    store: DashMap<String, Experiment>,
}

impl ExperimentRepository {
    pub fn new() -> Self {
        Self {
            store: DashMap::new(),
        }
    }

    pub fn save(&self, experiment: Experiment) -> EvolutionResult<()> {
        let key = experiment.config.id.to_string();
        self.store.insert(key, experiment);
        Ok(())
    }

    pub fn load(&self, id: &str) -> EvolutionResult<Option<Experiment>> {
        Ok(self.store.get(id).map(|e| e.value().clone()))
    }

    pub fn list_all(&self) -> Vec<Experiment> {
        self.store.iter().map(|e| e.value().clone()).collect()
    }

    pub fn list_by_target(&self, target: SubsystemTarget) -> Vec<Experiment> {
        self.store
            .iter()
            .filter(|e| e.config.target == target)
            .map(|e| e.value().clone())
            .collect()
    }

    pub fn list_by_status(&self, status: EvolutionStatus) -> Vec<Experiment> {
        self.store
            .iter()
            .filter(|e| e.status == status)
            .map(|e| e.value().clone())
            .collect()
    }

    pub fn delete(&self, id: &str) -> EvolutionResult<bool> {
        Ok(self.store.remove(id).is_some())
    }

    pub fn count(&self) -> usize {
        self.store.len()
    }
}

impl Default for ExperimentRepository {
    fn default() -> Self {
        Self::new()
    }
}
