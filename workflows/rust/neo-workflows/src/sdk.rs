use std::collections::HashMap;

use crate::checkpoint::CheckpointManager;
use crate::core::*;
use crate::definition::*;
use crate::error::WorkflowResult;
use crate::event::{EventId, EventType, WorkflowEvent, WorkflowEventSystem};
use crate::execution::{NodeExecutor, WorkflowExecutor, WorkflowInstance};
use crate::rollback::RollbackManager;
use crate::schedule::{ScheduleConfig, ScheduleManager, ScheduleType};
use crate::variable::{VariableManager, VariableType};
use chrono::Utc;

use neo_capabilities::core::CapabilityId as CapId;

/// Fluent builder for constructing workflow definitions.
pub struct WorkflowBuilder {
    name: String,
    description: String,
    nodes: Vec<NodeDefinition>,
    edges: Vec<EdgeDefinition>,
    config: WorkflowConfig,
    metadata: WorkflowMetadata,
}

impl WorkflowBuilder {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        let name_str = name.into();
        let meta = WorkflowMetadata::new(&name_str);
        Self {
            name: name_str,
            description: String::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
            config: WorkflowConfig::default(),
            metadata: meta,
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn config(mut self, config: WorkflowConfig) -> Self {
        self.config = config;
        self
    }

    pub fn timeout_ms(mut self, ms: u64) -> Self {
        self.config.timeout_ms = ms;
        self
    }

    pub fn max_concurrency(mut self, n: u32) -> Self {
        self.config.max_concurrency = n;
        self
    }

    pub fn enable_checkpoints(mut self, enabled: bool) -> Self {
        self.config.enable_checkpoints = enabled;
        self
    }

    pub fn enable_rollback(mut self, enabled: bool) -> Self {
        self.config.enable_rollback = enabled;
        self
    }

    /// Add a start node and return the node ID for edge creation.
    pub fn add_start(mut self) -> NodeId {
        let id = NodeId::new();
        self.nodes.push(NodeDefinition::Start(StartNodeDef {
            node_id: id,
            name: "start".into(),
        }));
        id
    }

    /// Add an end node and return the node ID.
    pub fn add_end(mut self) -> NodeId {
        let id = NodeId::new();
        self.nodes.push(NodeDefinition::End(EndNodeDef {
            node_id: id,
            name: "end".into(),
        }));
        id
    }

    /// Add a capability node.
    pub fn add_capability(
        mut self,
        name: impl Into<String>,
        capability_id: CapId,
    ) -> (NodeId, Self) {
        let id = NodeId::new();
        self.nodes
            .push(NodeDefinition::Capability(CapabilityNodeDef {
                node_id: id,
                name: name.into(),
                capability_id,
                input_mapping: HashMap::new(),
                output_mapping: HashMap::new(),
                retry_policy: RetryPolicy::default(),
                timeout_ms: 300_000,
                is_critical: true,
            }));
        (id, self)
    }

    /// Add a decision node.
    pub fn add_decision(
        mut self,
        name: impl Into<String>,
        conditions: Vec<ConditionBranch>,
    ) -> (NodeId, Self) {
        let id = NodeId::new();
        self.nodes.push(NodeDefinition::Decision(DecisionNodeDef {
            node_id: id,
            name: name.into(),
            conditions,
        }));
        (id, self)
    }

    /// Add a parallel node.
    pub fn add_parallel(
        mut self,
        name: impl Into<String>,
        branches: Vec<BranchDef>,
    ) -> (NodeId, Self) {
        let id = NodeId::new();
        self.nodes.push(NodeDefinition::Parallel(ParallelNodeDef {
            node_id: id,
            name: name.into(),
            branches,
        }));
        (id, self)
    }

    /// Add a delay node.
    pub fn add_delay(mut self, name: impl Into<String>, delay_ms: u64) -> (NodeId, Self) {
        let id = NodeId::new();
        self.nodes.push(NodeDefinition::Delay(DelayNodeDef {
            node_id: id,
            name: name.into(),
            delay_ms,
        }));
        (id, self)
    }

    /// Connect two nodes with an edge.
    pub fn connect(mut self, from: NodeId, to: NodeId) -> Self {
        self.edges.push(EdgeDefinition {
            id: EdgeId::new(),
            from,
            to,
            condition: None,
            label: None,
            is_critical: false,
        });
        self
    }

    /// Connect two nodes with a conditional edge.
    pub fn connect_if(
        mut self,
        from: NodeId,
        to: NodeId,
        condition: Condition,
        label: impl Into<String>,
    ) -> Self {
        self.edges.push(EdgeDefinition {
            id: EdgeId::new(),
            from,
            to,
            condition: Some(condition),
            label: Some(label.into()),
            is_critical: false,
        });
        self
    }

    /// Build the workflow definition.
    pub fn build(self) -> WorkflowResult<WorkflowDefinition> {
        let definition = WorkflowDefinition {
            id: WorkflowId::new(),
            name: self.name,
            description: self.description,
            version: WorkflowVersion::initial(),
            nodes: self.nodes,
            edges: self.edges,
            config: self.config,
            metadata: self.metadata,
            created_at: Utc::now(),
            modified_at: Utc::now(),
        };
        definition.validate()?;
        Ok(definition)
    }
}

/// Complete workflow runtime combining execution, scheduling, checkpointing,
/// events, rollback, variables, and analytics.
pub struct WorkflowRuntime {
    executor: WorkflowExecutor,
    pub event_system: WorkflowEventSystem,
    pub checkpoint_manager: CheckpointManager,
    pub schedule_manager: ScheduleManager,
    pub rollback_manager: RollbackManager,
    pub variable_manager: VariableManager,
}

impl WorkflowRuntime {
    pub fn new(definition: WorkflowDefinition) -> WorkflowResult<Self> {
        let executor = WorkflowExecutor::new(definition)?;
        Ok(Self {
            executor,
            event_system: WorkflowEventSystem::new(),
            checkpoint_manager: CheckpointManager::new(100),
            schedule_manager: ScheduleManager::new(),
            rollback_manager: RollbackManager::new(true),
            variable_manager: VariableManager::new(),
        })
    }

