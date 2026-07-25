use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::goal::{Goal, GoalId};
use crate::task::{Task, TaskId};
use crate::error::{ExecutiveError, ExecutiveResult};

/// Execution mode governing how the executive operates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExecutionMode {
    Safe,
    Interactive,
    Autonomous,
    Developer,
}

/// Global state shared across all executive subsystems.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalState {
    pub active_goal_count: usize,
    pub active_task_count: usize,
    pub completed_goals: u64,
    pub completed_tasks: u64,
    pub failed_goals: u64,
    pub failed_tasks: u64,
    pub cancelled_goals: u64,
    pub cancelled_tasks: u64,
    pub total_inference_calls: u64,
    pub total_reasoning_calls: u64,
    pub total_memory_accesses: u64,
    pub total_knowledge_accesses: u64,
    pub resource_utilization: HashMap<String, f64>,
    pub uptime_ms: u64,
    pub last_updated: DateTime<Utc>,
}

impl GlobalState {
    /// Create a new empty global state.
    pub fn new() -> Self {
        Self {
            active_goal_count: 0,
            active_task_count: 0,
            completed_goals: 0,
            completed_tasks: 0,
            failed_goals: 0,
            failed_tasks: 0,
            cancelled_goals: 0,
            cancelled_tasks: 0,
            total_inference_calls: 0,
            total_reasoning_calls: 0,
            total_memory_accesses: 0,
            total_knowledge_accesses: 0,
            resource_utilization: HashMap::new(),
            uptime_ms: 0,
            last_updated: Utc::now(),
        }
    }
}

impl Default for GlobalState {
    fn default() -> Self {
        Self::new()
    }
}

/// Execution context carries the current state and environment for all executive operations.
#[derive(Clone)]
pub struct ExecutiveContext {
    inner: Arc<ExecutiveContextInner>,
}

struct ExecutiveContextInner {
    mode: RwLock<ExecutionMode>,
    goals: RwLock<HashMap<GoalId, Goal>>,
    tasks: RwLock<HashMap<TaskId, Task>>,
    global_state: RwLock<GlobalState>,
    environment: RwLock<HashMap<String, serde_json::Value>>,
    available_tools: RwLock<Vec<String>>,
    max_concurrent_goals: RwLock<usize>,
    max_concurrent_tasks: RwLock<usize>,
    start_time: std::time::Instant,
}

impl ExecutiveContext {
    /// Create a new executive context.
    pub fn new(mode: ExecutionMode) -> Self {
        Self {
            inner: Arc::new(ExecutiveContextInner {
                mode: RwLock::new(mode),
                goals: RwLock::new(HashMap::new()),
                tasks: RwLock::new(HashMap::new()),
                global_state: RwLock::new(GlobalState::new()),
                environment: RwLock::new(HashMap::new()),
                available_tools: RwLock::new(Vec::new()),
                max_concurrent_goals: RwLock::new(16),
                max_concurrent_tasks: RwLock::new(64),
                start_time: std::time::Instant::now(),
            }),
        }
    }

    /// Get the current execution mode.
    pub fn mode(&self) -> ExecutionMode {
        *self.inner.mode.read()
    }

    /// Set the execution mode.
    pub fn set_mode(&self, mode: ExecutionMode) {
        *self.inner.mode.write() = mode;
    }

    /// Add a goal to the context.
    pub fn add_goal(&self, goal: Goal) {
        let mut goals = self.inner.goals.write();
        goals.insert(goal.id, goal);
    }

    /// Get a goal by ID.
    pub fn get_goal(&self, id: GoalId) -> Option<Goal> {
        self.inner.goals.read().get(&id).cloned()
    }

    /// Update a goal in the context.
    pub fn update_goal(&self, goal: Goal) {
        self.inner.goals.write().insert(goal.id, goal);
    }

    /// Remove a goal from the context.
    pub fn remove_goal(&self, id: GoalId) -> Option<Goal> {
        self.inner.goals.write().remove(&id)
    }

