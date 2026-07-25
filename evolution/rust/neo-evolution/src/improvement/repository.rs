use std::collections::HashMap;

use dashmap::DashMap;

use crate::error::{EvolutionError, EvolutionResult};
use crate::types::{EvolutionId, EvolutionStatus, SubsystemTarget};

use super::candidate::ImprovementCandidate;
use super::proposal::ImprovementProposal;

/// Thread-safe repository for improvement candidates and proposals.
///
/// Uses [`DashMap`] for concurrent reads and writes without external locking.
pub struct ImprovementRepository {
    candidates: DashMap<EvolutionId, ImprovementCandidate>,
    proposals: DashMap<EvolutionId, ImprovementProposal>,
}

impl ImprovementRepository {
    /// Create an empty repository.
    pub fn new() -> Self {
        Self {
            candidates: DashMap::new(),
            proposals: DashMap::new(),
        }
    }

    // ------------------------------------------------------------------
    // Candidate operations
    // ------------------------------------------------------------------

    /// Insert a new candidate. Returns its ID.
    pub fn add_candidate(&self, candidate: ImprovementCandidate) -> EvolutionId {
        let id = candidate.id;
        self.candidates.insert(id, candidate);
        id
    }

    /// Retrieve a clone of a candidate by ID.
    pub fn get_candidate(&self, id: &EvolutionId) -> EvolutionResult<ImprovementCandidate> {
        self.candidates
            .get(id)
            .map(|r| r.value().clone())
            .ok_or_else(|| EvolutionError::InternalError(format!("candidate {id} not found")))
    }

    /// Return clones of all candidates.
    pub fn list_candidates(&self) -> Vec<ImprovementCandidate> {
        self.candidates.iter().map(|r| r.value().clone()).collect()
    }

    /// Remove a candidate by ID.
    pub fn remove_candidate(&self, id: &EvolutionId) -> EvolutionResult<ImprovementCandidate> {
        self.candidates
            .remove(id)
            .map(|(_, v)| v)
            .ok_or_else(|| EvolutionError::InternalError(format!("candidate {id} not found")))
    }

    // ------------------------------------------------------------------
    // Proposal operations
    // ------------------------------------------------------------------

    /// Insert a new proposal. Returns its ID.
    pub fn add_proposal(&self, proposal: ImprovementProposal) -> EvolutionId {
        let id = proposal.id;
        self.proposals.insert(id, proposal);
        id
    }

    /// Retrieve a clone of a proposal by ID.
    pub fn get_proposal(&self, id: &EvolutionId) -> EvolutionResult<ImprovementProposal> {
        self.proposals
            .get(id)
            .map(|r| r.value().clone())
            .ok_or_else(|| EvolutionError::InternalError(format!("proposal {id} not found")))
    }

    /// Return clones of all proposals.
    pub fn list_proposals(&self) -> Vec<ImprovementProposal> {
        self.proposals.iter().map(|r| r.value().clone()).collect()
    }

    /// Approve a proposal by ID. The proposal must exist and not already be approved.
    pub fn approve_proposal(
        &self,
        id: &EvolutionId,
        approver: impl Into<String>,
    ) -> EvolutionResult<()> {
        let mut entry = self
            .proposals
            .get_mut(id)
            .ok_or_else(|| EvolutionError::InternalError(format!("proposal {id} not found")))?;
        entry.approve(approver)
    }

    /// Reject a proposal by ID. The proposal must exist and not already be approved.
    pub fn reject_proposal(&self, id: &EvolutionId) -> EvolutionResult<()> {
        let mut entry = self
            .proposals
            .get_mut(id)
            .ok_or_else(|| EvolutionError::InternalError(format!("proposal {id} not found")))?;
        entry.reject()
    }

    /// Return proposals whose underlying candidate targets the given subsystem.
    pub fn get_proposals_by_target(&self, target: SubsystemTarget) -> Vec<ImprovementProposal> {
        self.proposals
            .iter()
            .filter(|r| r.value().candidate.target == target)
            .map(|r| r.value().clone())
            .collect()
    }