    pub fn with_executor(
        definition: WorkflowDefinition,
        node_executor: Box<dyn NodeExecutor>,
    ) -> WorkflowResult<Self> {
        let executor = WorkflowExecutor::with_executor(definition, node_executor)?;
        Ok(Self {
            executor,
            event_system: WorkflowEventSystem::new(),
            checkpoint_manager: CheckpointManager::new(100),
            schedule_manager: ScheduleManager::new(),
            rollback_manager: RollbackManager::new(true),
            variable_manager: VariableManager::new(),
        })
    }

    /// Execute a workflow with full runtime support.
    pub async fn execute(
        &mut self,
        context: WorkflowContext,
    ) -> WorkflowResult<WorkflowResultOutput> {
        let wf_id = self.executor.definition().id;
        let instance = self.executor.create_instance(context.clone());

        // Emit started event
        self.event_system.emit(WorkflowEvent {
            id: EventId::new(),
            event_type: EventType::WorkflowStarted,
            workflow_id: wf_id,
            execution_id: instance.id,
            node_id: None,
            payload: serde_json::Value::Null,
            timestamp: Utc::now(),
        });

        // Create initial checkpoint if enabled
        if self.executor.definition().config.enable_checkpoints {
            use std::collections::HashMap;
            self.checkpoint_manager.create_checkpoint(
                instance.id,
                wf_id,
                WorkflowState::Running,
                &context,
                HashMap::new(),
            );
        }

        // Execute
        let result = self.executor.execute(context).await;

        // Emit completion event
        let exec_id = instance.id;
        match &result {
            Ok(output) => {
                self.event_system.emit(WorkflowEvent {
                    id: EventId::new(),
                    event_type: if output.success {
                        EventType::WorkflowCompleted
                    } else {
                        EventType::WorkflowFailed
                    },
                    workflow_id: wf_id,
                    execution_id: exec_id,
                    node_id: None,
                    payload: serde_json::to_value(output).unwrap_or_default(),
                    timestamp: Utc::now(),
                });
            }
            Err(e) => {
                self.event_system.emit(WorkflowEvent {
                    id: EventId::new(),
                    event_type: EventType::WorkflowFailed,
                    workflow_id: wf_id,
                    execution_id: exec_id,
                    node_id: None,
                    payload: serde_json::json!({ "error": e.to_string() }),
                    timestamp: Utc::now(),
                });
            }
        }

        result
    }

