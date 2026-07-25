use super::api::{
    Citation, ExtractedFact, ResearchContradiction, ResearchEvidence,
    ResearchProvenance, ValidatedFact, ContradictionSeverity,
};
use super::config::ValidatorConfig;
use super::error::ResearchResult;
use crate::time::Timestamp;

/// Validates extracted facts against multiple sources and assigns confidence.
pub struct FactValidator {
    config: ValidatorConfig,
}

impl FactValidator {
    pub fn new(config: ValidatorConfig) -> Self {
        Self { config }
    }

    /// Validate a single extracted fact.
    pub fn validate_fact(
        &self,
        fact: &ExtractedFact,
        source_url: &str,
        source_name: &str,
    ) -> ResearchResult<ValidatedFact> {
        let confidence = self.compute_confidence(fact, 1, false);

        Ok(ValidatedFact {
            fact: fact.clone(),
            confidence,
            provenance: ResearchProvenance {
                chain: vec![super::api::ProvenanceEntry {
                    source: source_name.to_string(),
                    operation: "extracted".to_string(),
                    timestamp: Timestamp::now(),
                    confidence,
                }],
                root_source: source_url.to_string(),
                derivation_method: "rule_based_extraction".to_string(),
            },
            supporting_evidence_count: 1,
            conflicting_evidence_count: 0,
            is_conflict: false,
            validated_at: Timestamp::now(),
        })
    }

    /// Validate facts by cross-referencing across multiple extractions.
    pub fn validate_cross_source(
        &self,
        facts: &[ExtractedFact],
        source_urls: &[String],
        source_names: &[String],
    ) -> ResearchResult<Vec<ValidatedFact>> {
        let mut validated = Vec::new();
        let mut groups: std::collections::HashMap<String, Vec<&ExtractedFact>> =
            std::collections::HashMap::new();

        for fact in facts {
            let key = format!(
                "{}|{}|{}",
                fact.subject.to_lowercase(),
                fact.predicate.to_lowercase(),
                fact.object.to_lowercase()
            );
            groups.entry(key).or_default().push(fact);
        }

        for (_key, group) in &groups {
            let supporting_count = group.len();
            let has_multiple_sources = supporting_count > 1;
            let avg_confidence: f32 =
                group.iter().map(|f| f.confidence).sum::<f32>() / supporting_count as f32;

            let final_confidence = self.compute_confidence(
                group[0],
                supporting_count,
                has_multiple_sources,
            );

            let provenance_entries: Vec<super::api::ProvenanceEntry> = group
                .iter()
                .enumerate()
                .map(|(i, f)| super::api::ProvenanceEntry {
                    source: source_names
                        .get(i % source_names.len())
                        .cloned()
                        .unwrap_or_else(|| "unknown".to_string()),
                    operation: "cross_validated".to_string(),
                    timestamp: Timestamp::now(),
                    confidence: f.confidence,
                })
                .collect();

            validated.push(ValidatedFact {
                fact: (*group[0]).clone(),
                confidence: final_confidence,
                provenance: ResearchProvenance {
                    chain: provenance_entries,
                    root_source: source_urls
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "unknown".to_string()),
                    derivation_method: if has_multiple_sources {
                        "cross_source_validation".to_string()
                    } else {
                        "single_source_validation".to_string()
                    },
                },
                supporting_evidence_count: supporting_count,
                conflicting_evidence_count: 0,
                is_conflict: false,
                validated_at: Timestamp::now(),
            });

            let _ = avg_confidence;
        }

        Ok(validated)
    }

    /// Detect contradictions between validated facts.
    pub fn detect_contradictions(
        &self,
        facts: &[ValidatedFact],
    ) -> Vec<ResearchContradiction> {
        let mut contradictions = Vec::new();

        for i in 0..facts.len() {
            for j in (i + 1)..facts.len() {
                if facts[i].fact.subject.to_lowercase() == facts[j].fact.subject.to_lowercase()
                    && facts[i].fact.predicate.to_lowercase()
                        == facts[j].fact.predicate.to_lowercase()
                    && facts[i].fact.object.to_lowercase() != facts[j].fact.object.to_lowercase()
                {
                    let severity = if (facts[i].confidence - facts[j].confidence).abs() > 0.4 {
                        ContradictionSeverity::Critical
                    } else if (facts[i].confidence - facts[j].confidence).abs() > 0.2 {
                        ContradictionSeverity::Moderate
                    } else {
                        ContradictionSeverity::Minor
                    };

                    contradictions.push(ResearchContradiction {
                        finding_a_id: uuid::Uuid::new_v4(),
                        finding_b_id: uuid::Uuid::new_v4(),
                        statement_a: format!(
                            "{} {} {}",
                            facts[i].fact.subject, facts[i].fact.predicate, facts[i].fact.object
                        ),
                        statement_b: format!(
                            "{} {} {}",
                            facts[j].fact.subject, facts[j].fact.predicate, facts[j].fact.object
                        ),
                        severity,
                        resolution: None,
                    });
                }
            }
        }

        contradictions
    }

    /// Filter validated facts by minimum confidence threshold.
    pub fn filter_by_confidence(
        &self,
        facts: Vec<ValidatedFact>,
        threshold: Option<f32>,
    ) -> Vec<ValidatedFact> {
        let min = threshold.unwrap_or(self.config.min_confidence);
        facts.into_iter().filter(|f| f.confidence >= min).collect()
    }

    /// Create research evidence from validated facts.
    pub fn to_evidence(
        &self,
        fact: &ValidatedFact,
        citation: Option<&Citation>,
    ) -> ResearchEvidence {
        let mut content = format!(
            "{} {} {}",
            fact.fact.subject, fact.fact.predicate, fact.fact.object
        );

        if fact.supporting_evidence_count > 1 {
            content.push_str(&format!(
                " (supported by {} sources)",
                fact.supporting_evidence_count
            ));
        }

        ResearchEvidence {
            id: uuid::Uuid::new_v4(),
            content,
            source_url: citation.and_then(|c| c.source_url.clone()),
            source_name: citation
                .map(|c| c.source_name.clone())
                .unwrap_or_else(|| "unknown".to_string()),
            content_type: "validated_fact".to_string(),
            confidence: fact.confidence,
            extracted_at: fact.validated_at,
            relevance_score: fact.confidence,
        }
    }

    fn compute_confidence(
        &self,
        fact: &ExtractedFact,
        supporting_sources: usize,
        cross_validated: bool,
    ) -> f32 {
        let base = fact.confidence;
        let source_bonus = if cross_validated {
            (supporting_sources as f32 * 0.1).min(0.3)
        } else {
            0.0
        };

        (base + source_bonus).min(1.0)
    }
}
