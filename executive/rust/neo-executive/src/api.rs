use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::context::{ExecutiveContext, ExecutionMode, GlobalState};
use crate::error::{ExecutiveError, ExecutiveResult};
use crate::goal::{Goal, GoalId, GoalManager, GoalPriority, GoalState};
use crate::session::{Session, SessionManager, SessionId, SessionState};
use crate::task::{Task, TaskId, TaskManager, TaskPriority, TaskState};
use crate::scheduler::{ExecutiveScheduler, SchedulerStats};
use crate::analytics::ExecutiveAnalytics;
use crate::recovery::FailureRecovery;
use crate::policies::PolicyEngine;

/// Executive API provides the high-level interface for all executive operations.
pub struct ExecutiveApi {
    session_manager: SessionManager,
    goal_manager: GoalManager,
    task_manager: TaskManager,
    context: ExecutiveContext,
    scheduler: ExecutiveScheduler,
    analytics: ExecutiveAnalytics,
    recovery: FailureRecovery,
    policy_engine: PolicyEngine,
}

/// Summary of an execution session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSummary {
    pub session_id: SessionId,
    pub session_state: SessionState,
    pub goals_created: usize,
    pub goals_completed: usize,
    pub goals_failed: usize,
    pub goals_cancelled: usize,
    pub tasks_created: usize,
    pub tasks_completed: usize,
    pub tasks_failed: usize,
    pub tasks_cancelled: usize,
    pub uptime_ms: u64,
    pub created_at: DateTime<Utc>,
}

impl ExecutiveApi {
    /// Create a new Executive API instance.
    pub fn new(mode: ExecutionMode) -> Self {
        Self {
            session_manager: SessionManager::new(),
            goal_manager: GoalManager::new(),
            task_manager: TaskManager::new(),
            context: ExecutiveContext::new(mode.clone()),
            scheduler: ExecutiveScheduler::default(),
            analytics: ExecutiveAnalytics::new(),
            recovery: FailureRecovery::new(),
            policy_engine: PolicyEngine::new(mode),
        }
    }

    /// Create a new session.
    pub fn create_session(&self) -> Session {
        self.session_manager.create_session()
    }

    /// Create a goal.
    pub fn create_goal(
        &self,
        description: String,
        priority: GoalPriority,
    ) -> ExecutiveResult<Goal> {
        let goal = self.goal_manager.create_goal(description, priority);
        self.context.add_goal(goal.clone());
        Ok(goal)
    }

    /// Pause a goal.
    pub fn pause_goal(&self, goal_id: GoalId) -> ExecutiveResult<()> {
        self.goal_manager.pause_goal(goal_id)
    }

    /// Resume a goal.
    pub fn resume_goal(&self, goal_id: GoalId) -> ExecutiveResult<()> {
        self.goal_manager.resume_goal(goal_id)
    }

    /// Cancel a goal.
    pub fn cancel_goal(&self, goal_id: GoalId) -> ExecutiveResult<()> {
        self.goal_manager.cancel_goal(goal_id)?;
        self.context.record_goal_cancelled();
        Ok(())
    }

    /// Complete a goal.
    pub fn complete_goal(&self, goal_id: GoalId) -> ExecutiveResult<()> {
        self.goal_manager.complete_goal(goal_id)?;
        self.context.record_goal_completed();
        self.analytics
            .record_goal_completion(&self.goal_manager.get_goal(goal_id)?.description);
        Ok(())
    }

    /// Submit a task.
    pub fn submit_task(
        &self,
        name: String,
        priority: TaskPriority,
        goal_id: Option<GoalId>,
    ) -> ExecutiveResult<Task> {
        let mut task = self.task_manager.create_task(name);
        task.priority = priority;
        task.goal_id = goal_id;
        self.task_manager.submit_task(task.clone())?;
        let submitted = self.task_manager.get_task(task.id)?;
        self.context.add_task(submitted.clone());
        Ok(submitted)
    }

    /// Cancel a task.
    pub fn cancel_task(&self, task_id: TaskId) -> ExecutiveResult<()> {
        self.task_manager.cancel_task(task_id)?;
        self.context.record_task_cancelled();
        Ok(())
    }

    /// Complete a task.
    pub fn complete_task(
        &self,
        task_id: TaskId,
        result: serde_json::Value,
    ) -> ExecutiveResult<()> {
        self.task_manager.complete_task(task_id, result)?;
        self.context.record_task_completed();
        Ok(())
    }

