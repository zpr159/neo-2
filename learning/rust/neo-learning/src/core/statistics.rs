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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningStatistics {
    pub total_sessions: u64,
    pub completed_sessions: u64,
    pub failed_sessions: u64,
    pub active_sessions: u64,
    pub total_experiences: u64,
    pub total_episodes: u64,
    pub total_reflections: u64,
    pub total_artifacts: u64,
    pub total_patterns: u64,
    pub total_skills: u64,
    pub total_heuristics: u64,
    pub learning_efficiency: f32,
    pub knowledge_growth: f32,
    pub improvement_rate: f32,
    pub pattern_discovery_rate: f32,
    pub skill_extraction_rate: f32,
    pub heuristic_improvement: f32,
    pub success_rate: f32,
    pub average_session_duration: u64,
    pub total_learning_time: u64,
    pub cpu_utilization: f32,
    pub memory_utilization: f32,
    pub learning_frequency: f32,
    pub generated_at: DateTime<Utc>,
}

impl LearningStatistics {
    pub fn new() -> Self {
        Self {
            total_sessions: 0,
            completed_sessions: 0,
            failed_sessions: 0,
            active_sessions: 0,
            total_experiences: 0,
            total_episodes: 0,
            total_reflections: 0,
            total_artifacts: 0,
            total_patterns: 0,
            total_skills: 0,
            total_heuristics: 0,
            learning_efficiency: 0.0,
            knowledge_growth: 0.0,
            improvement_rate: 0.0,
            pattern_discovery_rate: 0.0,
            skill_extraction_rate: 0.0,
            heuristic_improvement: 0.0,
            success_rate: 0.0,
            average_session_duration: 0,
            total_learning_time: 0,
            cpu_utilization: 0.0,
            memory_utilization: 0.0,
            learning_frequency: 0.0,
            generated_at: Utc::now(),
        }
    }

    pub fn session_completed(&mut self, duration_seconds: u64) {
        self.total_sessions += 1;
        self.completed_sessions += 1;
        self.total_learning_time += duration_seconds;
        self.average_session_duration = self.total_learning_time / self.total_sessions;
        self.learning_efficiency = (self.completed_sessions as f32) / (self.total_sessions as f32);
        self.success_rate = (self.completed_sessions as f32) / (self.total_sessions as f32);
        self.generated_at = Utc::now();
    }

    pub fn session_failed(&mut self) {
        self.total_sessions += 1;
        self.failed_sessions += 1;
        self.learning_efficiency = (self.completed_sessions as f32) / (self.total_sessions as f32);
        self.success_rate = (self.completed_sessions as f32) / (self.total_sessions as f32);
        self.generated_at = Utc::now();
    }

    pub fn increment_active_sessions(&mut self) {
        self.active_sessions += 1;
        self.generated_at = Utc::now();
    }

    pub fn decrement_active_sessions(&mut self) {
        if self.active_sessions > 0 {
            self.active_sessions -= 1;
        }
        self.generated_at = Utc::now();
    }

    pub fn add_experience(&mut self) {
        self.total_experiences += 1;
        self.generated_at = Utc::now();
    }

    pub fn add_episode(&mut self) {
        self.total_episodes += 1;
        self.generated_at = Utc::now();
    }

    pub fn add_reflection(&mut self) {
        self.total_reflections += 1;
        self.generated_at = Utc::now();
    }

    pub fn add_artifact(&mut self) {
        self.total_artifacts += 1;
        self.generated_at = Utc::now();
    }

    pub fn add_pattern(&mut self) {
        self.total_patterns += 1;
        self.pattern_discovery_rate = (self.total_patterns as f32) / (self.total_experiences as f32);
        self.generated_at = Utc::now();
    }

    pub fn add_skill(&mut self) {
        self.total_skills += 1;
        self.skill_extraction_rate = (self.total_skills as f32) / (self.total_experiences as f32);
        self.generated_at = Utc::now();
    }

    pub fn add_heuristic(&mut self) {
        self.total_heuristics += 1;
        self.generated_at = Utc::now();
    }

    pub fn set_performance(&mut self, cpu: f32, memory: f32) {
        self.cpu_utilization = cpu;
        self.memory_utilization = memory;
        self.generated_at = Utc::now();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningMetrics {
    pub session_metrics: Vec<SessionMetrics>,
    pub experience_metrics: Vec<ExperienceMetrics>,
    pub pattern_metrics: HashMap<String, PatternMetrics>,
    pub skill_metrics: HashMap<String, SkillMetrics>,
    pub heuristic_metrics: HashMap<String, HeuristicMetrics>,
    pub performance_metrics: PerformanceMetrics,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetrics {
    pub session_id: LearningSessionId,
    pub objectives_achieved: Vec<LearningObjectiveId>,
    pub artifacts_created: Vec<ArtifactId>,
    pub learning_efficiency: f32,
    pub knowledge_gain: f32,
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub duration_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceMetrics {
    pub experience_id: ExperienceId,
    pub experience_type: ExperienceType,
    pub outcome_quality: f32,
    pub learning_value: f32,
    pub resource_consumed: f32,
    pub time_to_learn: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternMetrics {
    pub pattern_id: PatternId,
    pub pattern_type: PatternType,
    pub confidence: f32,
    pub support: f32,
    pub lift: f32,
    pub coverage: f32,
    pub predictions_correct: u32,
    pub predictions_incorrect: u32,
    pub precision: f32,
    pub recall: f32,
    pub f1_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetrics {
    pub skill_id: SkillId,
    pub skill_type: SkillType,
    pub proficiency: f32,
    pub applicability: f32,
    pub time_to_acquire: u64,
    pub examples_learned: u32,
    pub success_rate: f32,
    pub improvement_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeuristicMetrics {
    pub heuristic_id: HeuristicId,
    pub heuristic_type: HeuristicType,
    pub effectiveness: f32,
    pub efficiency: f32,
    pub accuracy: f32,
    pub confidence: f32,
    pub time_to_apply: u64,
    pub applications: u32,
    pub success_rate: f32,
    pub improvement_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub throughput: f32,
    pub latency: f32,
    pub availability: f32,
    pub reliability: f32,
    pub utilization: f32,
    pub scalability: f32,
    pub robustness: f32,
}
