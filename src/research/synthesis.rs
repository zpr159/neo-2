use super::api::{
    Citation, Finding, RankedFinding, ResearchContradiction,
    ValidatedFact, ContradictionSeverity,
};
use super::config::SynthesisConfig;
use super::error::ResearchResult;

/// Merges, summarizes, and synthesizes validated research findings.
pub struct ResearchSynthesizer {
    config: SynthesisConfig,
}

impl ResearchSynthesizer {
    pub fn new(config: SynthesisConfig) -> Self {
        Self { config }
    }

    /// Synthesize ranked findings into a final research result.
    pub fn synthesize(
        &self,
        ranked_findings: Vec<RankedFinding>,
        validated_facts: &[ValidatedFact],
        citations: &[Citation],
        objective: &str,
    ) -> ResearchResult<super::api::ResearchOutput> {
        let findings: Vec<Finding> = ranked_findings.into_iter().map(|rf| rf.finding).collect();

        let contradictions = self.detect_contradictions(&findings);

        let evidence_count: usize = findings.iter().map(|f| f.evidence.len()).sum();

        let summary = self.generate_summary(&findings, objective);
        let confidence = self.compute_overall_confidence(&findings);

        let knowledge_updates = self.propose_knowledge_updates(validated_facts);
        let world_updates = self.propose_world_updates(validated_facts);
        let memory_updates = self.propose_memory_updates(&findings, objective);

        Ok(super::api::ResearchOutput {
            summary,
            findings,
            citations: citations.to_vec(),
            contradictions,
            knowledge_updates,
            world_updates,
            memory_updates,
            confidence,
            sources_count: citations.len(),
            evidence_count,
        })
    }

    /// Merge findings from multiple research phases.
    pub fn merge_findings(
        &self,
        finding_groups: Vec<Vec<Finding>>,
    ) -> Vec<Finding> {
        match self.config.merge_strategy {
            super::config::MergeStrategy::WeightedAverage => {
                self.merge_weighted(finding_groups)
            }
            super::config::MergeStrategy::MostConfident => {
                self.merge_most_confident(finding_groups)
            }
            super::config::MergeStrategy::MajorityVote => {
                self.merge_majority(finding_groups)
            }
            super::config::MergeStrategy::SourcePriority => {
                self.merge_source_priority(finding_groups)
            }
        }
    }

    /// Generate a structured summary from findings.
    pub fn generate_summary(&self, findings: &[Finding], objective: &str) -> String {
        let mut summary = format!("Research Summary: {}\n\n", objective);

        let high_confidence: Vec<&Finding> = findings
            .iter()
            .filter(|f| f.confidence >= 0.7)
            .collect();
        let medium_confidence: Vec<&Finding> = findings
            .iter()
            .filter(|f| f.confidence >= 0.4 && f.confidence < 0.7)
            .collect();
        let low_confidence: Vec<&Finding> = findings
            .iter()
            .filter(|f| f.confidence < 0.4)
            .collect();

        if !high_confidence.is_empty() {
            summary.push_str("High Confidence Findings:\n");
            for finding in &high_confidence {
                summary.push_str(&format!(
                    "  - {} (confidence: {:.2})\n",
                    finding.statement, finding.confidence
                ));
            }
            summary.push('\n');
        }

        if !medium_confidence.is_empty() {
            summary.push_str("Moderate Confidence Findings:\n");
            for finding in &medium_confidence {
                summary.push_str(&format!(
                    "  - {} (confidence: {:.2})\n",
                    finding.statement, finding.confidence
                ));
            }
            summary.push('\n');
        }

        if !low_confidence.is_empty() {
            summary.push_str(&format!(
                "{} additional low-confidence findings noted.\n\n",
                low_confidence.len()
            ));
        }

        summary.push_str(&format!(
            "Total: {} findings from {} sources.",
            findings.len(),
            self.count_unique_sources(findings),
        ));

        summary
    }

    fn detect_contradictions(&self, findings: &[Finding]) -> Vec<ResearchContradiction> {
        let mut contradictions = Vec::new();

        for i in 0..findings.len() {
            for j in (i + 1)..findings.len() {
                let sim = super::deduplication::compute_similarity(
                    &findings[i].statement,
                    &findings[j].statement,
                );

                if sim > 0.5 && sim < 0.95 {
                    let semantic_diff = compute_semantic_differences(
                        &findings[i].statement,
                        &findings[j].statement,
                    );

                    if semantic_diff > 0.3 {
                        let severity = if (findings[i].confidence - findings[j].confidence).abs() > 0.4 {
                            ContradictionSeverity::Critical
                        } else if (findings[i].confidence - findings[j].confidence).abs() > 0.2 {
                            ContradictionSeverity::Moderate
                        } else {
                            ContradictionSeverity::Minor
                        };

                        contradictions.push(ResearchContradiction {
                            finding_a_id: findings[i].id,
                            finding_b_id: findings[j].id,
                            statement_a: findings[i].statement.clone(),
                            statement_b: findings[j].statement.clone(),
                            severity,
                            resolution: None,
                        });
                    }
                }
            }
        }

        contradictions
    }

    fn compute_overall_confidence(&self, findings: &[Finding]) -> f32 {
        if findings.is_empty() {
            return 0.0;
        }

        let total: f32 = findings.iter().map(|f| f.confidence).sum();
        let base = total / findings.len() as f32;

        let source_diversity = self.count_unique_sources(findings) as f32;
        let diversity_bonus = (source_diversity * 0.05).min(0.2);

        (base + diversity_bonus).min(1.0)
    }

