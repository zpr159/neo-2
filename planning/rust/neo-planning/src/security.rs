//! Security and policy enforcement for the planning system.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::error::{PlanningError, PlanningResult};

/// Security level classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SecurityLevel {
    Public,
    Internal,
    Confidential,
    Restricted,
}

impl SecurityLevel {
    /// Numeric minimum level (higher = more restrictive).
    pub fn min_level(&self) -> u8 {
        match self {
            Self::Public => 0,
            Self::Internal => 1,
            Self::Confidential => 2,
            Self::Restricted => 3,
        }
    }
}

/// Permission types for plan operations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    Read,
    Write,
    Execute,
    Admin,
    PlanCreate,
    PlanExecute,
    PlanModify,
    GoalCreate,
    GoalModify,
}

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read => write!(f, "Read"),
            Self::Write => write!(f, "Write"),
            Self::Execute => write!(f, "Execute"),
            Self::Admin => write!(f, "Admin"),
            Self::PlanCreate => write!(f, "PlanCreate"),
            Self::PlanExecute => write!(f, "PlanExecute"),
            Self::PlanModify => write!(f, "PlanModify"),
            Self::GoalCreate => write!(f, "GoalCreate"),
            Self::GoalModify => write!(f, "GoalModify"),
        }
    }
}

/// An entry in the audit log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// When the action occurred.
    pub timestamp: DateTime<Utc>,
    /// What action was performed.
    pub action: String,
    /// Who performed the action.
    pub actor: String,
    /// Which resource was targeted.
    pub resource_id: String,
    /// Whether access was granted.
    pub granted: bool,
    /// Additional details.
    pub details: String,
}

/// Execution policy governing what is allowed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPolicy {
    /// The set of permissions this policy grants.
    pub allowed_permissions: Vec<Permission>,
    /// Maximum security level this policy permits.
    pub max_security_level: SecurityLevel,
    /// Whether audit logging is required.
    pub require_audit: bool,
    /// Maximum cost allowed per plan.
    pub max_cost_per_plan: f64,
    /// Optional list of allowed algorithms (None = all allowed).
    pub allowed_algorithms: Option<Vec<String>>,
}

impl ExecutionPolicy {
    /// Create a permissive default policy.
    pub fn new() -> Self {
        Self {
            allowed_permissions: vec![
                Permission::Read,
                Permission::Write,
                Permission::Execute,
                Permission::PlanCreate,
                Permission::PlanExecute,
                Permission::PlanModify,
                Permission::GoalCreate,
                Permission::GoalModify,
            ],
            max_security_level: SecurityLevel::Restricted,
            require_audit: true,
            max_cost_per_plan: 10000.0,
            allowed_algorithms: None,
        }
    }

    /// Check whether a permission is allowed by this policy.
    pub fn is_permitted(&self, permission: &Permission) -> bool {
        self.allowed_permissions.contains(permission)
    }

    /// Validate that a plan's cost is within budget.
    pub fn validate_plan_cost(&self, cost: f64) -> bool {
        cost <= self.max_cost_per_plan
    }
}

