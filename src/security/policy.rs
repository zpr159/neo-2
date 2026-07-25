/// Policy enforcement engine for the Neo security layer.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Whether a policy rule permits or denies the action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PolicyEffect {
    /// The action is allowed.
    Allow,
    /// The action is denied.
    Deny,
}

impl std::fmt::Display for PolicyEffect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Allow => write!(f, "allow"),
            Self::Deny => write!(f, "deny"),
        }
    }
}

/// A single rule within a policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Unique rule identifier.
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// Pattern matched against the subject (user_id or role).
    pub subject_pattern: String,
    /// Pattern matched against the resource.
    pub resource_pattern: String,
    /// The action this rule applies to.
    pub action: String,
    /// Whether the rule allows or denies the action.
    pub effect: PolicyEffect,
    /// Optional conditions (key-value pairs that must hold for the rule to apply).
    pub conditions: HashMap<String, String>,
}

/// A named, versioned collection of policy rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Unique policy identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// The rules in this policy.
    pub rules: Vec<PolicyRule>,
    /// Version string.
    pub version: String,
}

/// Evaluates policies to determine access.
#[derive(Debug)]
pub struct PolicyEngine {
    policies: RwLock<Vec<Policy>>,
}

impl PolicyEngine {
    /// Create a new, empty PolicyEngine.
    pub fn new() -> Self {
        Self {
            policies: RwLock::new(Vec::new()),
        }
    }

    /// Add a policy to the engine.
    pub async fn add_policy(&self, policy: Policy) {
        tracing::info!(policy_id = %policy.id, name = %policy.name, "policy added");
        let mut policies = self.policies.write().await;
        policies.push(policy);
    }

    /// Evaluate whether a subject may perform an action on a resource.
    pub async fn evaluate(
        &self,
        subject: &str,
        resource: &str,
        action: &str,
    ) -> PolicyEffect {
        let policies = self.policies.read().await;
        // Rules are evaluated in order; first match wins.
        for policy in policies.iter() {
            for rule in &policy.rules {
                if Self::matches(&rule.subject_pattern, subject)
                    && Self::matches(&rule.resource_pattern, resource)
                    && rule.action == action
                {
                    tracing::debug!(
                        rule_id = %rule.id,
                        effect = %rule.effect,
                        "policy rule matched"
                    );
                    return rule.effect.clone();
                }
            }
        }
        // Default deny when no rule matches.
        PolicyEffect::Deny
    }

    /// Convenience method: returns true if access is allowed.
    pub async fn check_access(
        &self,
        user_id: &str,
        roles: &[String],
        resource: &str,
        action: &str,
    ) -> bool {
        // Check against user_id first.
        if self.evaluate(user_id, resource, action).await == PolicyEffect::Allow {
            return true;
        }
        // Then check each role.
        for role in roles {
            if self.evaluate(role, resource, action).await == PolicyEffect::Allow {
                return true;
            }
        }
        false
    }

    /// List all policies currently registered.
    pub async fn list_policies(&self) -> Vec<Policy> {
        let policies = self.policies.read().await;
        policies.clone()
    }

    /// Simple glob-style pattern match: `*` matches any sequence of characters.
    fn matches(pattern: &str, value: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        if !pattern.contains('*') {
            return pattern == value;
        }
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            let prefix = parts[0];
            let suffix = parts[1];
            value.starts_with(prefix) && value.ends_with(suffix)
        } else {
            pattern == value
        }
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}
