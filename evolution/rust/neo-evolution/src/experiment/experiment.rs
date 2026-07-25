use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::{EvolutionId, EvolutionStatus, SubsystemTarget};

/// Type of experiment to run.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExperimentType {
    ABTest,
    IsolatedExecution,
    ShadowExecution,
    ReplayTest,
    BenchmarkComparison,
}

impl std::fmt::Display for ExperimentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ABTest => write!(f, "ab_test"),
            Self::IsolatedExecution => write!(f, "isolated_execution"),
            Self::ShadowExecution => write!(f, "shadow_execution"),
            Self::ReplayTest => write!(f, "replay_test"),
            Self::BenchmarkComparison => write!(f, "benchmark_comparison"),
        }
    }
}

/// Configuration for a single experiment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentConfig {
    pub id: EvolutionId,
    pub name: String,
    pub experiment_type: ExperimentType,
    pub target: SubsystemTarget,
    pub parameters: HashMap<String, f64>,
    pub timeout_secs: u64,
    pub max_iterations: usize,
    pub compare_with_baseline: bool,
}

impl ExperimentConfig {
    pub fn new(
        name: impl Into<String>,
        experiment_type: ExperimentType,
        target: SubsystemTarget,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            name: name.into(),
            experiment_type,
            target,
            parameters: HashMap::new(),
            timeout_secs: 300,
            max_iterations: 100,
            compare_with_baseline: true,
        }
    }
}

/// A running or completed experiment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experiment {
    pub config: ExperimentConfig,
    pub status: EvolutionStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl Experiment {
    pub fn new(config: ExperimentConfig) -> Self {
        Self {
            config,
            status: EvolutionStatus::Pending,
            started_at: None,
            completed_at: None,
            created_at: Utc::now(),
        }
    }

    pub fn start(&mut self) {
        self.status = EvolutionStatus::Running;
        self.started_at = Some(Utc::now());
    }

    pub fn complete(&mut self) {
        self.status = EvolutionStatus::Completed;
        self.completed_at = Some(Utc::now());
    }

    pub fn fail(&mut self) {
        self.status = EvolutionStatus::Failed;
        self.completed_at = Some(Utc::now());
    }

    pub fn cancel(&mut self) {
        self.status = EvolutionStatus::Cancelled;
        self.completed_at = Some(Utc::now());
    }

    pub fn is_running(&self) -> bool {
        self.status == EvolutionStatus::Running
    }

    pub fn is_complete(&self) -> bool {
        self.status == EvolutionStatus::Completed
    }

    pub fn duration_ms(&self) -> Option<u64> {
        match (self.started_at, self.completed_at) {
            (Some(s), Some(e)) => Some((e - s).num_milliseconds() as u64),
            _ => None,
        }
    }
}
