use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::conversation::error::ConversationResult;
use crate::conversation::types::ConversationContext;

/// Workflow status.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    Discovered,
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// A discovered or running workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: WorkflowStatus,
    pub steps: Vec<WorkflowStep>,
    pub progress: f32,
    pub output: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub name: String,
    pub status: WorkflowStatus,
    pub output: Option<serde_json::Value>,
}

/// Progress update for long-running workflows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowProgress {
    pub workflow_id: String,
    pub status: WorkflowStatus,
    pub progress: f32,
    pub current_step: Option<String>,
    pub message: Option<String>,
    pub timestamp: crate::time::Timestamp,
}

/// Bridge between the Workflow Engine subsystem and the Conversation layer.
#[async_trait]
pub trait WorkflowConversationBridge: Send + Sync {
    /// Discover available workflows.
    async fn discover_workflows(
        &self,
        context: &ConversationContext,
        query: &str,
    ) -> ConversationResult<Vec<WorkflowInfo>>;

    /// Execute a workflow.
    async fn execute_workflow(
        &self,
        context: &ConversationContext,
        workflow_id: &str,
        parameters: &std::collections::HashMap<String, serde_json::Value>,
    ) -> ConversationResult<WorkflowInfo>;

    /// Monitor workflow progress.
    async fn monitor_progress(
        &self,
        context: &ConversationContext,
        workflow_id: &str,
    ) -> ConversationResult<WorkflowProgress>;

    /// Cancel a running workflow.
    async fn cancel_workflow(
        &self,
        context: &ConversationContext,
        workflow_id: &str,
    ) -> ConversationResult<()>;

    /// Resume a paused workflow.
    async fn resume_workflow(
        &self,
        context: &ConversationContext,
        workflow_id: &str,
    ) -> ConversationResult<WorkflowInfo>;

    /// Retrieve workflow output.
    async fn get_output(
        &self,
        context: &ConversationContext,
        workflow_id: &str,
    ) -> ConversationResult<Option<serde_json::Value>>;
}

/// Mock implementation for testing.
pub struct MockWorkflowBridge;

#[async_trait]
impl WorkflowConversationBridge for MockWorkflowBridge {
    async fn discover_workflows(
        &self,
        _context: &ConversationContext,
        _query: &str,
    ) -> ConversationResult<Vec<WorkflowInfo>> {
        Ok(Vec::new())
    }

    async fn execute_workflow(
        &self,
        _context: &ConversationContext,
        workflow_id: &str,
        _parameters: &std::collections::HashMap<String, serde_json::Value>,
    ) -> ConversationResult<WorkflowInfo> {
        Ok(WorkflowInfo {
            id: workflow_id.to_string(),
            name: workflow_id.to_string(),
            description: "Mock workflow".to_string(),
            status: WorkflowStatus::Completed,
            steps: Vec::new(),
            progress: 1.0,
            output: None,
        })
    }

    async fn monitor_progress(
        &self,
        _context: &ConversationContext,
        workflow_id: &str,
    ) -> ConversationResult<WorkflowProgress> {
        Ok(WorkflowProgress {
            workflow_id: workflow_id.to_string(),
            status: WorkflowStatus::Completed,
            progress: 1.0,
            current_step: None,
            message: None,
            timestamp: crate::time::Timestamp::now(),
        })
    }

    async fn cancel_workflow(
        &self,
        _context: &ConversationContext,
        _workflow_id: &str,
    ) -> ConversationResult<()> {
        Ok(())
    }

    async fn resume_workflow(
        &self,
        _context: &ConversationContext,
        workflow_id: &str,
    ) -> ConversationResult<WorkflowInfo> {
        Ok(WorkflowInfo {
            id: workflow_id.to_string(),
            name: workflow_id.to_string(),
            description: "Mock workflow".to_string(),
            status: WorkflowStatus::Running,
            steps: Vec::new(),
            progress: 0.0,
            output: None,
        })
    }

    async fn get_output(
        &self,
        _context: &ConversationContext,
        _workflow_id: &str,
    ) -> ConversationResult<Option<serde_json::Value>> {
        Ok(None)
    }
}
