//! Fluent SDK builders for the planning system.

use crate::engine::{PlanningEngine, PlanningEngineBuilder};
use crate::goal::{Goal, GoalPriority, GoalStatus, GoalType};
use crate::id::{PlanningGoalId, PlanningNodeId};
use crate::plan::{Plan, PlanContext, PlanDefinition, PlanState, PlanTask};
use crate::strategy::Strategy;
use crate::types::{
    AlgorithmType, ExecutionBudget, PlanMetadata, PlanTaskType, ResourceRequirements, TaskStatus,
};

/// Builder for constructing a `Goal`.
pub struct GoalBuilder {
    name: String,
    priority: GoalPriority,
    description: Option<String>,
    goal_type: GoalType,
    parent_id: Option<PlanningGoalId>,
    deadline: Option<chrono::DateTime<chrono::Utc>>,
    budget: Option<ExecutionBudget>,
    resources: Option<ResourceRequirements>,
    context: std::collections::HashMap<String, serde_json::Value>,
}

impl GoalBuilder {
    /// Create a new goal builder.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            priority: GoalPriority::Normal,
            description: None,
            goal_type: GoalType::Achievement,
            parent_id: None,
            deadline: None,
            budget: None,
            resources: None,
            context: std::collections::HashMap::new(),
        }
    }

    /// Set the priority.
    pub fn priority(mut self, priority: GoalPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set the description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set the goal type.
    pub fn goal_type(mut self, goal_type: GoalType) -> Self {
        self.goal_type = goal_type;
        self
    }

    /// Set the parent goal.
    pub fn parent(mut self, parent_id: PlanningGoalId) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Set a deadline.
    pub fn deadline(mut self, deadline: chrono::DateTime<chrono::Utc>) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// Set the budget.
    pub fn budget(mut self, budget: ExecutionBudget) -> Self {
        self.budget = Some(budget);
        self
    }

    /// Set resource requirements.
    pub fn resources(mut self, resources: ResourceRequirements) -> Self {
        self.resources = Some(resources);
        self
    }

    /// Add context.
    pub fn context(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.context.insert(key.into(), value);
        self
    }

    /// Build the goal.
    pub fn build(self) -> Goal {
        let mut goal = Goal::new(self.name, self.priority).with_type(self.goal_type);

        if let Some(desc) = self.description {
            goal = goal.with_description(desc);
        }
        if let Some(parent) = self.parent_id {
            goal = goal.with_parent(parent);
        }
        if let Some(deadline) = self.deadline {
            goal = goal.with_deadline(deadline);
        }
        if let Some(budget) = self.budget {
            goal = goal.with_budget(budget);
        }
        if let Some(resources) = self.resources {
            goal = goal.with_resources(resources);
        }
        for (k, v) in self.context {
            goal = goal.with_context(k, v);
        }
        goal
    }
}

/// Builder for constructing a `Plan`.
pub struct PlanBuilder {
    name: String,
    goal_id: Option<PlanningGoalId>,
    algorithm: AlgorithmType,
    budget: ExecutionBudget,
    allow_parallelism: bool,
    tasks: Vec<PlanTask>,
    context: PlanContext,
}

