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
use std::collections::{HashMap, VecDeque};
use async_trait::async_trait;
use super::types::*;

/// # LearningSession
/// 
/// Represents a discrete learning cycle or period where experiences are
/// collected, analyzed, and consolidated to improve the system.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningSession {
    pub id: LearningSessionId,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub status: LearningSessionStatus,
    pub learning_objectives: Vec<LearningObjective>,
    pub configuration: LearningConfiguration,
    pub metrics: SessionMetrics,
    pub episodes: Vec<EpisodeId>,
    pub experiences: Vec<ExperienceId>,
    pub reflections: Vec<ReflectionId>,
    pub artifacts: Vec<ArtifactId>,
}

impl LearningSession {
    /// Create a new learning session
    pub fn new(objectives: Vec<LearningObjective>, config: LearningConfiguration) -> Self {
        Self {
            id: LearningSessionId::new(),
            start_time: Utc::now(),
            end_time: None,
            status: LearningSessionStatus::Active,
            learning_objectives: objectives,
            configuration: config,
            metrics: SessionMetrics::new(),
            episodes: Vec::new(),
            experiences: Vec::new(),
            reflections: Vec::new(),
            artifacts: Vec::new(),
        }
    }

    /// Add an experience to the session
    pub fn add_experience(&mut self, experience_id: ExperienceId) {
        self.experiences.push(experience_id);
        self.metrics.experiences_processed += 1;
    }

    /// Add an episode to the session
    pub fn add_episode(&mut self, episode_id: EpisodeId) {
        self.episodes.push(episode_id);
        self.metrics.episodes_created += 1;
    }

    /// Add a reflection result
    pub fn add_reflection(&mut self, reflection_id: ReflectionId) {
        self.reflections.push(reflection_id);
        self.metrics.reflections_completed += 1;
    }

    /// Add a learned artifact
    pub fn add_artifact(&mut self, artifact_id: ArtifactId) {
        self.artifacts.push(artifact_id);
        self.metrics.artifacts_created += 1;
    }

    /// Complete the learning session
    pub fn complete(&mut self) {
        self.end_time = Some(Utc::now());
        self.status = LearningSessionStatus::Completed;
    }

    /// Check if session is still active
    pub fn is_active(&self) -> bool {
        matches!(self.status, LearningSessionStatus::Active)
    }

    /// Get session duration in seconds
    pub fn duration_seconds(&self) -> i64 {
        if let Some(end_time) = self.end_time {
            (end_time - self.start_time).num_seconds()
        } else {
            (Utc::now() - self.start_time).num_seconds()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LearningSessionStatus {
    Active,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetrics {
    pub experiences_processed: u32,
    pub episodes_created: u32,
    pub reflections_completed: u32,
    pub artifacts_created: u32,
    pub learning_efficiency: f32,
    pub knowledge_gain: f32,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
}

impl SessionMetrics {
    pub fn new() -> Self {
        Self {
            experiences_processed: 0,
            episodes_created: 0,
            reflections_completed: 0,
            artifacts_created: 0,
            learning_efficiency: 0.0,
            knowledge_gain: 0.0,
            start_time: Utc::now(),
            end_time: None,
        }
    }
}
