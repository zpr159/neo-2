use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::{EvolutionId, RiskLevel, SubsystemTarget};

/// Depth of authority granted for evolution operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorizationLevel {
    /// No authorisation.
    None,
    /// Basic read-only access.
    Basic,
    /// Elevated access including non-destructive mutations.
    Elevated,
    /// Full access to all operations.
    Full,
}

/// A scoped, time-bounded authorisation for evolution operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionAuthorization {
    /// The authority level granted.
    pub level: AuthorizationLevel,
    /// Subsystems this authorisation covers.
    pub allowed_subsystems: Vec<SubsystemTarget>,
    /// Maximum risk level permitted under this authorisation.
    pub max_risk_level: RiskLevel,
    /// Who granted the authorisation.
    pub approver: Option<String>,
    /// When the authorisation expires (`None` = no expiry).
    pub expires_at: Option<DateTime<Utc>>,
}

impl EvolutionAuthorization {
    /// Create a new authorisation record.
    pub fn new(
        level: AuthorizationLevel,
        allowed_subsystems: Vec<SubsystemTarget>,
        max_risk_level: RiskLevel,
        approver: Option<String>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            level,
            allowed_subsystems,
            max_risk_level,
            approver,
            expires_at,
        }
    }

    /// Returns `true` if the given evolution operation is permitted under
    /// this authorisation.
    ///
    /// Checks:
    /// - The authorisation has not expired.
    /// - The level is not `None`.
    /// - The risk level is within bounds.
    /// - The target subsystem is in the allowed list (if non-empty).
    pub fn authorize(&self, target: SubsystemTarget, risk_level: RiskLevel) -> bool {
        if self.is_expired() {
            return false;
        }
        if self.level == AuthorizationLevel::None {
            return false;
        }
        if risk_level > self.max_risk_level {
            return false;
        }
        if !self.allowed_subsystems.is_empty() && !self.allowed_subsystems.contains(&target) {
            return false;
        }
        true
    }

    /// Returns `true` if the authorisation has expired.
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(expiry) => Utc::now() > expiry,
            None => false,
        }
    }

    /// Returns `true` if this authorisation covers the given subsystem.
    pub fn has_permission(&self, target: &SubsystemTarget) -> bool {
        if self.is_expired() || self.level == AuthorizationLevel::None {
            return false;
        }
        self.allowed_subsystems.is_empty() || self.allowed_subsystems.contains(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorize_within_bounds() {
        let auth = EvolutionAuthorization::new(
            AuthorizationLevel::Full,
            vec![SubsystemTarget::Core, SubsystemTarget::Memory],
            RiskLevel::High,
            Some("admin".to_string()),
            None,
        );
        assert!(auth.authorize(SubsystemTarget::Core, RiskLevel::Medium));
    }

    #[test]
    fn authorize_rejects_unknown_subsystem() {
        let auth = EvolutionAuthorization::new(
            AuthorizationLevel::Full,
            vec![SubsystemTarget::Core],
            RiskLevel::High,
            None,
            None,
        );
        assert!(!auth.authorize(SubsystemTarget::Agents, RiskLevel::Low));
    }

    #[test]
    fn authorize_rejects_high_risk() {
        let auth = EvolutionAuthorization::new(
            AuthorizationLevel::Basic,
            vec![],
            RiskLevel::Low,
            None,
            None,
        );
        assert!(!auth.authorize(SubsystemTarget::Core, RiskLevel::Critical));
    }

    #[test]
    fn expired_authorization_rejected() {
        let auth = EvolutionAuthorization::new(
            AuthorizationLevel::Full,
            vec![],
            RiskLevel::Critical,
            None,
            Some(Utc::now() - chrono::Duration::hours(1)),
        );
        assert!(auth.is_expired());
        assert!(!auth.authorize(SubsystemTarget::Core, RiskLevel::Low));
    }

    #[test]
    fn has_permission_empty_list_means_all() {
        let auth = EvolutionAuthorization::new(
            AuthorizationLevel::Elevated,
            vec![],
            RiskLevel::High,
            None,
            None,
        );
        assert!(auth.has_permission(&SubsystemTarget::Runtime));
    }
}
