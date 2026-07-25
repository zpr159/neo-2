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
use std::collections::HashMap;
use async_trait::async_trait;
use super::types::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LearningObjectivePriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningObjective {
    pub id: LearningObjectiveId,
    pub description: String,
    pub priority: LearningObjectivePriority,
    pub success_criteria: Vec<String>,
    pub target_improvement: f32,
    pub estimated_cost: f32,
    pub dependencies: Vec<LearningObjectiveId>,
    pub status: LearningObjectiveStatus,
    pub progress: f32,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LearningObjectiveStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningConfiguration {
    pub learning_policy: LearningPolicy,
    pub memory_limit_mb: u32,
    pub learning_frequency: LearningFrequency,
    pub persistence_enabled: bool,
    pub analytics_enabled: bool,
    pub pattern_mining_enabled: bool,
    pub skill_extraction_enabled: bool,
    pub heuristic_optimization_enabled: bool,
    pub environment: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LearningFrequency {
    Continuous,
    Scheduled { interval_seconds: u64 },
    OnDemand,
    Batch { interval_seconds: u64 },
}

impl Default for LearningConfiguration {
    fn default() -> Self {
        Self {
            learning_policy: LearningPolicy::default(),
            memory_limit_mb: 1024,
            learning_frequency: LearningFrequency::EventDriven,
            persistence_enabled: true,
            analytics_enabled: true,
            pattern_mining_enabled: true,
            skill_extraction_enabled: true,
            heuristic_optimization_enabled: true,
            environment: HashMap::new(),n        }
    }
}

impl LearningConfiguration {
    pub fn builder() -> LearningConfigurationBuilder {
        LearningConfigurationBuilder::default()
    }
}

pub struct LearningConfigurationBuilder {
    learning_policy: Option<LearningPolicy>,
    memory_limit_mb: Option<u32>,
    learning_frequency: Option<LearningFrequency>,
    persistence_enabled: Option<bool>,
    analytics_enabled: Option<bool>,
    pattern_mining_enabled: Option<bool>,
    skill_extraction_enabled: Option<bool>,
    heuristic_optimization_enabled: Option<bool>,
    environment: Option<HashMap<String, serde_json::Value>>,
}

impl Default for LearningConfigurationBuilder {
    fn default() -> Self {
        Self {
            learning_policy: None,
            memory_limit_mb: None,
            learning_frequency: None,
            persistence_enabled: None,
            analytics_enabled: None,
            pattern_mining_enabled: None,
            skill_extraction_enabled: None,
            heuristic_optimization_enabled: None,
            environment: None,
        }
    }
}

impl LearningConfigurationBuilder {
    pub fn learning_policy(mut self, policy: LearningPolicy) -> Self {
        self.learning_policy = Some(policy);
        self
    }

    pub fn memory_limit_mb(mut self, limit: u32) -> Self {
        self.memory_limit_mb = Some(limit);
        self
    }

    pub fn learning_frequency(mut self, frequency: LearningFrequency) -> Self {
        self.learning_frequency = Some(frequency);
        self
    }

    pub fn persistence_enabled(mut self, enabled: bool) -> Self {
        self.persistence_enabled = Some(enabled);
        self
    }

    pub fn analytics_enabled(mut self, enabled: bool) -> Self {
        self.analytics_enabled = Some(enabled);
        self
    }

    pub fn pattern_mining_enabled(mut self, enabled: bool) -> Self {
        self.pattern_mining_enabled = Some(enabled);
        self
    }

    pub fn skill_extraction_enabled(mut self, enabled: bool) -> Self {
        self.skill_extraction_enabled = Some(enabled);
        self
    }

    pub fn heuristic_optimization_enabled(mut self, enabled: bool) -> Self {
        self.heuristic_optimization_enabled = Some(enabled);
        self
    }

    pub fn environment(mut self, env: HashMap<String, serde_json::Value>) -> Self {
        self.environment = Some(env);
        self
    }

    pub fn build(self) -> LearningConfiguration {
        LearningConfiguration {
            learning_policy: self.learning_policy.unwrap_or_default(),
            memory_limit_mb: self.memory_limit_mb.unwrap_or(1024),
            learning_frequency: self.learning_frequency.unwrap_or(LearningFrequency::EventDriven),
            persistence_enabled: self.persistence_enabled.unwrap_or(true),
            analytics_enabled: self.analytics_enabled.unwrap_or(true),
            pattern_mining_enabled: self.pattern_mining_enabled.unwrap_or(true),
            skill_extraction_enabled: self.skill_extraction_enabled.unwrap_or(true),
            heuristic_optimization_enabled: self.heuristic_optimization_enabled.unwrap_or(true),
            environment: self.environment.unwrap_or_default(),
        }
    }
}
