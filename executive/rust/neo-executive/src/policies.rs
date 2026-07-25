use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::context::ExecutionMode;
use crate::error::{ExecutiveError, ExecutiveResult};

/// Permission granted by an execution policy.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    ExecuteCode,
    AccessNetwork,
    ModifyFiles,
    AccessGPU,
    RunInference,
    ModifyKnowledge,
    AccessMemory,
    InvokeReasoning,
    UseTools,
    OverridePriority,
    BypassSafetyChecks,
    SpawnSubprocess,
    AccessHardware,
}

/// Execution policy defines what actions are permitted in a given execution mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPolicy {
    pub mode: ExecutionMode,
    pub permissions: Vec<Permission>,
    pub max_concurrent_goals: usize,
    pub max_concurrent_tasks: usize,
    pub max_inference_tokens_per_decision: u64,
    pub max_reasoning_depth: u32,
    pub require_confirmation_above_risk: f64,
    pub audit_all_decisions: bool,
    pub allow_autonomous_actions: bool,
    pub resource_limits: HashMap<String, u64>,
}

impl ExecutionPolicy {
    /// Create a safe mode policy (most restrictive).
    pub fn safe_mode() -> Self {
        let mut permissions = Vec::new();
        permissions.push(Permission::AccessMemory);
        permissions.push(Permission::InvokeReasoning);
        permissions.push(Permission::AccessNetwork);

        Self {
            mode: ExecutionMode::Safe,
            permissions,
            max_concurrent_goals: 1,
            max_concurrent_tasks: 2,
            max_inference_tokens_per_decision: 1024,
            max_reasoning_depth: 3,
            require_confirmation_above_risk: 0.1,
            audit_all_decisions: true,
            allow_autonomous_actions: false,
            resource_limits: {
                let mut limits = HashMap::new();
                limits.insert("cpu".to_string(), 2);
                limits.insert("ram_mb".to_string(), 4096);
                limits
            },
        }
    }

    /// Create an interactive mode policy (balanced).
    pub fn interactive_mode() -> Self {
        let permissions = vec![
            Permission::ExecuteCode,
            Permission::AccessNetwork,
            Permission::ModifyFiles,
            Permission::AccessGPU,
            Permission::RunInference,
            Permission::ModifyKnowledge,
            Permission::AccessMemory,
            Permission::InvokeReasoning,
            Permission::UseTools,
            Permission::SpawnSubprocess,
        ];

        Self {
            mode: ExecutionMode::Interactive,
            permissions,
            max_concurrent_goals: 4,
            max_concurrent_tasks: 16,
            max_inference_tokens_per_decision: 4096,
            max_reasoning_depth: 8,
            require_confirmation_above_risk: 0.5,
            audit_all_decisions: false,
            allow_autonomous_actions: false,
            resource_limits: {
                let mut limits = HashMap::new();
                limits.insert("cpu".to_string(), 4);
                limits.insert("ram_mb".to_string(), 16384);
                limits
            },
        }
    }

    /// Create an autonomous mode policy (permissive).
    pub fn autonomous_mode() -> Self {
        let permissions = vec![
            Permission::ExecuteCode,
            Permission::AccessNetwork,
            Permission::ModifyFiles,
            Permission::AccessGPU,
            Permission::RunInference,
            Permission::ModifyKnowledge,
            Permission::AccessMemory,
            Permission::InvokeReasoning,
            Permission::UseTools,
            Permission::OverridePriority,
            Permission::SpawnSubprocess,
            Permission::AccessHardware,
        ];

        Self {
            mode: ExecutionMode::Autonomous,
            permissions,
            max_concurrent_goals: 16,
            max_concurrent_tasks: 64,
            max_inference_tokens_per_decision: 32768,
            max_reasoning_depth: 20,
            require_confirmation_above_risk: 0.8,
            audit_all_decisions: false,
            allow_autonomous_actions: true,
            resource_limits: {
                let mut limits = HashMap::new();
                limits.insert("cpu".to_string(), 8);
                limits.insert("ram_mb".to_string(), 32768);
                limits
            },
        }
    }

    /// Create a developer mode policy (unrestricted).
    pub fn developer_mode() -> Self {
        let permissions = vec![
            Permission::ExecuteCode,
            Permission::AccessNetwork,
            Permission::ModifyFiles,
            Permission::AccessGPU,
            Permission::RunInference,
            Permission::ModifyKnowledge,
            Permission::AccessMemory,
            Permission::InvokeReasoning,
            Permission::UseTools,
            Permission::OverridePriority,
            Permission::BypassSafetyChecks,
            Permission::SpawnSubprocess,
            Permission::AccessHardware,
        ];

        Self {
            mode: ExecutionMode::Developer,
            permissions,
            max_concurrent_goals: 32,
            max_concurrent_tasks: 128,
            max_inference_tokens_per_decision: 131072,
            max_reasoning_depth: 50,
            require_confirmation_above_risk: 1.0,
            audit_all_decisions: false,
            allow_autonomous_actions: true,
            resource_limits: {
                let mut limits = HashMap::new();
                limits.insert("cpu".to_string(), 16);
                limits.insert("ram_mb".to_string(), 65536);
                limits
            },
        }
    }

    /// Get the policy for a given execution mode.
    pub fn for_mode(mode: ExecutionMode) -> Self {
        match mode {
            ExecutionMode::Safe => Self::safe_mode(),
            ExecutionMode::Interactive => Self::interactive_mode(),
            ExecutionMode::Autonomous => Self::autonomous_mode(),
            ExecutionMode::Developer => Self::developer_mode(),
        }
    }

