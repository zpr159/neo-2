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
pub struct LearningCheckpoint {
    pub checkpoint_id: LearningCheckpointId,
    pub timestamp: DateTime<Utc>,
    pub phase: LearningPhase,
    pub version: LearningVersion,
    pub artifacts: Vec<ArtifactId>,
    pub metrics: CheckpointMetrics,
    pub metadata: HashMap<String, serde_json::Value>,
    pub next_checkpoint: Option<LearningCheckpointId>,
}

impl LearningCheckpoint {
    pub fn new(phase: LearningPhase, version: LearningVersion) -> Self {
        Self {
            checkpoint_id: LearningCheckpointId::new(),
            timestamp: Utc::now(),
            phase,
            version,
            artifacts: Vec::new(),
            metrics: CheckpointMetrics::new(),
            metadata: HashMap::new(),
            next_checkpoint: None,
        }
    }

    pub fn add_artifact(&mut self, artifact_id: ArtifactId) {
        self.artifacts.push(artifact_id);
        self.metrics.artifacts_created += 1;
    }

    pub fn set_next_checkpoint(&mut self, next_id: LearningCheckpointId) {
        self.next_checkpoint = Some(next_id);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMetrics {
    pub artifacts_created: u32,
    pub artifacts_updated: u32,
    pub insights_generated: u32,
    pub patterns_discovered: u32,
    pub skills_extracted: u32,
    pub heuristics_optimized: u32,
    pub knowledge_gained: f32,
    pub cpu_used: f32,
    pub memory_used: f32,
    pub duration_seconds: u64,
    pub error_count: u32,
    pub warning_count: u32,
}

impl CheckpointMetrics {
    pub fn new() -> Self {
        Self {
            artifacts_created: 0,
            artifacts_updated: 0,
            insights_generated: 0,
            patterns_discovered: 0,
            skills_extracted: 0,
            heuristics_optimized: 0,
            knowledge_gained: 0.0,
            cpu_used: 0.0,
            memory_used: 0.0,
            duration_seconds: 0,
            error_count: 0,
            warning_count: 0,
        }
    }

    pub fn artifact_created(&mut self) {
        self.artifacts_created += 1;
    }

    pub fn artifact_updated(&mut self) {
        self.artifacts_updated += 1;
    }

    pub fn insight_generated(&mut self) {
        self.insights_generated += 1;
    }

    pub fn pattern_discovered(&mut self) {
        self.patterns_discovered += 1;
    }

    pub fn skill_extracted(&mut self) {
        self.skills_extracted += 1;
    }

    pub fn heuristic_optimized(&mut self) {
        self.heuristics_optimized += 1;
    }

    pub fn knowledge_gained(&mut self, gain: f32) {
        self.knowledge_gained += gain;
    }

    pub fn performance(&mut self, cpu: f32, memory: f32, duration: u64) {
        self.cpu_used = cpu;
        self.memory_used = memory;
        self.duration_seconds = duration;
    }

    pub fn error(&mut self) {
        self.error_count += 1;
    }

    pub fn warning(&mut self) {
        self.warning_count += 1;
    }
}