impl Default for ExecutionPolicy {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe policy engine with audit logging.
pub struct PolicyEngine {
    policy: ExecutionPolicy,
    audit_log: Arc<RwLock<Vec<AuditEntry>>>,
}

impl PolicyEngine {
    /// Create a new policy engine with the given policy.
    pub fn new(policy: ExecutionPolicy) -> Self {
        Self {
            policy,
            audit_log: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Check whether an actor has a permission for a resource.
    pub fn check_permission(&self, actor: &str, permission: &Permission, resource: &str) -> bool {
        let permitted = self.policy.is_permitted(permission);

        let entry = AuditEntry {
            timestamp: Utc::now(),
            action: permission.to_string(),
            actor: actor.to_string(),
            resource_id: resource.to_string(),
            granted: permitted,
            details: String::new(),
        };

        if self.policy.require_audit {
            self.audit_log.write().push(entry);
        }

        permitted
    }

    /// Manually log an audit entry.
    pub fn log_audit(&self, entry: AuditEntry) {
        self.audit_log.write().push(entry);
    }

    /// Return the most recent audit entries up to `limit`.
    pub fn audit_history(&self, limit: usize) -> Vec<AuditEntry> {
        let log = self.audit_log.read();
        let start = log.len().saturating_sub(limit);
        log[start..].to_vec()
    }

    /// Validate a plan's cost and algorithm against the policy.
    pub fn validate_plan_security(&self, plan_cost: f64, algorithm: &str) -> PlanningResult<()> {
        if !self.policy.validate_plan_cost(plan_cost) {
            return Err(PlanningError::policy_violation(format!(
                "plan cost {} exceeds maximum {}",
                plan_cost, self.policy.max_cost_per_plan
            )));
        }

        if let Some(ref allowed) = self.policy.allowed_algorithms {
            if !allowed.iter().any(|a| a == algorithm) {
                return Err(PlanningError::policy_violation(format!(
                    "algorithm '{}' is not in the allowed list",
                    algorithm
                )));
            }
        }

        Ok(())
    }

    /// Return the underlying policy.
    pub fn policy(&self) -> &ExecutionPolicy {
        &self.policy
    }
}

impl Clone for PolicyEngine {
    fn clone(&self) -> Self {
        Self {
            policy: self.policy.clone(),
            audit_log: Arc::clone(&self.audit_log),
        }
    }
}

/// Authorizer for plan creation and execution.
pub struct PlanAuthorizer {
    policy_engine: PolicyEngine,
}

impl PlanAuthorizer {
    /// Create a new authorizer wrapping the given policy engine.
    pub fn new(policy_engine: PolicyEngine) -> Self {
        Self { policy_engine }
    }

    /// Authorize plan creation for an actor.
    pub fn authorize_plan_creation(&self, actor: &str, cost: f64) -> PlanningResult<()> {
        if !self
            .policy_engine
            .check_permission(actor, &Permission::PlanCreate, "plan")
        {
            return Err(PlanningError::policy_violation(format!(
                "actor '{}' is not permitted to create plans",
                actor
            )));
        }

        if !self.policy_engine.policy().validate_plan_cost(cost) {
            return Err(PlanningError::policy_violation(format!(
                "plan cost {} exceeds policy maximum {}",
                cost,
                self.policy_engine.policy().max_cost_per_plan
            )));
        }

        Ok(())
    }

    /// Authorize plan execution for an actor.
    pub fn authorize_plan_execution(&self, actor: &str, plan_id: &str) -> PlanningResult<()> {
        if !self
            .policy_engine
            .check_permission(actor, &Permission::PlanExecute, plan_id)
        {
            return Err(PlanningError::policy_violation(format!(
                "actor '{}' is not permitted to execute plan '{}'",
                actor, plan_id
            )));
        }

        Ok(())
    }

    /// Return a reference to the inner policy engine.
    pub fn policy_engine(&self) -> &PolicyEngine {
        &self.policy_engine
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn security_level_ordering() {
        assert!(SecurityLevel::Public < SecurityLevel::Internal);
        assert!(SecurityLevel::Internal < SecurityLevel::Confidential);
        assert!(SecurityLevel::Confidential < SecurityLevel::Restricted);
    }

    #[test]
    fn security_level_min_level() {
        assert_eq!(SecurityLevel::Public.min_level(), 0);
        assert_eq!(SecurityLevel::Internal.min_level(), 1);
        assert_eq!(SecurityLevel::Confidential.min_level(), 2);
        assert_eq!(SecurityLevel::Restricted.min_level(), 3);
    }

    #[test]
    fn permission_display() {
        assert_eq!(Permission::Read.to_string(), "Read");
        assert_eq!(Permission::PlanCreate.to_string(), "PlanCreate");
        assert_eq!(Permission::GoalModify.to_string(), "GoalModify");
    }

    #[test]
    fn execution_policy_new() {
        let policy = ExecutionPolicy::new();
        assert!(policy.is_permitted(&Permission::Read));
        assert!(policy.is_permitted(&Permission::PlanCreate));
        assert!(policy.validate_plan_cost(100.0));
        assert!(!policy.validate_plan_cost(100000.0));
    }

    #[test]
    fn execution_policy_custom() {
        let policy = ExecutionPolicy {
            allowed_permissions: vec![Permission::Read],
            max_security_level: SecurityLevel::Internal,
            require_audit: false,
            max_cost_per_plan: 50.0,
            allowed_algorithms: Some(vec!["htn".to_string()]),
        };
        assert!(policy.is_permitted(&Permission::Read));
        assert!(!policy.is_permitted(&Permission::Write));
        assert!(policy.validate_plan_cost(50.0));
        assert!(!policy.validate_plan_cost(51.0));
    }

    #[test]
    fn execution_policy_default() {
        let policy = ExecutionPolicy::default();
        assert!(policy.is_permitted(&Permission::Admin));
        assert!(policy.require_audit);
    }

    #[test]
    fn policy_engine_check_permission_granted() {
        let engine = PolicyEngine::new(ExecutionPolicy::new());
        assert!(engine.check_permission("alice", &Permission::Read, "plan-1"));
    }

    #[test]
    fn policy_engine_check_permission_denied() {
        let policy = ExecutionPolicy {
            allowed_permissions: vec![Permission::Read],
            ..ExecutionPolicy::new()
        };
        let engine = PolicyEngine::new(policy);
        assert!(!engine.check_permission("bob", &Permission::Write, "plan-1"));
    }

    #[test]
    fn policy_engine_audit_log() {
        let engine = PolicyEngine::new(ExecutionPolicy::new());
        engine.check_permission("alice", &Permission::Read, "res");
        engine.check_permission("bob", &Permission::Write, "res");
        let history = engine.audit_history(10);
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].actor, "alice");
        assert_eq!(history[1].actor, "bob");
        assert!(history[0].granted);
        assert!(!history[1].granted);
    }

    #[test]
    fn policy_engine_audit_log_limit() {
        let engine = PolicyEngine::new(ExecutionPolicy::new());
        for i in 0..5 {
            engine.check_permission(&format!("user{}", i), &Permission::Read, "res");
        }
        let history = engine.audit_history(2);
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn policy_engine_manual_audit_log() {
        let engine = PolicyEngine::new(ExecutionPolicy::new());
        let entry = AuditEntry {
            timestamp: Utc::now(),
            action: "custom".to_string(),
            actor: "system".to_string(),
            resource_id: "r1".to_string(),
            granted: true,
            details: "manual entry".to_string(),
        };
        engine.log_audit(entry);
        let history = engine.audit_history(10);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].action, "custom");
    }

