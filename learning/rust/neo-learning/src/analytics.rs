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

#[derive(Debug)]
pub struct LearningAnalytics {
    pub total_sessions: u64,
    pub completed_sessions: u64,
    pub failed_sessions: u64,
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
    pub performance_metrics: PerformanceMetrics,
    pub event_analytics: EventAnalytics,
    pub generated_at: DateTime<Utc>,
}

impl LearningAnalytics {
    pub fn new() -> Self {
        Self {
            total_sessions: 0,
            completed_sessions: 0,
            failed_sessions: 0,
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
            performance_metrics: PerformanceMetrics::new(),
            event_analytics: EventAnalytics::new(),
            generated_at: Utc::now(),
        }
    }

    pub async fn record_learning_session(&mut self, session_status: LearningSessionStatus, duration_seconds: u64) {
        self.total_sessions += 1;
        match session_status {
            LearningSessionStatus::Completed => self.completed_sessions += 1,
            LearningSessionStatus::Failed => self.failed_sessions += 1,
            _ => {}
        }
        
        self.total_learning_time += duration_seconds;
        self.average_session_duration = self.total_learning_time / self.total_sessions;
        self.learning_efficiency = if self.total_sessions > 0 {
            (self.completed_sessions as f32) / (self.total_sessions as f32)
        } else {
            0.0
        };
        self.success_rate = if self.total_sessions > 0 {
            (self.completed_sessions as f32) / (self.total_sessions as f32)
        } else {
            0.0
        };
        self.generated_at = Utc::now();
    }

    pub async fn record_experience(&mut self) {
        self.total_experiences += 1;
        self.generated_at = Utc::now();
    }

    pub async fn record_episode(&mut self) {
        self.total_episodes += 1;
        self.generated_at = Utc::now();
    }

    pub async fn record_reflection(&mut self) {
        self.total_reflections += 1;
        self.generated_at = Utc::now();
    }

    pub async fn record_artifact(&mut self) {
        self.total_artifacts += 1;
        self.generated_at = Utc::now();
    }

    pub async fn record_pattern(&mut self) {
        self.total_patterns += 1;
        self.pattern_discovery_rate = if self.total_experiences > 0 {
            (self.total_patterns as f32) / (self.total_experiences as f32)
        } else {
            0.0
        };
        self.generated_at = Utc::now();
    }

    pub async fn record_skill(&mut self) {
        self.total_skills += 1;
        self.skill_extraction_rate = if self.total_experiences > 0 {
            (self.total_skills as f32) / (self.total_experiences as f32)
        } else {
            0.0
        };
        self.generated_at = Utc::now();
    }

    pub async fn record_heuristic(&mut self) {
        self.total_heuristics += 1;
        self.generated_at = Utc::now();
    }

    pub fn set_performance(&mut self, cpu: f32, memory: f32, throughput: f32, latency: f32) {
        self.performance_metrics.cpu_usage = cpu;
        self.performance_metrics.memory_usage = memory;
        self.performance_metrics.throughput = throughput;
        self.performance_metrics.latency = latency;
        self.generated_at = Utc::now();
    }

    pub fn get_status(&self) -> LearningStatus {
        LearningStatus {
            total_sessions: self.total_sessions,
            completed_sessions: self.completed_sessions,
            failed_sessions: self.failed_sessions,
            total_experiences: self.total_experiences,
            total_episodes: self.total_episodes,
            total_reflections: self.total_reflections,
            total_artifacts: self.total_artifacts,
            total_patterns: self.total_patterns,
            total_skills: self.total_skills,
            total_heuristics: self.total_heuristics,
            learning_efficiency: self.learning_efficiency,
            knowledge_growth: self.knowledge_growth,
            improvement_rate: self.improvement_rate,
            pattern_discovery_rate: self.pattern_discovery_rate,
            skill_extraction_rate: self.skill_extraction_rate,
            heuristic_improvement: self.heuristic_improvement,
            success_rate: self.success_rate,
            performance_metrics: self.performance_metrics.clone(),
            event_analytics: self.event_analytics.clone(),
            generated_at: self.generated_at,
        }
    }
}
