use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{WorkflowError, WorkflowResult};

/// A typed variable stored in the workflow context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub value: serde_json::Value,
    pub var_type: VariableType,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VariableType {
    Input,
    Output,
    Internal,
    Constant,
}

/// Manages workflow variables with validation and history tracking.
#[derive(Debug)]
pub struct VariableManager {
    variables: HashMap<String, Variable>,
    history: Vec<VariableChange>,
}

#[derive(Debug, Clone)]
pub struct VariableChange {
    pub name: String,
    pub old_value: Option<serde_json::Value>,
    pub new_value: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

impl VariableManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            history: Vec::new(),
        }
    }

    pub fn set(
        &mut self,
        name: String,
        value: serde_json::Value,
        var_type: VariableType,
    ) -> WorkflowResult<()> {
        let now = Utc::now();
        let old_value = self.variables.get(&name).map(|v| v.value.clone());

        if let Some(old) = &old_value {
            if *old == value {
                return Ok(());
            }
        }

        self.history.push(VariableChange {
            name: name.clone(),
            old_value: old_value.clone(),
            new_value: value.clone(),
            timestamp: now,
        });

        let var = Variable {
            name: name.clone(),
            value,
            var_type,
            created_at: now,
            updated_at: now,
        };
        self.variables.insert(name, var);
        Ok(())
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&serde_json::Value> {
        self.variables.get(name).map(|v| &v.value)
    }

    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.variables.contains_key(name)
    }

    #[must_use]
    pub fn all(&self) -> &HashMap<String, Variable> {
        &self.variables
    }

    #[must_use]
    pub fn history(&self) -> &[VariableChange] {
        &self.history
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.variables.len()
    }

    /// Merge another variable manager's variables into this one.
    pub fn merge(&mut self, other: &VariableManager) {
        for (name, var) in &other.variables {
            let _ = self.set(name.clone(), var.value.clone(), var.var_type);
        }
    }

    /// Get all variables of a given type.
    #[must_use]
    pub fn get_by_type(&self, var_type: VariableType) -> HashMap<&str, &serde_json::Value> {
        self.variables
            .iter()
            .filter(|(_, v)| v.var_type == var_type)
            .map(|(k, v)| (k.as_str(), &v.value))
            .collect()
    }
}

impl Default for VariableManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get() {
        let mut vm = VariableManager::new();
        vm.set("x".into(), serde_json::json!(1), VariableType::Internal)
            .unwrap();
        assert_eq!(vm.get("x"), Some(&serde_json::json!(1)));
        assert!(vm.contains("x"));
        assert!(!vm.contains("y"));
    }

    #[test]
    fn history_tracking() {
        let mut vm = VariableManager::new();
        vm.set("x".into(), serde_json::json!(1), VariableType::Internal)
            .unwrap();
        vm.set("x".into(), serde_json::json!(2), VariableType::Internal)
            .unwrap();
        assert_eq!(vm.history().len(), 2);
        assert_eq!(vm.history()[1].old_value, Some(serde_json::json!(1)));
    }

    #[test]
    fn skip_noop() {
        let mut vm = VariableManager::new();
        vm.set("x".into(), serde_json::json!(1), VariableType::Internal)
            .unwrap();
        vm.set("x".into(), serde_json::json!(1), VariableType::Internal)
            .unwrap();
        assert_eq!(vm.history().len(), 1);
    }

    #[test]
    fn merge() {
        let mut vm1 = VariableManager::new();
        vm1.set("a".into(), serde_json::json!(1), VariableType::Internal)
            .unwrap();
        let mut vm2 = VariableManager::new();
        vm2.set("b".into(), serde_json::json!(2), VariableType::Output)
            .unwrap();
        vm1.merge(&vm2);
        assert_eq!(vm1.count(), 2);
    }

    #[test]
    fn get_by_type() {
        let mut vm = VariableManager::new();
        vm.set("a".into(), serde_json::json!(1), VariableType::Input)
            .unwrap();
        vm.set("b".into(), serde_json::json!(2), VariableType::Output)
            .unwrap();
        let inputs = vm.get_by_type(VariableType::Input);
        assert_eq!(inputs.len(), 1);
        assert!(inputs.contains_key("a"));
    }
}