    #[test]
    fn policy_engine_validate_plan_security_ok() {
        let engine = PolicyEngine::new(ExecutionPolicy::new());
        assert!(engine.validate_plan_security(100.0, "htn").is_ok());
    }

    #[test]
    fn policy_engine_validate_plan_security_cost_exceeded() {
        let policy = ExecutionPolicy {
            max_cost_per_plan: 50.0,
            ..ExecutionPolicy::new()
        };
        let engine = PolicyEngine::new(policy);
        assert!(engine.validate_plan_security(100.0, "htn").is_err());
    }

    #[test]
    fn policy_engine_validate_plan_security_algorithm_not_allowed() {
        let policy = ExecutionPolicy {
            allowed_algorithms: Some(vec!["htn".to_string()]),
            ..ExecutionPolicy::new()
        };
        let engine = PolicyEngine::new(policy);
        assert!(engine.validate_plan_security(10.0, "custom_algo").is_err());
        assert!(engine.validate_plan_security(10.0, "htn").is_ok());
    }

    #[test]
    fn policy_engine_clone() {
        let engine = PolicyEngine::new(ExecutionPolicy::new());
        engine.check_permission("alice", &Permission::Read, "r1");
        let cloned = engine.clone();
        let history = cloned.audit_history(10);
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn plan_authorizer_create_authorized() {
        let engine = PolicyEngine::new(ExecutionPolicy::new());
        let auth = PlanAuthorizer::new(engine);
        assert!(auth.authorize_plan_creation("alice", 100.0).is_ok());
    }

    #[test]
    fn plan_authorizer_create_no_permission() {
        let policy = ExecutionPolicy {
            allowed_permissions: vec![Permission::Read],
            ..ExecutionPolicy::new()
        };
        let engine = PolicyEngine::new(policy);
        let auth = PlanAuthorizer::new(engine);
        assert!(auth.authorize_plan_creation("bob", 100.0).is_err());
    }

    #[test]
    fn plan_authorizer_create_cost_exceeded() {
        let policy = ExecutionPolicy {
            max_cost_per_plan: 50.0,
            ..ExecutionPolicy::new()
        };
        let engine = PolicyEngine::new(policy);
        let auth = PlanAuthorizer::new(engine);
        assert!(auth.authorize_plan_creation("alice", 100.0).is_err());
    }

    #[test]
    fn plan_authorizer_execute_authorized() {
        let engine = PolicyEngine::new(ExecutionPolicy::new());
        let auth = PlanAuthorizer::new(engine);
        assert!(auth.authorize_plan_execution("alice", "plan-1").is_ok());
    }

    #[test]
    fn plan_authorizer_execute_no_permission() {
        let policy = ExecutionPolicy {
            allowed_permissions: vec![Permission::Read],
            ..ExecutionPolicy::new()
        };
        let engine = PolicyEngine::new(policy);
        let auth = PlanAuthorizer::new(engine);
        assert!(auth.authorize_plan_execution("bob", "plan-1").is_err());
    }

    #[test]
    fn plan_authorizer_has_policy_engine() {
        let engine = PolicyEngine::new(ExecutionPolicy::new());
        let auth = PlanAuthorizer::new(engine);
        assert!(auth.policy_engine().policy().require_audit);
    }

    #[test]
    fn audit_entry_serialization() {
        let entry = AuditEntry {
            timestamp: Utc::now(),
            action: "test".to_string(),
            actor: "user".to_string(),
            resource_id: "r1".to_string(),
            granted: true,
            details: "detail".to_string(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let back: AuditEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.action, "test");
        assert!(back.granted);
    }

    #[test]
    fn policy_engine_no_audit_when_disabled() {
        let policy = ExecutionPolicy {
            require_audit: false,
            ..ExecutionPolicy::new()
        };
        let engine = PolicyEngine::new(policy);
        engine.check_permission("alice", &Permission::Read, "r1");
        assert!(engine.audit_history(10).is_empty());
    }
}
