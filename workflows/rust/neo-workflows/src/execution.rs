use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tracing::{error, info, warn};

use crate::core::*;
use crate::dag::{Dag, ExecutionPlan};
use crate::definition::*;
use crate::error::{WorkflowError, WorkflowResult};

// ---------------------------------------------------------------------------
// WorkflowInstance
// ---------------------------------------------------------------------------

/// Runtime representation of a running workflow execution.
#[derive(Debug, Clone)]
pub struct WorkflowInstance {
    /// Unique execution ID.
    pub id: ExecutionId,
    /// The workflow definition ID.
    pub workflow_id: WorkflowId,
    /// Current state.
    pub state: WorkflowState,
    /// Execution context with variables.
    pub context: WorkflowContext,
    /// Current state of each node.
    pub node_states: HashMap<NodeId, NodeState>,
    /// Output from each completed node.
    pub node_outputs: HashMap<NodeId, serde_json::Value>,
    /// When each node started executing.
    pub node_start_times: HashMap<NodeId, DateTime<Utc>>,
    /// When each node finished executing.
    pub node_end_times: HashMap<NodeId, DateTime<Utc>>,
    /// Nodes in execution order.
    pub execution_order: Vec<NodeId>,
    /// Index into execution_order for current processing.
    pub current_index: usize,
    /// When the workflow started.
    pub started_at: Option<DateTime<Utc>>,
    /// When the workflow completed.
    pub completed_at: Option<DateTime<Utc>>,
    /// Final result.
    pub result: Option<WorkflowResultOutput>,
    /// Error message if failed.
    pub error: Option<String>,
    /// Retry count per node.
    pub retry_count: HashMap<NodeId, u32>,
    /// Nodes that have been compensated (rolled back).
    pub compensated: HashSet<NodeId>,
}

impl WorkflowInstance {
    /// Create a new instance for the given workflow.
    #[must_use]
    pub fn new(workflow_id: WorkflowId, context: WorkflowContext) -> Self {
        Self {
            id: ExecutionId::new(),
            workflow_id,
            state: WorkflowState::Created,
            context,
            node_states: HashMap::new(),
            node_outputs: HashMap::new(),
            node_start_times: HashMap::new(),
            node_end_times: HashMap::new(),
            execution_order: Vec::new(),
            current_index: 0,
            started_at: None,
            completed_at: None,
            result: None,
            error: None,
            retry_count: HashMap::new(),
            compensated: HashSet::new(),
        }
    }

    /// Initialize node states for all nodes in the definition.
    pub fn initialize_nodes(&mut self, definition: &WorkflowDefinition) {
        for node in &definition.nodes {
            self.node_states.insert(node.node_id(), NodeState::Pending);
        }
    }

    /// Get the current workflow state.
    #[must_use]
    pub fn state(&self) -> WorkflowState {
        self.state
    }

    /// Transition to a new state.
    pub fn advance_state(&mut self, target: WorkflowState) -> WorkflowResult<()> {
        self.state = self.state.try_transition(target)?;
        if self.state == WorkflowState::Running && self.started_at.is_none() {
            self.started_at = Some(Utc::now());
        }
        if self.state.is_terminal() && self.completed_at.is_none() {
            self.completed_at = Some(Utc::now());
        }
        Ok(())
    }

