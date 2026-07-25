use std::collections::HashMap;
use std::sync::Arc;

use crate::config::EvolutionConfiguration;
use crate::heuristic_evolution::candidate::HeuristicCandidate;
use crate::heuristic_evolution::heuristic::Heuristic;
use crate::heuristic_evolution::repository::HeuristicRepository;
use crate::types::SubsystemTarget;

#[derive(Debug, Clone)]
pub struct HeuristicEvolutionStats {
    pub total_heuristics: usize,
    pub active_count: usize,
    pub retired_count: usize,
    pub avg_score: f64,
    pub best_score: f64,
}

pub struct HeuristicEvolution {
    repository: HeuristicRepository,
    config: EvolutionConfiguration,
}

impl HeuristicEvolution {
    pub fn new(config: EvolutionConfiguration) -> Arc<Self> {
        Arc::new(Self {
            repository: HeuristicRepository::new(),
            config,
        })
    }

    pub fn repository(&self) -> &HeuristicRepository {
        &self.repository
    }

    pub fn generate_candidates(&self, category: SubsystemTarget) -> Vec<HeuristicCandidate> {
        let heuristics = self.repository.list_by_category(category);
        heuristics
            .iter()
            .map(|h| {
                let changes: HashMap<String, f64> = h
                    .parameters
                    .iter()
                    .map(|(k, v)| (k.clone(), v * (1.0 + (rand::random::<f64>() - 0.5) * 0.2)))
                    .collect();
                HeuristicCandidate::from_existing(h, changes)
            })
            .collect()
    }

    pub fn evaluate_candidate(&self, candidate: &HeuristicCandidate) -> f64 {
        candidate.confidence * 0.6 + candidate.expected_improvement * 0.4
    }

    pub fn evolve(&self, category: SubsystemTarget) -> Option<Heuristic> {
        let candidates = self.generate_candidates(category);
        let best = candidates.iter().max_by(|a, b| {
            self.evaluate_candidate(a)
                .partial_cmp(&self.evaluate_candidate(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })?;

        let mut new_heuristic = best
            .base_heuristic
            .clone()
            .unwrap_or_else(|| Heuristic::new("evolved", "auto-generated", category));
        for (k, v) in &best.proposed_changes {
            new_heuristic.parameters.insert(k.clone(), *v);
        }
        new_heuristic.update_score(self.evaluate_candidate(best));

        self.repository.save(new_heuristic.clone());
        Some(new_heuristic)
    }

    pub fn prune(&self, threshold: f64) {
        let active = self.repository.get_active();
        for h in active {
            if h.score < threshold && h.usage_count == 0 {
                let _ = self.repository.retire(h.id);
            }
        }
    }

    pub fn get_stats(&self) -> HeuristicEvolutionStats {
        let all = self.repository.list_all();
        let total = all.len();
        let active = all.iter().filter(|h| !h.retired).count();
        let retired = total - active;
        let avg = if total > 0 {
            all.iter().map(|h| h.score).sum::<f64>() / total as f64
        } else {
            0.0
        };
        let best = all.iter().map(|h| h.score).fold(0.0f64, f64::max);

        HeuristicEvolutionStats {
            total_heuristics: total,
            active_count: active,
            retired_count: retired,
            avg_score: avg,
            best_score: best,
        }
    }
}
