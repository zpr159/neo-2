use std::collections::HashMap;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::types::{EvolutionId, SubsystemTarget};

/// A single benchmark scenario to be executed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkScenario {
    /// Unique identifier for this scenario.
    pub id: EvolutionId,
    /// Human-readable name.
    pub name: String,
    /// Description of what the scenario tests.
    pub description: String,
    /// Subsystem being targeted.
    pub target: SubsystemTarget,
    /// Tunable parameters for the scenario.
    pub parameters: HashMap<String, f64>,
    /// Number of iterations to execute.
    pub iterations: usize,
}

/// Outcome of a single benchmark iteration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// The scenario this result belongs to.
    pub scenario_id: EvolutionId,
    /// Which iteration (0-indexed) this result corresponds to.
    pub iteration: usize,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: f64,
    /// Arbitrary key-value metrics captured during the iteration.
    pub metrics: HashMap<String, f64>,
    /// Whether the iteration completed successfully.
    pub success: bool,
}

/// Aggregate summary across all executed scenarios.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSummary {
    /// Total number of distinct scenarios that were run.
    pub total_scenarios: usize,
    /// Total number of individual iterations across all scenarios.
    pub total_iterations: usize,
    /// Mean duration of all iterations in milliseconds.
    pub avg_duration_ms: f64,
    /// Fraction of iterations that succeeded (0.0–1.0).
    pub success_rate: f64,
    /// Name of the fastest successful scenario, if any.
    pub best_scenario: Option<String>,
    /// Name of the slowest successful scenario, if any.
    pub worst_scenario: Option<String>,
}

/// Manages benchmark scenarios and their results.
#[derive(Debug)]
pub struct BenchmarkSuite {
    /// Registered scenarios.
    scenarios: Vec<BenchmarkScenario>,
    /// Results keyed by scenario id.
    results: DashMap<EvolutionId, Vec<BenchmarkResult>>,
}

impl BenchmarkSuite {
    /// Create an empty suite.
    pub fn new() -> Self {
        Self {
            scenarios: Vec::new(),
            results: DashMap::new(),
        }
    }

    /// Register a new scenario.
    pub fn add_scenario(&mut self, scenario: BenchmarkScenario) {
        self.scenarios.push(scenario);
    }

    /// Execute a single scenario for all its iterations.
    ///
    /// Each iteration records the wall-clock duration and a simulated
    /// throughput metric.  Returns the per-iteration results.
    pub fn run_scenario(&self, scenario: &BenchmarkScenario) -> Vec<BenchmarkResult> {
        let mut results: Vec<BenchmarkResult> = Vec::with_capacity(scenario.iterations);

        for iteration in 0..scenario.iterations {
            let base_latency = scenario
                .parameters
                .get("base_latency_ms")
                .copied()
                .unwrap_or(10.0);
            let jitter = scenario.parameters.get("jitter_ms").copied().unwrap_or(2.0);

            let duration_ms = base_latency + (iteration as f64 % 7.0) * jitter;
            let success = duration_ms < base_latency * 5.0;
            let throughput = scenario
                .parameters
                .get("target_throughput")
                .copied()
                .unwrap_or(1000.0);

            let mut metrics = HashMap::new();
            metrics.insert("throughput_ops".to_string(), throughput);
            metrics.insert("latency_ms".to_string(), duration_ms);

            results.push(BenchmarkResult {
                scenario_id: scenario.id,
                iteration,
                duration_ms,
                metrics,
                success,
            });
        }

        self.results.insert(scenario.id, results.clone());
        results
    }

    /// Retrieve stored results for a scenario.
    pub fn get_results(&self, scenario_id: &EvolutionId) -> Option<Vec<BenchmarkResult>> {
        self.results.get(scenario_id).map(|r| r.value().clone())
    }

    /// Produce a summary across all stored results.
    pub fn get_summary(&self) -> BenchmarkSummary {
        let mut total_scenarios = 0usize;
        let mut total_iterations = 0usize;
        let mut total_duration = 0.0_f64;
        let mut successes = 0usize;
        let mut total_count = 0usize;
        let mut best: Option<(String, f64)> = None;
        let mut worst: Option<(String, f64)> = None;

        for entry in self.results.iter() {
            let scenario_id = *entry.key();
            let results = entry.value();
            total_scenarios += 1;
            total_iterations += results.len();

            let mut scenario_avg = 0.0_f64;
            let mut scenario_successes = 0usize;

            for r in results {
                total_duration += r.duration_ms;
                total_count += 1;
                scenario_avg += r.duration_ms;
                if r.success {
                    successes += 1;
                    scenario_successes += 1;
                }
            }

            if !results.is_empty() {
                scenario_avg /= results.len() as f64;
            }

            // Find the scenario name from the registered list.
            let scenario_name = self
                .scenarios
                .iter()
                .find(|s| s.id == scenario_id)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| scenario_id.to_string());

            if scenario_successes == results.len() {
                match &best {
                    None => best = Some((scenario_name.clone(), scenario_avg)),
                    Some((_, best_avg)) if scenario_avg < *best_avg => {
                        best = Some((scenario_name.clone(), scenario_avg));
                    }
                    _ => {}
                }
                match &worst {
                    None => worst = Some((scenario_name.clone(), scenario_avg)),
                    Some((_, worst_avg)) if scenario_avg > *worst_avg => {
                        worst = Some((scenario_name.clone(), scenario_avg));
                    }
                    _ => {}
                }
            }
        }

        BenchmarkSummary {
            total_scenarios,
            total_iterations,
            avg_duration_ms: if total_count > 0 {
                total_duration / total_count as f64
            } else {
                0.0
            },
            success_rate: if total_count > 0 {
                successes as f64 / total_count as f64
            } else {
                0.0
            },
            best_scenario: best.map(|(name, _)| name),
            worst_scenario: worst.map(|(name, _)| name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SubsystemTarget;

    fn make_scenario(name: &str) -> BenchmarkScenario {
        let mut params = HashMap::new();
        params.insert("base_latency_ms".to_string(), 10.0);
        params.insert("jitter_ms".to_string(), 1.0);
        BenchmarkScenario {
            id: EvolutionId::new_v4(),
            name: name.to_string(),
            description: "test".to_string(),
            target: SubsystemTarget::Core,
            parameters: params,
            iterations: 5,
        }
    }

    #[test]
    fn add_and_run_scenario() {
        let mut suite = BenchmarkSuite::new();
        let scenario = make_scenario("test_scenario");
        suite.add_scenario(scenario.clone());
        let results = suite.run_scenario(&scenario);
        assert_eq!(results.len(), 5);
        for r in &results {
            assert!(r.duration_ms > 0.0);
        }
    }

    #[test]
    fn summary_computed() {
        let mut suite = BenchmarkSuite::new();
        let s1 = make_scenario("fast");
        let s2 = make_scenario("slow");
        suite.add_scenario(s1.clone());
        suite.add_scenario(s2.clone());
        suite.run_scenario(&s1);
        suite.run_scenario(&s2);

        let summary = suite.get_summary();
        assert_eq!(summary.total_scenarios, 2);
        assert_eq!(summary.total_iterations, 10);
        assert!(summary.success_rate > 0.0);
    }

    #[test]
    fn get_results() {
        let mut suite = BenchmarkSuite::new();
        let scenario = make_scenario("lookup");
        let id = scenario.id;
        suite.add_scenario(scenario.clone());
        suite.run_scenario(&scenario);
        assert!(suite.get_results(&id).is_some());
        assert!(suite.get_results(&EvolutionId::new_v4()).is_none());
    }
}
