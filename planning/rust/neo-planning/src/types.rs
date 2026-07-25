#![forbid(unsafe_code)]
#![deny(
    missing_docs,
    warnings,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_extern_crates
)]

//! Neo Planning System — hierarchical planning, goal decomposition,
//! strategy generation, plan optimization, and dynamic replanning.
//!
//! This module provides the complete production-grade planning system for Neo AGI OS,
//! including goal-based planning, strategy generation, plan optimization,
//! dynamic replanning, and support for hierarchical task networks (HTN),
//! goal-oriented action planning (GOAP), and other advanced planning algorithms.
//!
//! The planning system bridges between the Reasoning Engine and the Execution System,
//! transforming abstract goals into executable plans while continuously adapting
//! to environmental changes and new information.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use derive_more::Display;
use neo_core::error::Result;

pub type PlanId = crate::id::PlanId;
pub type PlanVersion = crate::id::PlanVersion;
pub type PlanCheckpointId = crate::id::PlanCheckpointId;
pub type PlanningSessionId = crate::id::PlanningSessionId;
pub type StrategyId = crate::id::StrategyId;
pub type PlanningGoalId = crate::id::PlanningGoalId;
pub type PlanningNodeId = crate::id::PlanningNodeId;
pub type AlgorithmId = crate::id::AlgorithmId;
pub type ResourceAllocationId = crate::id::ResourceAllocationId;
pub type RiskAssessmentId = crate::id::RiskAssessmentId;
pub type CostEstimateId = crate::id::CostEstimateId;
pub type OptimizationPassId = crate::id::OptimizationPassId;
pub type ReplanEventId = crate::id::ReplanEventId;
pub type AgentAllocationId = crate::id::AgentAllocationId;
pub type GeneratedWorkflowId = crate::id::GeneratedWorkflowId;

pub mod infrastructure;
pub mod goal;
pub mod plan;
pub mod algorithm;
pub mod strategy;
pub mod engine;
pub mod optimizer;
pub mod replanner;
pub mod risk;
pub mod cost;
pub mod analytics;
pub mod persistence;
pub mod multi_agent;
pub mod resource;
pub mod workflow_integration;
pub mod tool_integration;
pub mod capability_integration;
pub mod knowledge_integration;
pub mod memory_integration;
pub mod reasoning_integration;
pub mod executive_integration;
pub mod security;
pub mod sdk;

/// Convenient re-exports for common types.
pub mod prelude {
    pub use super::engine::PlanningEngine;
    pub use super::goal::{Goal, GoalBuilder, GoalId, GoalPriority, GoalStatus, GoalType};
    pub use super::plan::{Plan, PlanBuilder, PlanState};
    pub use super::strategy::{Strategy, StrategyComparison, StrategyPolicy};
    pub use super::algorithm::PlanningAlgorithm;
    pub use super::types::{PlanContext, PlanMetrics, PlanStatistics};
    pub use super::engine::{PlanningSession, PlanningConfiguration};
    pub use super::optimizer::{PlanOptimizer, OptimizationRule};
}