    /// Get all goals.
    pub fn all_goals(&self) -> Vec<Goal> {
        self.inner.goals.read().values().cloned().collect()
    }

    /// Get active (non-terminal) goals.
    pub fn active_goals(&self) -> Vec<Goal> {
        self.inner
            .goals
            .read()
            .values()
            .filter(|g| !g.state.is_terminal())
            .cloned()
            .collect()
    }

    /// Add a task to the context.
    pub fn add_task(&self, task: Task) {
        let mut tasks = self.inner.tasks.write();
        tasks.insert(task.id, task);
    }

    /// Get a task by ID.
    pub fn get_task(&self, id: TaskId) -> Option<Task> {
        self.inner.tasks.read().get(&id).cloned()
    }

    /// Update a task in the context.
    pub fn update_task(&self, task: Task) {
        self.inner.tasks.write().insert(task.id, task);
    }

    /// Remove a task from the context.
    pub fn remove_task(&self, id: TaskId) -> Option<Task> {
        self.inner.tasks.write().remove(&id)
    }

    /// Get all tasks.
    pub fn all_tasks(&self) -> Vec<Task> {
        self.inner.tasks.read().values().cloned().collect()
    }

    /// Get active tasks.
    pub fn active_tasks(&self) -> Vec<Task> {
        self.inner
            .tasks
            .read()
            .values()
            .filter(|t| !t.state.is_terminal())
            .cloned()
            .collect()
    }

    /// Get the global state.
    pub fn global_state(&self) -> GlobalState {
        let mut state = self.inner.global_state.read().clone();
        state.uptime_ms = self.inner.start_time.elapsed().as_millis() as u64;
        state.last_updated = Utc::now();
        state
    }

    /// Update the global state.
    pub fn update_global_state(&self, state: GlobalState) {
        *self.inner.global_state.write() = state;
    }

    /// Increment goal completion counter.
    pub fn record_goal_completed(&self) {
        self.inner.global_state.write().completed_goals += 1;
    }

    /// Increment task completion counter.
    pub fn record_task_completed(&self) {
        self.inner.global_state.write().completed_tasks += 1;
    }

    /// Increment goal failure counter.
    pub fn record_goal_failed(&self) {
        self.inner.global_state.write().failed_goals += 1;
    }

    /// Increment task failure counter.
    pub fn record_task_failed(&self) {
        self.inner.global_state.write().failed_tasks += 1;
    }

    /// Increment goal cancellation counter.
    pub fn record_goal_cancelled(&self) {
        self.inner.global_state.write().cancelled_goals += 1;
    }

    /// Increment task cancellation counter.
    pub fn record_task_cancelled(&self) {
        self.inner.global_state.write().cancelled_tasks += 1;
    }

    /// Record an inference call.
    pub fn record_inference_call(&self) {
        self.inner.global_state.write().total_inference_calls += 1;
    }

    /// Record a reasoning call.
    pub fn record_reasoning_call(&self) {
        self.inner.global_state.write().total_reasoning_calls += 1;
    }

    /// Record a memory access.
    pub fn record_memory_access(&self) {
        self.inner.global_state.write().total_memory_accesses += 1;
    }

    /// Record a knowledge access.
    pub fn record_knowledge_access(&self) {
        self.inner.global_state.write().total_knowledge_accesses += 1;
    }

    /// Set an environment variable.
    pub fn set_variable(&self, key: String, value: serde_json::Value) {
        self.inner.environment.write().insert(key, value);
    }

    /// Get an environment variable.
    pub fn get_variable(&self, key: &str) -> Option<serde_json::Value> {
        self.inner.environment.read().get(key).cloned()
    }

    /// Get all environment variables.
    pub fn environment(&self) -> HashMap<String, serde_json::Value> {
        self.inner.environment.read().clone()
    }

    /// Register an available tool.
    pub fn register_tool(&self, name: String) {
        let mut tools = self.inner.available_tools.write();
        if !tools.contains(&name) {
            tools.push(name);
        }
    }

