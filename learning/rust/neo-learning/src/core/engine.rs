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
use derive_more::Display;
use super::types::*;

/// # LearningEngine
/// 
/// Main orchestrator that coordinates the learning pipeline:
/// 1. Captures experiences from execution
/// 2. Stores experiences in episodic memory
/// 3. Runs reflection to extract insights
/// 4. Consolidates knowledge
/// 5. Refines strategies and heuristics
/// 6. Discovers patterns
/// 7. Updates skill library
/// 
/// The engine operates under configurable learning policies and
/// continuously improves system's decision-making capabilities.

#[derive(Debug)]
pub struct LearningEngine {
    /// Configuration for learning behavior
    config: LearningConfiguration,
    
    /// Stores experiences and episodes
    memory: Box<dyn EpisodeStore + Send + Sync>,
    
    /// Records raw interactions
    experience_repo: Box<dyn ExperienceRepository + Send + Sync>,
    
    /// Analyzes experiences for insights
    reflection_engine: ReflectionEngine,
    
    /// Consolidates knowledge and concepts
    knowledge_consolidator: Box<dyn KnowledgeConsolidator + Send + Sync>,
    
    /// Mines recurring patterns
    pattern_miner: PatternMiner,
    
    /// Manages learned skills and capabilities
    skill_library: Box<dyn SkillLibrary + Send + Sync>,
    
    /// Refines planning heuristics
    strategy_refiner: Box<dyn StrategyRefiner + Send + Sync>,
    
    /// Optimizes performance
    performance_optimizer: PerformanceOptimizer,
    
    /// Analyzes failures
    failure_analyzer: FailureAnalyzer,
    
    /// Current learning policy
    policy: Box<dyn LearningPolicy + Send + Sync>,
    
    /// Stores learned artifacts
    repository: Box<dyn LearningRepository + Send + Sync>,
    
    /// Publishes learning events
    event_bus: EventBus,
    
    /// Tracks learning metrics
    analytics: LearningAnalytics,
}

impl LearningEngine {
    /// Create a new learning engine with default settings
    pub fn new(config: LearningConfiguration) -> Self {
        let memory: Box<dyn EpisodeStore + Send + Sync> = Box::new(MemoryStore::new());
        let experience_repo: Box<dyn ExperienceRepository + Send + Sync> = Box::new(ExperienceRepositoryImpl::new());
        let reflection_engine = ReflectionEngine::new();
        let knowledge_consolidator: Box<dyn KnowledgeConsolidator + Send + Sync> = Box::new(KnowledgeConsolidatorImpl::new());
        let pattern_miner = PatternMiner::new();
        let skill_library: Box<dyn SkillLibrary + Send + Sync> = Box::new(SkillLibraryImpl::new());
        let strategy_refiner: Box<dyn StrategyRefiner + Send + Sync> = Box::new(StrategyRefinerImpl::new());
        let performance_optimizer = PerformanceOptimizer::new();
        let failure_analyzer = FailureAnalyzer::new();
        let policy: Box<dyn LearningPolicy + Send + Sync> = Box::new(SafeLearningPolicy::new(config.learning_policy.clone()));
        let repository: Box<dyn LearningRepository + Send + Sync> = Box::new(LearningRepositoryImpl::new());
        let event_bus = EventBus::new(1024);
        let analytics = LearningAnalytics::new();

        Self {
            config,
            memory,
            experience_repo,
            reflection_engine,
            knowledge_consolidator,
            pattern_miner,
            skill_library,
            strategy_refiner,
            performance_optimizer,
            failure_analyzer,
            policy,
            repository,
            event_bus,
            analytics,
        }
    }

    /// Record a new experience from system execution
    pub async fn record_experience(&mut self, experience: Experience) -> Result<()> {
        self.policy.validate(&experience).await?;
        
        self.event_bus.publish(LearningEvent::new(
            LearningEventType::ExperienceRecorded,
            "engine"
        ).with_payload(serde_json::json!(experience)));
        
        self.experience_repo.store(experience.clone()).await?;
        self.analytics.record_experience(&experience).await?;
        
        Ok(())
    }

    /// Create an episode from related experiences
    pub async fn create_episode(&mut self, experiences: Vec<Experience>) -> Result<EpisodeId> {
        let episode = Episode::new(experiences.clone());
        let episode_id = episode.id;
        
        self.policy.validate(&episode).await?;
        
        self.event_bus.publish(LearningEvent::new(
            LearningEventType::EpisodeCreated,
            "engine"
        ).with_payload(serde_json::json!(episode.clone())));
        
        self.memory.store(episode).await?;
        self.analytics.record_episode(&experiences).await?;
        
        Ok(episode_id)
    }

