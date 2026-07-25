use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use chrono::Timelike;

use crate::types::{EvolutionId, RiskLevel, SubsystemTarget};

/// The kind of constraint a governance rule enforces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceRuleType {
    /// Maximum allowed risk level.
    RiskLimit,
    /// Restricts which subsystems may be targeted.
    SubsystemRestriction,
    /// Requires human approval before proceeding.
    ApprovalRequired,
    /// Restricts execution to specific time windows.
    TimeRestriction,
    /// Limits resource budget for an evolution cycle.
    BudgetLimit,
}

/// A single governance rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceRule {
    /// Unique identifier for this rule.
    pub id: EvolutionId,
    /// Human-readable name.
    pub name: String,
    /// Category of constraint.
    pub rule_type: GovernanceRuleType,
    /// Rule-specific parameters stored as key-value pairs.
    pub parameters: HashMap<String, Value>,
    /// Whether the rule is currently enforced.
    pub enabled: bool,
}

/// Validates proposed evolution actions against a set of governance rules.
#[derive(Debug, Clone)]
pub struct EvolutionPolicyValidator {
    /// Active rules.
    rules: Vec<GovernanceRule>,
}

/// Result of a governance validation check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the proposal passes all enabled rules.
    pub valid: bool,
    /// Human-readable messages for every rule that was evaluated.
    pub messages: Vec<String>,
}

impl EvolutionPolicyValidator {
    /// Create a validator with no rules.
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Add a governance rule.
    pub fn add_rule(&mut self, rule: GovernanceRule) {
        self.rules.push(rule);
    }

