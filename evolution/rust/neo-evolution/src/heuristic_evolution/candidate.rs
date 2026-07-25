use std::collections::HashMap;

use crate::heuristic_evolution::heuristic::Heuristic;

#[derive(Debug, Clone)]
pub struct HeuristicCandidate {
    pub base_heuristic: Option<Heuristic>,
    pub proposed_changes: HashMap<String, f64>,
    pub expected_improvement: f64,
    pub confidence: f64,
}

impl HeuristicCandidate {
    pub fn new(expected_improvement: f64, confidence: f64) -> Self {
        Self {
            base_heuristic: None,
            proposed_changes: HashMap::new(),
            expected_improvement,
            confidence,
        }
    }

    pub fn from_existing(heuristic: &Heuristic, changes: HashMap<String, f64>) -> Self {
        let expected = changes.values().map(|v| v.abs()).sum::<f64>() / changes.len().max(1) as f64;
        Self {
            base_heuristic: Some(heuristic.clone()),
            proposed_changes: changes,
            expected_improvement: expected,
            confidence: 0.7,
        }
    }

    pub fn get_parameters(&self) -> HashMap<String, f64> {
        let mut params = self
            .base_heuristic
            .as_ref()
            .map_or(HashMap::new(), |h| h.parameters.clone());
        for (k, v) in &self.proposed_changes {
            params.insert(k.clone(), *v);
        }
        params
    }
}
