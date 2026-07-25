use dashmap::DashMap;

use crate::error::EvolutionResult;
use crate::heuristic_evolution::heuristic::Heuristic;
use crate::types::{EvolutionId, SubsystemTarget};

pub struct HeuristicRepository {
    store: DashMap<EvolutionId, Heuristic>,
}

impl HeuristicRepository {
    pub fn new() -> Self {
        Self {
            store: DashMap::new(),
        }
    }

    pub fn save(&self, heuristic: Heuristic) {
        let id = heuristic.id;
        self.store.insert(id, heuristic);
    }

    pub fn load(&self, id: EvolutionId) -> Option<Heuristic> {
        self.store.get(&id).map(|h| h.value().clone())
    }

    pub fn list_all(&self) -> Vec<Heuristic> {
        self.store.iter().map(|h| h.value().clone()).collect()
    }

    pub fn list_by_category(&self, category: SubsystemTarget) -> Vec<Heuristic> {
        self.store
            .iter()
            .filter(|h| h.category == category)
            .map(|h| h.value().clone())
            .collect()
    }

    pub fn get_top_n(&self, n: usize) -> Vec<Heuristic> {
        let mut all: Vec<Heuristic> = self
            .store
            .iter()
            .filter(|h| !h.retired)
            .map(|h| h.value().clone())
            .collect();
        all.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        all.truncate(n);
        all
    }

    pub fn retire(&self, id: EvolutionId) -> EvolutionResult<()> {
        if let Some(mut h) = self.store.get_mut(&id) {
            h.retire();
            Ok(())
        } else {
            Err(crate::error::EvolutionError::NotFound(format!(
                "heuristic {id}"
            )))
        }
    }

    pub fn get_active(&self) -> Vec<Heuristic> {
        self.store
            .iter()
            .filter(|h| !h.retired)
            .map(|h| h.value().clone())
            .collect()
    }

    pub fn count(&self) -> usize {
        self.store.len()
    }
}

impl Default for HeuristicRepository {
    fn default() -> Self {
        Self::new()
    }
}
