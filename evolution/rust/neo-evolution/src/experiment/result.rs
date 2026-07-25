use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::EvolutionId;

/// Metrics captured during an experiment run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExperimentMetrics {
    pub throughput: f64,
    pub latency_ms: f64,
    pub accuracy: f64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub custom: HashMap<String, f64>,
}

/// Comparison between baseline and candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResult {
    pub improved: Vec<String>,
    pub degraded: Vec<String>,
    pub unchanged: Vec<String>,
    pub overall_improvement: f64,
}

/// Result of a completed experiment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResult {
    pub experiment_id: EvolutionId,
    pub success: bool,
    pub metrics: ExperimentMetrics,
    pub baseline_metrics: Option<ExperimentMetrics>,
    pub comparison: Option<ComparisonResult>,
    pub errors: Vec<String>,
    pub output_data: HashMap<String, serde_json::Value>,
    pub duration_ms: u64,
    pub completed_at: DateTime<Utc>,
}

impl ExperimentResult {
    pub fn is_improvement(&self) -> bool {
        self.comparison
            .as_ref()
            .map_or(false, |c| c.overall_improvement > 0.0)
    }

    pub fn improvement_ratio(&self) -> f64 {
        self.comparison
            .as_ref()
            .map_or(0.0, |c| c.overall_improvement)
    }

    pub fn summary(&self) -> String {
        let status = if self.success { "success" } else { "failed" };
        let imp = if self.is_improvement() {
            format!(" (+{:.1}%)", self.improvement_ratio() * 100.0)
        } else {
            String::new()
        };
        format!(
            "[{}] {} — {:.1}ms{}",
            status, self.experiment_id, self.duration_ms as f64, imp
        )
    }
}
