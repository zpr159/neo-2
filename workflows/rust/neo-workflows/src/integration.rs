use async_trait::async_trait;
use std::collections::HashMap;

use crate::checkpoint::{CheckpointManager, NodeSnapshot, WorkflowCheckpoint};
use crate::core::*;
use crate::definition::*;
use crate::error::{WorkflowError, WorkflowResult};
use crate::event::{EventId, EventType, WorkflowEvent, WorkflowEventSystem};
use crate::execution::{DefaultNodeExecutor, NodeExecutor, WorkflowExecutor, WorkflowInstance};
use crate::rollback::RollbackManager;
use crate::schedule::ScheduleManager;
use crate::variable::{VariableManager, VariableType};

use chrono::Utc;

/// Trait for workflow persistence backends.
#[async_trait]
pub trait WorkflowPersistence: Send + Sync {
    /// Save a workflow definition.
    async fn save_definition(&self, definition: &WorkflowDefinition) -> WorkflowResult<()>;

    /// Load a workflow definition by ID.
    async fn load_definition(&self, workflow_id: &WorkflowId)
        -> WorkflowResult<WorkflowDefinition>;

    /// List all workflow definitions.
    async fn list_definitions(&self) -> WorkflowResult<Vec<WorkflowDefinition>>;

    /// Delete a workflow definition.
    async fn delete_definition(&self, workflow_id: &WorkflowId) -> WorkflowResult<()>;

    /// Save an execution state.
    async fn save_execution(&self, instance: &WorkflowInstance) -> WorkflowResult<()>;

    /// Load an execution state by ID.
    async fn load_execution(&self, execution_id: &ExecutionId) -> WorkflowResult<WorkflowInstance>;

    /// List executions for a workflow.
    async fn list_executions(&self, workflow_id: &WorkflowId) -> WorkflowResult<Vec<ExecutionId>>;

    /// Save a checkpoint.
    async fn save_checkpoint(&self, checkpoint: &WorkflowCheckpoint) -> WorkflowResult<()>;

    /// Load a checkpoint.
    async fn load_checkpoint(
        &self,
        checkpoint_id: &CheckpointId,
    ) -> WorkflowResult<WorkflowCheckpoint>;

    /// Delete checkpoints for an execution.
    async fn delete_checkpoints(&self, execution_id: &ExecutionId) -> WorkflowResult<()>;
}

/// In-memory persistence backend (for testing and development).
#[derive(Debug, Default)]
pub struct InMemoryPersistence {
    definitions: std::sync::RwLock<HashMap<WorkflowId, WorkflowDefinition>>,
    executions: std::sync::RwLock<HashMap<ExecutionId, WorkflowInstance>>,
    checkpoints: std::sync::RwLock<HashMap<CheckpointId, WorkflowCheckpoint>>,
    execution_index: std::sync::RwLock<HashMap<WorkflowId, Vec<ExecutionId>>>,
    checkpoint_index: std::sync::RwLock<HashMap<ExecutionId, Vec<CheckpointId>>>,
}

#[async_trait]
impl WorkflowPersistence for InMemoryPersistence {
    async fn save_definition(&self, definition: &WorkflowDefinition) -> WorkflowResult<()> {
        let mut defs = self
            .definitions
            .write()
            .map_err(|e| WorkflowError::internal(format!("lock poisoned: {e}")))?;
        defs.insert(definition.id, definition.clone());
        Ok(())
    }

    async fn load_definition(
        &self,
        workflow_id: &WorkflowId,
    ) -> WorkflowResult<WorkflowDefinition> {
        let defs = self
            .definitions
            .read()
            .map_err(|e| WorkflowError::internal(format!("lock poisoned: {e}")))?;
        defs.get(workflow_id)
            .cloned()
            .ok_or_else(|| WorkflowError::not_found(*workflow_id))
    }

    async fn list_definitions(&self) -> WorkflowResult<Vec<WorkflowDefinition>> {
        let defs = self
            .definitions
            .read()
            .map_err(|e| WorkflowError::internal(format!("lock poisoned: {e}")))?;
        Ok(defs.values().cloned().collect())
    }

    async fn delete_definition(&self, workflow_id: &WorkflowId) -> WorkflowResult<()> {
        let mut defs = self
            .definitions
            .write()
            .map_err(|e| WorkflowError::internal(format!("lock poisoned: {e}")))?;
        defs.remove(workflow_id);
        Ok(())
    }

    async fn save_execution(&self, instance: &WorkflowInstance) -> WorkflowResult<()> {
        let mut execs = self
            .executions
            .write()
            .map_err(|e| WorkflowError::internal(format!("lock poisoned: {e}")))?;
        execs.insert(instance.id, instance.clone());

        let mut idx = self
            .execution_index
            .write()
            .map_err(|e| WorkflowError::internal(format!("lock poisoned: {e}")))?;
        idx.entry(instance.workflow_id)
            .or_default()
            .push(instance.id);
        Ok(())
    }

