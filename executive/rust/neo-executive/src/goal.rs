use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{ExecutiveError, ExecutiveResult};

/// Unique identifier for a goal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GoalId(pub Uuid);

impl GoalId {
    /// Create a new goal identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Get the inner UUID as a string.
    pub fn as_str(&self) -> String {
        self.0.to_string()
    }
}

impl Default for GoalId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for GoalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Priority level for goals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum GoalPriority {
    Critical = 4,
    High = 3,
    Normal = 2,
    Low = 1,
    Background = 0,
}

impl Default for GoalPriority {
    fn default() -> Self {
        Self::Normal
    }
}

impl GoalPriority {
    /// Convert to a numeric score.
    pub fn score(self) -> u32 {
        self as u32
    }
}

/// State of a goal in its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GoalState {
    Proposed,
    Accepted,
    Planning,
    Executing,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl GoalState {
    /// Check if the state is terminal.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            GoalState::Completed | GoalState::Failed | GoalState::Cancelled
        )
    }

    /// Valid transitions from this state.
    pub fn valid_transitions(self) -> &'static [GoalState] {
        match self {
            Self::Proposed => &[Self::Accepted, Self::Cancelled],
            Self::Accepted => &[Self::Planning, Self::Executing, Self::Cancelled],
            Self::Planning => &[Self::Executing, Self::Failed, Self::Cancelled],
            Self::Executing => &[Self::Paused, Self::Completed, Self::Failed, Self::Cancelled],
            Self::Paused => &[Self::Executing, Self::Cancelled],
            Self::Completed => &[],
            Self::Failed => &[],
            Self::Cancelled => &[],
        }
    }

    /// Check if a transition to the target state is valid.
    pub fn can_transition_to(self, target: GoalState) -> bool {
        self.valid_transitions().contains(&target)
    }
}

/// Persistence configuration for a goal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalPersistence {
    pub persist_to_disk: bool,
    pub snapshot_interval_secs: u64,
    pub retain_after_completion: bool,
}

impl Default for GoalPersistence {
    fn default() -> Self {
        Self {
            persist_to_disk: true,
            snapshot_interval_secs: 60,
            retain_after_completion: true,
        }
    }
}

/// A goal represents a desired outcome the executive system works to achieve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: GoalId,
    pub description: String,
    pub priority: GoalPriority,
    pub state: GoalState,
    pub parent_id: Option<GoalId>,
    pub sub_goals: Vec<GoalId>,
    pub dependencies: Vec<GoalId>,
    pub dependents: Vec<GoalId>,
    pub decomposition: Vec<GoalDecompositionStep>,
    pub context: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deadline: Option<DateTime<Utc>>,
    pub progress: f32,
    pub persistence: GoalPersistence,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// A step in goal decomposition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalDecompositionStep {
    pub id: Uuid,
    pub description: String,
    pub order: u32,
    pub completed: bool,
}

impl Goal {
    /// Create a new goal.
    pub fn new(description: String, priority: GoalPriority) -> Self {
        let now = Utc::now();
        Self {
            id: GoalId::new(),
            description,
            priority,
            state: GoalState::Proposed,
            parent_id: None,
            sub_goals: Vec::new(),
            dependencies: Vec::new(),
            dependents: Vec::new(),
            decomposition: Vec::new(),
            context: HashMap::new(),
            created_at: now,
            updated_at: now,
            deadline: None,
            progress: 0.0,
            persistence: GoalPersistence::default(),
            metadata: HashMap::new(),
        }
    }

    /// Set the parent goal.
    pub fn with_parent(mut self, parent_id: GoalId) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Set a deadline.
    pub fn with_deadline(mut self, deadline: DateTime<Utc>) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// Add a dependency.
    pub fn with_dependency(mut self, dep_id: GoalId) -> Self {
        if !self.dependencies.contains(&dep_id) {
            self.dependencies.push(dep_id);
        }
        self
    }

    /// Add context.
    pub fn with_context(mut self, key: String, value: serde_json::Value) -> Self {
        self.context.insert(key, value);
        self
    }

