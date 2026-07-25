use serde::{Deserialize, Serialize};

use crate::types::{EvolutionId, SubsystemTarget};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetEvolutionStatusResponse {
    pub status: String,
    pub phase: String,
    pub completed_cycles: u64,
    pub failed_cycles: u64,
    pub active_subsystems: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub target: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListExperimentsResponse {
    pub experiments: Vec<ExperimentInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSummaryInfo {
    pub total_scenarios: usize,
    pub avg_duration_ms: f64,
    pub success_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListBenchmarksResponse {
    pub summaries: Vec<BenchmarkSummaryInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationInfo {
    pub target: String,
    pub improvement_percent: f64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListOptimizationsResponse {
    pub optimizations: Vec<OptimizationInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunEvolutionRequest {
    pub target: SubsystemTarget,
    pub max_iterations: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunEvolutionResponse {
    pub evolution_id: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackRequest {
    pub evolution_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetMetricsResponse {
    pub successful_improvements: u64,
    pub failed_experiments: u64,
    pub rollbacks: u64,
    pub total_experiments: u64,
    pub total_benchmarks: u64,
}
