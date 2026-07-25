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
pub struct LearningSnapshot {
    pub snapshot_id: LearningSnapshotId,
    pub timestamp: DateTime<Utc>,
    pub version: LearningVersion,
    pub statistics: LearningStatistics,
    pub policies: HashMap<String, serde_json::Value>,
    pub experiences: Vec<ExperienceSnapshot>,
    pub episodes: Vec<EpisodeSnapshot>,
    pub knowledge: KnowledgeSnapshot,
    pub patterns: Vec<PatternSnapshot>,
    pub skills: Vec<SkillSnapshot>,
    pub heuristics: Vec<HeuristicSnapshot>,
    pub reflections: Vec<ReflectionSnapshot>,
    pub learning_objectives: Vec<LearningObjectiveSnapshot>,
    pub metadata: SnapshotMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    pub created_by: String,
    pub reason: String,
    pub size_bytes: u64,
    pub checksum: String,
    pub compressed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceSnapshot {
    pub experience_id: ExperienceId,
    pub snapshot_data: serde_json::Value,
    pub context: ExperienceContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeSnapshot {
    pub episode_id: EpisodeId,
    pub snapshot_data: serde_json::Value,
    pub sequence: EpisodeSequence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSnapshot {
    pub concepts: HashMap<ConceptId, ConceptSnapshot>,
    pub relationships: Vec<RelationshipSnapshot>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptSnapshot {
    pub concept_id: ConceptId,
    pub concept_type: ConceptType,
    pub name: String,
    pub description: String,
    pub attributes: HashMap<String, serde_json::Value>,
    pub confidence: f32,
    pub evidence: Vec<EvidenceSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceSnapshot {
    pub evidence_id: EvidenceId,
    pub source: String,
    pub value: serde_json::Value,
    pub weight: f32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipSnapshot {
    pub source_id: ConceptId,
    pub target_id: ConceptId,
    pub relationship_type: RelationshipType,
    pub strength: f32,
    pub evidence: Vec<EvidenceId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternSnapshot {
    pub pattern_id: PatternId,
    pub pattern_type: PatternType,
    pub pattern_data: serde_json::Value,
    pub confidence: f32,
    pub support: f32,
    pub examples: Vec<ExperienceId>,
    pub predictions: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSnapshot {
    pub skill_id: SkillId,
    pub skill_type: SkillType,
    pub name: String,
    pub description: String,
    pub proficiency: f32,
    pub applicability: HashMap<String, f32>,
    pub examples: Vec<ExampleSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleSnapshot {
    pub example_id: ExampleId,
    pub data: serde_json::Value,
    pub outcome: serde_json::Value,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeuristicSnapshot {
    pub heuristic_id: HeuristicId,
    pub heuristic_type: HeuristicType,
    pub name: String,
    pub description: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub effectiveness: f32,
    pub confidence: f32,
    pub examples: Vec<ExperienceId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionSnapshot {
    pub reflection_id: ReflectionId,
    pub timestamp: DateTime<Utc>,
    pub insight: String,
    pub recommendations: Vec<ReflectionRecommendation>,
    pub confidence: f32,
    pub evidence: Vec<EvidenceId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningObjectiveSnapshot {
    pub objective_id: LearningObjectiveId,
    pub description: String,
    pub priority: LearningObjectivePriority,
    pub status: LearningObjectiveStatus,
    pub progress: f32,
    pub success_criteria: Vec<String>,
    pub evidence: Vec<EvidenceId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConceptType {
    Goal,
    Strategy,
    Skill,
    Heuristic,
    Pattern,
    Relationship,
    Property,
    Function,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    Workflow,
    Skill,
    Heuristic,
    ResourceUsage,
    Performance,
    Failure,
    Recovery,
    Collaboration,
    Dependency,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkillType {
    Cognitive,
    Procedural,
    Collaborative,
    Analytical,
    Creative,
    Technical,
    Relational,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HeuristicType {
    Planning,
    Execution,
    Selection,
    Optimization,
    Allocation,
    Validation,
    Estimation,
    Adaptation,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipType {
    Causes,
    Influences,
    DependsOn,
    Contradicts,
    Supports,
    Recommends,
    Similar,
    Contrasting,
    Sequential,
    Parallel,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub checkpoint_id: CheckpointId,
    pub timestamp: DateTime<Utc>,
    pub phase: LearningPhase,
    pub metrics: CheckpointMetrics,
    pub artifacts: Vec<ArtifactId>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LearningPhase {
    DataCollection,
    EpisodicMemory,
    Reflection,
    KnowledgeConsolidation,
    PatternDiscovery,
    SkillExtraction,
    HeuristicOptimization,
    StrategyRefinement,
    PerformanceOptimization,
    FailureAnalysis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMetrics {
    pub artifacts_created: u32,
    pub insights_generated: u32,
    pub patterns_discovered: u32,
    pub skills_extracted: u32,
    pub heuristics_optimized: u32,
    pub knowledge_gained: f32,
    pub cpu_used: f32,
    pub memory_used: f32,
    pub duration_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub build: Option<String>,
}

impl LearningVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            build: None,
        }
    }

    pub fn bump_minor(&mut self) {
        self.minor += 1;
        self.patch = 0;
    }

    pub fn bump_patch(&mut self) {
        self.patch += 1;
    }
}

impl Default for LearningVersion {
    fn default() -> Self {
        Self {
            major: 0,
            minor: 1,
            patch: 0,
            build: None,
        }
    }
}