    /// Return proposals that are pending approval or awaiting implementation.
    pub fn get_pending_proposals(&self) -> Vec<ImprovementProposal> {
        self.proposals
            .iter()
            .filter(|r| {
                matches!(
                    r.value().candidate.status,
                    EvolutionStatus::Pending | EvolutionStatus::AwaitingApproval
                )
            })
            .map(|r| r.value().clone())
            .collect()
    }

    /// Aggregate counts by proposal status.
    pub fn status_counts(&self) -> HashMap<EvolutionStatus, usize> {
        let mut map = HashMap::new();
        for entry in self.proposals.iter() {
            let status = entry.value().candidate.status;
            *map.entry(status).or_insert(0) += 1;
        }
        map
    }
}

impl Default for ImprovementRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::improvement::candidate::ImprovementCandidate;
    use crate::improvement::priority::ImprovementPriority;
    use crate::types::{ImprovementCategory, RiskLevel};

    fn make_candidate() -> ImprovementCandidate {
        ImprovementCandidate::new(
            "Test",
            "A test candidate",
            ImprovementCategory::Performance,
            SubsystemTarget::Core,
            ImprovementPriority::High,
            0.8,
            RiskLevel::Low,
            "Plan",
        )
    }

    fn make_proposal() -> ImprovementProposal {
        ImprovementProposal::new(
            make_candidate(),
            "Justification",
            vec!["Faster".into()],
            vec!["p99 < 20ms".into()],
            "Revert",
            true,
        )
    }

    #[test]
    fn add_and_get_candidate() {
        let repo = ImprovementRepository::new();
        let id = repo.add_candidate(make_candidate());
        assert!(repo.get_candidate(&id).is_ok());
        assert_eq!(repo.list_candidates().len(), 1);
    }

    #[test]
    fn remove_candidate() {
        let repo = ImprovementRepository::new();
        let id = repo.add_candidate(make_candidate());
        let removed = repo.remove_candidate(&id).unwrap();
        assert_eq!(removed.id, id);
        assert!(repo.get_candidate(&id).is_err());
    }

    #[test]
    fn add_and_approve_proposal() {
        let repo = ImprovementRepository::new();
        let mut prop = make_proposal();
        let id = repo.add_proposal(prop.clone());
        repo.approve_proposal(&id, "admin").unwrap();
        let fetched = repo.get_proposal(&id).unwrap();
        assert!(fetched.approved);
        assert_eq!(fetched.approver.as_deref(), Some("admin"));
    }

    #[test]
    fn reject_proposal() {
        let repo = ImprovementRepository::new();
        let prop = make_proposal();
        let id = repo.add_proposal(prop);
        repo.reject_proposal(&id).unwrap();
        let fetched = repo.get_proposal(&id).unwrap();
        assert_eq!(fetched.candidate.status, EvolutionStatus::Cancelled);
    }

    #[test]
    fn filter_by_target() {
        let repo = ImprovementRepository::new();
        repo.add_proposal(make_proposal());
        let mut other = make_proposal();
        other.candidate.target = SubsystemTarget::Agents;
        repo.add_proposal(other);

        let core = repo.get_proposals_by_target(SubsystemTarget::Core);
        assert_eq!(core.len(), 1);
        let agents = repo.get_proposals_by_target(SubsystemTarget::Agents);
        assert_eq!(agents.len(), 1);
    }

    #[test]
    fn pending_proposals() {
        let repo = ImprovementRepository::new();
        repo.add_proposal(make_proposal());
        let pending = repo.get_pending_proposals();
        assert_eq!(pending.len(), 1);
    }

    #[test]
    fn get_nonexistent_errors() {
        let repo = ImprovementRepository::new();
        let fake = EvolutionId::new_v4();
        assert!(repo.get_candidate(&fake).is_err());
        assert!(repo.get_proposal(&fake).is_err());
        assert!(repo.approve_proposal(&fake, "x").is_err());
        assert!(repo.reject_proposal(&fake).is_err());
    }
}
