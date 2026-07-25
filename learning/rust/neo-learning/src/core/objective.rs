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

/// # Learning Objective
/// 
/// Represents a specific learning goal or outcome that the system aims to achieve
/// through experience collection, reflection, and knowledge consolidation.

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
pub enum LearningObjectivePriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LearningObjectiveStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
}