/// Core planning types and error types.
pub mod types {
    use super::*;
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PlanMetadata {
        pub name: String,
        pub description: Option<String>,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
        pub tags: Vec<String>,
        pub extra: HashMap<String, String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ResourceRequirements {
        pub agents: u32,
        pub cpu_units: u32,
        pub memory_mb: u32,
        pub network_bandwidth: u32,
        pub storage_gb: u32,
        pub custom: HashMap<String, f64>,
    }

    impl Default for ResourceRequirements {
        fn default() -> Self {
            Self {
                agents: 0,
                cpu_units: 0,
                memory_mb: 0,
                network_bandwidth: 0,
                storage_gb: 0,
                custom: HashMap::new(),
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ExecutionBudget {
        pub max_cost: f64,
        pub max_time_seconds: u64,
        pub max_resources: ResourceRequirements,
        pub required_capabilities: Vec<String>,
    }

    impl Default for ExecutionBudget {
        fn default() -> Self {
            Self {
                max_cost: 0.0,
                max_time_seconds: 0,
                max_resources: ResourceRequirements::default(),
                required_capabilities: Vec::new(),
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display)]
    pub enum AlgorithmType {
        #[display(fmt = "HTN")]
        HierarchicalTaskNetwork,
        #[display(fmt = "GOAP")]
        GoalOrientedActionPlanning,
        #[display(fmt = "A*")]
        AStar,
        #[display(fmt = "BFS")]
        BreadthFirstSearch,
        #[display(fmt = "DFS")]
        DepthFirstSearch,
        #[display(fmt = "DependencyGraph")]
        DependencyGraphPlanning,
        #[display(fmt = "ConstraintSatisfaction")]
        ConstraintSatisfactionPlanning,
        #[display(fmt = "ResourceConstrained")]
        ResourceConstrainedPlanning,
        #[display(fmt = "CostBasedOptimization")]
        CostBasedOptimization,
        #[display(fmt = "Custom")]
        Custom(String),
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AlgorithmConfig {
        pub algorithm_type: AlgorithmType,
        pub max_depth: usize,
        pub max_iterations: usize,
        pub timeout_ms: u64,
        pub heuristic_weight: f64,
        pub allow_suboptimal: bool,
    }

    impl Default for AlgorithmConfig {
        fn default() -> Self {
            Self {
                algorithm_type: AlgorithmType::HierarchicalTaskNetwork,
                max_depth: 16,
                max_iterations: 1000,
                timeout_ms: 30_000,
                heuristic_weight: 1.0,
                allow_suboptimal: true,
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PlanningEdgeType {
        #[serde(rename = "type")]
        pub type_: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Duration {
        pub duration_ms: u64,
        pub planning_time_ms: u64,
        pub execution_time_ms: u64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PlanningAnalytics {
        pub planning_latency: Duration,
        pub optimization_gains: f32,
        pub success_rates: f32,
        pub replanning_frequency: f32,
        pub execution_efficiency: f32,
        pub resource_utilization: f32,
        pub generated_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PlanningResult {
        pub plan_id: PlanId,
        pub success: bool,
        pub plan: Option<Plan>,
        pub execution: Option<PlanExecution>,
        pub error: Option<String>,
        pub metrics: Option<PlanMetrics>,
        pub started_at: DateTime<Utc>,
        pub completed_at: Option<DateTime<Utc>>,
    }

    impl PlanningResult {
        /// Create a successful result
        pub fn success(plan_id: PlanId, metrics: PlanMetrics) -> Self {
            Self {
                plan_id,
                success: true,
                plan: None,
                execution: None,
                error: None,
                metrics: Some(metrics),
                started_at: Utc::now(),
                completed_at: Some(Utc::now()),
            }
        }

        /// Create a failed result
        pub fn failure(plan_id: PlanId, error: impl Into<String>) -> Self {
            Self {
                plan_id,
                success: false,
                plan: None,
                execution: None,
                error: Some(error.into()),
                metrics: None,
                started_at: Utc::now(),
                completed_at: Some(Utc::now()),
            }
        }

        /// Get status for CLI output
        pub fn status(&self) -> &'static str {
            if self.success {
                "Success"
            } else {
                "Failed"
            }
        }
    }

    #[derive(Debug, Clone, Default)]
    pub struct PlanningAnalytics {
        pub planning_latency_ms: u64,
        pub optimization_gains: f32,
        pub success_rate: f32,
        pub replanning_frequency: f32,
        pub execution_efficiency: f32,
        pub resource_utilization: f32,
        pub generated_at: DateTime<Utc>,
    }

    impl PlanningAnalytics {
        pub fn new() -> Self {
            Self {
                planning_latency_ms: 0,
                optimization_gains: 0.0,
                success_rate: 0.0,
                replanning_frequency: 0.0,
                execution_efficiency: 0.0,
                resource_utilization: 0.0,
                generated_at: Utc::now(),
            }
        }
    }
}

pub mod infrastructure {
    pub mod events;
    pub mod cache;
    pub mod repository;
    pub mod session;
    pub mod analytics;
}
