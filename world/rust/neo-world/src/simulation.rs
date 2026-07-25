use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::{Confidence, SimulationId, SimulationState};

/// A simulation scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationScenario {
    pub name: String,
    pub description: String,
    pub initial_state: serde_json::Value,
    pub actions: Vec<SimulationAction>,
    pub expected_outcome: Option<String>,
}

/// An action within a simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationAction {
    pub action_type: String,
    pub target: Option<String>,
    pub parameters: serde_json::Value,
}

/// Result of a simulation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub simulation_id: SimulationId,
    pub scenario_name: String,
    pub state: SimulationState,
    pub final_state: serde_json::Value,
    pub events_generated: usize,
    pub entities_affected: Vec<String>,
    pub outcome: String,
    pub confidence: Confidence,
    pub duration_ms: u64,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Executes what-if simulations on isolated state copies.
pub struct SimulationEngine {
    results: dashmap::DashMap<SimulationId, SimulationResult>,
    max_concurrent: usize,
    running_count: std::sync::atomic::AtomicUsize,
}

impl SimulationEngine {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            results: dashmap::DashMap::new(),
            max_concurrent,
            running_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Run a simulation on an isolated copy of state.
    pub fn run(
        &self,
        scenario: &SimulationScenario,
        world_state: &serde_json::Value,
    ) -> Result<SimulationId, String> {
        let current = self.running_count.load(std::sync::atomic::Ordering::Relaxed);
        if current >= self.max_concurrent {
            return Err("max concurrent simulations reached".into());
        }

        self.running_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let sim_id = SimulationId::random();
        let start = Utc::now();

        // Execute on isolated copy — never touch production state.
        let mut isolated_state = world_state.clone();
        let mut events_generated = 0;

        for action in &scenario.actions {
            // Apply action to isolated state.
            if let Some(state_obj) = isolated_state.as_object_mut() {
                state_obj.insert(
                    format!("sim_event_{}", events_generated),
                    serde_json::json!({
                        "type": action.action_type,
                        "target": action.target,
                    }),
                );
            }
            events_generated += 1;
        }

        let completed_at = Utc::now();
        let duration = (completed_at - start).num_milliseconds() as u64;

        let result = SimulationResult {
            simulation_id: sim_id.clone(),
            scenario_name: scenario.name.clone(),
            state: SimulationState::Completed,
            final_state: isolated_state,
            events_generated,
            entities_affected: Vec::new(),
            outcome: format!("Simulation '{}' completed: {events_generated} events", scenario.name),
            confidence: Confidence::MEDIUM,
            duration_ms: duration,
            started_at: start,
            completed_at: Some(completed_at),
        };

        self.results.insert(sim_id.clone(), result);
        self.running_count.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        Ok(sim_id)
    }

    pub fn get_result(&self, id: &SimulationId) -> Option<SimulationResult> {
        self.results.get(id).map(|r| r.value().clone())
    }

    pub fn recent(&self, count: usize) -> Vec<SimulationResult> {
        let mut results: Vec<SimulationResult> = self.results.iter().map(|r| r.value().clone()).collect();
        results.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        results.into_iter().take(count).collect()
    }

    pub fn total_runs(&self) -> usize {
        self.results.len()
    }

    pub fn currently_running(&self) -> usize {
        self.running_count.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl Default for SimulationEngine {
    fn default() -> Self {
        Self::new(10)
    }
}