impl PlanBuilder {
    /// Create a new plan builder.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            goal_id: None,
            algorithm: AlgorithmType::HierarchicalTaskNetwork,
            budget: ExecutionBudget::default(),
            allow_parallelism: true,
            tasks: Vec::new(),
            context: PlanContext::default(),
        }
    }

    /// Set the goal.
    pub fn goal_id(mut self, goal_id: PlanningGoalId) -> Self {
        self.goal_id = Some(goal_id);
        self
    }

    /// Set the algorithm.
    pub fn algorithm(mut self, algorithm: AlgorithmType) -> Self {
        self.algorithm = algorithm;
        self
    }

    /// Set the budget.
    pub fn budget(mut self, budget: ExecutionBudget) -> Self {
        self.budget = budget;
        self
    }

    /// Set the max cost in the budget.
    pub fn max_cost(mut self, cost: f64) -> Self {
        self.budget.max_cost = cost;
        self
    }

    /// Allow or disallow parallelism.
    pub fn allow_parallelism(mut self, allow: bool) -> Self {
        self.allow_parallelism = allow;
        self
    }

    /// Add a task.
    pub fn task(mut self, task: PlanTask) -> Self {
        self.tasks.push(task);
        self
    }

    /// Set the context.
    pub fn context(mut self, context: PlanContext) -> Self {
        self.context = context;
        self
    }

    /// Build the plan.
    pub fn build(self) -> Plan {
        let goal_id = self.goal_id.unwrap_or_else(PlanningGoalId::new);

        let definition = PlanDefinition {
            goal_id,
            tasks: self.tasks,
            budget: self.budget,
            algorithm: self.algorithm,
            allow_parallelism: self.allow_parallelism,
        };

        Plan::new(definition, PlanMetadata::new(&self.name)).with_context(self.context)
    }
}

/// Builder for constructing a `PlanTask`.
pub struct PlanTaskBuilder {
    name: String,
    description: String,
    task_type: PlanTaskType,
    dependencies: Vec<PlanningNodeId>,
    cost_estimate: f64,
    duration_estimate_secs: u64,
}

impl PlanTaskBuilder {
    /// Create a new task builder.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            task_type: PlanTaskType::Atomic,
            dependencies: Vec::new(),
            cost_estimate: 0.0,
            duration_estimate_secs: 0,
        }
    }

    /// Set the description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set the task type.
    pub fn task_type(mut self, task_type: PlanTaskType) -> Self {
        self.task_type = task_type;
        self
    }

    /// Add a dependency.
    pub fn depends_on(mut self, task_id: PlanningNodeId) -> Self {
        self.dependencies.push(task_id);
        self
    }

    /// Set cost estimate.
    pub fn cost_estimate(mut self, cost: f64) -> Self {
        self.cost_estimate = cost;
        self
    }

    /// Set duration estimate.
    pub fn duration_estimate(mut self, secs: u64) -> Self {
        self.duration_estimate_secs = secs;
        self
    }

    /// Build the task.
    pub fn build(self) -> PlanTask {
        let mut task = PlanTask::new(self.name, self.task_type).with_description(self.description);
        for dep in self.dependencies {
            task = task.with_dependency(dep);
        }
        task = task
            .with_cost_estimate(self.cost_estimate)
            .with_duration_estimate(self.duration_estimate_secs);
        task
    }
}

/// Builder for constructing a `Strategy`.
pub struct StrategyBuilder {
    name: String,
    description: String,
    algorithm: AlgorithmType,
    estimated_cost: f64,
    estimated_duration_secs: u64,
    success_probability: f64,
    risk_score: f64,
    tasks: Vec<PlanTask>,
}

impl StrategyBuilder {
    /// Create a new strategy builder.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            algorithm: AlgorithmType::HierarchicalTaskNetwork,
            estimated_cost: 0.0,
            estimated_duration_secs: 0,
            success_probability: 0.8,
            risk_score: 0.2,
            tasks: Vec::new(),
        }
    }

    /// Set the description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set the algorithm.
    pub fn algorithm(mut self, algorithm: AlgorithmType) -> Self {
        self.algorithm = algorithm;
        self
    }

    /// Set estimated cost.
    pub fn estimated_cost(mut self, cost: f64) -> Self {
        self.estimated_cost = cost;
        self
    }

    /// Set estimated duration.
    pub fn estimated_duration(mut self, secs: u64) -> Self {
        self.estimated_duration_secs = secs;
        self
    }

    /// Set success probability.
    pub fn success_probability(mut self, prob: f64) -> Self {
        self.success_probability = prob;
        self
    }

    /// Set risk score.
    pub fn risk_score(mut self, score: f64) -> Self {
        self.risk_score = score;
        self
    }

    /// Add tasks.
    pub fn tasks(mut self, tasks: Vec<PlanTask>) -> Self {
        self.tasks = tasks;
        self
    }

    /// Build the strategy.
    pub fn build(self) -> Strategy {
        Strategy::new(self.name, self.algorithm)
            .with_description(self.description)
            .with_cost(self.estimated_cost)
            .with_duration(self.estimated_duration_secs)
            .with_success_probability(self.success_probability)
            .with_risk_score(self.risk_score)
            .with_tasks(self.tasks)
    }
}

