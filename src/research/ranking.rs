use super::api::{Finding, RankedFinding, ResearchEvidence};
use super::config::RankingConfig;
use crate::time::Timestamp;

/// Ranks research findings by composite score considering multiple factors.
pub struct FindingRanker {
    config: RankingConfig,
}

impl FindingRanker {
    pub fn new(config: RankingConfig) -> Self {
        Self { config }
    }

    /// Rank a set of findings and return sorted results.
    pub fn rank(&self, findings: Vec<Finding>) -> Vec<RankedFinding> {
        let mut ranked: Vec<RankedFinding> = findings
            .into_iter()
            .enumerate()
            .map(|(i, finding)| {
                let composite_score =
                    self.compute_composite_score(&finding, i);
                RankedFinding {
                    finding,
                    rank: 0,
                    composite_score,
                    diversity_contribution: 0.0,
                }
            })
            .collect();

        ranked.sort_by(|a, b| {
            b.composite_score
                .partial_cmp(&a.composite_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let diversity_scores = self.compute_diversity(&ranked);

        for (i, item) in ranked.iter_mut().enumerate() {
            item.rank = i + 1;
            item.diversity_contribution = diversity_scores[i];
            item.composite_score += item.diversity_contribution * self.config.diversity_weight;
        }

        ranked.sort_by(|a, b| {
            b.composite_score
                .partial_cmp(&a.composite_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for (i, item) in ranked.iter_mut().enumerate() {
            item.rank = i + 1;
        }

        ranked
    }

    /// Filter by minimum relevance score.
    pub fn filter_min_relevance(
        &self,
        findings: Vec<RankedFinding>,
        min_score: Option<f32>,
    ) -> Vec<RankedFinding> {
        let threshold = min_score.unwrap_or(self.config.min_relevance_score);
        findings
            .into_iter()
            .filter(|f| f.composite_score >= threshold)
            .collect()
    }

    fn compute_composite_score(&self, finding: &Finding, index: usize) -> f32 {
        let confidence_score = finding.confidence;

        let recency_score = compute_recency_score(&finding.timestamp);

        let evidence_count = finding.evidence.len() as f32;
        let authority_score = (evidence_count * 0.1).min(1.0);

        let relevance_score = finding
            .evidence
            .iter()
            .map(|e| e.relevance_score)
            .sum::<f32>()
            / evidence_count.max(1.0);

        let position_penalty = (index as f32 * 0.02).min(0.2);

        let score = (confidence_score * self.config.confidence_weight)
            + (recency_score * self.config.recency_weight)
            + (authority_score * self.config.source_authority_weight)
            + (relevance_score * self.config.relevance_weight)
            - position_penalty;

        score.max(0.0).min(1.0)
    }

    fn compute_diversity(&self, findings: &[RankedFinding]) -> Vec<f32> {
        let mut diversity_scores = vec![0.0f32; findings.len()];

        for i in 0..findings.len() {
            let mut unique_sources: std::collections::HashSet<String> = std::collections::HashSet::new();
            for evidence in &findings[i].finding.evidence {
                unique_sources.insert(evidence.source_name.clone());
            }
            diversity_scores[i] = (unique_sources.len() as f32 * 0.1).min(0.3);

            for j in 0..i {
                let overlap = compute_source_overlap(&findings[i].finding.evidence, &findings[j].finding.evidence);
                let penalty = overlap * 0.15;
                diversity_scores[i] -= penalty;
                diversity_scores[j] -= penalty * 0.5;
            }

            diversity_scores[i] = diversity_scores[i].max(0.0);
        }

        diversity_scores
    }
}

fn compute_recency_score(timestamp: &Timestamp) -> f32 {
    let age_hours = timestamp.elapsed_secs() / 3600.0;

    if age_hours < 1.0 {
        1.0
    } else if age_hours < 24.0 {
        0.9
    } else if age_hours < 168.0 {
        0.7
    } else if age_hours < 720.0 {
        0.5
    } else if age_hours < 8760.0 {
        0.3
    } else {
        0.1
    }
}

fn compute_source_overlap(evidence_a: &[ResearchEvidence], evidence_b: &[ResearchEvidence]) -> f32 {
    let sources_a: std::collections::HashSet<&str> = evidence_a
        .iter()
        .map(|e| e.source_name.as_str())
        .collect();
    let sources_b: std::collections::HashSet<&str> = evidence_b
        .iter()
        .map(|e| e.source_name.as_str())
        .collect();

    let intersection = sources_a.intersection(&sources_b).count();
    let union = sources_a.union(&sources_b).count();

    if union == 0 {
        0.0
    } else {
        intersection as f32 / union as f32
    }
}