    async fn load_execution(&self, execution_id: &ExecutionId) -> WorkflowResult<WorkflowInstance> {
        let execs = self
            .executions
            .read()
            .map_err(|e| WorkflowError::internal(format!("lock poisoned: {e}")))?;
        execs
            .get(execution_id)
            .cloned()
            .ok_or_else(|| WorkflowError::internal("execution not found"))
    }

    async fn list_executions(&self, workflow_id: &WorkflowId) -> WorkflowResult<Vec<ExecutionId>> {
        let idx = self
            .execution_index
            .read()
            .map_err(|e| WorkflowError::internal(format!("lock poisoned: {e}")))?;
        Ok(idx.get(workflow_id).cloned().unwrap_or_default())
    }

    async fn save_checkpoint(&self, checkpoint: &WorkflowCheckpoint) -> WorkflowResult<()> {
        let mut cps = self
            .checkpoints
            .write()
            .map_err(|e| WorkflowError::internal(format!("lock poisoned: {e}")))?;
        cps.insert(checkpoint.id, checkpoint.clone());

        let mut idx = self
            .checkpoint_index
            .write()
            .map_err(|e| WorkflowError::internal(format!("lock poisoned: {e}")))?;
        idx.entry(checkpoint.execution_id)
            .or_default()
            .push(checkpoint.id);
        Ok(())
    }

    async fn load_checkpoint(
        &self,
        checkpoint_id: &CheckpointId,
    ) -> WorkflowResult<WorkflowCheckpoint> {
        let cps = self
            .checkpoints
            .read()
            .map_err(|e| WorkflowError::internal(format!("lock poisoned: {e}")))?;
        cps.get(checkpoint_id)
            .cloned()
            .ok_or_else(|| WorkflowError::internal("checkpoint not found"))
    }

    async fn delete_checkpoints(&self, execution_id: &ExecutionId) -> WorkflowResult<()> {
        let mut idx = self
            .checkpoint_index
            .write()
            .map_err(|e| WorkflowError::internal(format!("lock poisoned: {e}")))?;
        let cp_ids = idx.remove(execution_id).unwrap_or_default();

        let mut cps = self
            .checkpoints
            .write()
            .map_err(|e| WorkflowError::internal(format!("lock poisoned: {e}")))?;
        for id in cp_ids {
            cps.remove(&id);
        }
        Ok(())
    }
}

/// Orchestrator that ties together execution, persistence, events, checkpoints,
/// schedules, rollback, and variables.
pub struct WorkflowOrchestrator {
    persistence: Box<dyn WorkflowPersistence>,
    event_system: WorkflowEventSystem,
    checkpoint_manager: CheckpointManager,
    schedule_manager: ScheduleManager,
    rollback_manager: RollbackManager,
}

impl WorkflowOrchestrator {
    pub fn new(persistence: Box<dyn WorkflowPersistence>) -> Self {
        Self {
            persistence,
            event_system: WorkflowEventSystem::new(),
            checkpoint_manager: CheckpointManager::new(100),
            schedule_manager: ScheduleManager::new(),
            rollback_manager: RollbackManager::new(true),
        }
    }

    pub fn with_persistence(
        persistence: Box<dyn WorkflowPersistence>,
        max_checkpoints: usize,
    ) -> Self {
        Self {
            persistence,
            event_system: WorkflowEventSystem::new(),
            checkpoint_manager: CheckpointManager::new(max_checkpoints),
            schedule_manager: ScheduleManager::new(),
            rollback_manager: RollbackManager::new(true),
        }
    }

    /// Register a workflow definition.
    pub async fn register_workflow(&self, definition: WorkflowDefinition) -> WorkflowResult<()> {
        definition.validate()?;
        self.persistence.save_definition(&definition).await
    }

    /// Get a workflow definition.
    pub async fn get_workflow(
        &self,
        workflow_id: &WorkflowId,
    ) -> WorkflowResult<WorkflowDefinition> {
        self.persistence.load_definition(workflow_id).await
    }

    /// List all workflows.
    pub async fn list_workflows(&self) -> WorkflowResult<Vec<WorkflowDefinition>> {
        self.persistence.list_definitions().await
    }

    /// Delete a workflow.
    pub async fn delete_workflow(&self, workflow_id: &WorkflowId) -> WorkflowResult<()> {
        self.persistence.delete_definition(workflow_id).await
    }

