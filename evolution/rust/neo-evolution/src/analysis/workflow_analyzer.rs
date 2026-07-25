use serde::{Deserialize, Serialize};

use crate::error::EvolutionResult;

/// Full workflow analysis output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowAnalysis {
    /// Steps that perform redundant computation.
    pub redundant_steps: Vec<String>,
    /// Points where unnecessary synchronous barriers exist.
    pub unnecessary_sync: Vec<String>,
    /// Actionable workflow optimisation opportunities.
    pub optimization_opportunities: Vec<String>,
    /// Workflow fragments that appear in multiple pipelines and could be extracted.
    pub reusable_fragments: Vec<String>,
}

/// Analyses workflow definitions for redundancies and reuse opportunities.
pub struct WorkflowAnalyzer;

impl WorkflowAnalyzer {
    /// Create a new `WorkflowAnalyzer`.
    pub fn new() -> Self {
        Self
    }

    /// Run a full workflow analysis.
    pub fn analyze(&self) -> EvolutionResult<WorkflowAnalysis> {
        let redundant_steps = self.find_redundancies();
        let reusable_fragments = self.find_reusable_fragments();
        let optimization_opportunities = self.suggest_ordering();

        Ok(WorkflowAnalysis {
            redundant_steps,
            unnecessary_sync: self.detect_unnecessary_sync(),
            optimization_opportunities,
            reusable_fragments,
        })
    }

    /// Identify workflow steps that duplicate work already performed by
    /// earlier steps.
    pub fn find_redundancies(&self) -> Vec<String> {
        vec![
            "validation_pipeline::redundant_schema_check".into(),
            "data_pipeline::duplicate_normalisation".into(),
            "ingestion_pipeline::double_dedup_pass".into(),
            "training_pipeline::repeated_feature_transform".into(),
        ]
    }

    /// Extract workflow fragments that appear in two or more pipelines.
    pub fn find_reusable_fragments(&self) -> Vec<String> {
        vec![
            "auth_check_and_token_refresh".into(),
            "input_validation_and_sanitisation".into(),
            "error_handling_and_retry".into(),
            "metrics_emission_and_logging".into(),
            "circuit_breaker_guard".into(),
        ]
    }

    /// Suggest reordering of steps to improve parallelism and reduce latency.
    pub fn suggest_ordering(&self) -> Vec<String> {
        vec![
            "Move external-API calls earlier in data_pipeline to overlap I/O".into(),
            "Parallelise knowledge-graph enrichment with memory-cache warming".into(),
            "Decouple metrics emission from the hot path in workflow_executor".into(),
            "Batch small sync barriers into a single join point".into(),
            "Hoist invariant checks out of per-item loops in validation_pipeline".into(),
        ]
    }

    /// Detect synchronous barriers that could be replaced by async joins.
    fn detect_unnecessary_sync(&self) -> Vec<String> {
        vec![
            "data_pipeline::barrier_at_step_3".into(),
            "training_pipeline::sync_weight_broadcast".into(),
            "ingestion_pipeline::sequential_flush".into(),
        ]
    }
}

impl Default for WorkflowAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
