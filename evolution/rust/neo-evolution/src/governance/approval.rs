use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::types::EvolutionId;

/// Disposition of an approval request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    /// Awaiting a decision.
    Pending,
    /// Granted.
    Approved,
    /// Denied.
    Rejected,
    /// Timed out without a decision.
    Expired,
}

/// A single approval request and its resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionApproval {
    /// Unique identifier for this approval record.
    pub id: EvolutionId,
    /// The proposal that triggered this approval request.
    pub proposal_id: EvolutionId,
    /// Current status.
    pub status: ApprovalStatus,
    /// Who resolved (or will resolve) the request.
    pub approver: String,
    /// Reason provided by the approver.
    pub reason: String,
    /// When the request was submitted.
    pub requested_at: DateTime<Utc>,
    /// When the request was resolved (`None` if still pending).
    pub resolved_at: Option<DateTime<Utc>>,
    /// When the request expires without resolution (`None` = no expiry).
    pub expires_at: Option<DateTime<Utc>>,
}

impl EvolutionApproval {
    /// Mark the approval as granted with the given reason.
    pub fn approve(&mut self, reason: impl Into<String>) {
        self.status = ApprovalStatus::Approved;
        self.reason = reason.into();
        self.resolved_at = Some(Utc::now());
    }

    /// Mark the approval as denied with the given reason.
    pub fn reject(&mut self, reason: impl Into<String>) {
        self.status = ApprovalStatus::Rejected;
        self.reason = reason.into();
        self.resolved_at = Some(Utc::now());
    }

    /// Returns `true` if the approval has expired without resolution.
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(expiry) => Utc::now() > expiry && self.status == ApprovalStatus::Pending,
            None => false,
        }
    }

    /// Returns `true` if the approval is in a terminal valid state
    /// (Approved or still Pending and not expired).
    pub fn is_valid(&self) -> bool {
        matches!(
            self.status,
            ApprovalStatus::Approved | ApprovalStatus::Pending
        ) && !self.is_expired()
    }
}

/// Thread-safe manager for evolution approval workflows.
#[derive(Debug)]
pub struct ApprovalManager {
    /// All tracked approvals keyed by their unique id.
    approvals: DashMap<EvolutionId, EvolutionApproval>,
}

impl ApprovalManager {
    /// Create an empty approval manager.
    pub fn new() -> Self {
        Self {
            approvals: DashMap::new(),
        }
    }

    /// Submit a new approval request.
    ///
    /// Returns the generated [`EvolutionApproval`].
    pub fn request_approval(
        &self,
        proposal_id: EvolutionId,
        approver: String,
        expires_at: Option<DateTime<Utc>>,
    ) -> EvolutionApproval {
        let id = EvolutionId::new_v4();
        let approval = EvolutionApproval {
            id,
            proposal_id,
            status: ApprovalStatus::Pending,
            approver,
            reason: String::new(),
            requested_at: Utc::now(),
            resolved_at: None,
            expires_at,
        };
        self.approvals.insert(id, approval.clone());
        approval
    }

    /// Approve an existing request by id.
    pub fn approve(
        &self,
        approval_id: &EvolutionId,
        reason: impl Into<String>,
    ) -> Result<EvolutionApproval, String> {
        self.approvals
            .get_mut(approval_id)
            .ok_or_else(|| format!("approval {approval_id} not found"))
            .map(|mut entry| {
                entry.approve(reason);
                entry.value().clone()
            })
    }

    /// Reject an existing request by id.
    pub fn reject(
        &self,
        approval_id: &EvolutionId,
        reason: impl Into<String>,
    ) -> Result<EvolutionApproval, String> {
        self.approvals
            .get_mut(approval_id)
            .ok_or_else(|| format!("approval {approval_id} not found"))
            .map(|mut entry| {
                entry.reject(reason);
                entry.value().clone()
            })
    }

    /// Retrieve an approval by its id.
    pub fn get_approval(&self, approval_id: &EvolutionId) -> Option<EvolutionApproval> {
        self.approvals.get(approval_id).map(|r| r.value().clone())
    }

    /// Return all approvals that are still pending.
    pub fn list_pending(&self) -> Vec<EvolutionApproval> {
        self.approvals
            .iter()
            .filter(|r| r.status == ApprovalStatus::Pending)
            .map(|r| r.value().clone())
            .collect()
    }

    /// Return all approvals matching the given status.
    pub fn list_by_status(&self, status: ApprovalStatus) -> Vec<EvolutionApproval> {
        self.approvals
            .iter()
            .filter(|r| r.status == status)
            .map(|r| r.value().clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_and_approve() {
        let mgr = ApprovalManager::new();
        let proposal = EvolutionId::new_v4();
        let approval = mgr.request_approval(proposal, "admin".to_string(), None);
        assert_eq!(approval.status, ApprovalStatus::Pending);

        let result = mgr.approve(&approval.id, "Looks good");
        assert!(result.is_ok());
        let approved = result.unwrap();
        assert_eq!(approved.status, ApprovalStatus::Approved);
        assert_eq!(approved.reason, "Looks good");
    }

    #[test]
    fn reject_works() {
        let mgr = ApprovalManager::new();
        let proposal = EvolutionId::new_v4();
        let approval = mgr.request_approval(proposal, "admin".to_string(), None);
        let result = mgr.reject(&approval.id, "Too risky");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, ApprovalStatus::Rejected);
    }

    #[test]
    fn approve_nonexistent_fails() {
        let mgr = ApprovalManager::new();
        assert!(mgr.approve(&EvolutionId::new_v4(), "n/a").is_err());
    }

    #[test]
    fn list_pending_filters_correctly() {
        let mgr = ApprovalManager::new();
        let p1 = EvolutionId::new_v4();
        let p2 = EvolutionId::new_v4();
        let a1 = mgr.request_approval(p1, "a".to_string(), None);
        let a2 = mgr.request_approval(p2, "b".to_string(), None);
        mgr.approve(&a1.id, "ok").unwrap();
        let pending = mgr.list_pending();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, a2.id);
    }

    #[test]
    fn is_expired_for_pending() {
        let mut approval = EvolutionApproval {
            id: EvolutionId::new_v4(),
            proposal_id: EvolutionId::new_v4(),
            status: ApprovalStatus::Pending,
            approver: "admin".to_string(),
            reason: String::new(),
            requested_at: Utc::now(),
            resolved_at: None,
            expires_at: Some(Utc::now() - chrono::Duration::hours(1)),
        };
        assert!(approval.is_expired());
        assert!(!approval.is_valid());
    }
}
