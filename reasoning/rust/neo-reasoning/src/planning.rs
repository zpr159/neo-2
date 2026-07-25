use std::collections::{HashMap, HashSet};
use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{ReasoningError, ReasoningResult};
use crate::strategy::ReasoningStrategy;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GoalPriority {
    Critical,
    High,
    Medium,
    Low,
}

impl Default for GoalPriority {
    fn default() -> Self {
        Self::Medium
    }
}

impl fmt::Display for GoalPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Critical => write!(f, "critical"),
            Self::High => write!(f, "high"),
            Self::Medium => write!(f, "medium"),
            Self::Low => write!(f, "low"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Blocked,
    Skipped,
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Blocked => write!(f, "blocked"),
            Self::Skipped => write!(f, "skipped"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: Uuid,
    pub description: String,
    pub priority: GoalPriority,
    pub sub_goals: Vec<Goal>,
    pub success_criteria: Vec<String>,
    pub constraints: Vec<String>,
}

impl Goal {
    pub fn new(description: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            description,
            priority: GoalPriority::default(),
            sub_goals: Vec::new(),
            success_criteria: Vec::new(),
            constraints: Vec::new(),
        }
    }

    pub fn with_priority(mut self, priority: GoalPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_criterion(mut self, criterion: String) -> Self {
        self.success_criteria.push(criterion);
        self
    }

    pub fn with_constraint(mut self, constraint: String) -> Self {
        self.constraints.push(constraint);
        self
    }

    pub fn add_sub_goal(&mut self, sub_goal: Goal) {
        self.sub_goals.push(sub_goal);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanTask {
    pub id: Uuid,
    pub description: String,
    pub strategy: ReasoningStrategy,
    pub status: TaskStatus,
    pub dependencies: Vec<Uuid>,
    pub estimated_cost: f64,
    pub actual_cost: Option<f64>,
    pub output: Option<serde_json::Value>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl PlanTask {
    pub fn new(description: String, strategy: ReasoningStrategy) -> Self {
        Self {
            id: Uuid::new_v4(),
            description,
            strategy,
            status: TaskStatus::Pending,
            dependencies: Vec::new(),
            estimated_cost: 1.0,
            actual_cost: None,
            output: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_dependency(mut self, task_id: Uuid) -> Self {
        self.dependencies.push(task_id);
        self
    }

    pub fn with_cost(mut self, cost: f64) -> Self {
        self.estimated_cost = cost;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: Uuid,
    pub goal: Goal,
    pub tasks: Vec<PlanTask>,
    pub total_estimated_cost: f64,
    pub strategy: ReasoningStrategy,
    pub alternatives: Vec<Plan>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Plan {
    pub fn new(goal: Goal, strategy: ReasoningStrategy) -> Self {
        Self {
            id: Uuid::new_v4(),
            goal,
            tasks: Vec::new(),
            total_estimated_cost: 0.0,
            strategy,
            alternatives: Vec::new(),
            created_at: chrono::Utc::now(),
        }
    }

    pub fn add_task(&mut self, task: PlanTask) {
        self.total_estimated_cost += task.estimated_cost;
        self.tasks.push(task);
    }

    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    pub fn all_completed(&self) -> bool {
        self.tasks.iter().all(|t| {
            matches!(t.status, TaskStatus::Completed | TaskStatus::Skipped)
        })
    }

    pub fn has_failures(&self) -> bool {
        self.tasks.iter().any(|t| t.status == TaskStatus::Failed)
    }

    pub fn pending_tasks(&self) -> Vec<&PlanTask> {
        self.tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Pending)
            .collect()
    }

    pub fn ready_tasks(&self) -> Vec<&PlanTask> {
        let completed: HashSet<Uuid> = self
            .tasks
            .iter()
            .filter(|t| matches!(t.status, TaskStatus::Completed | TaskStatus::Skipped))
            .map(|t| t.id)
            .collect();

        self.tasks
            .iter()
            .filter(|t| {
                t.status == TaskStatus::Pending
                    && t.dependencies.iter().all(|d| completed.contains(d))
            })
            .collect()
    }
}

#[derive(Debug)]
pub struct PlanningEngine {
    max_alternatives: usize,
    cost_weight: f32,
    risk_weight: f32,
}

impl PlanningEngine {
    pub fn new() -> Self {
        Self {
            max_alternatives: 3,
            cost_weight: 0.6,
            risk_weight: 0.4,
        }
    }

    pub fn decompose_goal(&self, goal: &Goal) -> ReasoningResult<Vec<PlanTask>> {
        let mut tasks = Vec::new();

        if goal.sub_goals.is_empty() {
            let task = PlanTask::new(goal.description.clone(), ReasoningStrategy::ChainOfThought);
            tasks.push(task);
            return Ok(tasks);
        }

        let mut prev_id: Option<Uuid> = None;

        for sub_goal in &goal.sub_goals {
            let mut task = PlanTask::new(sub_goal.description.clone(), ReasoningStrategy::ChainOfThought);

            if let Some(dep_id) = prev_id {
                task = task.with_dependency(dep_id);
            }

            for sub_sub in &sub_goal.sub_goals {
                let sub_task = PlanTask::new(sub_sub.description.clone(), ReasoningStrategy::ChainOfThought)
                    .with_dependency(task.id);
                tasks.push(sub_task);
            }

            prev_id = Some(task.id);
            tasks.push(task);
        }

        Ok(tasks)
    }

    pub fn create_plan(
        &self,
        goal: Goal,
        strategy: ReasoningStrategy,
    ) -> ReasoningResult<Plan> {
        let tasks = self.decompose_goal(&goal)?;
        let total_cost: f64 = tasks.iter().map(|t| t.estimated_cost).sum();

        let mut plan = Plan::new(goal, strategy);
        plan.tasks = tasks;
        plan.total_estimated_cost = total_cost;

        Ok(plan)
    }

    pub fn create_alternative_plans(
        &self,
        goal: &Goal,
    ) -> ReasoningResult<Vec<Plan>> {
        let strategies = ReasoningStrategy::all_default();
        let mut plans = Vec::new();

        for strategy in strategies.into_iter().take(self.max_alternatives) {
            let plan = self.create_plan(goal.clone(), strategy)?;
            plans.push(plan);
        }

        Ok(plans)
    }

    pub fn select_best_plan<'a>(&self, plans: &'a [Plan]) -> Option<&'a Plan> {
        plans.iter().min_by(|a, b| {
            let score_a = self.plan_score(a);
            let score_b = self.plan_score(b);
            score_a
                .partial_cmp(&score_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    fn plan_score(&self, plan: &Plan) -> f64 {
        let cost_score = plan.total_estimated_cost;
        let risk_score = plan.tasks.len() as f64 * 0.1;
        (cost_score * self.cost_weight as f64) + (risk_score * self.risk_weight as f64)
    }

    pub fn validate_plan(&self, plan: &Plan) -> ReasoningResult<()> {
        let task_ids: HashSet<Uuid> = plan.tasks.iter().map(|t| t.id).collect();

        for task in &plan.tasks {
            for dep in &task.dependencies {
                if !task_ids.contains(dep) {
                    return Err(ReasoningError::PlanningFailed(format!(
                        "task '{}' depends on non-existent task {}",
                        task.description, dep
                    )));
                }
            }
        }

        if self.detect_cycle(plan) {
            return Err(ReasoningError::CircularDependency(
                "plan contains circular dependency".to_string(),
            ));
        }

        Ok(())
    }

    fn detect_cycle(&self, plan: &Plan) -> bool {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();

        for task in &plan.tasks {
            if !visited.contains(&task.id) {
                if self.dfs_cycle(task, plan, &mut visited, &mut stack) {
                    return true;
                }
            }
        }
        false
    }

    fn dfs_cycle(
        &self,
        task: &PlanTask,
        plan: &Plan,
        visited: &mut HashSet<Uuid>,
        stack: &mut HashSet<Uuid>,
    ) -> bool {
        visited.insert(task.id);
        stack.insert(task.id);

        for dep_id in &task.dependencies {
            if let Some(dep_task) = plan.tasks.iter().find(|t| t.id == *dep_id) {
                if !visited.contains(dep_id) {
                    if self.dfs_cycle(dep_task, plan, visited, stack) {
                        return true;
                    }
                } else if stack.contains(dep_id) {
                    return true;
                }
            }
        }

        stack.remove(&task.id);
        false
    }

    pub fn estimate_task_cost(&self, task: &PlanTask, _context: &HashMap<String, serde_json::Value>) -> f64 {
        let base_cost = 1.0;
        let depth_penalty = task.dependencies.len() as f64 * 0.2;
        base_cost + depth_penalty
    }

    pub fn reorder_for_efficiency(&self, plan: &mut Plan) {
        let mut completed = HashSet::new();
        let mut ordered = Vec::new();
        let mut remaining: Vec<PlanTask> = plan.tasks.drain(..).collect();

        while !remaining.is_empty() {
            let ready: Vec<PlanTask> = remaining
                .drain(..)
                .filter(|t| t.dependencies.iter().all(|d| completed.contains(d)))
                .collect();

            if ready.is_empty() {
                ordered.extend(remaining);
                break;
            }

            for task in ready {
                completed.insert(task.id);
                ordered.push(task);
            }
        }

        plan.tasks = ordered;
    }
}

impl Default for PlanningEngine {
    fn default() -> Self {
        Self::new()
    }
}