    /// Check if a permission is granted.
    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.contains(permission)
    }

    /// Check if a risk level requires confirmation.
    pub fn requires_confirmation(&self, risk_level: f64) -> bool {
        risk_level >= self.require_confirmation_above_risk
    }
}

/// Policy engine manages execution policies and enforces them.
#[derive(Clone)]
pub struct PolicyEngine {
    inner: Arc<PolicyEngineInner>,
}

struct PolicyEngineInner {
    current_policy: RwLock<ExecutionPolicy>,
    policy_history: RwLock<Vec<(ExecutionMode, chrono::DateTime<chrono::Utc>)>>,
    violations: RwLock<Vec<PolicyViolation>>,
}

/// A policy violation record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyViolation {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub permission: Permission,
    pub description: String,
    pub blocked: bool,
}

impl PolicyEngine {
    /// Create a new policy engine with the given mode.
    pub fn new(mode: ExecutionMode) -> Self {
        let policy = ExecutionPolicy::for_mode(mode);
        Self {
            inner: Arc::new(PolicyEngineInner {
                current_policy: RwLock::new(policy),
                policy_history: RwLock::new(Vec::new()),
                violations: RwLock::new(Vec::new()),
            }),
        }
    }

    /// Get the current policy.
    pub fn current_policy(&self) -> ExecutionPolicy {
        self.inner.current_policy.read().clone()
    }

    /// Switch to a new execution mode.
    pub fn switch_mode(&self, mode: ExecutionMode) {
        let policy = ExecutionPolicy::for_mode(mode);
        *self.inner.current_policy.write() = policy;
        self.inner
            .policy_history
            .write()
            .push((mode, chrono::Utc::now()));

        tracing::info!(mode = ?mode, "execution mode switched");
    }

    /// Check if a permission is allowed.
    pub fn check_permission(&self, permission: &Permission) -> bool {
        let allowed = self.inner.current_policy.read().has_permission(permission);
        if !allowed {
            self.inner.violations.write().push(PolicyViolation {
                timestamp: chrono::Utc::now(),
                permission: permission.clone(),
                description: format!("permission {:?} denied in current mode", permission),
                blocked: true,
            });
        }
        allowed
    }

    /// Enforce a permission, returning an error if denied.
    pub fn enforce_permission(&self, permission: &Permission) -> ExecutiveResult<()> {
        if self.check_permission(permission) {
            Ok(())
        } else {
            Err(ExecutiveError::policy_violation(format!(
                "permission {:?} is not allowed in {:?} mode",
                permission,
                self.inner.current_policy.read().mode
            )))
        }
    }

    /// Check if an action requires user confirmation.
    pub fn requires_confirmation(&self, risk_level: f64) -> bool {
        self.inner.current_policy.read().requires_confirmation(risk_level)
    }

    /// Get all violations.
    pub fn violations(&self) -> Vec<PolicyViolation> {
        self.inner.violations.read().clone()
    }

    /// Get violation count.
    pub fn violation_count(&self) -> usize {
        self.inner.violations.read().len()
    }

    /// Get policy history.
    pub fn policy_history(&self) -> Vec<(ExecutionMode, chrono::DateTime<chrono::Utc>)> {
        self.inner.policy_history.read().clone()
    }

    /// Get the current execution mode.
    pub fn current_mode(&self) -> ExecutionMode {
        self.inner.current_policy.read().mode
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new(ExecutionMode::Safe)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_mode_policy() {
        let policy = ExecutionPolicy::safe_mode();
        assert!(policy.has_permission(&Permission::AccessMemory));
        assert!(policy.has_permission(&Permission::InvokeReasoning));
        assert!(!policy.has_permission(&Permission::ExecuteCode));
        assert!(!policy.has_permission(&Permission::UseTools));
    }

    #[test]
    fn interactive_mode_policy() {
        let policy = ExecutionPolicy::interactive_mode();
        assert!(policy.has_permission(&Permission::ExecuteCode));
        assert!(policy.has_permission(&Permission::UseTools));
        assert!(!policy.has_permission(&Permission::OverridePriority));
    }

    #[test]
    fn autonomous_mode_policy() {
        let policy = ExecutionPolicy::autonomous_mode();
        assert!(policy.has_permission(&Permission::OverridePriority));
        assert!(!policy.has_permission(&Permission::BypassSafetyChecks));
    }

    #[test]
    fn developer_mode_policy() {
        let policy = ExecutionPolicy::developer_mode();
        assert!(policy.has_permission(&Permission::BypassSafetyChecks));
        assert!(policy.has_permission(&Permission::AccessHardware));
    }

    #[test]
    fn policy_engine_mode_switch() {
        let engine = PolicyEngine::new(ExecutionMode::Safe);
        assert_eq!(engine.current_mode(), ExecutionMode::Safe);

        engine.switch_mode(ExecutionMode::Autonomous);
        assert_eq!(engine.current_mode(), ExecutionMode::Autonomous);
        assert!(engine.current_policy().allow_autonomous_actions);
    }

    #[test]
    fn permission_enforcement() {
        let engine = PolicyEngine::new(ExecutionMode::Safe);
        assert!(engine
            .enforce_permission(&Permission::AccessMemory)
            .is_ok());
        assert!(engine
            .enforce_permission(&Permission::ExecuteCode)
            .is_err());
    }

    #[test]
    fn confirmation_required() {
        let engine = PolicyEngine::new(ExecutionMode::Interactive);
        assert!(!engine.requires_confirmation(0.3));
        assert!(engine.requires_confirmation(0.6));
    }
}
