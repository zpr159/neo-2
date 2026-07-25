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

pub struct LearningRepository {
    data_dir: String,
    episodes: Box<dyn EpisodeStore + Send + Sync>,
    experiences: Box<dyn ExperienceRepository + Send + Sync>,
    artifacts: Box<dyn ArtifactRepository + Send + Sync>,
    knowledge: Box<dyn KnowledgeRepository + Send + Sync>,
    patterns: Box<dyn PatternRepository + Send + Sync>,
    skills: Box<dyn SkillRepository + Send + Sync>,
    heuristics: Box<dyn HeuristicRepository + Send + Sync>,
    reflections: Box<dyn ReflectionRepository + Send + Sync>,
    checkpoints: Box<dyn CheckpointRepository + Send + Sync>,
}

impl LearningRepository {
    pub fn new(data_dir: String) -> Self {
        let episodes: Box<dyn EpisodeStore + Send + Sync> = Box::new(FileEpisodeStore::new(&data_dir));
        let experiences: Box<dyn ExperienceRepository + Send + Sync> = Box::new(FileExperienceRepository::new(&data_dir));
        let artifacts: Box<dyn ArtifactRepository + Send + Sync> = Box::new(FileArtifactRepository::new(&data_dir));
        let knowledge: Box<dyn KnowledgeRepository + Send + Sync> = Box::new(FileKnowledgeRepository::new(&data_dir));
        let patterns: Box<dyn PatternRepository + Send + Sync> = Box::new(FilePatternRepository::new(&data_dir));
        let skills: Box<dyn SkillRepository + Send + Sync> = Box::new(FileSkillRepository::new(&data_dir));
        let heuristics: Box<dyn HeuristicRepository + Send + Sync> = Box::new(FileHeuristicRepository::new(&data_dir));
        let reflections: Box<dyn ReflectionRepository + Send + Sync> = Box::new(FileReflectionRepository::new(&data_dir));
        let checkpoints: Box<dyn CheckpointRepository + Send + Sync> = Box::new(FileCheckpointRepository::new(&data_dir));

        Self {
            data_dir,
            episodes,
            experiences,
            artifacts,
            knowledge,
            patterns,
            skills,
            heuristics,
            reflections,
            checkpoints,
        }
    }

    pub async fn store_experience(&self, experience: Experience) -> Result<ExperienceId, LearningError> {
        self.experiences.store(experience).await
    }

    pub async fn get_experience(&self, id: ExperienceId) -> Result<Option<Experience>, LearningError> {
        self.experiences.get(id).await
    }

    pub async fn store_episode(&self, episode: Episode) -> Result<EpisodeId, LearningError> {
        self.episodes.store(episode).await
    }

    pub async fn get_episode(&self, id: EpisodeId) -> Result<Option<Episode>, LearningError> {
        self.episodes.get(id).await
    }

    pub async fn store_reflection(&self, reflection: ReflectionResult) -> Result<ReflectionId, LearningError> {
        self.reflections.store(reflection).await
    }

    pub async fn get_reflection(&self, id: ReflectionId) -> Result<Option<ReflectionResult>, LearningError> {
        self.reflections.get(id).await
    }

    pub async fn store_knowledge(&self, knowledge: ConsolidatedKnowledge) -> Result<KnowledgeId, LearningError> {
        self.knowledge.store(knowledge).await
    }

    pub async fn get_knowledge(&self, id: KnowledgeId) -> Result<Option<ConsolidatedKnowledge>, LearningError> {
        self.knowledge.get(id).await
    }

    pub async fn store_pattern(&self, pattern: Pattern) -> Result<PatternId, LearningError> {
        self.patterns.store(pattern).await
    }

    pub async fn get_pattern(&self, id: PatternId) -> Result<Option<Pattern>, LearningError> {
        self.patterns.get(id).await
    }

    pub async fn store_skill(&self, skill: Skill) -> Result<SkillId, LearningError> {
        self.skills.store(skill).await
    }

    pub async fn get_skill(&self, id: SkillId) -> Result<Option<Skill>, LearningError> {
        self.skills.get(id).await
    }

    pub async fn store_heuristic(&self, heuristic: Heuristic) -> Result<HeuristicId, LearningError> {
        self.heuristics.store(heuristic).await
    }