    fn count_unique_sources(&self, findings: &[Finding]) -> usize {
        let mut sources: std::collections::HashSet<String> = std::collections::HashSet::new();
        for finding in findings {
            for evidence in &finding.evidence {
                sources.insert(evidence.source_name.clone());
            }
        }
        sources.len()
    }

    fn merge_weighted(&self, groups: Vec<Vec<Finding>>) -> Vec<Finding> {
        let mut all_findings: Vec<Finding> = groups.into_iter().flatten().collect();
        all_findings.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut deduped = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

        for finding in all_findings {
            let key = finding.statement.to_lowercase().trim().to_string();
            if !seen.contains(&key) {
                seen.insert(key);
                deduped.push(finding);
            }
        }

        deduped
    }

    fn merge_most_confident(&self, groups: Vec<Vec<Finding>>) -> Vec<Finding> {
        let mut all_findings: Vec<Finding> = groups.into_iter().flatten().collect();
        all_findings.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut result = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

        for finding in all_findings {
            let key = normalize_finding_key(&finding.statement);
            if !seen.contains(&key) {
                seen.insert(key);
                result.push(finding);
            }
        }

        result
    }

    fn merge_majority(&self, groups: Vec<Vec<Finding>>) -> Vec<Finding> {
        let all_findings: Vec<Finding> = groups.into_iter().flatten().collect();

        let mut by_key: std::collections::HashMap<String, Vec<Finding>> =
            std::collections::HashMap::new();
        for finding in all_findings {
            let key = normalize_finding_key(&finding.statement);
            by_key.entry(key).or_default().push(finding);
        }

        by_key
            .into_values()
            .map(|group| {
                let count = group.len() as f32;
                let avg_conf: f32 = group.iter().map(|f| f.confidence).sum::<f32>() / count;
                let mut best = group.into_iter().max_by(|a, b| {
                    a.confidence
                        .partial_cmp(&b.confidence)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }).unwrap();
                best.confidence = (avg_conf + (count * 0.05).min(0.2)).min(1.0);
                best
            })
            .collect()
    }

    fn merge_source_priority(&self, groups: Vec<Vec<Finding>>) -> Vec<Finding> {
        self.merge_most_confident(groups)
    }

    fn propose_knowledge_updates(
        &self,
        facts: &[ValidatedFact],
    ) -> Vec<super::api::KnowledgeUpdateProposal> {
        facts
            .iter()
            .filter(|f| f.confidence >= 0.7)
            .map(|f| super::api::KnowledgeUpdateProposal {
                entity_name: f.fact.subject.clone(),
                entity_type: "ResearchDerived".to_string(),
                relationships: Vec::new(),
                facts: vec![super::api::FactProposal {
                    subject: f.fact.subject.clone(),
                    predicate: f.fact.predicate.clone(),
                    object: f.fact.object.clone(),
                    confidence: f.confidence,
                }],
                confidence: f.confidence,
                source_citations: Vec::new(),
                requires_approval: true,
            })
            .collect()
    }

    fn propose_world_updates(
        &self,
        facts: &[ValidatedFact],
    ) -> Vec<super::api::WorldUpdateProposal> {
        facts
            .iter()
            .filter(|f| f.confidence >= 0.7)
            .filter(|f| {
                f.fact.predicate.to_lowercase().contains("located")
                    || f.fact.predicate.to_lowercase().contains("event")
                    || f.fact.predicate.to_lowercase().contains("happened")
            })
            .map(|f| super::api::WorldUpdateProposal {
                entity_name: f.fact.subject.clone(),
                entity_type: "ResearchDerived".to_string(),
                state_changes: {
                    let mut m = std::collections::HashMap::new();
                    m.insert(f.fact.predicate.clone(), f.fact.object.clone());
                    m
                },
                location: None,
                events: Vec::new(),
                confidence: f.confidence,
                source_citations: Vec::new(),
                requires_approval: true,
            })
            .collect()
    }

    fn propose_memory_updates(
        &self,
        findings: &[Finding],
        objective: &str,
    ) -> Vec<super::api::MemoryUpdateProposal> {
        let high_conf: Vec<&Finding> = findings.iter().filter(|f| f.confidence >= 0.6).collect();

        if high_conf.is_empty() {
            return Vec::new();
        }

        let content = high_conf
            .iter()
            .map(|f| f.statement.as_str())
            .collect::<Vec<_>>()
            .join("; ");

        let mut context = std::collections::HashMap::new();
        context.insert("research_objective".to_string(), objective.to_string());
        context.insert("findings_count".to_string(), findings.len().to_string());

        vec![super::api::MemoryUpdateProposal {
            content,
            memory_type: "semantic".to_string(),
            importance: high_conf.iter().map(|f| f.confidence).sum::<f32>()
                / high_conf.len() as f32,
            context,
            source_citations: Vec::new(),
        }]
    }
}

fn normalize_finding_key(statement: &str) -> String {
    statement
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

fn compute_semantic_differences(a: &str, b: &str) -> f32 {
    let tokens_a: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let tokens_b: std::collections::HashSet<&str> = b.split_whitespace().collect();

    let intersection = tokens_a.intersection(&tokens_b).count();
    let total = tokens_a.len() + tokens_b.len();

    if total == 0 {
        return 0.0;
    }

    1.0 - (2.0 * intersection as f32 / total as f32)
}
