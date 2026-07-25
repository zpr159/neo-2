use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::{EvolutionId, EvolutionStatus};

use super::candidate::ImprovementCandidate;

/// A formal proposal wrapping an [`ImprovementCandidate`] with governance metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementProposal {
    /// Unique identifier for this proposal.
    pub id: EvolutionId,
    /// The underlying candidate being proposed.
    pub candidate: ImprovementCandidate,
    /// Justification for why this improvement should be applied.
    pub justification: String,
    /// Expected positive outcomes if implemented.
    pub expected_outcomes: Vec<String>,
    /// Criteria used to verify success after implementation.
    pub success_criteria: Vec<String>,
    /// Plan for reverting if the change causes regressions.
    pub rollback_plan: String,
    /// Whether this proposal requires explicit approval before implementation.
    pub approval_required: bool,
    /// Whether the proposal has been approved.
    pub approved: bool,
    /// Identity of the approver, if approved.
    pub approver: Option<String>,
    /// When this proposal was created.
    pub proposed_at: DateTime<Utc>,
    /// When this proposal was approved, if at all.
    pub approved_at: Option<DateTime<Utc>>,
}

impl ImprovementProposal {
    /// Wrap a candidate into a proposal.
    pub fn new(
        candidate: ImprovementCandidate,
        justification: impl Into<String>,
        expected_outcomes: Vec<String>,
        success_criteria: Vec<String>,
        rollback_plan: impl Into<String>,
        approval_required: bool,
    ) -> Self {
        Self {
            id: EvolutionId::new_v4(),
            candidate,
            justification: justification.into(),
            expected_outcomes,
            success_criteria,
            rollback_plan: rollback_plan.into(),
            approval_required,
            approved: false,
            approver: None,
            proposed_at: Utc::now(),
            approved_at: None,
        }
    }

    /// Approve this proposal.
    ///
    /// Returns `Ok(())` on success, or `EvolutionError` if already approved
    /// or if approval is not required (approval is a no-op in that case but
    /// still succeeds).
    pub fn approve(&mut self, approver: impl Into<String>) -> crate::error::EvolutionResult<()> {
        if self.approved {
            return Err(crate::error::EvolutionError::InvalidStateTransition(
                format!("proposal {} is already approved", self.id),
            ));
        }
        self.approved = true;
        self.approver = Some(approver.into());
        self.approved_at = Some(Utc::now());
        self.candidate
            .set_status(crate::types::EvolutionStatus::AwaitingApproval);
        Ok(())
    }

    /// Reject this proposal.
    ///
    /// Transitions the underlying candidate to `Cancelled`.
    pub fn reject(&mut self) -> crate::error::EvolutionResult<()> {
        if self.approved {
            return Err(crate::error::EvolutionError::InvalidStateTransition(
                format!(
                    "proposal {} is already approved and cannot be rejected",
                    self.id
                ),
            ));
        }
        self.candidate
            .set_status(crate::types::EvolutionStatus::Cancelled);
        Ok(())
    }

    /// Returns `true` if this proposal is ready for implementation:
    /// it is approved (or does not require approval) and the candidate is
    /// in a non-terminal state.
    pub fn is_ready_for_implementation(&self) -> bool {
        let approval_ok = !self.approval_required || self.approved;
        let candidate_alive = matches!(
            self.candidate.status,
            EvolutionStatus::Pending | EvolutionStatus::AwaitingApproval
        );
        approval_ok && candidate_alive
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::improvement::candidate::ImprovementCandidate;
    use crate::improvement::priority::ImprovementPriority;
    use crate::types::{ImprovementCategory, RiskLevel, SubsystemTarget};

    fn make_candidate() -> ImprovementCandidate {
        ImprovementCandidate::new(
            "Test",
            "Test improvement",
            ImprovementCategory::Performance,
            SubsystemTarget::Core,
            ImprovementPriority::High,
            0.7,
            RiskLevel::Low,
            "Do the thing",
        )
    }

    #[test]
    fn approve_and_ready() {
        let mut proposal = ImprovementProposal::new(
            make_candidate(),
            "Because",
            vec!["Faster".into()],
            vec!["p99 < 20ms".into()],
            "Revert",
            true,
        );
        assert!(!proposal.is_ready_for_implementation());
        proposal.approve("admin").unwrap();
        assert!(proposal.is_ready_for_implementation());
    }

    #[test]
    fn double_approve_errors() {
        let mut proposal =
            ImprovementProposal::new(make_candidate(), "Because", vec![], vec![], "Revert", true);
        proposal.approve("admin").unwrap();
        assert!(proposal.approve("other").is_err());
    }

    #[test]
    fn reject_cancels() {
        let mut proposal =
            ImprovementProposal::new(make_candidate(), "Because", vec![], vec![], "Revert", true);
        proposal.reject().unwrap();
        assert_eq!(
            proposal.candidate.status,
            crate::types::EvolutionStatus::Cancelled
        );
        assert!(!proposal.is_ready_for_implementation());
    }

    #[test]
    fn no_approval_required_is_ready() {
        let proposal =
            ImprovementProposal::new(make_candidate(), "Because", vec![], vec![], "Revert", false);
        assert!(proposal.is_ready_for_implementation());
    }

    #[test]
    fn reject_after_approve_errors() {
        let mut proposal =
            ImprovementProposal::new(make_candidate(), "Because", vec![], vec![], "Revert", true);
        proposal.approve("admin").unwrap();
        assert!(proposal.reject().is_err());
    }
}
