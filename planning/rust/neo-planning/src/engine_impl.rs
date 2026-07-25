use crate::goal::*;
use crate::types::*;
use crate::event::*;
use crate::task_graph::*;
use crate::algorithm::*;
use crate::strategy::*;
use crate::optimization::*;
use crate::error::*;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;

#[derive(Debug, Clone)]
pub struct PlanningEngine {
    orchestrator: Arc<PlanningOrchestrator>,
    cache: Arc<PlanningCache>,
    repository: Arc<PlanningRepository>,
    event_bus: EventBus,
}

impl PlanningEngine {
    pub async fn generate(&self, goal: Goal) -> Result<Plan> {
        let plan_id = self.repository.generate_plan_id().await?;
        let start_time = Utc::now();

        self.event_bus.publish(PlanningEvent::new(
            PlanningEventType::PlanningStarted,
            "engine"
        ).with_goal_id(Some(goal.id)));

        let plan = self.orchestrator.plan_from_goal(goal).await?;

        let end_time = Utc::now();
        let planning_latency = (end_time - start_time).num_milliseconds() as u64;

        self.analytics.update_planning_latency(planning_latency);

        self.event_bus.publish(PlanningEvent::new(
            PlanningEventType::PlanGenerated,
            "engine"
        ).with_plan_id(Some(plan.id)));

        Ok(plan)
    }

    pub async fn optimize(&self, plan_id: PlanId) -> Result<Plan> {
        let mut plan = self.repository.get_plan(plan_id).await?;

        self.event_bus.publish(PlanningEvent::new(
            PlanningEventType::PlanOptimized,
            "engine"
        ).with_plan_id(Some(plan_id)));

        let optimization_pipeline = OptimizationPipeline::new();
        let optimized_plan = optimization_pipeline.optimize(plan).await?;

        Ok(optimized_plan)
    }

    pub async fn validate(&self, plan_id: PlanId) -> Result<bool> {
        let plan = self.repository.get_plan(plan_id).await?;
        self.validate_plan_tasks(&plan.definition.tasks).await
    }

    pub async fn execute(&self, plan_id: PlanId) -> Result<PlanResult> {
        let plan = self.repository.get_plan(plan_id).await?;

        self.event_bus.publish(PlanningEvent::new(
            PlanningEventType::ExecutionStarted,
            "engine"
        ).with_plan_id(Some(plan_id)));

        let mut execution = PlanExecution::new(plan_id);
        let tasks = plan.definition.tasks;

        for task in tasks {
            let task_execution = TaskExecution::from(task.clone());
            execution.add_task(task_execution).await;
        }

        execution.start().await?;

        let mut all_completed = true;
        for mut task_execution in execution.tasks.iter_mut() {
            if task_execution.status == TaskStatus::Pending {
                task_execution.status = TaskStatus::Running;
                task_execution.start_time = Some(Utc::now());

                task_execution.status = TaskStatus::Completed;
                task_execution.end_time = Some(Utc::now());

                if let Some(start) = task_execution.start_time {
                    if let Some(end) = task_execution.end_time {
                        task_execution.duration_ms = (end - start).num_milliseconds() as u64;
                    }
                }
            }

            if task_execution.status != TaskStatus::Completed {
                all_completed = false;
            }
        }

        if all_completed {
            execution.status = PlanState::Completed;
            execution.end_time = Some(Utc::now());
        } else {
            execution.status = PlanState::Failed;
            execution.end_time = Some(Utc::now());
        }

        let metrics = execution.calculate_metrics().await?;
        let result = PlanResult {
            plan_id,
            success: execution.status == PlanState::Completed,
            plan: Some(plan),
            execution: Some(execution.clone()),
            error: if !all_completed { Some("Some tasks failed".to_string()) } else { None },
            metrics: Some(metrics),
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
        };

        self.event_bus.publish(PlanningEvent::new(
            if result.success { PlanningEventType::ExecutionCompleted } else { PlanningEventType::ExecutionFailed },
            "engine"
        ).with_plan_id(Some(plan_id)));

        Ok(result)
    }

    async fn validate_plan_tasks(&self, tasks: Vec<PlanTask>) -> Result<bool> {
        for task in &tasks {
            if task.status == TaskStatus::Pending {
                for dep_id in &task.dependencies {
                    if !tasks.iter().any(|t| t.id == *dep_id && t.status == TaskStatus::Completed) {
                        return Ok(false);
                    }
                }
            }
        }
        Ok(true)
    }
}