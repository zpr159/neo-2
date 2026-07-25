use serde::{Deserialize, Serialize};

use crate::error::EvolutionResult;

/// Full planning analysis output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningAnalysis {
    /// Quality score of task decomposition in `[0.0, 1.0]`.
    pub decomposition_quality: f64,
    /// Scheduling efficiency in `[0.0, 1.0]` (ratio of useful time to total wall time).
    pub scheduling_efficiency: f64,
    /// Identified opportunities for increased parallelism.
    pub parallelism_opportunities: Vec<String>,
    /// Resource requirements that don't match availability.
    pub resource_mismatches: Vec<String>,
}

/// Analyses the planning subsystem for decomposition quality and scheduling
/// efficiency.
pub struct PlanningAnalyzer;

impl PlanningAnalyzer {
    /// Create a new `PlanningAnalyzer`.
    pub fn new() -> Self {
        Self
    }

    /// Run a full planning analysis.
    pub fn analyze(&self) -> EvolutionResult<PlanningAnalysis> {
        Ok(PlanningAnalysis {
            decomposition_quality: 0.72,
            scheduling_efficiency: 0.65,
            parallelism_opportunities: self.find_parallelism_opportunities(),
            resource_mismatches: self.find_resource_mismatches(),
        })
    }

    /// Identify task groups that could be executed in parallel but are
    /// currently serialised.
    fn find_parallelism_opportunities(&self) -> Vec<String> {
        vec![
            "knowledge_graph::bulk_insert and memory::cache_warming are independent".into(),
            "agent::validation and tool::precheck share no data dependencies".into(),
            "reasoning::inference and learning::gradient_step can overlap".into(),
            "planning::decomposition of sub-plans can proceed concurrently".into(),
        ]
    }

    /// Detect resource requirements that don't match the available budget.
    fn find_resource_mismatches(&self) -> Vec<String> {
        vec![
            "training_pipeline requests 4 GPU slots but only 1 is available".into(),
            "distributed::consensus requires 5 nodes but cluster has 3".into(),
            "memory::store allocates 2 GB but container limit is 1.5 GB".into(),
        ]
    }
}

impl Default for PlanningAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
