#!\[forbid(unsafe_code)\]
#![deny(
    missing_docs,
    warnings,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_extern_crates
)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use async_trait::async_trait;
use super::types::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LearningPolicyType {
    Passive,
    Scheduled,
    EventDriven,
    ManualReview,
    SimulationOnly,
    Production,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningPolicy {
    pub policy_type: LearningPolicyType,
    pub cpu_limit: f64,
    pub memory_limit_mb: u32,
    pub storage_limit_gb: u32,
    pub max_duration_seconds: u64,
    pub learning_objectives: Vec<LearningObjective>,
    pub safety_checks: bool,
    pub audit_logging: bool,
    pub enable_retry_on_failure: bool,
    pub max_failures_before_stop: u32,
    pub confidence_threshold: f32,
    pub learning_rate: f32,
    pub exploration_factor: f32,
    pub consolidation_interval_seconds: u64,
    pub reflection_interval_seconds: u64,
    pub pattern_mining_enabled: bool,
    pub skill_extraction_enabled: bool,
    pub heuristic_optimization_enabled: bool,
}

impl Default for LearningPolicy {
    fn default() -> Self {
        Self {
            policy_type: LearningPolicyType::EventDriven,
            cpu_limit: 0.8,
            memory_limit_mb: 1024,
            storage_limit_gb: 10,
            max_duration_seconds: 3600,
            learning_objectives: Vec::new(),
            safety_checks: true,
            audit_logging: true,
            enable_retry_on_failure: true,
            max_failures_before_stop: 5,
            confidence_threshold: 0.7,
            learning_rate: 0.01,
            exploration_factor: 0.2,
            consolidation_interval_seconds: 300,
            reflection_interval_seconds: 60,
            pattern_mining_enabled: true,
            skill_extraction_enabled: true,
            heuristic_optimization_enabled: true,
        }
    }
}

impl LearningPolicy {
    pub fn safe_mode() -> Self {
        Self {
            policy_type: LearningPolicyType::ManualReview,
            cpu_limit: 0.5,
            memory_limit_mb: 512,
            storage_limit_gb: 5,
            max_duration_seconds: 1800,
            learning_objectives: vec![LearningObjective {
                id: LearningObjectiveId::new(),
                description: "Verify all learned artifacts".to_string(),
                priority: LearningObjectivePriority::High,
                success_criteria: vec!["All artifacts validated".to_string()],
            }],
            safety_checks: true,
            audit_logging: true,
            enable_retry_on_failure: true,
            max_failures_before_stop: 3,
            confidence_threshold: 0.8,
            learning_rate: 0.05,
            exploration_factor: 0.1,
            consolidation_interval_seconds: 600,
            reflection_interval_seconds: 300,
            pattern_mining_enabled: true,
            skill_extraction_enabled: false,
            heuristic_optimization_enabled: true,
        }
    }

    pub fn production_mode() -> Self {
        Self {
            policy_type: LearningPolicyType::EventDriven,
            cpu_limit: 0.9,
            memory_limit_mb: 2048,
            storage_limit_gb: 20,
            max_duration_seconds: 7200,
            learning_objectives: vec![LearningObjective {
                id: LearningObjectiveId::new(),
                description: "Improve planning heuristics".to_string(),
                priority: LearningObjectivePriority::Critical,
                success_criteria: vec!["All critical metrics improved".to_string()],
            }],
            safety_checks: true,
            audit_logging: true,
            enable_retry_on_failure: true,
            max_failures_before_stop: 10,
            confidence_threshold: 0.6,
            learning_rate: 0.02,
            exploration_factor: 0.3,
            consolidation_interval_seconds: 180,
            reflection_interval_seconds: 45,
            pattern_mining_enabled: true,
            skill_extraction_enabled: true,
            heuristic_optimization_enabled: true,
        }
    }

    pub fn simulation_mode() -> Self {
        Self {
            policy_type: LearningPolicyType::SimulationOnly,
            cpu_limit: 0.3,
            memory_limit_mb: 256,
            storage_limit_gb: 2,
            max_duration_seconds: 300,
            learning_objectives: vec![LearningObjective {
                id: LearningObjectiveId::new(),
                description: "Generate safe learning insights".to_string(),
                priority: LearningObjectivePriority::High,
                success_criteria: vec!["No unsafe artifacts generated".to_string()],
            }],
            safety_checks: true,
            audit_logging: true,
            enable_retry_on_failure: false,
            max_failures_before_stop: 1,
            confidence_threshold: 0.8,
            learning_rate: 0.01,
            exploration_factor: 0.5,
            consolidation_interval_seconds: 60,
            reflection_interval_seconds: 30,
            pattern_mining_enabled: true,
            skill_extraction_enabled: false,
            heuristic_optimization_enabled: false,
        }
    }
}