    /// Check if all nodes have reached a terminal state.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.node_states.values().all(|s| s.is_terminal())
    }

    /// Get execution progress as a fraction (0.0 to 1.0).
    #[must_use]
    pub fn progress(&self) -> f32 {
        if self.node_states.is_empty() {
            return 0.0;
        }
        let completed = self
            .node_states
            .values()
            .filter(|s| s.is_terminal())
            .count();
        completed as f32 / self.node_states.len() as f32
    }

    /// Total workflow duration in milliseconds.
    #[must_use]
    pub fn duration_ms(&self) -> u64 {
        match (self.started_at, self.completed_at) {
            (Some(start), Some(end)) => (end - start).num_milliseconds().max(0) as u64,
            (Some(start), None) => (Utc::now() - start).num_milliseconds().max(0) as u64,
            _ => 0,
        }
    }

    /// Elapsed time since start in milliseconds.
    #[must_use]
    pub fn elapsed_ms(&self) -> u64 {
        self.started_at.map_or(0, |start| {
            (Utc::now() - start).num_milliseconds().max(0) as u64
        })
    }

    /// Mark a node as running.
    pub fn start_node(&mut self, node_id: NodeId) {
        self.node_states.insert(node_id, NodeState::Running);
        self.node_start_times.insert(node_id, Utc::now());
        self.execution_order.push(node_id);
    }

    /// Mark a node as completed with output.
    pub fn complete_node(&mut self, node_id: NodeId, output: serde_json::Value) {
        self.node_states.insert(node_id, NodeState::Completed);
        self.node_end_times.insert(node_id, Utc::now());
        self.node_outputs.insert(node_id, output);
    }

    /// Mark a node as failed.
    pub fn fail_node(&mut self, node_id: NodeId, error: String) {
        self.node_states.insert(node_id, NodeState::Failed);
        self.node_end_times.insert(node_id, Utc::now());
        self.error = Some(error);
    }

    /// Mark a node as skipped.
    pub fn skip_node(&mut self, node_id: NodeId) {
        self.node_states.insert(node_id, NodeState::Skipped);
        self.node_end_times.insert(node_id, Utc::now());
    }

    /// Get the retry count for a node.
    #[must_use]
    pub fn node_retry_count(&self, node_id: &NodeId) -> u32 {
        self.retry_count.get(node_id).copied().unwrap_or(0)
    }

    /// Increment retry count for a node.
    pub fn increment_retry(&mut self, node_id: NodeId) {
        *self.retry_count.entry(node_id).or_insert(0) += 1;
    }

    /// Get output from a predecessor node.
    #[must_use]
    pub fn predecessor_output(&self, node_id: NodeId, dag: &Dag) -> serde_json::Value {
        let predecessors = dag.predecessors(&node_id);
        if predecessors.len() == 1 {
            return self
                .node_outputs
                .get(&predecessors[0])
                .cloned()
                .unwrap_or(serde_json::Value::Null);
        }
        let mut map = serde_json::Map::new();
        for pred in &predecessors {
            if let Some(output) = self.node_outputs.get(pred) {
                map.insert(pred.to_string(), output.clone());
            }
        }
        serde_json::Value::Object(map)
    }
}

// ---------------------------------------------------------------------------
// NodeExecutor trait
// ---------------------------------------------------------------------------

/// Trait for executing individual workflow nodes.
#[async_trait]
pub trait NodeExecutor: Send + Sync {
    /// Execute a node and produce output.
    async fn execute(
        &self,
        node: &NodeDefinition,
        context: &WorkflowContext,
        inputs: serde_json::Value,
    ) -> WorkflowResult<serde_json::Value>;

    /// Whether this node can be compensated (rolled back).
    fn can_compensate(&self, _node: &NodeDefinition) -> bool {
        false
    }

