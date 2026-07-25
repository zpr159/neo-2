use std::sync::Arc;

use tokio::sync::RwLock;

use super::api::{
    ResearchRequest, ResearchTask, ResearchTaskId,
    ResearchTaskStatus,
};
use super::config::ResearchConfig;
use super::error::{ResearchError, ResearchResult};
use super::workflow::ResearchWorkflow;
use crate::component::{Component, ComponentState};
use crate::error::NeoResult;

/// The top-level research manager that owns and coordinates the research subsystem.
///
/// This is the primary entry point for all research operations. It implements
/// the `Component` trait for lifecycle management and integrates with Neo's
/// executive, planning, reasoning, knowledge graph, world model, and memory
/// subsystems via integration bridges.
pub struct ResearchManager {
    config: ResearchConfig,
    state: ComponentState,
    workflow: Option<ResearchWorkflow>,
    tasks: Arc<RwLock<std::collections::HashMap<ResearchTaskId, ResearchTask>>>,
}

impl ResearchManager {
    pub fn new(config: ResearchConfig) -> Self {
        Self {
            config,
            state: ComponentState::Created,
            workflow: None,
            tasks: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Submit a research request and get back a task handle.
    pub async fn submit_research(
        &self,
        request: ResearchRequest,
    ) -> ResearchResult<ResearchTask> {
        let workflow = self
            .workflow
            .as_ref()
            .ok_or_else(|| ResearchError::InternalError("workflow not initialized".to_string()))?;

        workflow.create_task(request).await
    }

    /// Execute a research task to completion.
    pub async fn execute_research(
        &self,
        task_id: ResearchTaskId,
    ) -> ResearchResult<super::api::ResearchOutput> {
        let workflow = self
            .workflow
            .as_ref()
            .ok_or_else(|| ResearchError::InternalError("workflow not initialized".to_string()))?;

        workflow.execute_task(task_id).await
    }

    /// Submit and immediately execute a research request.
    pub async fn research(
        &self,
        request: ResearchRequest,
    ) -> ResearchResult<super::api::ResearchOutput> {
        let task = self.submit_research(request).await?;
        self.execute_research(task.id).await
    }

    /// Get the status of a research task.
    pub async fn get_task_status(
        &self,
        task_id: ResearchTaskId,
    ) -> ResearchResult<ResearchTask> {
        let workflow = self
            .workflow
            .as_ref()
            .ok_or_else(|| ResearchError::InternalError("workflow not initialized".to_string()))?;

        workflow
            .get_task(&task_id)
            .await
            .ok_or_else(|| ResearchError::TaskNotFound(task_id.to_string()))
    }

    /// List all research tasks.
    pub async fn list_tasks(&self) -> Vec<ResearchTask> {
        self.tasks.read().await.values().cloned().collect()
    }

    /// Cancel a running research task.
    pub async fn cancel_task(&self, task_id: ResearchTaskId) -> ResearchResult<()> {
        let workflow = self
            .workflow
            .as_ref()
            .ok_or_else(|| ResearchError::InternalError("workflow not initialized".to_string()))?;

        workflow.cancel_task(task_id).await
    }

    /// Get subsystem metrics.
    pub async fn metrics(&self) -> ResearchSubsystemMetrics {
        let tasks = self.tasks.read().await;
        let total = tasks.len();
        let completed = tasks
            .values()
            .filter(|t| t.status == ResearchTaskStatus::Completed)
            .count();
        let failed = tasks
            .values()
            .filter(|t| t.status == ResearchTaskStatus::Failed)
            .count();
        let running = tasks
            .values()
            .filter(|t| {
                !matches!(
                    t.status,
                    ResearchTaskStatus::Completed
                        | ResearchTaskStatus::Failed
                        | ResearchTaskStatus::Cancelled
                        | ResearchTaskStatus::Created
                )
            })
            .count();

        let total_facts_extracted: usize = tasks.values().map(|t| t.metrics.facts_extracted).sum();
        let total_facts_validated: usize = tasks.values().map(|t| t.metrics.facts_validated).sum();

        ResearchSubsystemMetrics {
            total_tasks: total,
            completed_tasks: completed,
            failed_tasks: failed,
            running_tasks: running,
            total_facts_extracted,
            total_facts_validated,
        }
    }
}

impl Component for ResearchManager {
    fn name(&self) -> &str {
        "research"
    }

    fn state(&self) -> ComponentState {
        self.state
    }

    async fn initialize(&mut self) -> NeoResult<()> {
        self.state = ComponentState::Initializing;

        let workflow = ResearchWorkflow::new(self.config.clone())
            .map_err(|e| crate::error::NeoError::Internal(format!("research init: {}", e)))?;

        self.workflow = Some(workflow);
        self.state = ComponentState::Running;
        Ok(())
    }

    async fn start(&mut self) -> NeoResult<()> {
        if self.state != ComponentState::Running {
            return Err(crate::error::NeoError::Internal(
                "research manager not initialized".to_string(),
            ));
        }
        Ok(())
    }

    async fn stop(&mut self) -> NeoResult<()> {
        self.state = ComponentState::Stopping;

        {
            let mut tasks = self.tasks.write().await;
            for task in tasks.values_mut() {
                if !matches!(
                    task.status,
                    ResearchTaskStatus::Completed
                        | ResearchTaskStatus::Failed
                        | ResearchTaskStatus::Cancelled
                ) {
                    task.status = ResearchTaskStatus::Cancelled;
                    task.completed_at = Some(crate::time::Timestamp::now());
                }
            }
        }

        self.state = ComponentState::Stopped;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Aggregated metrics for the research subsystem.
#[derive(Debug, Clone, Default)]
pub struct ResearchSubsystemMetrics {
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub running_tasks: usize,
    pub total_facts_extracted: usize,
    pub total_facts_validated: usize,
}
