//! # Neo Evolution
//!
//! Self-Evolution Infrastructure for the Neo AGI Operating System.
//!
//! This crate enables Neo to continuously analyse itself, safely evaluate
//! improvements, optimise its behaviour through controlled experimentation,
//! benchmark changes, evolve heuristics and workflows, and continuously
//! improve under strict governance and rollback controls.

pub mod agent_evolution;
pub mod analysis;
pub mod benchmark;
pub mod capability_evolution;
pub mod cli;
pub mod config;
pub mod context;
pub mod distributed_evolution;
pub mod error;
pub mod evolution_engine;
pub mod experiment;
pub mod governance;
pub mod heuristic_evolution;
pub mod improvement;
pub mod learning_evolution;
pub mod lifecycle;
pub mod metrics;
pub mod performance;
pub mod planning_evolution;
pub mod policy_evolution;
pub mod reasoning_evolution;
pub mod rest;
pub mod sandbox;
pub mod sdk;
pub mod state;
pub mod strategies;
pub mod types;
pub mod workflow_evolution;

pub use agent_evolution::evolution::AgentEvolution;
pub use benchmark::suite::BenchmarkSuite;
pub use capability_evolution::evolution::CapabilityEvolution;
pub use config::EvolutionConfiguration;
pub use context::EvolutionContext;
pub use distributed_evolution::evolution::DistributedEvolution;
pub use error::{EvolutionError, EvolutionResult};
pub use evolution_engine::EvolutionEngine;
pub use experiment::experiment::{Experiment, ExperimentConfig};
pub use experiment::manager::ExperimentManager;
pub use governance::approval::ApprovalManager;
pub use governance::audit::EvolutionAudit;
pub use governance::authorization::EvolutionAuthorization;
pub use governance::validator::EvolutionPolicyValidator;
pub use heuristic_evolution::evolution::HeuristicEvolution;
pub use improvement::engine::ImprovementEngine;
pub use lifecycle::EvolutionLifecycle;
pub use metrics::tracker::MetricsTracker;
pub use performance::engine::OptimizationEngine;
pub use policy_evolution::engine::PolicyEvolutionEngine;
pub use sandbox::sandbox::{Sandbox, SandboxConfig};
pub use state::{EvolutionSnapshot, EvolutionState};
pub use types::{
    EvolutionId, EvolutionPhase, EvolutionStatus, ImprovementCategory, RiskLevel, SubsystemTarget,
};
pub use workflow_evolution::evolution::WorkflowEvolution;

/// Convenience prelude with all commonly used types.
pub mod prelude {
    pub use crate::agent_evolution::evolution::AgentEvolution;
    pub use crate::benchmark::suite::BenchmarkSuite;
    pub use crate::capability_evolution::evolution::CapabilityEvolution;
    pub use crate::config::EvolutionConfiguration;
    pub use crate::context::EvolutionContext;
    pub use crate::distributed_evolution::evolution::DistributedEvolution;
    pub use crate::error::{EvolutionError, EvolutionResult};
    pub use crate::evolution_engine::EvolutionEngine;
    pub use crate::experiment::experiment::{Experiment, ExperimentConfig};
    pub use crate::experiment::manager::ExperimentManager;
    pub use crate::governance::approval::ApprovalManager;
    pub use crate::governance::audit::EvolutionAudit;
    pub use crate::governance::authorization::EvolutionAuthorization;
    pub use crate::governance::validator::EvolutionPolicyValidator;
    pub use crate::heuristic_evolution::evolution::HeuristicEvolution;
    pub use crate::improvement::engine::ImprovementEngine;
    pub use crate::lifecycle::EvolutionLifecycle;
    pub use crate::metrics::tracker::MetricsTracker;
    pub use crate::performance::engine::OptimizationEngine;
    pub use crate::policy_evolution::engine::PolicyEvolutionEngine;
    pub use crate::sandbox::sandbox::{Sandbox, SandboxConfig};
    pub use crate::sdk::builder::EvolutionEngineBuilder;
    pub use crate::state::{EvolutionSnapshot, EvolutionState};
    pub use crate::types::{
        EvolutionId, EvolutionPhase, EvolutionStatus, ImprovementCategory, RiskLevel,
        SubsystemTarget,
    };
    pub use crate::workflow_evolution::evolution::WorkflowEvolution;
}