/// Builder for constructing a `PlanningEngine`.
///
/// This is a convenience wrapper that delegates to `PlanningEngineBuilder`.
pub struct PlanningEngineSdkBuilder {
    inner: PlanningEngineBuilder,
}

impl PlanningEngineSdkBuilder {
    /// Create a new SDK builder.
    pub fn new() -> Self {
        Self {
            inner: PlanningEngineBuilder::new(),
        }
    }

    /// Set default algorithm.
    pub fn default_algorithm(mut self, algo: AlgorithmType) -> Self {
        self.inner = self.inner.default_algorithm(algo);
        self
    }

    /// Allow parallelism.
    pub fn parallelism(mut self, allow: bool) -> Self {
        self.inner = self.inner.allow_parallelism(allow);
        self
    }

    /// Set budget.
    pub fn budget(mut self, budget: ExecutionBudget) -> Self {
        self.inner = self.inner.budget(budget);
        self
    }

    /// Set max cost.
    pub fn max_cost(mut self, cost: f64) -> Self {
        self.inner = self.inner.budget(ExecutionBudget {
            max_cost: cost,
            ..ExecutionBudget::default()
        });
        self
    }

    /// Build the engine.
    pub fn build(self) -> PlanningEngine {
        self.inner.build()
    }
}

impl Default for PlanningEngineSdkBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goal_builder() {
        let goal = GoalBuilder::new("build web app")
            .priority(GoalPriority::High)
            .description("Full-stack web application")
            .goal_type(GoalType::Achievement)
            .context("language", serde_json::json!("rust"))
            .build();
        assert_eq!(goal.name, "build web app");
        assert_eq!(goal.priority, GoalPriority::High);
        assert_eq!(goal.goal_type, GoalType::Achievement);
        assert_eq!(goal.context.get("language").unwrap(), "rust");
    }

    #[test]
    fn plan_builder() {
        let plan = PlanBuilder::new("deployment plan")
            .max_cost(500.0)
            .allow_parallelism(true)
            .build();
        assert_eq!(plan.metadata.name, "deployment plan");
        assert_eq!(plan.definition.budget.max_cost, 500.0);
        assert!(plan.definition.allow_parallelism);
    }

    #[test]
    fn task_builder() {
        let task = PlanTaskBuilder::new("compile")
            .description("Compile the project")
            .task_type(PlanTaskType::Atomic)
            .cost_estimate(10.0)
            .duration_estimate(300)
            .build();
        assert_eq!(task.name, "compile");
    }

    #[test]
    fn strategy_builder() {
        let strategy = StrategyBuilder::new("htn approach")
            .description("Use HTN planning")
            .algorithm(AlgorithmType::HierarchicalTaskNetwork)
            .estimated_cost(100.0)
            .estimated_duration(600)
            .success_probability(0.9)
            .risk_score(0.1)
            .build();
        assert_eq!(strategy.name, "htn approach");
        assert_eq!(strategy.algorithm, AlgorithmType::HierarchicalTaskNetwork);
        assert_eq!(strategy.estimated_cost, 100.0);
    }

    #[test]
    fn engine_sdk_builder() {
        let engine = PlanningEngineSdkBuilder::new()
            .max_cost(1000.0)
            .parallelism(true)
            .build();
        assert_eq!(engine.configuration().default_budget.max_cost, 1000.0);
        assert!(engine.configuration().allow_parallelism);
    }

    #[test]
    fn integration_build_goal_and_plan() {
        let goal = GoalBuilder::new("test")
            .priority(GoalPriority::Critical)
            .build();
        let plan = PlanBuilder::new("test plan").goal_id(goal.id).build();
        assert_eq!(plan.definition.goal_id, goal.id);
    }
}
