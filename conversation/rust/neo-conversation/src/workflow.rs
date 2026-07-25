use crate::error::ConversationResult;

/// Interface to the workflow engine.
///
/// Manages multi-step automated workflows triggered from conversation.
pub trait WorkflowInterface: Send + Sync {
    /// Get the status of running workflows.
    fn running_workflows(&self) -> ConversationResult<Vec<WorkflowStatus>>;

    /// Trigger a workflow.
    fn trigger(&self, workflow_name: &str, params: std::collections::HashMap<String, String>)
        -> ConversationResult<String>;

    /// Get workflow output.
    fn output(&self, workflow_id: &str) -> ConversationResult<Option<WorkflowOutput>>;
}

/// Status of a workflow.
#[derive(Debug, Clone)]
pub struct WorkflowStatus {
    pub workflow_id: String,
    pub name: String,
    pub status: String,
    pub progress: f64,
    pub started_at: String,
}

/// Output from a completed workflow.
#[derive(Debug, Clone)]
pub struct WorkflowOutput {
    pub workflow_id: String,
    pub result: String,
    pub success: bool,
    pub completed_at: String,
}

/// Default workflow interface.
pub struct DefaultWorkflow;

impl WorkflowInterface for DefaultWorkflow {
    fn running_workflows(&self) -> ConversationResult<Vec<WorkflowStatus>> {
        Ok(Vec::new())
    }

    fn trigger(
        &self,
        workflow_name: &str,
        _params: std::collections::HashMap<String, String>,
    ) -> ConversationResult<String> {
        let id = uuid::Uuid::new_v4().to_string();
        tracing::info!("Workflow triggered: {workflow_name} (id={id})");
        Ok(id)
    }

    fn output(&self, _workflow_id: &str) -> ConversationResult<Option<WorkflowOutput>> {
        Ok(None)
    }
}