    /// Create a schedule for a workflow.
    pub fn create_schedule(
        &mut self,
        workflow_id: WorkflowId,
        schedule_type: ScheduleType,
    ) -> ScheduleConfig {
        let config = ScheduleConfig::new(workflow_id, schedule_type);
        self.schedule_manager.add(config.clone());
        config
    }

    /// Add a variable.
    pub fn set_variable(
        &mut self,
        name: String,
        value: serde_json::Value,
        var_type: VariableType,
    ) -> WorkflowResult<()> {
        self.variable_manager.set(name, value, var_type)
    }

    /// Create a workflow definition using the builder pattern.
    pub fn build_workflow<F>(name: impl Into<String>, f: F) -> WorkflowResult<WorkflowDefinition>
    where
        F: FnOnce(WorkflowBuilder) -> WorkflowBuilder,
    {
        let builder = WorkflowBuilder::new(name);
        let builder = f(builder);
        builder.build()
    }
}

impl std::fmt::Debug for WorkflowRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkflowRuntime")
            .field("definition", &self.executor.definition().name)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn builder_simple() {
        let result = WorkflowBuilder::new("test").build();
        assert!(result.is_err());
    }

    #[test]
    fn builder_linear() {
        let start = NodeId::new();
        let end = NodeId::new();

        let builder = WorkflowBuilder::new("linear")
            .description("a linear workflow")
            .timeout_ms(5000);
        assert_eq!(builder.name, "linear");
        assert_eq!(builder.config.timeout_ms, 5000);

        // Use direct construction for complex tests
        let def = WorkflowDefinition {
            id: WorkflowId::new(),
            name: "linear".into(),
            description: "a linear workflow".into(),
            version: WorkflowVersion::initial(),
            nodes: vec![
                NodeDefinition::Start(StartNodeDef {
                    node_id: start,
                    name: "start".into(),
                }),
                NodeDefinition::End(EndNodeDef {
                    node_id: end,
                    name: "end".into(),
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
            metadata: WorkflowMetadata::new("linear"),
            created_at: Utc::now(),
            modified_at: Utc::now(),
        };
        assert!(def.validate().is_ok());
    }

    #[test]
    fn runtime_creation() {
        let start = NodeId::new();
        let end = NodeId::new();
        let def = WorkflowDefinition {
            id: WorkflowId::new(),
            name: "rt".into(),
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
            metadata: WorkflowMetadata::new("rt"),
            created_at: Utc::now(),
            modified_at: Utc::now(),
        };
        let mut rt = WorkflowRuntime::new(def).unwrap();
        assert_eq!(rt.executor.definition().name, "rt");
    }

    #[test]
    fn schedule_creation() {
        let start = NodeId::new();
        let end = NodeId::new();
        let def = WorkflowDefinition {
            id: WorkflowId::new(),
            name: "sched".into(),
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
            metadata: WorkflowMetadata::new("sched"),
            created_at: Utc::now(),
            modified_at: Utc::now(),
        };
        let mut rt = WorkflowRuntime::new(def.clone()).unwrap();
        let wf_id = def.id;
        let sched = rt.create_schedule(wf_id, ScheduleType::Once);
        assert_eq!(rt.schedule_manager.count(), 1);
    }
}