    pub async fn get_heuristic(&self, id: HeuristicId) -> Result<Option<Heuristic>, LearningError> {
        self.heuristics.get(id).await
    }

    pub async fn store_checkpoint(&self, checkpoint: LearningCheckpoint) -> Result<LearningCheckpointId, LearningError> {
        self.checkpoints.store(checkpoint).await
    }

    pub async fn get_checkpoint(&self, id: LearningCheckpointId) -> Result<Option<LearningCheckpoint>, LearningError> {
        self.checkpoints.get(id).await
    }

    pub async fn get_all_consolidated_knowledge(&self) -> Result<ConsolidatedKnowledge, LearningError> {
        self.knowledge.get_all().await
    }

    pub async fn get_all_reflections(&self) -> Result<Vec<ReflectionResult>, LearningError> {
        self.reflections.list_all().await
    }

    pub async fn get_all_patterns(&self) -> Result<Vec<Pattern>, LearningError> {
        self.patterns.list_all().await
    }

    pub async fn get_all_skills(&self) -> Result<Vec<Skill>, LearningError> {
        self.skills.list_all().await
    }

    pub async fn get_all_heuristics(&self) -> Result<Vec<Heuristic>, LearningError> {
        self.heuristics.list_all().await
    }

    pub async fn update_knowledge(&self, knowledge: ConsolidatedKnowledge) -> Result<(), LearningError> {
        self.knowledge.update(knowledge).await
    }

    pub async fn store_patterns(&self, patterns: Vec<Pattern>) -> Result<(), LearningError> {
        for pattern in patterns {
            self.patterns.store(pattern).await?;
        }
        Ok(())
    }

    pub async fn store_skills(&self, skills: Vec<Skill>) -> Result<(), LearningError> {
        for skill in skills {
            self.skills.store(skill).await?;
        }
        Ok(())
    }

    pub async fn store_heuristics(&self, heuristics: Vec<Heuristic>) -> Result<(), LearningError> {
        for heuristic in heuristics {
            self.heuristics.store(heuristic).await?;
        }
        Ok(())
    }

    pub async fn store_reflections(&self, reflections: Vec<ReflectionResult>) -> Result<(), LearningError> {
        for reflection in reflections {
            self.reflections.store(reflection).await?;
        }
        Ok(())
    }

    pub async fn store_failure_analysis(&self, analysis: FailureAnalysisReport) -> Result<(), LearningError> {
        let artifact = LearningArtifact::FailureAnalysis(analysis);
        self.artifacts.store_artifact(artifact).await
    }

    pub async fn export_all(&self) -> Result<ExportedLearningData, LearningError> {
        let mut exported = ExportedLearningData::new();
        exported.experiences = self.experiences.export_all().await?;
        exported.episodes = self.episodes.export_all().await?;
        exported.knowledge = self.knowledge.export_all().await?;
        exported.patterns = self.patterns.export_all().await?;
        exported.skills = self.skills.export_all().await?;
        exported.reflections = self.reflections.export_all().await?;
        exported.heuristics = self.heuristics.export_all().await?;
        Ok(exported)
    }

    pub async fn import_all(&self, data: ImportedLearningData) -> Result<(), LearningError> {
        self.experiences.import_all(data.experiences).await?;
        self.episodes.import_all(data.episodes).await?;
        self.knowledge.import_all(data.knowledge).await?;
        self.patterns.import_all(data.patterns).await?;
        self.skills.import_all(data.skills).await?;
        self.reflections.import_all(data.reflections).await?;
        self.heuristics.import_all(data.heuristics).await?;
        Ok(())
    }

    pub async fn get_status(&self) -> Result<RepositoryStatus, LearningError> {
        RepositoryStatus {
            experiences: self.experiences.count().await?,
            episodes: self.episodes.count().await?,
            reflections: self.reflections.count().await?,
            knowledge: self.knowledge.count().await?,
            patterns: self.patterns.count().await?,
            skills: self.skills.count().await?,
            heuristics: self.heuristics.count().await?,
            checkpoints: self.checkpoints.count().await?,
            data_dir: self.data_dir.clone(),
        }
    }
}