    /// Transition to a new state.
    pub fn transition(&mut self, target: GoalState) -> ExecutiveResult<()> {
        if !self.state.can_transition_to(target) {
            return Err(ExecutiveError::internal(format!(
                "cannot transition goal from {:?} to {:?}",
                self.state, target
            )));
        }
        self.state = target;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Update progress.
    pub fn update_progress(&mut self, progress: f32) {
        self.progress = progress.clamp(0.0, 1.0);
        self.updated_at = Utc::now();
    }

    /// Add a decomposition step.
    pub fn add_decomposition_step(&mut self, description: String) {
        let order = self.decomposition.len() as u32;
        self.decomposition.push(GoalDecompositionStep {
            id: Uuid::new_v4(),
            description,
            order,
            completed: false,
        });
        self.updated_at = Utc::now();
    }

    /// Mark a decomposition step as completed.
    pub fn complete_decomposition_step(&mut self, step_id: Uuid) -> bool {
        if let Some(step) = self.decomposition.iter_mut().find(|s| s.id == step_id) {
            step.completed = true;
            self.updated_at = Utc::now();

            let total = self.decomposition.len();
            let completed = self.decomposition.iter().filter(|s| s.completed).count();
            self.progress = if total > 0 {
                completed as f32 / total as f32
            } else {
                0.0
            };
            true
        } else {
            false
        }
    }

    /// Add a sub-goal.
    pub fn add_sub_goal(&mut self, sub_goal_id: GoalId) {
        if !self.sub_goals.contains(&sub_goal_id) {
            self.sub_goals.push(sub_goal_id);
            self.updated_at = Utc::now();
        }
    }

    /// Check if the goal has expired its deadline.
    pub fn is_overdue(&self) -> bool {
        self.deadline
            .map(|d| Utc::now() > d)
            .unwrap_or(false)
    }

    /// Get the time remaining until deadline in seconds.
    pub fn time_remaining_secs(&self) -> Option<i64> {
        self.deadline.map(|d| {
            let now = Utc::now();
            if d > now {
                (d - now).num_seconds()
            } else {
                0
            }
        })
    }
}

/// Thread-safe goal manager responsible for goal lifecycle, hierarchy, dependencies, and persistence.
#[derive(Clone)]
pub struct GoalManager {
    inner: Arc<GoalManagerInner>,
}

struct GoalManagerInner {
    goals: RwLock<HashMap<GoalId, Goal>>,
    dependency_graph: RwLock<HashMap<GoalId, HashSet<GoalId>>>,
    reverse_dependency_graph: RwLock<HashMap<GoalId, HashSet<GoalId>>>,
}

impl GoalManager {
    /// Create a new goal manager.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(GoalManagerInner {
                goals: RwLock::new(HashMap::new()),
                dependency_graph: RwLock::new(HashMap::new()),
                reverse_dependency_graph: RwLock::new(HashMap::new()),
            }),
        }
    }

    /// Create a new goal and register it.
    pub fn create_goal(&self, description: String, priority: GoalPriority) -> Goal {
        let goal = Goal::new(description, priority);
        let id = goal.id;
        self.inner.goals.write().insert(id, goal.clone());
        self.inner
            .dependency_graph
            .write()
            .entry(id)
            .or_default();
        tracing::info!(goal_id = %id, "goal created");
        goal
    }

    /// Get a goal by ID.
    pub fn get_goal(&self, id: GoalId) -> ExecutiveResult<Goal> {
        self.inner
            .goals
            .read()
            .get(&id)
            .cloned()
            .ok_or_else(|| ExecutiveError::goal_not_found(&id.as_str()))
    }

    /// Update a goal.
    pub fn update_goal(&self, goal: Goal) -> ExecutiveResult<()> {
        if !self.inner.goals.read().contains_key(&goal.id) {
            return Err(ExecutiveError::goal_not_found(&goal.id.as_str()));
        }
        self.inner.goals.write().insert(goal.id, goal);
        Ok(())
    }

    /// Transition a goal to a new state.
    pub fn transition_goal(&self, id: GoalId, target: GoalState) -> ExecutiveResult<()> {
        let mut goal = self.get_goal(id)?;
        goal.transition(target)?;
        self.inner.goals.write().insert(id, goal);
        tracing::info!(goal_id = %id, ?target, "goal transitioned");
        Ok(())
    }

    /// Accept a goal.
    pub fn accept_goal(&self, id: GoalId) -> ExecutiveResult<()> {
        self.transition_goal(id, GoalState::Accepted)
    }

    /// Start planning a goal.
    pub fn start_planning(&self, id: GoalId) -> ExecutiveResult<()> {
        self.transition_goal(id, GoalState::Planning)
    }

    /// Start executing a goal.
    pub fn start_executing(&self, id: GoalId) -> ExecutiveResult<()> {
        self.transition_goal(id, GoalState::Executing)
    }

    /// Pause a goal.
    pub fn pause_goal(&self, id: GoalId) -> ExecutiveResult<()> {
        self.transition_goal(id, GoalState::Paused)
    }

    /// Resume a paused goal.
    pub fn resume_goal(&self, id: GoalId) -> ExecutiveResult<()> {
        let goal = self.get_goal(id)?;
        if goal.state == GoalState::Paused {
            self.transition_goal(id, GoalState::Executing)
        } else {
            Err(ExecutiveError::internal(format!(
                "cannot resume goal in state {:?}",
                goal.state
            )))
        }
    }

    /// Complete a goal.
    pub fn complete_goal(&self, id: GoalId) -> ExecutiveResult<()> {
        let mut goal = self.get_goal(id)?;
        goal.transition(GoalState::Completed)?;
        goal.update_progress(1.0);
        self.inner.goals.write().insert(id, goal);
        tracing::info!(goal_id = %id, "goal completed");
        Ok(())
    }

    /// Fail a goal.
    pub fn fail_goal(&self, id: GoalId, reason: String) -> ExecutiveResult<()> {
        let mut goal = self.get_goal(id)?;
        goal.transition(GoalState::Failed)?;
        goal.metadata
            .insert("failure_reason".to_string(), serde_json::json!(reason));
        self.inner.goals.write().insert(id, goal);
        tracing::warn!(goal_id = %id, reason = %reason, "goal failed");
        Ok(())
    }

    /// Cancel a goal.
    pub fn cancel_goal(&self, id: GoalId) -> ExecutiveResult<()> {
        let mut goal = self.get_goal(id)?;
        goal.transition(GoalState::Cancelled)?;
        self.inner.goals.write().insert(id, goal);
        tracing::info!(goal_id = %id, "goal cancelled");
        Ok(())
    }

    /// Update goal progress.
    pub fn update_progress(&self, id: GoalId, progress: f32) -> ExecutiveResult<()> {
        let mut goal = self.get_goal(id)?;
        goal.update_progress(progress);
        self.inner.goals.write().insert(id, goal);
        Ok(())
    }

    /// Add a dependency between goals.
    pub fn add_dependency(&self, goal_id: GoalId, depends_on: GoalId) -> ExecutiveResult<()> {
        if goal_id == depends_on {
            return Err(ExecutiveError::new(
                crate::error::ExecutiveErrorCode::GoalDependencyCycle,
                "goal cannot depend on itself",
            ));
        }

        self.inner
            .goals
            .read()
            .get(&goal_id)
            .ok_or_else(|| ExecutiveError::goal_not_found(&goal_id.as_str()))?;
        self.inner
            .goals
            .read()
            .get(&depends_on)
            .ok_or_else(|| ExecutiveError::goal_not_found(&depends_on.as_str()))?;

        {
            let mut deps = self.inner.dependency_graph.write();
            deps.entry(goal_id).or_default().insert(depends_on);
        }
        {
            let mut rev_deps = self.inner.reverse_dependency_graph.write();
            rev_deps.entry(depends_on).or_default().insert(goal_id);
        }

        let mut goal = self.get_goal(goal_id)?;
        if !goal.dependencies.contains(&depends_on) {
            goal.dependencies.push(depends_on);
            goal.updated_at = Utc::now();
            self.inner.goals.write().insert(goal_id, goal);
        }

        if self.detect_cycle(goal_id) {
            let mut deps = self.inner.dependency_graph.write();
            deps.entry(goal_id).or_default().remove(&depends_on);
            let mut rev_deps = self.inner.reverse_dependency_graph.write();
            rev_deps.entry(depends_on).or_default().remove(&goal_id);

            return Err(ExecutiveError::new(
                crate::error::ExecutiveErrorCode::GoalDependencyCycle,
                "adding dependency would create a cycle",
            ));
        }

        Ok(())
    }

    /// Remove a dependency between goals.
    pub fn remove_dependency(&self, goal_id: GoalId, depends_on: GoalId) -> ExecutiveResult<()> {
        {
            let mut deps = self.inner.dependency_graph.write();
            if let Some(set) = deps.get_mut(&goal_id) {
                set.remove(&depends_on);
            }
        }
        {
            let mut rev_deps = self.inner.reverse_dependency_graph.write();
            if let Some(set) = rev_deps.get_mut(&depends_on) {
                set.remove(&goal_id);
            }
        }

        let mut goal = self.get_goal(goal_id)?;
        goal.dependencies.retain(|id| *id != depends_on);
        goal.updated_at = Utc::now();
        self.inner.goals.write().insert(goal_id, goal);
        Ok(())
    }

    /// Decompose a goal into steps.
    pub fn decompose_goal(
        &self,
        id: GoalId,
        steps: Vec<String>,
    ) -> ExecutiveResult<()> {
        let mut goal = self.get_goal(id)?;
        goal.decomposition.clear();
        for (i, step) in steps.into_iter().enumerate() {
            goal.decomposition.push(GoalDecompositionStep {
                id: Uuid::new_v4(),
                description: step,
                order: i as u32,
                completed: false,
            });
        }
        goal.updated_at = Utc::now();
        self.inner.goals.write().insert(id, goal);
        Ok(())
    }

    /// Get goals ready for execution (all dependencies satisfied).
    pub fn ready_goals(&self) -> Vec<Goal> {
        let goals = self.inner.goals.read();
        let deps = self.inner.dependency_graph.read();

        goals
            .values()
            .filter(|g| {
                g.state == GoalState::Accepted
                    && deps
                        .get(&g.id)
                        .map_or(true, |d| {
                            d.iter().all(|dep_id| {
                                goals.get(dep_id).map_or(false, |dg| {
                                    dg.state == GoalState::Completed
                                })
                            })
                        })
            })
            .cloned()
            .collect()
    }

    /// Get goals sorted by priority.
    pub fn goals_by_priority(&self) -> Vec<Goal> {
        let mut goals: Vec<Goal> = self
            .inner
            .goals
            .read()
            .values()
            .filter(|g| !g.state.is_terminal())
            .cloned()
            .collect();
        goals.sort_by(|a, b| b.priority.cmp(&a.priority));
        goals
    }

    /// Get overdue goals.
    pub fn overdue_goals(&self) -> Vec<Goal> {
        self.inner
            .goals
            .read()
            .values()
            .filter(|g| !g.state.is_terminal() && g.is_overdue())
            .cloned()
            .collect()
    }

    /// Get all goals.
    pub fn all_goals(&self) -> Vec<Goal> {
        self.inner.goals.read().values().cloned().collect()
    }

    /// Get goals by state.
    pub fn goals_by_state(&self, state: GoalState) -> Vec<Goal> {
        self.inner
            .goals
            .read()
            .values()
            .filter(|g| g.state == state)
            .cloned()
            .collect()
    }

    /// Get sub-goals of a parent.
    pub fn sub_goals(&self, parent_id: GoalId) -> ExecutiveResult<Vec<Goal>> {
        let goal = self.get_goal(parent_id)?;
        let goals = self.inner.goals.read();
        Ok(goal
            .sub_goals
            .iter()
            .filter_map(|id| goals.get(id).cloned())
            .collect())
    }

    /// Get the goal count.
    pub fn goal_count(&self) -> usize {
        self.inner.goals.read().len()
    }

    /// Remove a completed or cancelled goal.
    pub fn remove_goal(&self, id: GoalId) -> ExecutiveResult<()> {
        let goal = self.get_goal(id)?;
        if !goal.state.is_terminal() {
            return Err(ExecutiveError::internal(
                "cannot remove non-terminal goal",
            ));
        }

        self.inner.goals.write().remove(&id);
        self.inner.dependency_graph.write().remove(&id);
        self.inner.reverse_dependency_graph.write().remove(&id);

        for deps in self.inner.dependency_graph.write().values_mut() {
            deps.remove(&id);
        }
        for rev_deps in self.inner.reverse_dependency_graph.write().values_mut() {
            rev_deps.remove(&id);
        }

        Ok(())
    }

    /// Detect a cycle from a given goal.
    fn detect_cycle(&self, start: GoalId) -> bool {
        let deps = self.inner.dependency_graph.read();
        let mut visited = HashSet::new();
        let mut stack = VecDeque::new();
        stack.push_back(start);

        while let Some(current) = stack.pop_back() {
            if current == start && visited.contains(&current) {
                return true;
            }
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);

            if let Some(dependencies) = deps.get(&current) {
                for &dep in dependencies {
                    if dep == start {
                        return true;
                    }
                    stack.push_back(dep);
                }
            }
        }
        false
    }
}

