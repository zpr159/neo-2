use crate::analytics::ExecutionAnalytics;
use crate::checkpoint::WorkflowCheckpoint;
use crate::core::*;
use crate::definition::WorkflowDefinition;
use crate::error::{WorkflowError, WorkflowResult};
use crate::execution::WorkflowInstance;
use crate::schedule::ScheduleConfig;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// REST API types
// ---------------------------------------------------------------------------

/// Request to create/execute a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkflowRequest {
    pub name: String,
    pub description: Option<String>,
    pub definition: WorkflowDefinition,
    pub variables: std::collections::HashMap<String, serde_json::Value>,
    pub timeout_ms: Option<u64>,
}

/// Response from workflow creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkflowResponse {
    pub workflow_id: WorkflowId,
    pub execution_id: ExecutionId,
    pub status: WorkflowState,
}

/// Request to trigger a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerWorkflowRequest {
    pub workflow_id: WorkflowId,
    pub variables: std::collections::HashMap<String, serde_json::Value>,
    pub async_execution: bool,
}

/// Response from workflow trigger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerWorkflowResponse {
    pub execution_id: ExecutionId,
    pub status: WorkflowState,
    pub message: String,
}

/// Workflow status query parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStatusQuery {
    pub workflow_id: WorkflowId,
    pub execution_id: Option<ExecutionId>,
}

/// Workflow status response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStatusResponse {
    pub workflow_id: WorkflowId,
    pub execution_id: ExecutionId,
    pub state: WorkflowState,
    pub progress: f32,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub duration_ms: u64,
    pub nodes_total: usize,
    pub nodes_completed: usize,
    pub nodes_failed: usize,
    pub error: Option<String>,
}

/// Request to cancel a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelWorkflowRequest {
    pub execution_id: ExecutionId,
    pub reason: Option<String>,
}

/// Response from workflow cancellation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelWorkflowResponse {
    pub execution_id: ExecutionId,
    pub previous_state: WorkflowState,
    pub new_state: WorkflowState,
    pub message: String,
}

/// Request to pause a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PauseWorkflowRequest {
    pub execution_id: ExecutionId,
}

/// Response from workflow pause.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PauseWorkflowResponse {
    pub execution_id: ExecutionId,
    pub state: WorkflowState,
    pub message: String,
}

/// Request to resume a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeWorkflowRequest {
    pub execution_id: ExecutionId,
}

/// Request to set a variable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetVariableRequest {
    pub execution_id: ExecutionId,
    pub name: String,
    pub value: serde_json::Value,
}

/// Request to restore from checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreCheckpointRequest {
    pub checkpoint_id: crate::core::CheckpointId,
}

/// Workflow definition response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinitionResponse {
    pub workflow_id: WorkflowId,
    pub name: String,
    pub description: String,
    pub version: WorkflowVersion,
    pub nodes_count: usize,
    pub edges_count: usize,
    pub is_valid: bool,
    pub created_at: String,
    pub modified_at: String,
}

/// List of workflows response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowListResponse {
    pub workflows: Vec<WorkflowDefinitionResponse>,
    pub total: usize,
}

/// Checkpoint list response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointListResponse {
    pub checkpoints: Vec<CheckpointSummary>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointSummary {
    pub checkpoint_id: CheckpointId,
    pub execution_id: ExecutionId,
    pub state: WorkflowState,
    pub created_at: String,
    pub is_valid: bool,
}

/// Schedule creation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateScheduleRequest {
    pub workflow_id: WorkflowId,
    pub schedule_type: crate::schedule::ScheduleType,
    pub payload: Option<serde_json::Value>,
}

/// Schedule response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleResponse {
    pub schedule_id: ScheduleId,
    pub workflow_id: WorkflowId,
    pub enabled: bool,
    pub next_execution: Option<String>,
    pub execution_count: u32,
}

/// Analytics response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsResponse {
    pub execution_id: ExecutionId,
    pub workflow_id: WorkflowId,
    pub state: WorkflowState,
    pub duration_ms: u64,
    pub nodes_total: usize,
    pub nodes_completed: usize,
    pub nodes_failed: usize,
    pub retries: u32,
    pub success_rate: f64,
}

/// Workflow validation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResponse {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Error response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

impl From<&WorkflowError> for ErrorResponse {
    fn from(err: &WorkflowError) -> Self {
        ErrorResponse {
            code: format!("{:?}", err.code()),
            message: err.to_string(),
            details: None,
        }
    }
}

/// API pagination parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

/// Paginated response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub total: usize,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

// ---------------------------------------------------------------------------
// CLI types
// ---------------------------------------------------------------------------

/// CLI command for workflow operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowCliCommand {
    List {
        filter: Option<String>,
        page: Option<u32>,
    },
    Get {
        workflow_id: WorkflowId,
    },
    Validate {
        workflow_id: WorkflowId,
    },
    Execute {
        workflow_id: WorkflowId,
        variables: Vec<String>,
        async_mode: bool,
    },
    Status {
        execution_id: ExecutionId,
    },
    Cancel {
        execution_id: ExecutionId,
        reason: Option<String>,
    },
    Pause {
        execution_id: ExecutionId,
    },
    Resume {
        execution_id: ExecutionId,
    },
    Checkpoints {
        execution_id: ExecutionId,
    },
    RestoreCheckpoint {
        checkpoint_id: CheckpointId,
    },
    Schedule {
        workflow_id: WorkflowId,
        schedule_type: String,
    },
    Analytics {
        workflow_id: WorkflowId,
    },
    Export {
        workflow_id: WorkflowId,
        output_path: String,
    },
    Import {
        file_path: String,
    },
}

/// CLI output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Table,
    Json,
    Yaml,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn error_response_from_error() {
        let err = WorkflowError::CycleDetected("test cycle".into());
        let resp = ErrorResponse::from(&err);
        assert!(!resp.code.is_empty());
        assert!(!resp.message.is_empty());
    }

    #[test]
    fn serialization_roundtrip() {
        let req = CreateWorkflowRequest {
            name: "test".into(),
            description: None,
            definition: WorkflowDefinition {
                id: WorkflowId::new(),
                name: "test".into(),
                description: "".into(),
                version: WorkflowVersion::initial(),
                nodes: vec![],
                edges: vec![],
                config: WorkflowConfig::default(),
                metadata: WorkflowMetadata::new("test"),
                created_at: Utc::now(),
                modified_at: Utc::now(),
            },
            variables: std::collections::HashMap::new(),
            timeout_ms: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        let _: CreateWorkflowRequest = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn pagination_params() {
        let params = PaginationParams {
            page: Some(1),
            page_size: Some(10),
            sort_by: Some("created_at".into()),
            sort_order: Some("desc".into()),
        };
        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("page"));
    }
}
