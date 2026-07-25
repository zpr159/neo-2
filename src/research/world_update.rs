use super::api::ValidatedFact;
use super::config::WorldUpdateConfig;
use super::error::{ResearchError, ResearchResult};

/// Manages validated world model updates proposed by research.
pub struct WorldUpdateManager {
    config: WorldUpdateConfig,
}

impl WorldUpdateManager {
    pub fn new(config: WorldUpdateConfig) -> Self {
        Self { config }
    }

    /// Check if a world model update is allowed.
    pub fn can_update(&self, confidence: f32) -> bool {
        self.config.enabled && confidence >= self.config.min_confidence_to_update
    }

    /// Filter proposals to those that pass governance.
    pub fn filter_approved(
        &self,
        proposals: Vec<super::api::WorldUpdateProposal>,
    ) -> Vec<super::api::WorldUpdateProposal> {
        if !self.config.enabled {
            return Vec::new();
        }

        proposals
            .into_iter()
            .filter(|p| p.confidence >= self.config.min_confidence_to_update)
            .take(self.config.max_updates_per_task)
            .collect()
    }

    /// Propose a world model update from a validated fact.
    pub fn propose_update(
        &self,
        fact: &ValidatedFact,
    ) -> ResearchResult<super::api::WorldUpdateProposal> {
        if !self.can_update(fact.confidence) {
            return Err(ResearchError::GovernanceRejected(format!(
                "confidence {} below threshold {}",
                fact.confidence, self.config.min_confidence_to_update
            )));
        }

        let mut state_changes = std::collections::HashMap::new();
        state_changes.insert(fact.fact.predicate.clone(), fact.fact.object.clone());

        Ok(super::api::WorldUpdateProposal {
            entity_name: fact.fact.subject.clone(),
            entity_type: "ResearchDerived".to_string(),
            state_changes,
            location: None,
            events: Vec::new(),
            confidence: fact.confidence,
            source_citations: Vec::new(),
            requires_approval: self.config.require_governance_approval,
        })
    }
}