    /// Execute compensation/rollback for a node.
    async fn compensate(
        &self,
        _node: &NodeDefinition,
        _context: &WorkflowContext,
    ) -> WorkflowResult<()> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// DefaultNodeExecutor
// ---------------------------------------------------------------------------

/// Default node executor that handles all standard node types.
#[derive(Debug, Default)]
pub struct DefaultNodeExecutor;

#[async_trait]
impl NodeExecutor for DefaultNodeExecutor {
    async fn execute(
        &self,
        node: &NodeDefinition,
        context: &WorkflowContext,
        inputs: serde_json::Value,
    ) -> WorkflowResult<serde_json::Value> {
        match node {
            NodeDefinition::Start(_) => Ok(serde_json::Value::Null),

            NodeDefinition::End(_) => Ok(inputs),

            NodeDefinition::Capability(cap) => {
                info!(
                    "Executing capability node '{}' (cap_id={})",
                    cap.name, cap.capability_id
                );
                Ok(serde_json::json!({
                    "capability_id": cap.capability_id.to_string(),
                    "status": "completed",
                    "input": inputs,
                }))
            }

            NodeDefinition::Decision(decision) => {
                for branch in &decision.conditions {
                    if evaluate_condition(&branch.condition, context) {
                        info!(
                            "Decision '{}': matched branch '{}'",
                            decision.name, branch.label
                        );
                        return Ok(serde_json::json!({
                            "branch": branch.label,
                            "target": branch.target_node_id.to_string(),
                        }));
                    }
                }
                Ok(serde_json::json!({
                    "branch": "__default__",
                }))
            }

            NodeDefinition::Parallel(parallel) => {
                let mut results = Vec::new();
                for branch in &parallel.branches {
                    results.push(serde_json::json!({
                        "branch_id": branch.branch_id.to_string(),
                        "branch_name": branch.name,
                        "status": "completed",
                    }));
                }
                Ok(serde_json::json!({ "branches": results }))
            }

            NodeDefinition::Merge(merge) => {
                let merged = match merge.merge_strategy {
                    MergeStrategy::All => inputs,
                    MergeStrategy::Any => inputs,
                    MergeStrategy::First => {
                        if let serde_json::Value::Object(map) = &inputs {
                            map.values()
                                .next()
                                .cloned()
                                .unwrap_or(serde_json::Value::Null)
                        } else {
                            inputs
                        }
                    }
                };
                Ok(merged)
            }

            NodeDefinition::Loop(loop_node) => {
                let max_iter = if loop_node.max_iterations == 0 {
                    100
                } else {
                    loop_node.max_iterations
                };
                let mut results = Vec::new();
                for i in 0..max_iter {
                    if let Some(ref cond) = loop_node.break_condition {
                        if evaluate_condition(cond, context) {
                            info!("Loop '{}' breaking at iteration {}", loop_node.name, i);
                            break;
                        }
                    }
                    results.push(serde_json::json!({ "iteration": i }));
                }
                Ok(serde_json::json!({
                    "iterations": results.len(),
                    "results": results,
                }))
            }

            NodeDefinition::Delay(delay) => {
                info!("Delaying for {}ms", delay.delay_ms);
                tokio::time::sleep(tokio::time::Duration::from_millis(delay.delay_ms)).await;
                Ok(serde_json::Value::Null)
            }

            NodeDefinition::Wait(_) => {
                // Simplified: return immediately
                Ok(serde_json::Value::Null)
            }

            NodeDefinition::HumanApproval(ha) => {
                info!(
                    "Human approval required for '{}' (assignee: {})",
                    ha.name, ha.assignee
                );
                Ok(serde_json::json!({
                    "approved": true,
                    "assignee": ha.assignee,
                }))
            }

            NodeDefinition::SubWorkflow(sw) => {
                info!("Sub-workflow '{}' (id={})", sw.name, sw.sub_workflow_id);
                Ok(serde_json::json!({
                    "sub_workflow_id": sw.sub_workflow_id.to_string(),
                    "status": "completed",
                }))
            }
        }
    }

    fn can_compensate(&self, node: &NodeDefinition) -> bool {
        matches!(
            node,
            NodeDefinition::Capability(_)
                | NodeDefinition::SubWorkflow(_)
                | NodeDefinition::HumanApproval(_)
        )
    }
}

/// Evaluate a condition against the workflow context.
#[must_use]
pub fn evaluate_condition(condition: &Condition, context: &WorkflowContext) -> bool {
    match condition {
        Condition::Always => true,
        Condition::VariableEquals { key, value } => context
            .get_variable(key)
            .map_or(false, |v| v.to_string() == value.to_string()),
        Condition::VariableGreaterThan { key, value } => {
            if let Some(v) = context.get_variable(key) {
                if let Some(num) = v.as_f64() {
                    return num > *value;
                }
            }
            false
        }
        Condition::Expression(expr) => {
            // Simplified: non-empty expression evaluates to true
            !expr.is_empty()
        }
    }
}

// ---------------------------------------------------------------------------
// WorkflowExecutor
// ---------------------------------------------------------------------------

/// The main workflow execution engine.
pub struct WorkflowExecutor {
    definition: WorkflowDefinition,
    dag: Dag,
    execution_plan: ExecutionPlan,
    node_executor: Box<dyn NodeExecutor>,
}

impl WorkflowExecutor {
    /// Create a new executor from a workflow definition.
    pub fn new(definition: WorkflowDefinition) -> WorkflowResult<Self> {
        let dag = Dag::from_definition(&definition)?;
        crate::dag::DagValidator::validate(&dag)?;
        let execution_plan = ExecutionPlan::from_dag(&dag)?;
        Ok(Self {
            definition,
            dag,
            execution_plan,
            node_executor: Box::new(DefaultNodeExecutor),
        })
    }

