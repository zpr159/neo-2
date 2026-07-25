use std::collections::HashMap;

use crate::error::AgentResult;
use crate::manager::AgentManager;
use crate::task::{Task, TaskScheduler};
use crate::types::{AgentConfiguration, AgentId, AgentSnapshot, AgentStatistics, AgentStatus};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// AgentApiRequest / AgentApiResponse
// ---------------------------------------------------------------------------

/// Request to create a new agent.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateAgentRequest {
    /// Agent configuration.
    pub config: AgentConfiguration,
}

/// Response from creating a new agent.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateAgentResponse {
    /// The created agent's ID.
    pub agent_id: AgentId,
    /// The agent name.
    pub name: String,
    /// Initial status.
    pub status: AgentStatus,
}

/// Response listing agents.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ListAgentsResponse {
    /// Total agent count.
    pub total: usize,
    /// Agent snapshots.
    pub agents: Vec<AgentSnapshot>,
}

/// Response with agent statistics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StatisticsResponse {
    /// System statistics.
    pub statistics: AgentStatistics,
}

/// Request to create a task.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateTaskRequest {
    /// Task definition.
    pub task: Task,
}

/// Response from creating a task.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateTaskResponse {
    /// The task ID.
    pub task_id: crate::task::TaskId,
    /// Initial status.
    pub status: String,
}

/// Response listing tasks.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ListTasksResponse {
    /// Total task count.
    pub total: usize,
    /// Task snapshots.
    pub tasks: Vec<Task>,
}

/// Generic API error response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApiErrorResponse {
    /// Error code.
    pub code: u16,
    /// Error message.
    pub message: String,
    /// Additional details.
    pub details: Option<HashMap<String, String>>,
}

// ---------------------------------------------------------------------------
// AgentApi
// ---------------------------------------------------------------------------

/// REST-like API surface for the agent framework.
///
/// Provides a structured API that can be mapped to HTTP endpoints or
/// used programmatically.
pub struct AgentApi {
    /// Reference to the agent manager.
    manager: Arc<AgentManager>,
    /// Reference to the task scheduler.
    scheduler: Arc<TaskScheduler>,
}

impl AgentApi {
    /// Create a new API instance.
    #[must_use]
    pub fn new(manager: Arc<AgentManager>, scheduler: Arc<TaskScheduler>) -> Self {
        Self { manager, scheduler }
    }

    // -- Agent endpoints --

    /// POST /agents - Create a new agent.
    pub async fn create_agent(
        &self,
        request: CreateAgentRequest,
    ) -> AgentResult<CreateAgentResponse> {
        let agent_id = self.manager.create_agent(request.config.clone()).await?;
        Ok(CreateAgentResponse {
            agent_id,
            name: request.config.name,
            status: AgentStatus::Ready,
        })
    }

    /// GET /agents - List all agents.
    #[must_use]
    pub fn list_agents(&self, status_filter: Option<AgentStatus>) -> ListAgentsResponse {
        let ids = self.manager.list_agents(status_filter);
        let agents: Vec<AgentSnapshot> = ids
            .iter()
            .filter_map(|id| {
                let registry = self.manager.registry();
                registry.get(id)
            })
            .collect();
        let total = agents.len();
        ListAgentsResponse { total, agents }
    }

    /// GET /agents/{id} - Get agent details.
    pub async fn get_agent(&self, agent_id: AgentId) -> AgentResult<AgentSnapshot> {
        self.manager.inspect_agent(agent_id).await
    }

    /// DELETE /agents/{id} - Terminate and remove an agent.
    pub async fn delete_agent(&self, agent_id: AgentId) -> AgentResult<()> {
        self.manager.terminate_agent(agent_id).await
    }

    /// POST /agents/{id}/start - Start an agent.
    pub async fn start_agent(&self, agent_id: AgentId) -> AgentResult<()> {
        self.manager.start_agent(agent_id).await
    }

    /// POST /agents/{id}/stop - Stop an agent.
    pub async fn stop_agent(&self, agent_id: AgentId) -> AgentResult<()> {
        self.manager.stop_agent(agent_id).await
    }

    /// POST /agents/{id}/pause - Pause an agent.
    pub async fn pause_agent(&self, agent_id: AgentId) -> AgentResult<()> {
        self.manager.pause_agent(agent_id).await
    }

    /// POST /agents/{id}/resume - Resume an agent.
    pub async fn resume_agent(&self, agent_id: AgentId) -> AgentResult<()> {
        self.manager.resume_agent(agent_id).await
    }

    /// GET /agents/{id}/metrics - Get agent metrics.
    pub async fn get_agent_metrics(
        &self,
        agent_id: AgentId,
    ) -> AgentResult<crate::types::AgentMetrics> {
        let snapshot = self.manager.inspect_agent(agent_id).await?;
        Ok(snapshot.metrics)
    }

    /// GET /statistics - Get system-wide statistics.
    #[must_use]
    pub fn get_statistics(&self) -> StatisticsResponse {
        StatisticsResponse {
            statistics: self.manager.statistics(),
        }
    }

    // -- Task endpoints --

    /// POST /tasks - Create a new task.
    pub async fn create_task(&self, request: CreateTaskRequest) -> AgentResult<CreateTaskResponse> {
        let task_id = self.scheduler.submit_task(request.task).await?;
        Ok(CreateTaskResponse {
            task_id,
            status: "queued".to_string(),
        })
    }

    /// GET /tasks - List all tasks.
    pub fn list_tasks(&self) -> ListTasksResponse {
        let tasks = self.scheduler.task_queue.list_tasks();
        let task_list: Vec<Task> = tasks
            .iter()
            .filter_map(|id| self.scheduler.task_queue.get_task(id))
            .collect();
        let total = task_list.len();
        ListTasksResponse {
            total,
            tasks: task_list,
        }
    }

    /// GET /tasks/{id} - Get task details.
    pub fn get_task(&self, task_id: crate::task::TaskId) -> AgentResult<Task> {
        self.scheduler
            .task_queue
            .get_task(&task_id)
            .ok_or_else(|| crate::error::AgentError::NotFound(format!("task {task_id} not found")))
    }
}
