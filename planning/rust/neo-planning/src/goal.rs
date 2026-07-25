#![forbid(unsafe_code)]
#![deny(
    missing_docs,
    warnings,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_extern_crates
)]

//! Neo Planning System — goal management and lifecycle.
//!
//! This module provides the goal system for Neo AGI OS,
//! including goal types, priorities, constraints, dependencies,
//! and hierarchical structures.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use derive_more::Display;

/// Unique identifier for a planning goal.
pub use crate::id::PlanningGoalId as GoalId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display)]
pub enum GoalType {
    #[display(fmt = "Achievement")]
    Achievement,
    #[display(fmt = "Maintenance")]
    Maintenance,
    #[display(fmt = "Prevention")]
    Prevention,
    #[display(fmt = "Improvement")]
    Improvement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Display)]
pub enum GoalPriority {
    #[display(fmt = "Low")]
    Low,
    #[display(fmt = "Medium")]
    Medium,
    #[display(fmt = "High")]
    High,
    #[display(fmt = "Critical")]
    Critical,
}

impl GoalPriority {
    pub fn score(&self) -> u32 {
        match self {
            GoalPriority::Low => 1,
            GoalPriority::Medium => 2,
            GoalPriority::High => 3,
            GoalPriority::Critical => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display)]
pub enum GoalStatus {
    #[display(fmt = "Pending")]
    Pending,
    #[display(fmt = "Active")]
    Active,
    #[display(fmt = "Accomplished")]
    Accomplished,
    #[display(fmt = "Failed")]
    Failed,
    #[display(fmt = "Abandoned")]
    Abandoned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalMetadata {
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub owner: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalConstraint {
    pub id: String,
    pub description: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalDependency {
    pub goal_id: GoalId,
    pub dependency_type: DependencyType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display)]
pub enum DependencyType {
    #[display(fmt = "FinishToStart")]
    FinishToStart,
    #[display(fmt = "StartToStart")]
    StartToStart,
    #[display(fmt = "FinishToFinish")]
    FinishToFinish,
    #[display(fmt = "StartToFinish")]
    StartToFinish,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: GoalId,
    pub goal_type: GoalType,
    pub priority: GoalPriority,
    pub status: GoalStatus,
    pub metadata: GoalMetadata,
    pub constraints: Vec<GoalConstraint>,
    pub dependencies: Vec<GoalDependency>,
    pub parent_id: Option<GoalId>,
    pub deadlines: Option<DateTime<Utc>>,
    pub resource_requirements: HashMap<String, f64>,
    pub success_criteria: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalHierarchy {
    pub root: Goal,
    pub children: Vec<GoalHierarchy>,
}

impl GoalHierarchy {
    pub fn new() -> Self {
        Self {
            root: Goal::default(),
            children: Vec::new(),
        }
    }

    pub fn add_goal(&mut self, goal: Goal) {
        if goal.parent_id.is_none() {
            // This is a root goal, shouldn't be added if root already exists
            // or update the root
            if self.root.id == Uuid::nil().into() {
                self.root = goal;
            } else {
                panic!("Root goal already exists");
            }
        } else {
            // Find the appropriate parent and add as child
            if let Some(child) = find_child_by_id(&mut self.children, goal.parent_id.unwrap()) {
                child.add_goal(goal);
            } else {
                panic!("Parent goal not found");
            }
        }
    }
}

fn find_child_by_id(hierarchy: &mut Vec<GoalHierarchy>, id: GoalId) -> Option<&mut GoalHierarchy> {
    for child in hierarchy.iter_mut() {
        if child.root.id == id {
            return Some(child);
        }
        let found = find_child_by_id(&mut child.children, id);
        if found.is_some() {
            return found;
        }
    }
    None
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalTree {
    pub root_id: GoalId,
    pub nodes: HashMap<GoalId, Goal>,
    pub children: HashMap<GoalId, Vec<GoalId>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalGraph {
    pub goals: HashMap<GoalId, Goal>,
    pub edges: Vec<(GoalId, GoalId, DependencyType)>,
}

impl Goal {
    pub fn builder() -> GoalBuilder {
        GoalBuilder::default()
    }
}

#[derive(Default)]
pub struct GoalBuilder {
    name: String,
    description: Option<String>,
    goal_type: Option<GoalType>,
    priority: Option<GoalPriority>,
    parent_id: Option<GoalId>,
    deadlines: Option<DateTime<Utc>>,
    constraints: Vec<GoalConstraint>,
    dependencies: Vec<GoalDependency>,
}

impl GoalBuilder {
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn goal_type(mut self, goal_type: GoalType) -> Self {
        self.goal_type = Some(goal_type);
        self
    }

    pub fn priority(mut self, priority: GoalPriority) -> Self {
        self.priority = Some(priority);
        self
    }

    pub fn parent(mut self, parent_id: GoalId) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    pub fn deadline(mut self, deadline: DateTime<Utc>) -> Self {
        self.deadlines = Some(deadline);
        self
    }

    pub fn with_constraint(mut self, id: impl Into<String>, description: impl Into<String>, value: serde_json::Value) -> Self {
        self.constraints.push(GoalConstraint {
            id: id.into(),
            description: description.into(),
            value,
        });
        self
    }

    pub fn with_dependency(mut self, goal_id: GoalId, dep_type: DependencyType) -> Self {
        self.dependencies.push(GoalDependency {
            goal_id,
            dependency_type: dep_type,
        });
        self
    }

    pub fn build(self) -> Goal {
        let now = Utc::now();
        Goal {
            id: GoalId::new(),
            goal_type: self.goal_type.unwrap_or(GoalType::Achievement),
            priority: self.priority.unwrap_or(GoalPriority::Medium),
            status: GoalStatus::Pending,
            metadata: GoalMetadata {
                name: self.name,
                description: self.description,
                created_at: now,
                updated_at: now,
                owner: None,
            },
            constraints: self.constraints,
            dependencies: self.dependencies,
            parent_id: self.parent_id,
            deadlines: self.deadlines,
            resource_requirements: HashMap::new(),
            success_criteria: Vec::new(),
        }
    }
}

impl Default for Goal {
    fn default() -> Self {
        Goal {
            id: GoalId::default(),
            goal_type: GoalType::Achievement,
            priority: GoalPriority::Medium,
            status: GoalStatus::Pending,
            metadata: GoalMetadata {
                name: String::new(),
                description: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                owner: None,
            },
            constraints: Vec::new(),
            dependencies: Vec::new(),
            parent_id: None,
            deadlines: None,
            resource_requirements: HashMap::new(),
            success_criteria: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn goal_builder() {
        let now = Utc::now();
        let goal = Goal::builder()
            .name("Test Goal")
            .description("A test goal")
            .goal_type(GoalType::Achievement)
            .priority(GoalPriority::High)
            .deadline(now)
            .with_constraint("c1", "Must be completed", serde_json::json!(true))
            .with_dependency(GoalId::new(), DependencyType::FinishToStart)
            .build();

        assert_eq!(goal.metadata.name, "Test Goal");
        assert_eq!(goal.goal_type, GoalType::Achievement);
        assert_eq!(goal.priority, GoalPriority::High);
        assert_eq!(goal.status, GoalStatus::Pending);
        assert_eq!(goal.constraints.len(), 1);
        assert_eq!(goal.dependencies.len(), 1);
    }

    #[test]
    fn goal_priority_scoring() {
        assert_eq!(GoalPriority::Low.score(), 1);
        assert_eq!(GoalPriority::Medium.score(), 2);
        assert_eq!(GoalPriority::High.score(), 3);
        assert_eq!(GoalPriority::Critical.score(), 4);
    }

    #[test]
    fn goal_hierarchy() {
        let mut hierarchy = GoalHierarchy::new();
        let root = Goal::builder()
            .name("Root")
            .parent_id(None)
            .build();

        let child = Goal::builder()
            .name("Child")
            .parent(root.id)
            .build();

        let grandchild = Goal::builder()
            .name("Grandchild")
            .parent(child.id)
            .build();

        hierarchy.add_goal(child);
        hierarchy.add_goal(grandchild);

        assert!(!hierarchy.root.id.0.is_nil());
    }
}