    /// Run reflection analysis on stored episodes
    pub async fn run_reflection(&mut self, episode_id: EpisodeId) -> Result<ReflectionResult> {
        let episode = self.memory.get(episode_id).await?.ok_or_else(|| LearningError::EpisodeNotFound(episode_id))?;
        
        self.event_bus.publish(LearningEvent::new(
            LearningEventType::ReflectionCompleted,
            "engine"
        ).with_payload(serde_json::json!(episode_id)));
        
        let reflection = self.reflection_engine.analyze(episode).await?;
        self.repository.store_reflection(reflection.clone()).await?;
        
        if let Some(knowledge) = reflection.knowledge_consolidated {
            self.knowledge_consolidator.consolidate(knowledge).await?;
        }
        
        if let Some(patterns) = reflection.patterns_discovered {
            self.pattern_miner.process(patterns).await?;
        }
        
        if let Some(refiner) = reflection.strategy_refiner {
            self.strategy_refiner.refine(refiner).await?;
        }
        
        self.analytics.update_reflection(reflection.clone()).await?;
        
        Ok(reflection)
    }

    /// Consolidate all learned knowledge
    pub async fn consolidate_knowledge(&mut self) -> Result<()> {
        self.event_bus.publish(LearningEvent::new(
            LearningEventType::KnowledgeConsolidated,
            "engine"
        ).with_payload(serde_json::json!({})));
        
        let knowledge = self.repository.get_all_consolidated_knowledge().await?;
        
        self.knowledge_consolidator
            .merge(knowledge.clone())
            .await?;
        
        let skills = knowledge.skills;
        for skill in skills {
            self.skill_library.add(skill).await?;
        }
        
        let heuristics = knowledge.heuristics;
        for heuristic in heuristics {
            self.strategy_refiner.update_heuristic(heuristic).await?;
        }
        
        let patterns = knowledge.patterns;
        self.pattern_miner.update(patterns).await?;
        
        self.repository.update_knowledge(knowledge).await?;
        
        Ok(())
    }

    /// Mine patterns from all stored experiences
    pub async fn discover_patterns(&mut self) -> Result<Vec<Pattern>> {
        self.event_bus.publish(LearningEvent::new(
            LearningEventType::PatternDiscovered,
            "engine"
        ).with_payload(serde_json::json!({})));
        
        let patterns = self.pattern_miner.mine_all().await?;
        
        self.repository.store_patterns(patterns.clone()).await?;
        self.analytics.record_patterns(patterns.clone()).await?;
        
        Ok(patterns)
    }

    /// Extract skills from experiences
    pub async fn extract_skills(&mut self) -> Result<Vec<Skill>> {
        self.event_bus.publish(LearningEvent::new(
            LearningEventType::SkillExtracted,
            "engine"
        ).with_payload(serde_json::json!({})));
        
        let skills = self.skill_library.extract_all().await?;
        
        self.repository.store_skills(skills.clone()).await?;
        self.analytics.record_skills(skills.clone()).await?;
        
        Ok(skills)
    }

    /// Run performance optimization analysis
    pub async fn optimize_performance(&mut self) -> Result<OptimizationReport> {
        let mut report = OptimizationReport::new();
        
        self.event_bus.publish(LearningEvent::new(
            LearningEventType::OptimizationSuggested,
            "engine"
        ).with_payload(serde_json::json!(report.clone())));
        
        let recommendations = self.performance_optimizer.analyze().await?;
        
        self.strategy_refiner.update_from_optimization(recommendations).await?;
        
        Ok(recommendations)
    }

    /// Analyze failures and provide remediation
    pub async fn analyze_failures(&mut self) -> Result<FailureAnalysisReport> {
        let mut report = FailureAnalysisReport::new();
        
        self.event_bus.publish(LearningEvent::new(
            LearningEventType::FailureAnalyzed,
            "engine"
        ).with_payload(serde_json::json!(report.clone())));
        
        let analysis = self.failure_analyzer.analyze_all().await?;
        
        self.repository.store_failure_analysis(analysis.clone()).await?;
        
        Ok(analysis)
    }

    /// Export learned artifacts for persistence
    pub async fn export(&self) -> Result<ExportedLearningData> {
        let exported = ExportedLearningData {
            version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: Utc::now(),
            experiences: self.experience_repo.export_all().await?,
            episodes: self.memory.export_all().await?,
            knowledge: self.repository.get_all_knowledge().await?,
            patterns: self.pattern_miner.export_all().await?,
            skills: self.skill_library.export_all().await?,
            reflections: self.repository.get_all_reflections().await?,
            heuristics: self.strategy_refiner.export_all().await?,
        };
        
        Ok(exported)
    }

    /// Import learned artifacts from persistence
    pub async fn import(&mut self, data: ImportedLearningData) -> Result<()> {
        self.event_bus.publish(LearningEvent::new(
            LearningEventType::ExperienceRecorded,
            "engine"
        ).with_payload(serde_json::json!(data.clone())));
        
        self.experience_repo.import_all(data.experiences).await?;
        self.memory.import_all(data.episodes).await?;
        self.knowledge_consolidator.import(data.knowledge).await?;
        self.pattern_miner.import_all(data.patterns).await?;
        self.skill_library.import_all(data.skills).await?;
        self.strategy_refiner.import_all(data.heuristics).await?;
        
        Ok(())
    }

    /// Get current learning status and metrics
    pub async fn get_status(&self) -> Result<LearningStatus> {
        LearningStatus {
            engine_status: self.analytics.get_status().await?,
            policy_status: self.policy.get_status().await?,
            repository_status: self.repository.get_status().await?,
        }
    }
}