    /// Inspect current execution state.
    pub fn inspect_execution(&self) -> ExecutionSummary {
        let active_goals = self.goal_manager.all_goals();
        let active_tasks = self.task_manager.all_tasks();

        ExecutionSummary {
            session_id: SessionId::new(),
            session_state: SessionState::Active,
            goals_created: active_goals.len(),
            goals_completed: active_goals.iter().filter(|g| g.state == GoalState::Completed).count(),
            goals_failed: active_goals.iter().filter(|g| g.state == GoalState::Failed).count(),
            goals_cancelled: active_goals.iter().filter(|g| g.state == GoalState::Cancelled).count(),
            tasks_created: active_tasks.len(),
            tasks_completed: active_tasks.iter().filter(|t| t.state == TaskState::Completed).count(),
            tasks_failed: active_tasks.iter().filter(|t| t.state == TaskState::Failed).count(),
            tasks_cancelled: active_tasks.iter().filter(|t| t.state == TaskState::Cancelled).count(),
            uptime_ms: self.context.uptime_ms(),
            created_at: Utc::now(),
        }
    }

    /// Export execution summary as JSON.
    pub fn export_execution_summary(&self) -> ExecutiveResult<serde_json::Value> {
        let summary = self.inspect_execution();
        let global_state = self.context.global_state();

        let output = serde_json::json!({
            "session": summary,
            "global_state": global_state,
            "degradation_level": self.recovery.degradation_level(),
            "execution_mode": self.context.mode(),
            "analytics": {
                "task_latency_count": self.analytics.task_latency_count(),
                "goal_completions": self.analytics.goal_completion_count(),
                "decision_quality_avg": self.analytics.decision_quality_average(),
            }
        });

        Ok(output)
    }

    /// Get the goal manager.
    pub fn goal_manager(&self) -> &GoalManager {
        &self.goal_manager
    }

    /// Get the task manager.
    pub fn task_manager(&self) -> &TaskManager {
        &self.task_manager
    }

    /// Get the session manager.
    pub fn session_manager(&self) -> &SessionManager {
        &self.session_manager
    }

    /// Get the context.
    pub fn context(&self) -> &ExecutiveContext {
        &self.context
    }

    /// Get the scheduler.
    pub fn scheduler(&self) -> &ExecutiveScheduler {
        &self.scheduler
    }

    /// Get the analytics.
    pub fn analytics(&self) -> &ExecutiveAnalytics {
        &self.analytics
    }

    /// Get the recovery manager.
    pub fn recovery(&self) -> &FailureRecovery {
        &self.recovery
    }

    /// Get the policy engine.
    pub fn policy_engine(&self) -> &PolicyEngine {
        &self.policy_engine
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_goal_operations() {
        let api = ExecutiveApi::new(ExecutionMode::Autonomous);

        let goal = api
            .create_goal("test goal".to_string(), GoalPriority::High)
            .unwrap();
        assert_eq!(goal.state, GoalState::Proposed);

        api.goal_manager().accept_goal(goal.id).unwrap();

        let summary = api.inspect_execution();
        assert_eq!(summary.goals_created, 1);
    }

    #[test]
    fn api_task_operations() {
        let api = ExecutiveApi::new(ExecutionMode::Autonomous);

        let task = api
            .submit_task("test task".to_string(), TaskPriority::Normal, None)
            .unwrap();
        assert_eq!(task.state, TaskState::Queued);

        api.task_manager().start_task(task.id, "worker".to_string()).unwrap();
        api.complete_task(task.id, serde_json::json!({"done": true}))
            .unwrap();

        let summary = api.inspect_execution();
        assert_eq!(summary.tasks_completed, 1);
    }

    #[test]
    fn api_export() {
        let api = ExecutiveApi::new(ExecutionMode::Developer);
        let export = api.export_execution_summary().unwrap();
        assert!(export.is_object());
    }

    #[test]
    fn api_session() {
        let api = ExecutiveApi::new(ExecutionMode::Interactive);
        let session = api.create_session();
        assert_eq!(session.state, SessionState::Created);
    }

    #[test]
    fn api_cancel_goal() {
        let api = ExecutiveApi::new(ExecutionMode::Autonomous);
        let goal = api
            .create_goal("cancel me".to_string(), GoalPriority::Low)
            .unwrap();
        api.cancel_goal(goal.id).unwrap();
        assert!(api.goal_manager().get_goal(goal.id).unwrap().state.is_terminal());
    }
}