    /// Validate a workflow definition.
    pub async fn validate_workflow(
        &self,
        workflow_id: &WorkflowId,
    ) -> WorkflowResult<ValidationResult> {
        let definition = self.persistence.load_definition(workflow_id).await?;
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        if let Err(e) = definition.validate() {
            errors.push(e.to_string());
        }

        match crate::dag::Dag::from_definition(&definition) {
            Ok(dag) => {
                if let Err(e) = crate::dag::DagValidator::validate(&dag) {
                    errors.push(e.to_string());
                }
            }
            Err(e) => errors.push(e.to_string()),
        }

        if definition.config.timeout_ms == 0 {
            warnings.push("No timeout configured".into());
        }

        Ok(ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        })
    }

    /// Execute a workflow.
    pub async fn execute_workflow(
        &mut self,
        workflow_id: &WorkflowId,
        variables: HashMap<String, serde_json::Value>,
    ) -> WorkflowResult<WorkflowResultOutput> {
        let definition = self.persistence.load_definition(workflow_id).await?;
        let executor = WorkflowExecutor::new(definition)?;

        let mut context = WorkflowContext::new();
        for (k, v) in variables {
            context.set_variable(k, v);
        }

        self.event_system.emit(WorkflowEvent {
            id: EventId::new(),
            event_type: EventType::WorkflowStarted,
            workflow_id: *workflow_id,
            execution_id: ExecutionId::new(),
            node_id: None,
            payload: serde_json::Value::Null,
            timestamp: Utc::now(),
        });

        let result = executor.execute(context).await;

        match &result {
            Ok(output) => {
                let event_type = if output.success {
                    EventType::WorkflowCompleted
                } else {
                    EventType::WorkflowFailed
                };
                self.event_system.emit(WorkflowEvent {
                    id: EventId::new(),
                    event_type,
                    workflow_id: *workflow_id,
                    execution_id: ExecutionId::new(),
                    node_id: None,
                    payload: serde_json::to_value(output).unwrap_or_default(),
                    timestamp: Utc::now(),
                });
            }
            Err(e) => {
                self.event_system.emit(WorkflowEvent {
                    id: EventId::new(),
                    event_type: EventType::WorkflowFailed,
                    workflow_id: *workflow_id,
                    execution_id: ExecutionId::new(),
                    node_id: None,
                    payload: serde_json::json!({ "error": e.to_string() }),
                    timestamp: Utc::now(),
                });
            }
        }

        result
    }

    /// Get event system reference.
    #[must_use]
    pub fn event_system(&self) -> &WorkflowEventSystem {
        &self.event_system
    }

    /// Get schedule manager reference.
    #[must_use]
    pub fn schedule_manager(&self) -> &ScheduleManager {
        &self.schedule_manager
    }
}

/// Result of workflow validation.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn in_memory_persistence() {
        let persistence = InMemoryPersistence::default();
        let start = NodeId::new();
        let end = NodeId::new();
        let def = WorkflowDefinition {
            id: WorkflowId::new(),
            name: "test".into(),
            description: "".into(),
            version: WorkflowVersion::initial(),
            nodes: vec![
                NodeDefinition::Start(StartNodeDef {
                    node_id: start,
                    name: "s".into(),
                }),
                NodeDefinition::End(EndNodeDef {
                    node_id: end,
                    name: "e".into(),
                }),
            ],
            edges: vec![EdgeDefinition {
                id: EdgeId::new(),
                from: start,
                to: end,
                condition: None,
                label: None,
                is_critical: false,
            }],
            config: WorkflowConfig::default(),
            metadata: WorkflowMetadata::new("test"),
            created_at: Utc::now(),
            modified_at: Utc::now(),
        };

        persistence.save_definition(&def).await.unwrap();
        let loaded = persistence.load_definition(&def.id).await.unwrap();
        assert_eq!(loaded.name, "test");

        let list = persistence.list_definitions().await.unwrap();
        assert_eq!(list.len(), 1);

        persistence.delete_definition(&def.id).await.unwrap();
        assert!(persistence.load_definition(&def.id).await.is_err());
    }

    #[tokio::test]
    async fn orchestrator_validate() {
        let persistence = Box::new(InMemoryPersistence::default());
        let mut orchestrator = WorkflowOrchestrator::new(persistence);

        let start = NodeId::new();
        let end = NodeId::new();
        let def = WorkflowDefinition {
            id: WorkflowId::new(),
            name: "validate_test".into(),
            description: "".into(),
            version: WorkflowVersion::initial(),
            nodes: vec![
                NodeDefinition::Start(StartNodeDef {
                    node_id: start,
                    name: "s".into(),
                }),
                NodeDefinition::End(EndNodeDef {
                    node_id: end,
                    name: "e".into(),
                }),
            ],
            edges: vec![EdgeDefinition {
                id: EdgeId::new(),
                from: start,
                to: end,
                condition: None,
                label: None,
                is_critical: false,
            }],
            config: WorkflowConfig::default(),
            metadata: WorkflowMetadata::new("validate_test"),
            created_at: Utc::now(),
            modified_at: Utc::now(),
        };

        orchestrator.register_workflow(def.clone()).await.unwrap();
        let result = orchestrator.validate_workflow(&def.id).await.unwrap();
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }
}
