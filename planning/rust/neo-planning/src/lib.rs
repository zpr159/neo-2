//! Neo Planning System — hierarchical planning, goal decomposition,
//! strategy generation, plan optimization, and dynamic replanning.
//!
//! This module provides the complete production-grade planning system for Neo AGI OS,
//! including goal-based planning, strategy generation, plan optimization,
//! dynamic replanning, and support for hierarchical task networks (HTN),
//! goal-oriented action planning (GOAP), and other advanced planning algorithms.

pub mod algorithm;
pub mod analytics;
pub mod capability_integration;
pub mod cli;
pub mod cost;
pub mod engine;
pub mod error;
pub mod event;
pub mod executive_integration;
pub mod goal;
pub mod graph;
pub mod id;
pub mod knowledge_integration;
pub mod memory_integration;
pub mod multi_agent;
pub mod optimizer;
pub mod persistence;
pub mod plan;
pub mod reasoning_integration;
pub mod replanner;
pub mod resource;
pub mod risk;
pub mod rest;
pub mod sdk;
pub mod security;
pub mod strategy;
pub mod tool_integration;
pub mod types;
pub mod workflow_integration;

/// Library-level result alias.
pub type Result<T> = std::result::Result<T, error::PlanningError>;

/// Convenient re-exports for common types.
pub mod prelude {
    pub use super::engine::PlanningEngine;
    pub use super::goal::{Goal, GoalBuilder, GoalId, GoalPriority, GoalStatus, GoalType};
    pub use super::plan::{Plan, PlanBuilder, PlanId, PlanState};
    pub use super::strategy::{Strategy, StrategyComparison, StrategyPolicy};
    pub use super::algorithm::PlanningAlgorithm;
    pub use super::types::{PlanContext, PlanMetrics, PlanStatistics};
}