    /// Create with a custom node executor.
    pub fn with_executor(
        definition: WorkflowDefinition,
        executor: Box<dyn NodeExecutor>,
    ) -> WorkflowResult<Self> {
        let dag = Dag::from_definition(&definition)?;
        crate::dag::DagValidator::validate(&dag)?;
        let execution_plan = ExecutionPlan::from_dag(&dag)?;
        Ok(Self {
            definition,
            dag,
            execution_plan,
            node_executor: executor,
        })
    }

    /// Get a reference to the definition.
    #[must_use]
    pub fn definition(&self) -> &WorkflowDefinition {
        &self.definition
    }

    /// Create a new instance for execution.
    #[must_use]
    pub fn create_instance(&self, context: WorkflowContext) -> WorkflowInstance {
        let mut instance = WorkflowInstance::new(self.definition.id, context);
        instance.initialize_nodes(&self.definition);
        instance
    }

    /// Execute the entire workflow to completion.
    pub async fn execute(&self, context: WorkflowContext) -> WorkflowResult<WorkflowResultOutput> {
        let mut instance = self.create_instance(context);
        instance.advance_state(WorkflowState::Queued)?;
        instance.advance_state(WorkflowState::Running)?;

        let start = std::time::Instant::now();
        let mut nodes_executed: u32 = 0;
        let mut total_retries: u32 = 0;

        // Process level by level
        for level in self.execution_plan.iter() {
            if instance.state == WorkflowState::Cancelled {
                break;
            }

            // Check timeout
            if self.definition.config.timeout_ms > 0
                && instance.elapsed_ms() > self.definition.config.timeout_ms
            {
                instance.advance_state(WorkflowState::TimedOut)?;
                return Ok(WorkflowResultOutput::failure(
                    "workflow timed out".into(),
                    start.elapsed().as_millis() as u64,
                    nodes_executed,
                ));
            }

            // Check cancellation
            if instance.context.is_cancelled() {
                let _ = instance.advance_state(WorkflowState::Cancelled);
                return Ok(WorkflowResultOutput::failure(
                    "workflow cancelled".into(),
                    start.elapsed().as_millis() as u64,
                    nodes_executed,
                ));
            }

            for &node_id in level {
                if instance
                    .node_states
                    .get(&node_id)
                    .map_or(false, |s| s.is_terminal())
                {
                    continue;
                }

                // Check if this is a decision node - evaluate and skip non-chosen branches
                if let Some(node) = self.dag.node(&node_id) {
                    if let NodeDefinition::Decision(ref decision) = node {
                        let chosen = self.evaluate_decision(decision, &instance.context);
                        let chosen_set: HashSet<NodeId> = chosen.into_iter().collect();
                        // Skip branches that aren't chosen
                        for &child_id in &self.dag.successors(&node_id) {
                            if !chosen_set.contains(&child_id) {
                                instance.skip_node(child_id);
                            }
                        }
                    }
                }

                match self.execute_node_with_retry(&mut instance, node_id).await {
                    Ok(output) => {
                        instance.complete_node(node_id, output);
                        nodes_executed += 1;
                        info!(
                            "Node {:?} completed ({:.1}% progress)",
                            node_id,
                            instance.progress() * 100.0
                        );
                    }
                    Err(e) => {
                        instance.fail_node(node_id, e.to_string());
                        error!("Node {:?} failed: {}", node_id, e);

                        // Trigger rollback if enabled
                        if self.definition.config.enable_rollback {
                            let _ = instance.advance_state(WorkflowState::RollingBack);
                            self.execute_rollback(&mut instance).await;
                        }

                        let _ = instance.advance_state(WorkflowState::Failed);
                        return Ok(WorkflowResultOutput::failure(
                            e.to_string(),
                            start.elapsed().as_millis() as u64,
                            nodes_executed,
                        ));
                    }
                }
            }
        }

        let _ = instance.advance_state(WorkflowState::Completed);
        total_retries = instance.retry_count.values().sum();

        Ok(WorkflowResultOutput {
            success: true,
            output: serde_json::json!({
                "execution_id": instance.id.to_string(),
                "completed_nodes": nodes_executed,
            }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
            nodes_executed,
            retries: total_retries,
        })
    }

    /// Execute a single node with retry logic.
    async fn execute_node_with_retry(
        &self,
        instance: &mut WorkflowInstance,
        node_id: NodeId,
    ) -> WorkflowResult<serde_json::Value> {
        let node = self
            .dag
            .node(&node_id)
            .ok_or_else(|| WorkflowError::node_not_found(node_id))?;

        let retry_policy = self.get_retry_policy(node);
        let max_attempts = retry_policy.max_attempts.max(1);
        let mut last_error = None;

        for attempt in 0..max_attempts {
            if attempt > 0 {
                instance.increment_retry(node_id);
                let delay = retry_policy.delay_for_attempt(attempt);
                warn!(
                    "Retrying node {:?} (attempt {}/{}, delay={}ms)",
                    node_id,
                    attempt + 1,
                    max_attempts,
                    delay
                );
                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            }

            instance.start_node(node_id);

            let inputs = instance.predecessor_output(node_id, &self.dag);
            match self
                .node_executor
                .execute(node, &instance.context, inputs)
                .await
            {
                Ok(output) => return Ok(output),
                Err(e) => {
                    warn!("Node {:?} attempt {} failed: {}", node_id, attempt + 1, e);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            WorkflowError::internal("no error recorded after retries exhausted")
        }))
    }

    /// Get the retry policy for a node.
    fn get_retry_policy(&self, node: &NodeDefinition) -> RetryPolicy {
        match node {
            NodeDefinition::Capability(cap) => cap.retry_policy.clone(),
            _ => RetryPolicy {
                max_attempts: 1,
                ..RetryPolicy::default()
            },
        }
    }

    /// Evaluate a decision node and return the target node IDs for chosen branches.
    fn evaluate_decision(
        &self,
        decision: &DecisionNodeDef,
        context: &WorkflowContext,
    ) -> Vec<NodeId> {
        let mut targets = Vec::new();
        for branch in &decision.conditions {
            if evaluate_condition(&branch.condition, context) {
                targets.push(branch.target_node_id);
            }
        }
        targets
    }

    /// Execute rollback (compensation) in reverse order.
    async fn execute_rollback(&self, instance: &mut WorkflowInstance) {
        let executed: Vec<NodeId> = instance.execution_order.iter().rev().copied().collect();

        for node_id in executed {
            if instance.compensated.contains(&node_id) {
                continue;
            }
            if let Some(node) = self.dag.node(&node_id) {
                if self.node_executor.can_compensate(node) {
                    info!("Compensating node {:?}", node_id);
                    match self.node_executor.compensate(node, &instance.context).await {
                        Ok(()) => {
                            instance.compensated.insert(node_id);
                            instance.node_states.insert(node_id, NodeState::Compensated);
                        }
                        Err(e) => {
                            error!("Compensation failed for {:?}: {}", node_id, e);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definition::{EndNodeDef, StartNodeDef};

    fn make_linear_workflow() -> WorkflowDefinition {
        let start_id = NodeId::new();
        let end_id = NodeId::new();
        WorkflowDefinition {
            id: WorkflowId::new(),
            name: "linear".into(),
            description: "linear test workflow".into(),
            version: WorkflowVersion::initial(),
            nodes: vec![
                NodeDefinition::Start(StartNodeDef {
                    node_id: start_id,
                    name: "start".into(),
                }),
                NodeDefinition::End(EndNodeDef {
                    node_id: end_id,
                    name: "end".into(),
                }),
            ],
            edges: vec![EdgeDefinition {
                id: EdgeId::new(),
                from: start_id,
                to: end_id,
                condition: None,
                label: None,
                is_critical: false,
            }],
            config: WorkflowConfig::default(),
            metadata: WorkflowMetadata::new("linear"),
            created_at: chrono::Utc::now(),
            modified_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn instance_creation() {
        let wf = make_linear_workflow();
        let ctx = WorkflowContext::new();
        let executor = WorkflowExecutor::new(wf).unwrap();
        let instance = executor.create_instance(ctx);
        assert_eq!(instance.state(), WorkflowState::Created);
        assert_eq!(instance.progress(), 0.0);
    }

    #[test]
    fn instance_progress() {
        let wf = make_linear_workflow();
        let ctx = WorkflowContext::new();
        let executor = WorkflowExecutor::new(wf).unwrap();
        let mut instance = executor.create_instance(ctx);
        let nodes: Vec<NodeId> = instance.node_states.keys().copied().collect();
        assert_eq!(nodes.len(), 2);
        instance.complete_node(nodes[0], serde_json::Value::Null);
        assert!((instance.progress() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn instance_state_transitions() {
        let wf = make_linear_workflow();
        let ctx = WorkflowContext::new();
        let executor = WorkflowExecutor::new(wf).unwrap();
        let mut instance = executor.create_instance(ctx);
        let nodes: Vec<NodeId> = instance.node_states.keys().copied().collect();
        instance.advance_state(WorkflowState::Queued).unwrap();
        instance.advance_state(WorkflowState::Running).unwrap();
        assert!(instance.started_at.is_some());
        instance.complete_node(nodes[0], serde_json::Value::Null);
        instance.complete_node(nodes[1], serde_json::Value::Null);
        instance.advance_state(WorkflowState::Completed).unwrap();
        assert!(instance.completed_at.is_some());
        assert!(instance.is_complete());
    }

    #[tokio::test]
    async fn execute_linear_workflow() {
        let wf = make_linear_workflow();
        let ctx = WorkflowContext::new();
        let executor = WorkflowExecutor::new(wf).unwrap();
        let result = executor.execute(ctx).await.unwrap();
        assert!(result.success);
        assert_eq!(result.nodes_executed, 2);
    }

    #[test]
    fn condition_evaluation() {
        let mut ctx = WorkflowContext::new();
        ctx.set_variable("x".to_string(), serde_json::json!(42));
        assert!(evaluate_condition(&Condition::Always, &ctx));
        assert!(evaluate_condition(
            &Condition::VariableEquals {
                key: "x".into(),
                value: serde_json::json!(42),
            },
            &ctx
        ));
        assert!(!evaluate_condition(
            &Condition::VariableEquals {
                key: "x".into(),
                value: serde_json::json!(99),
            },
            &ctx
        ));
        assert!(evaluate_condition(
            &Condition::VariableGreaterThan {
                key: "x".into(),
                value: 10.0,
            },
            &ctx
        ));
    }

    #[test]
    fn default_node_executor_start_end() {
        let exec = DefaultNodeExecutor;
        let ctx = WorkflowContext::new();
        let start = NodeDefinition::Start(StartNodeDef {
            node_id: NodeId::new(),
            name: "s".into(),
        });
        let end = NodeDefinition::End(EndNodeDef {
            node_id: NodeId::new(),
            name: "e".into(),
        });
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(exec.execute(&start, &ctx, serde_json::Value::Null));
        assert!(result.is_ok());
        let result = rt.block_on(exec.execute(&end, &ctx, serde_json::json!("data")));
        assert_eq!(result.unwrap(), serde_json::json!("data"));
    }
}
