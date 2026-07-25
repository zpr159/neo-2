use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PolicyAction {
    Allow,
    Deny,
    Log,
    Alert,
    Terminate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub name: String,
    pub description: String,
    pub action: PolicyAction,
    pub conditions: HashMap<String, serde_json::Value>,
    pub priority: u32,
    pub enabled: bool,
}

impl PolicyRule {
    pub fn new(name: &str, description: &str, action: PolicyAction, priority: u32) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            action,
            conditions: HashMap::new(),
            priority,
            enabled: true,
        }
    }

    pub fn with_condition(mut self, key: &str, value: serde_json::Value) -> Self {
        self.conditions.insert(key.to_string(), value);
        self
    }

    pub fn matches(&self, context: &HashMap<String, serde_json::Value>) -> bool {
        self.conditions.iter().all(|(key, required)| {
            match context.get(key) {
                Some(actual) => actual == required,
                None => false,
            }
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub rules: Vec<PolicyRule>,
    pub name: String,
    pub version: u32,
}

impl SecurityPolicy {
    pub fn new(name: &str) -> Self {
        tracing::info!(policy_name = name, "security policy created");
        Self {
            rules: Vec::new(),
            name: name.to_string(),
            version: 1,
        }
    }

    pub fn add_rule(&mut self, rule: PolicyRule) {
        tracing::debug!(rule_name = %rule.name, priority = rule.priority, "rule added");
        self.rules.push(rule);
        self.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    pub fn evaluate(&self, context: &HashMap<String, serde_json::Value>) -> PolicyAction {
        for rule in &self.rules {
            if rule.enabled && rule.matches(context) {
                tracing::debug!(
                    rule_name = %rule.name,
                    action = ?rule.action,
                    "rule matched"
                );
                return rule.action.clone();
            }
        }
        tracing::debug!("no matching rule found, defaulting to Deny");
        PolicyAction::Deny
    }

    pub fn disable_rule(&mut self, name: &str) {
        if let Some(rule) = self.rules.iter_mut().find(|r| r.name == name) {
            rule.enabled = false;
            tracing::info!(rule_name = name, "rule disabled");
        }
    }

    pub fn enable_rule(&mut self, name: &str) {
        if let Some(rule) = self.rules.iter_mut().find(|r| r.name == name) {
            rule.enabled = true;
            tracing::info!(rule_name = name, "rule enabled");
        }
    }

    pub fn active_rules(&self) -> Vec<&PolicyRule> {
        self.rules.iter().filter(|r| r.enabled).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_creation() {
        let policy = SecurityPolicy::new("test-policy");
        assert_eq!(policy.name, "test-policy");
        assert_eq!(policy.version, 1);
    }

    #[test]
    fn test_rule_matching() {
        let rule = PolicyRule::new("allow-read", "allow read access", PolicyAction::Allow, 10)
            .with_condition("action".to_string(), serde_json::json!("read"));

        let mut context = HashMap::new();
        context.insert("action".to_string(), serde_json::json!("read"));
        assert!(rule.matches(&context));

        context.insert("action".to_string(), serde_json::json!("write"));
        assert!(!rule.matches(&context));
    }

    #[test]
    fn test_policy_evaluation() {
        let mut policy = SecurityPolicy::new("test");
        policy.add_rule(
            PolicyRule::new("high-pri", "high priority", PolicyAction::Allow, 100)
                .with_condition("role".to_string(), serde_json::json!("admin")),
        );
        policy.add_rule(
            PolicyRule::new("low-pri", "low priority", PolicyAction::Deny, 1)
                .with_condition("role".to_string(), serde_json::json!("guest")),
        );

        let mut ctx = HashMap::new();
        ctx.insert("role".to_string(), serde_json::json!("admin"));
        assert_eq!(policy.evaluate(&ctx), PolicyAction::Allow);

        ctx.insert("role".to_string(), serde_json::json!("guest"));
        assert_eq!(policy.evaluate(&ctx), PolicyAction::Deny);
    }

    #[test]
    fn test_disable_enable_rule() {
        let mut policy = SecurityPolicy::new("test");
        policy.add_rule(PolicyRule::new("r1", "rule 1", PolicyAction::Allow, 10));

        policy.disable_rule("r1");
        assert_eq!(policy.active_rules().len(), 0);

        policy.enable_rule("r1");
        assert_eq!(policy.active_rules().len(), 1);
    }
}