impl Default for GoalManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goal_creation() {
        let mgr = GoalManager::new();
        let goal = mgr.create_goal("test".to_string(), GoalPriority::Normal);
        assert_eq!(goal.state, GoalState::Proposed);
        assert_eq!(mgr.goal_count(), 1);
    }

    #[test]
    fn goal_lifecycle() {
        let mgr = GoalManager::new();
        let goal = mgr.create_goal("lifecycle".to_string(), GoalPriority::High);
        let id = goal.id;

        mgr.accept_goal(id).unwrap();
        assert_eq!(mgr.get_goal(id).unwrap().state, GoalState::Accepted);

        mgr.start_planning(id).unwrap();
        assert_eq!(mgr.get_goal(id).unwrap().state, GoalState::Planning);

        mgr.start_executing(id).unwrap();
        assert_eq!(mgr.get_goal(id).unwrap().state, GoalState::Executing);

        mgr.complete_goal(id).unwrap();
        assert!(mgr.get_goal(id).unwrap().state.is_terminal());
    }

    #[test]
    fn goal_pause_resume() {
        let mgr = GoalManager::new();
        let goal = mgr.create_goal("pause".to_string(), GoalPriority::Normal);
        let id = goal.id;

        mgr.accept_goal(id).unwrap();
        mgr.start_executing(id).unwrap();
        mgr.pause_goal(id).unwrap();
        assert_eq!(mgr.get_goal(id).unwrap().state, GoalState::Paused);

        mgr.resume_goal(id).unwrap();
        assert_eq!(mgr.get_goal(id).unwrap().state, GoalState::Executing);
    }

    #[test]
    fn goal_dependencies() {
        let mgr = GoalManager::new();
        let g1 = mgr.create_goal("dep1".to_string(), GoalPriority::Normal);
        let g2 = mgr.create_goal("dep2".to_string(), GoalPriority::Normal);

        mgr.add_dependency(g2.id, g1.id).unwrap();
        let goal2 = mgr.get_goal(g2.id).unwrap();
        assert!(goal2.dependencies.contains(&g1.id));
    }

    #[test]
    fn goal_self_dependency_rejected() {
        let mgr = GoalManager::new();
        let g1 = mgr.create_goal("self".to_string(), GoalPriority::Normal);
        let result = mgr.add_dependency(g1.id, g1.id);
        assert!(result.is_err());
    }

    #[test]
    fn goal_decomposition() {
        let mgr = GoalManager::new();
        let goal = mgr.create_goal("decompose".to_string(), GoalPriority::Normal);
        let id = goal.id;

        mgr.decompose_goal(
            id,
            vec!["step1".to_string(), "step2".to_string()],
        )
        .unwrap();

        let goal = mgr.get_goal(id).unwrap();
        assert_eq!(goal.decomposition.len(), 2);
    }

    #[test]
    fn goal_progress_tracking() {
        let mgr = GoalManager::new();
        let goal = mgr.create_goal("progress".to_string(), GoalPriority::Normal);
        let id = goal.id;

        mgr.update_progress(id, 0.5).unwrap();
        assert_eq!(mgr.get_goal(id).unwrap().progress, 0.5);
    }

    #[test]
    fn goal_priority_ordering() {
        let mgr = GoalManager::new();
        mgr.create_goal("low".to_string(), GoalPriority::Low);
        mgr.create_goal("high".to_string(), GoalPriority::High);
        mgr.create_goal("critical".to_string(), GoalPriority::Critical);

        let sorted = mgr.goals_by_priority();
        assert_eq!(sorted[0].priority, GoalPriority::Critical);
        assert_eq!(sorted[1].priority, GoalPriority::High);
        assert_eq!(sorted[2].priority, GoalPriority::Low);
    }

    #[test]
    fn goal_cancellation() {
        let mgr = GoalManager::new();
        let goal = mgr.create_goal("cancel".to_string(), GoalPriority::Normal);
        let id = goal.id;

        mgr.cancel_goal(id).unwrap();
        assert!(mgr.get_goal(id).unwrap().state.is_terminal());
    }

    #[test]
    fn goal_not_found() {
        let mgr = GoalManager::new();
        let result = mgr.get_goal(GoalId::new());
        assert!(result.is_err());
    }

    #[test]
    fn goal_sub_goals() {
        let mgr = GoalManager::new();
        let parent = mgr.create_goal("parent".to_string(), GoalPriority::High);
        let child = mgr.create_goal("child".to_string(), GoalPriority::Normal);

        let mut parent_goal = mgr.get_goal(parent.id).unwrap();
        parent_goal.add_sub_goal(child.id);
        mgr.update_goal(parent_goal).unwrap();

        let sub = mgr.sub_goals(parent.id).unwrap();
        assert_eq!(sub.len(), 1);
    }

    #[test]
    fn goal_overdue() {
        let mut goal = Goal::new("overdue".to_string(), GoalPriority::Normal);
        goal.deadline = Some(Utc::now() - chrono::Duration::hours(1));
        assert!(goal.is_overdue());
    }
}
