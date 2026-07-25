use serde::{Deserialize, Serialize};

use crate::conversation::evidence::{Evidence, EvidenceSource};
use crate::conversation::config::RankingConfig;

/// A scored evidence item after ranking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedEvidence {
    pub evidence: Evidence,
    pub semantic_similarity: f32,
    pub recency_score: f32,
    pub importance_score: f32,
    pub confidence_score: f32,
    pub source_reliability: f32,
    pub user_relevance: f32,
    pub task_relevance: f32,
    pub final_score: f32,
}

impl RankedEvidence {
    pub fn new(evidence: Evidence) -> Self {
        Self {
            evidence,
            semantic_similarity: 0.0,
            recency_score: 0.0,
            importance_score: 0.0,
            confidence_score: 0.0,
            source_reliability: 0.0,
            user_relevance: 0.0,
            task_relevance: 0.0,
            final_score: 0.0,
        }
    }

    pub fn compute_final_score(&mut self, config: &RankingConfig) {
        self.final_score = self.semantic_similarity * config.semantic_weight
            + self.recency_score * config.recency_weight
            + self.importance_score * config.importance_weight
            + self.confidence_score * config.confidence_weight
            + self.source_reliability * config.source_reliability_weight
            + self.user_relevance * config.user_relevance_weight
            + self.task_relevance * config.task_relevance_weight;
    }
}

/// Context ranker that scores and orders evidence items.
pub struct ContextRanker {
    config: RankingConfig,
}

impl ContextRanker {
    pub fn new(config: RankingConfig) -> Self {
        Self { config }
    }

    pub fn rank(
        &self,
        evidence: Vec<Evidence>,
        task_keywords: &[String],
        _user_context: &str,
    ) -> Vec<RankedEvidence> {
        let mut ranked: Vec<RankedEvidence> = evidence
            .into_iter()
            .map(|e| {
                let mut re = RankedEvidence::new(e);
                re.confidence_score = re.evidence.confidence;
                re.importance_score = re.evidence.relevance_score;
                re.recency_score = self.compute_recency_score(&re.evidence);
                re.source_reliability = self.compute_source_reliability(&re.evidence.source);
                re.semantic_similarity = self.compute_semantic_similarity(&re.evidence.content, task_keywords);
                re.user_relevance = re.importance_score;
                re.task_relevance = self.compute_task_relevance(&re.evidence.content, task_keywords);
                re.compute_final_score(&self.config);
                re
            })
            .collect();

        ranked.sort_by(|a, b| b.final_score.partial_cmp(&a.final_score).unwrap_or(std::cmp::Ordering::Equal));

        let max = self.config.max_items;
        ranked.truncate(max);
        ranked
    }

    fn compute_recency_score(&self, evidence: &Evidence) -> f32 {
        let age_secs = evidence.timestamp.elapsed_secs();
        let day_secs = 86400.0;
        if age_secs < 60.0 {
            1.0
        } else if age_secs < day_secs {
            0.8
        } else if age_secs < day_secs * 7.0 {
            0.5
        } else if age_secs < day_secs * 30.0 {
            0.3
        } else {
            0.1
        }
    }

    fn compute_source_reliability(&self, source: &EvidenceSource) -> f32 {
        match source {
            EvidenceSource::Executive => 0.95,
            EvidenceSource::Reasoning => 0.9,
            EvidenceSource::Planning => 0.85,
            EvidenceSource::KnowledgeGraph => 0.8,
            EvidenceSource::WorldModel => 0.75,
            EvidenceSource::Memory => 0.7,
            EvidenceSource::Agent => 0.7,
            EvidenceSource::Workflow => 0.65,
            EvidenceSource::Tool => 0.6,
            EvidenceSource::UserInput => 0.9,
            EvidenceSource::ConversationHistory => 0.65,
            EvidenceSource::External => 0.5,
            EvidenceSource::Custom(_) => 0.5,
        }
    }

    fn compute_semantic_similarity(&self, content: &str, keywords: &[String]) -> f32 {
        if keywords.is_empty() {
            return 0.5;
        }
        let content_lower = content.to_lowercase();
        let matches = keywords
            .iter()
            .filter(|kw| content_lower.contains(&kw.to_lowercase()))
            .count();
        (matches as f32 / keywords.len() as f32).min(1.0)
    }

    fn compute_task_relevance(&self, content: &str, keywords: &[String]) -> f32 {
        if keywords.is_empty() {
            return 0.5;
        }
        let content_lower = content.to_lowercase();
        let word_count = content_lower.split_whitespace().count() as f32;
        if word_count == 0.0 {
            return 0.0;
        }
        let keyword_count = keywords
            .iter()
            .filter(|kw| content_lower.contains(&kw.to_lowercase()))
            .count() as f32;
        (keyword_count / keywords.len() as f32).min(1.0)
    }
}