    /// Validate raw proposal data against all enabled rules.
    ///
    /// The `data` map is expected to contain at least:
    /// - `"risk_level"`: a string matching a `RiskLevel` variant name
    /// - `"target"`: a string matching a `SubsystemTarget` variant name
    pub fn validate(&self, data: &HashMap<String, Value>) -> Result<ValidationResult, String> {
        let mut messages: Vec<String> = Vec::new();
        let mut valid = true;

        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }
            match rule.rule_type {
                GovernanceRuleType::RiskLimit => {
                    if let Some(limit_val) = rule.parameters.get("max_risk_level") {
                        if let Some(proposal_risk) = data.get("risk_level").and_then(|v| v.as_str())
                        {
                            let max_risk = parse_risk_level(limit_val.as_str().unwrap_or("none"));
                            let proposal = parse_risk_level(proposal_risk);
                            if proposal > max_risk {
                                valid = false;
                                messages.push(format!(
                                    "Rule '{}': proposal risk level '{}' exceeds allowed maximum '{}'",
                                    rule.name, proposal_risk, limit_val
                                ));
                            }
                        }
                    }
                }
                GovernanceRuleType::SubsystemRestriction => {
                    if let Some(allowed_val) = rule.parameters.get("allowed_subsystems") {
                        if let Some(target_str) = data.get("target").and_then(|v| v.as_str()) {
                            if let Some(arr) = allowed_val.as_array() {
                                let allowed: Vec<String> = arr
                                    .iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect();
                                if !allowed.iter().any(|a| a == target_str) {
                                    valid = false;
                                    messages.push(format!(
                                        "Rule '{}': subsystem '{}' is not in the allowed list",
                                        rule.name, target_str
                                    ));
                                }
                            }
                        }
                    }
                }
                GovernanceRuleType::ApprovalRequired => {
                    if let Some(required) = rule.parameters.get("required") {
                        if required.as_bool().unwrap_or(false) {
                            if data.get("approval_id").is_none() {
                                valid = false;
                                messages.push(format!(
                                    "Rule '{}': approval is required but no approval_id was provided",
                                    rule.name
                                ));
                            }
                        }
                    }
                }
                GovernanceRuleType::TimeRestriction => {
                    if let (Some(start), Some(end)) = (
                        rule.parameters.get("start_hour"),
                        rule.parameters.get("end_hour"),
                    ) {
                        let now = Utc::now();
                        let hour = now.hour() as u64;
                        let s = start.as_u64().unwrap_or(0);
                        let e = end.as_u64().unwrap_or(24);
                        if hour < s || hour >= e {
                            valid = false;
                            messages.push(format!(
                                "Rule '{}': current hour {hour} is outside allowed window {s}..{e}",
                                rule.name
                            ));
                        }
                    }
                }
                GovernanceRuleType::BudgetLimit => {
                    if let Some(max_budget) = rule.parameters.get("max_budget") {
                        if let Some(proposal_cost) = data.get("estimated_cost") {
                            if let (Some(max), Some(cost)) =
                                (max_budget.as_f64(), proposal_cost.as_f64())
                            {
                                if cost > max {
                                    valid = false;
                                    messages.push(format!(
                                        "Rule '{}': estimated cost {cost} exceeds budget limit {max}",
                                        rule.name
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        if messages.is_empty() {
            messages.push("All governance rules passed.".to_string());
        }

        Ok(ValidationResult { valid, messages })
    }

    /// Convenience wrapper that validates a proposal's risk level and target.
    pub fn validate_proposal(
        &self,
        risk_level: RiskLevel,
        target: SubsystemTarget,
    ) -> Result<ValidationResult, String> {
        let mut data = HashMap::new();
        data.insert(
            "risk_level".to_string(),
            Value::String(format!("{risk_level:?}").to_lowercase()),
        );
        data.insert("target".to_string(), Value::String(target.to_string()));
        self.validate(&data)
    }

    /// Return all rules currently configured.
    pub fn get_rules(&self) -> &[GovernanceRule] {
        &self.rules
    }

    /// Validate and enforce: returns an error if validation fails.
    pub fn enforce(&self, risk_level: RiskLevel, target: SubsystemTarget) -> Result<(), String> {
        let result = self.validate_proposal(risk_level, target)?;
        if result.valid {
            Ok(())
        } else {
            Err(result.messages.join("; "))
        }
    }
}

/// Map a string representation to a `RiskLevel`.
fn parse_risk_level(s: &str) -> RiskLevel {
    match s.to_lowercase().as_str() {
        "none" => RiskLevel::None,
        "low" => RiskLevel::Low,
        "medium" => RiskLevel::Medium,
        "high" => RiskLevel::High,
        "critical" => RiskLevel::Critical,
        _ => RiskLevel::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn risk_limit_enforced() {
        let mut validator = EvolutionPolicyValidator::new();
        let mut params = HashMap::new();
        params.insert(
            "max_risk_level".to_string(),
            Value::String("medium".to_string()),
        );
        validator.add_rule(GovernanceRule {
            id: EvolutionId::new_v4(),
            name: "risk_cap".to_string(),
            rule_type: GovernanceRuleType::RiskLimit,
            parameters: params,
            enabled: true,
        });

        let result = validator
            .validate_proposal(RiskLevel::High, SubsystemTarget::Core)
            .unwrap();
        assert!(!result.valid);
        assert!(result.messages.iter().any(|m| m.contains("exceeds")));
    }

    #[test]
    fn subsystem_restriction() {
        let mut validator = EvolutionPolicyValidator::new();
        let mut params = HashMap::new();
        params.insert(
            "allowed_subsystems".to_string(),
            Value::Array(vec![
                Value::String("core".to_string()),
                Value::String("memory".to_string()),
            ]),
        );
        validator.add_rule(GovernanceRule {
            id: EvolutionId::new_v4(),
            name: "subsys_cap".to_string(),
            rule_type: GovernanceRuleType::SubsystemRestriction,
            parameters: params,
            enabled: true,
        });

        let ok = validator.validate_proposal(RiskLevel::Low, SubsystemTarget::Core);
        assert!(ok.unwrap().valid);

        let bad = validator.validate_proposal(RiskLevel::Low, SubsystemTarget::Agents);
        assert!(!bad.unwrap().valid);
    }

    #[test]
    fn enforce_returns_err() {
        let mut validator = EvolutionPolicyValidator::new();
        let mut params = HashMap::new();
        params.insert(
            "max_risk_level".to_string(),
            Value::String("low".to_string()),
        );
        validator.add_rule(GovernanceRule {
            id: EvolutionId::new_v4(),
            name: "strict".to_string(),
            rule_type: GovernanceRuleType::RiskLimit,
            parameters: params,
            enabled: true,
        });

        let result = validator.enforce(RiskLevel::High, SubsystemTarget::Core);
        assert!(result.is_err());
    }

    #[test]
    fn disabled_rule_ignored() {
        let mut validator = EvolutionPolicyValidator::new();
        let mut params = HashMap::new();
        params.insert(
            "max_risk_level".to_string(),
            Value::String("none".to_string()),
        );
        validator.add_rule(GovernanceRule {
            id: EvolutionId::new_v4(),
            name: "disabled_rule".to_string(),
            rule_type: GovernanceRuleType::RiskLimit,
            parameters: params,
            enabled: false,
        });

        let result = validator
            .validate_proposal(RiskLevel::Critical, SubsystemTarget::Core)
            .unwrap();
        assert!(result.valid);
    }

    #[test]
    fn budget_limit() {
        let mut validator = EvolutionPolicyValidator::new();
        let mut params = HashMap::new();
        params.insert(
            "max_budget".to_string(),
            Value::Number(serde_json::Number::from_f64(1000.0).unwrap()),
        );
        validator.add_rule(GovernanceRule {
            id: EvolutionId::new_v4(),
            name: "budget".to_string(),
            rule_type: GovernanceRuleType::BudgetLimit,
            parameters: params,
            enabled: true,
        });

        let mut data = HashMap::new();
        data.insert("risk_level".to_string(), Value::String("low".to_string()));
        data.insert("target".to_string(), Value::String("core".to_string()));
        data.insert(
            "estimated_cost".to_string(),
            Value::Number(serde_json::Number::from_f64(500.0).unwrap()),
        );

        assert!(validator.validate(&data).unwrap().valid);

        data.insert(
            "estimated_cost".to_string(),
            Value::Number(serde_json::Number::from_f64(2000.0).unwrap()),
        );
        assert!(!validator.validate(&data).unwrap().valid);
    }
}