    /// Check if a tool is available.
    pub fn has_tool(&self, name: &str) -> bool {
        self.inner.available_tools.read().contains(&name.to_string())
    }

    /// Get all available tools.
    pub fn available_tools(&self) -> Vec<String> {
        self.inner.available_tools.read().clone()
    }

    /// Get the maximum concurrent goals.
    pub fn max_concurrent_goals(&self) -> usize {
        *self.inner.max_concurrent_goals.read()
    }

    /// Set the maximum concurrent goals.
    pub fn set_max_concurrent_goals(&self, max: usize) {
        *self.inner.max_concurrent_goals.write() = max;
    }

    /// Get the maximum concurrent tasks.
    pub fn max_concurrent_tasks(&self) -> usize {
        *self.inner.max_concurrent_tasks.read()
    }

    /// Set the maximum concurrent tasks.
    pub fn set_max_concurrent_tasks(&self, max: usize) {
        *self.inner.max_concurrent_tasks.write() = max;
    }

    /// Check if the system can accept more goals.
    pub fn can_accept_goal(&self) -> bool {
        let active = self.inner.goals.read().values().filter(|g| !g.state.is_terminal()).count();
        active < *self.inner.max_concurrent_goals.read()
    }

    /// Check if the system can accept more tasks.
    pub fn can_accept_task(&self) -> bool {
        let active = self.inner.tasks.read().values().filter(|t| !t.state.is_terminal()).count();
        active < *self.inner.max_concurrent_tasks.read()
    }

    /// Update resource utilization.
    pub fn set_resource_utilization(&self, resource: String, utilization: f64) {
        self.inner
            .global_state
            .write()
            .resource_utilization
            .insert(resource, utilization);
    }

    /// Get uptime in milliseconds.
    pub fn uptime_ms(&self) -> u64 {
        self.inner.start_time.elapsed().as_millis() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goal::{GoalPriority};

    #[test]
    fn context_creation() {
        let ctx = ExecutiveContext::new(ExecutionMode::Safe);
        assert_eq!(ctx.mode(), ExecutionMode::Safe);
    }

    #[test]
    fn context_goals() {
        let ctx = ExecutiveContext::new(ExecutionMode::Autonomous);
        let goal = Goal::new("test".to_string(), GoalPriority::Normal);
        let id = goal.id;
        ctx.add_goal(goal);
        assert!(ctx.get_goal(id).is_some());
        assert_eq!(ctx.active_goals().len(), 1);
    }

    #[test]
    fn context_tasks() {
        let ctx = ExecutiveContext::new(ExecutionMode::Autonomous);
        let task = Task::new("test task".to_string());
        let id = task.id;
        ctx.add_task(task);
        assert!(ctx.get_task(id).is_some());
    }

    #[test]
    fn context_tools() {
        let ctx = ExecutiveContext::new(ExecutionMode::Developer);
        ctx.register_tool("shell".to_string());
        assert!(ctx.has_tool("shell"));
        assert!(!ctx.has_tool("missing"));
    }

    #[test]
    fn context_variables() {
        let ctx = ExecutiveContext::new(ExecutionMode::Interactive);
        ctx.set_variable("key".to_string(), serde_json::json!("value"));
        assert_eq!(
            ctx.get_variable("key"),
            Some(serde_json::json!("value"))
        );
    }

    #[test]
    fn context_global_state() {
        let ctx = ExecutiveContext::new(ExecutionMode::Autonomous);
        ctx.record_goal_completed();
        ctx.record_task_completed();
        let state = ctx.global_state();
        assert_eq!(state.completed_goals, 1);
        assert_eq!(state.completed_tasks, 1);
    }

    #[test]
    fn context_capacity() {
        let ctx = ExecutiveContext::new(ExecutionMode::Autonomous);
        ctx.set_max_concurrent_goals(2);
        assert!(ctx.can_accept_goal());
    }
}
