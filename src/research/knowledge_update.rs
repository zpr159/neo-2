use super::api::ValidatedFact;
use super::config::KnowledgeUpdateConfig;
use super::error::{ResearchError, ResearchResult};

/// Manages validated knowledge updates proposed by research.
pub struct KnowledgeUpdateManager {
    config: KnowledgeUpdateConfig,
}

impl KnowledgeUpdateManager {
    pub fn new(config: KnowledgeUpdateConfig) -> Self {
        Self { config }
    }

    /// Check if a knowledge update is allowed.
    pub fn can_update(&self, confidence: f32) -> bool {
        self.config.enabled && confidence >= self.config.min_confidence_to_update
    }

    /// Filter proposals to those that pass governance.
    pub fn filter_approved(
        &self,
        proposals: Vec<super::api::KnowledgeUpdateProposal>,
    ) -> Vec<super::api::KnowledgeUpdateProposal> {
        if !self.config.enabled {
            return Vec::new();
        }

        proposals
            .into_iter()
            .filter(|p| p.confidence >= self.config.min_confidence_to_update)
            .take(self.config.max_updates_per_task)
            .collect()
    }

    /// Propose a knowledge update from a validated fact.
    pub fn propose_update(
        &self,
        fact: &ValidatedFact,
    ) -> ResearchResult<super::api::KnowledgeUpdateProposal> {
        if !self.can_update(fact.confidence) {
            return Err(ResearchError::GovernanceRejected(format!(
                "confidence {} below threshold {}",
                fact.confidence, self.config.min_confidence_to_update
            )));
        }

        Ok(super::api::KnowledgeUpdateProposal {
            entity_name: fact.fact.subject.clone(),
            entity_type: "ResearchDerived".to_string(),
            relationships: Vec::new(),
            facts: vec![super::api::FactProposal {
                subject: fact.fact.subject.clone(),
                predicate: fact.fact.predicate.clone(),
                object: fact.fact.object.clone(),
                confidence: fact.confidence,
            }],
            confidence: fact.confidence,
            source_citations: Vec::new(),
            requires_approval: self.config.require_governance_approval,
        })
    }

    /// Merge related proposals into a single update.
    pub fn merge_proposals(
        &self,
        proposals: Vec<super::api::KnowledgeUpdateProposal>,
    ) -> Vec<super::api::KnowledgeUpdateProposal> {
        let mut by_entity: std::collections::HashMap<String, super::api::KnowledgeUpdateProposal> =
            std::collections::HashMap::new();

        for proposal in proposals {
            let key = proposal.entity_name.to_lowercase();
            by_entity
                .entry(key)
                .and_modify(|existing| {
                    existing.facts.extend(proposal.facts.clone());
                    existing.confidence = existing.confidence.max(proposal.confidence);
                    existing.relationships.extend(proposal.relationships.clone());
                })
                .or_insert(proposal);
        }

        by_entity.into_values().collect()
    }
}
