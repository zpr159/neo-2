//! Workflow definition types for the Neo AGI OS workflow engine.
//!
//! This module contains the core types that describe the structure of a workflow:
//! nodes, edges, conditions, branching logic, and the top-level [`WorkflowDefinition`]
//! that ties everything together into a directed acyclic graph (DAG).

use std::collections::HashMap;
use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::{NodeId, WorkflowConfig, WorkflowId, WorkflowMetadata, WorkflowVersion};
use crate::error::{WorkflowError, WorkflowResult};

// ---------------------------------------------------------------------------
// EdgeId
// ---------------------------------------------------------------------------

/// Unique identifier for an edge in the workflow graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EdgeId(pub Uuid);

impl EdgeId {
    /// Create a new random edge identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for EdgeId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EdgeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for EdgeId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<EdgeId> for Uuid {
    fn from(id: EdgeId) -> Self {
        id.0
    }
}

// ---------------------------------------------------------------------------
// NodeKind
// ---------------------------------------------------------------------------

/// Discriminator enum for determining the type of a [`NodeDefinition`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeKind {
    Start,
    End,
    Capability,
    Decision,
    Parallel,
    Merge,
    Loop,
    Delay,
    Wait,
    HumanApproval,
    SubWorkflow,
}

// ---------------------------------------------------------------------------
// Retry Policy
// ---------------------------------------------------------------------------

/// Strategy for computing the delay between retry attempts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RetryStrategy {
    /// Fixed delay between retries.
    Fixed,
    /// Exponentially increasing delay.
    ExponentialBackoff,
    /// Linearly increasing delay.
    LinearBackoff,
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self::Fixed
    }
}

/// Configuration for retrying failed node executions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts (0 means no retries).
    pub max_attempts: u32,
    /// The backoff strategy to use.
    pub strategy: RetryStrategy,
    /// Base delay between retries in milliseconds.
    pub base_delay_ms: u64,
    /// Maximum delay cap in milliseconds.
    pub max_delay_ms: u64,
    /// Whether to add random jitter to the delay.
    pub jitter: bool,
}

impl RetryPolicy {
    /// Create a new retry policy with the given maximum attempts.
    #[must_use]
    pub fn new(max_attempts: u32) -> Self {
        Self {
            max_attempts,
            strategy: RetryStrategy::default(),
            base_delay_ms: 1_000,
            max_delay_ms: 60_000,
            jitter: true,
        }
    }

    /// Create a policy with fixed strategy and no retries.
    #[must_use]
    pub fn none() -> Self {
        Self {
            max_attempts: 0,
            strategy: RetryStrategy::Fixed,
            base_delay_ms: 0,
            max_delay_ms: 0,
            jitter: false,
        }
    }

    /// Compute the delay in milliseconds for the given attempt (0-indexed).
    #[must_use]
    pub fn delay_for_attempt(&self, attempt: u32) -> u64 {
        let raw = match self.strategy {
            RetryStrategy::Fixed => self.base_delay_ms,
            RetryStrategy::ExponentialBackoff => self.base_delay_ms.saturating_mul(1u64 << attempt),
            RetryStrategy::LinearBackoff => {
                self.base_delay_ms.saturating_mul(u64::from(attempt + 1))
            }
        };
        let capped = raw.min(self.max_delay_ms);
        if self.jitter && capped > 0 {
            let jitter_range = (capped / 4).max(1);
            let jitter_offset = (Uuid::new_v4().as_u128() % u128::from(jitter_range)) as u64;
            capped.saturating_add(jitter_offset).min(self.max_delay_ms)
        } else {
            capped
        }
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            strategy: RetryStrategy::Fixed,
            base_delay_ms: 1_000,
            max_delay_ms: 60_000,
            jitter: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Condition
// ---------------------------------------------------------------------------

/// A condition that can be evaluated to determine control flow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Condition {
    /// Always evaluates to true.
    Always,
    /// Evaluates to true when the variable with `key` equals `value`.
    VariableEquals {
        /// The variable name.
        key: String,
        /// The expected value (as JSON).
        value: serde_json::Value,
    },
    /// Evaluates to true when the numeric variable with `key` exceeds `value`.
    VariableGreaterThan {
        /// The variable name.
        key: String,
        /// The threshold value.
        value: f64,
    },
    /// Evaluates a custom expression string against runtime context.
    Expression(String),
}

// ---------------------------------------------------------------------------
// ConditionBranch
// ---------------------------------------------------------------------------

/// A single branch in a decision node, pairing a condition with a target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionBranch {
    /// The condition to evaluate.
    pub condition: Condition,
    /// The node to transition to if the condition is true.
    pub target_node_id: NodeId,
    /// Human-readable label for this branch.
    pub label: String,
}

impl ConditionBranch {
    /// Create a new condition branch.
    #[must_use]
    pub fn new(condition: Condition, target_node_id: NodeId, label: impl Into<String>) -> Self {
        Self {
            condition,
            target_node_id,
            label: label.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// BranchDef
// ---------------------------------------------------------------------------

/// Definition of a parallel branch within a parallel-split node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchDef {
    /// Unique identifier for this branch.
    pub branch_id: Uuid,
    /// Human-readable name.
    pub name: String,
    /// The node where this branch begins execution.
    pub target_node_id: NodeId,
}

impl BranchDef {
    /// Create a new branch definition.
    #[must_use]
    pub fn new(name: impl Into<String>, target_node_id: NodeId) -> Self {
        Self {
            branch_id: Uuid::new_v4(),
            name: name.into(),
            target_node_id,
        }
    }
}

// ---------------------------------------------------------------------------
// MergeStrategy
// ---------------------------------------------------------------------------

/// Strategy for merging parallel branches back together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MergeStrategy {
    /// Wait for all branches to complete.
    All,
    /// Continue when any single branch completes.
    Any,
    /// Continue when the first branch completes; discard the rest.
    First,
}

impl Default for MergeStrategy {
    fn default() -> Self {
        Self::All
    }
}

// ---------------------------------------------------------------------------
// WaitFor
// ---------------------------------------------------------------------------

/// Describes what a wait node is waiting for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WaitFor {
    /// Wait for a named event.
    Event {
        /// The event type to wait for.
        event_type: String,
    },
    /// Wait for a fixed duration.
    Duration {
        /// Duration in milliseconds.
        ms: u64,
    },
    /// Wait until a condition expression becomes true.
    Condition {
        /// The expression to evaluate.
        expression: String,
    },
}

// ===========================================================================
// Node Definitions
// ===========================================================================

/// Entry point of the workflow. Exactly one must exist per definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartNodeDef {
    /// Unique node identifier.
    pub node_id: NodeId,
    /// Human-readable name.
    pub name: String,
}

impl StartNodeDef {
    /// Create a new start node.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            node_id: NodeId::new(),
            name: name.into(),
        }
    }
}

/// Terminal point of the workflow. At least one must exist per definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndNodeDef {
    /// Unique node identifier.
    pub node_id: NodeId,
    /// Human-readable name.
    pub name: String,
}

impl EndNodeDef {
    /// Create a new end node.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            node_id: NodeId::new(),
            name: name.into(),
        }
    }
}

/// A node that invokes an external capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityNodeDef {
    /// Unique node identifier.
    pub node_id: NodeId,
    /// Human-readable name.
    pub name: String,
    /// The capability to invoke.
    pub capability_id: neo_capabilities::core::CapabilityId,
    /// Mapping from workflow variables to capability input parameters.
    pub input_mapping: HashMap<String, String>,
    /// Mapping from capability output parameters to workflow variables.
    pub output_mapping: HashMap<String, String>,
    /// Retry policy for this node.
    pub retry_policy: RetryPolicy,
    /// Timeout in milliseconds (0 means no timeout).
    pub timeout_ms: u64,
    /// Whether this node is on the critical path (affects rollback).
    pub is_critical: bool,
}

impl CapabilityNodeDef {
    /// Create a new capability node.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        capability_id: neo_capabilities::core::CapabilityId,
    ) -> Self {
        Self {
            node_id: NodeId::new(),
            name: name.into(),
            capability_id,
            input_mapping: HashMap::new(),
            output_mapping: HashMap::new(),
            retry_policy: RetryPolicy::default(),
            timeout_ms: 300_000,
            is_critical: true,
        }
    }
}

/// A node that evaluates conditions and routes execution along one of several paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionNodeDef {
    /// Unique node identifier.
    pub node_id: NodeId,
    /// Human-readable name.
    pub name: String,
    /// Ordered list of condition branches. Evaluated top-to-bottom; first match wins.
    pub conditions: Vec<ConditionBranch>,
}

impl DecisionNodeDef {
    /// Create a new decision node.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            node_id: NodeId::new(),
            name: name.into(),
            conditions: Vec::new(),
        }
    }
}

/// A node that splits execution into multiple parallel branches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelNodeDef {
    /// Unique node identifier.
    pub node_id: NodeId,
    /// Human-readable name.
    pub name: String,
    /// The branches to execute in parallel.
    pub branches: Vec<BranchDef>,
}

impl ParallelNodeDef {
    /// Create a new parallel node.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            node_id: NodeId::new(),
            name: name.into(),
            branches: Vec::new(),
        }
    }
}

/// A node that merges multiple parallel branches back into a single path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeNodeDef {
    /// Unique node identifier.
    pub node_id: NodeId,
    /// Human-readable name.
    pub name: String,
    /// Strategy for determining when the merge completes.
    pub merge_strategy: MergeStrategy,
}

impl MergeNodeDef {
    /// Create a new merge node with the default strategy.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            node_id: NodeId::new(),
            name: name.into(),
            merge_strategy: MergeStrategy::default(),
        }
    }

    /// Create a new merge node with a specific strategy.
    #[must_use]
    pub fn with_strategy(name: impl Into<String>, strategy: MergeStrategy) -> Self {
        Self {
            node_id: NodeId::new(),
            name: name.into(),
            merge_strategy: strategy,
        }
    }
}

/// A node that repeats execution of a subgraph until a condition is met.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopNodeDef {
    /// Unique node identifier.
    pub node_id: NodeId,
    /// Human-readable name.
    pub name: String,
    /// Name of the variable that holds the loop index or iterator.
    pub loop_variable: String,
    /// Maximum number of iterations (0 means unlimited).
    pub max_iterations: u32,
    /// Condition that, when true, breaks the loop.
    pub break_condition: Option<Condition>,
}

impl LoopNodeDef {
    /// Create a new loop node.
    #[must_use]
    pub fn new(name: impl Into<String>, loop_variable: impl Into<String>) -> Self {
        Self {
            node_id: NodeId::new(),
            name: name.into(),
            loop_variable: loop_variable.into(),
            max_iterations: 100,
            break_condition: None,
        }
    }
}

/// A node that pauses execution for a fixed duration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelayNodeDef {
    /// Unique node identifier.
    pub node_id: NodeId,
    /// Human-readable name.
    pub name: String,
    /// Delay duration in milliseconds.
    pub delay_ms: u64,
}

impl DelayNodeDef {
    /// Create a new delay node.
    #[must_use]
    pub fn new(name: impl Into<String>, delay_ms: u64) -> Self {
        Self {
            node_id: NodeId::new(),
            name: name.into(),
            delay_ms,
        }
    }
}

/// A node that pauses execution until an external condition is satisfied.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitNodeDef {
    /// Unique node identifier.
    pub node_id: NodeId,
    /// Human-readable name.
    pub name: String,
    /// What the node is waiting for.
    pub wait_for: WaitFor,
}

impl WaitNodeDef {
    /// Create a wait node that waits for an event.
    #[must_use]
    pub fn for_event(name: impl Into<String>, event_type: impl Into<String>) -> Self {
        Self {
            node_id: NodeId::new(),
            name: name.into(),
            wait_for: WaitFor::Event {
                event_type: event_type.into(),
            },
        }
    }

    /// Create a wait node that waits for a duration.
    #[must_use]
    pub fn for_duration(name: impl Into<String>, ms: u64) -> Self {
        Self {
            node_id: NodeId::new(),
            name: name.into(),
            wait_for: WaitFor::Duration { ms },
        }
    }

    /// Create a wait node that waits for a condition.
    #[must_use]
    pub fn for_condition(name: impl Into<String>, expression: impl Into<String>) -> Self {
        Self {
            node_id: NodeId::new(),
            name: name.into(),
            wait_for: WaitFor::Condition {
                expression: expression.into(),
            },
        }
    }
}

/// A node that pauses execution until a designated human approves.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanApprovalNodeDef {
    /// Unique node identifier.
    pub node_id: NodeId,
    /// Human-readable name.
    pub name: String,
    /// Who should approve (role, email, or user ID).
    pub assignee: String,
    /// Message shown to the approver.
    pub message: String,
    /// Timeout in milliseconds after which the approval request expires (0 = no timeout).
    pub timeout_ms: u64,
}

impl HumanApprovalNodeDef {
    /// Create a new human approval node.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        assignee: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            node_id: NodeId::new(),
            name: name.into(),
            assignee: assignee.into(),
            message: message.into(),
            timeout_ms: 86_400_000, // 24 hours
        }
    }
}

/// A node that invokes another workflow as a sub-workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubWorkflowNodeDef {
    /// Unique node identifier.
    pub node_id: NodeId,
    /// Human-readable name.
    pub name: String,
    /// The ID of the workflow to invoke.
    pub sub_workflow_id: WorkflowId,
    /// Mapping from current workflow variables to sub-workflow input variables.
    pub input_mapping: HashMap<String, String>,
}

impl SubWorkflowNodeDef {
    /// Create a new sub-workflow node.
    #[must_use]
    pub fn new(name: impl Into<String>, sub_workflow_id: WorkflowId) -> Self {
        Self {
            node_id: NodeId::new(),
            name: name.into(),
            sub_workflow_id,
            input_mapping: HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// NodeDefinition
// ---------------------------------------------------------------------------

/// A single node in the workflow graph.
///
/// Each variant wraps a type-specific definition struct. Use [`NodeDefinition::kind`]
/// to get the [`NodeKind`] discriminator without borrowing the inner data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeDefinition {
    /// Entry point of the workflow.
    Start(StartNodeDef),
    /// Terminal point of the workflow.
    End(EndNodeDef),
    /// Invokes an external capability.
    Capability(CapabilityNodeDef),
    /// Evaluates conditions to route execution.
    Decision(DecisionNodeDef),
    /// Splits into parallel branches.
    Parallel(ParallelNodeDef),
    /// Merges parallel branches back together.
    Merge(MergeNodeDef),
    /// Repeats a subgraph.
    Loop(LoopNodeDef),
    /// Pauses for a fixed duration.
    Delay(DelayNodeDef),
    /// Pauses until an external condition.
    Wait(WaitNodeDef),
    /// Pauses until human approval.
    HumanApproval(HumanApprovalNodeDef),
    /// Invokes a sub-workflow.
    SubWorkflow(SubWorkflowNodeDef),
}

impl NodeDefinition {
    /// Returns the [`NodeKind`] discriminator for this variant.
    #[must_use]
    pub fn kind(&self) -> NodeKind {
        match self {
            Self::Start(_) => NodeKind::Start,
            Self::End(_) => NodeKind::End,
            Self::Capability(_) => NodeKind::Capability,
            Self::Decision(_) => NodeKind::Decision,
            Self::Parallel(_) => NodeKind::Parallel,
            Self::Merge(_) => NodeKind::Merge,
            Self::Loop(_) => NodeKind::Loop,
            Self::Delay(_) => NodeKind::Delay,
            Self::Wait(_) => NodeKind::Wait,
            Self::HumanApproval(_) => NodeKind::HumanApproval,
            Self::SubWorkflow(_) => NodeKind::SubWorkflow,
        }
    }

    /// Returns the [`NodeId`] of this node, regardless of variant.
    #[must_use]
    pub fn node_id(&self) -> NodeId {
        match self {
            Self::Start(n) => n.node_id,
            Self::End(n) => n.node_id,
            Self::Capability(n) => n.node_id,
            Self::Decision(n) => n.node_id,
            Self::Parallel(n) => n.node_id,
            Self::Merge(n) => n.node_id,
            Self::Loop(n) => n.node_id,
            Self::Delay(n) => n.node_id,
            Self::Wait(n) => n.node_id,
            Self::HumanApproval(n) => n.node_id,
            Self::SubWorkflow(n) => n.node_id,
        }
    }

    /// Returns the human-readable name of this node, regardless of variant.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Start(n) => &n.name,
            Self::End(n) => &n.name,
            Self::Capability(n) => &n.name,
            Self::Decision(n) => &n.name,
            Self::Parallel(n) => &n.name,
            Self::Merge(n) => &n.name,
            Self::Loop(n) => &n.name,
            Self::Delay(n) => &n.name,
            Self::Wait(n) => &n.name,
            Self::HumanApproval(n) => &n.name,
            Self::SubWorkflow(n) => &n.name,
        }
    }
}

// ---------------------------------------------------------------------------
// EdgeDefinition
// ---------------------------------------------------------------------------

/// A directed edge connecting two nodes in the workflow graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeDefinition {
    /// Unique edge identifier.
    pub id: EdgeId,
    /// Source node ID.
    pub from: NodeId,
    /// Destination node ID.
    pub to: NodeId,
    /// Optional condition that must be true for this edge to be traversed.
    pub condition: Option<Condition>,
    /// Optional label for display / documentation purposes.
    pub label: Option<String>,
    /// If true, this edge is part of the critical path and affects rollback behavior.
    pub is_critical: bool,
}

impl EdgeDefinition {
    /// Create a new unconditional edge.
    #[must_use]
    pub fn new(from: NodeId, to: NodeId) -> Self {
        Self {
            id: EdgeId::new(),
            from,
            to,
            condition: None,
            label: None,
            is_critical: false,
        }
    }

    /// Create a new edge with a condition.
    #[must_use]
    pub fn conditional(from: NodeId, to: NodeId, condition: Condition) -> Self {
        Self {
            id: EdgeId::new(),
            from,
            to,
            condition: Some(condition),
            label: None,
            is_critical: false,
        }
    }

    /// Set the label on this edge (builder pattern).
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Mark this edge as critical (builder pattern).
    #[must_use]
    pub fn critical(mut self) -> Self {
        self.is_critical = true;
        self
    }
}

// ===========================================================================
// WorkflowDefinition
// ===========================================================================

/// The complete definition of a workflow, comprising nodes, edges, configuration,
/// and metadata.
///
/// Use [`WorkflowDefinition::new`] to create a minimal definition, then populate it
/// with [`add_node`] and [`add_edge`] before calling [`validate`] to check structural
/// integrity.
///
/// [`add_node`]: WorkflowDefinition::add_node
/// [`add_edge`]: WorkflowDefinition::add_edge
/// [`validate`]: WorkflowDefinition::validate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    /// Unique workflow identifier.
    pub id: WorkflowId,
    /// Human-readable workflow name.
    pub name: String,
    /// Description of what this workflow does.
    pub description: String,
    /// Semantic version of this definition.
    pub version: WorkflowVersion,
    /// All nodes in the workflow graph.
    pub nodes: Vec<NodeDefinition>,
    /// All edges (transitions) between nodes.
    pub edges: Vec<EdgeDefinition>,
    /// Execution configuration.
    pub config: WorkflowConfig,
    /// Metadata about the workflow.
    pub metadata: WorkflowMetadata,
    /// When this definition was created.
    pub created_at: DateTime<Utc>,
    /// When this definition was last modified.
    pub modified_at: DateTime<Utc>,
}

impl WorkflowDefinition {
    /// Create a new workflow definition with the given name and default settings.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        let name_owned = name.into();
        let mut metadata = WorkflowMetadata::new(name_owned.clone());
        metadata.created_at = now;
        metadata.modified_at = now;
        Self {
            id: WorkflowId::new(),
            name: name_owned,
            description: String::new(),
            version: WorkflowVersion::initial(),
            nodes: Vec::new(),
            edges: Vec::new(),
            config: WorkflowConfig::default(),
            metadata,
            created_at: now,
            modified_at: now,
        }
    }

    /// Add a node to the workflow definition. Returns the node's [`NodeId`].
    pub fn add_node(&mut self, node: NodeDefinition) -> NodeId {
        let id = node.node_id();
        self.nodes.push(node);
        self.modified_at = Utc::now();
        id
    }

    /// Add an edge to the workflow definition. Returns the edge's [`EdgeId`].
    pub fn add_edge(&mut self, edge: EdgeDefinition) -> EdgeId {
        let id = edge.id;
        self.edges.push(edge);
        self.modified_at = Utc::now();
        id
    }

    /// Returns the number of nodes in the workflow.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns the number of edges in the workflow.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Find a node by its [`NodeId`]. Returns `None` if not found.
    #[must_use]
    pub fn find_node(&self, node_id: NodeId) -> Option<&NodeDefinition> {
        self.nodes.iter().find(|n| n.node_id() == node_id)
    }

    /// Returns all nodes with the given [`NodeKind`].
    #[must_use]
    pub fn get_nodes_by_type(&self, kind: NodeKind) -> Vec<&NodeDefinition> {
        self.nodes.iter().filter(|n| n.kind() == kind).collect()
    }

    /// Returns references to all start nodes.
    #[must_use]
    pub fn get_start_nodes(&self) -> Vec<&NodeDefinition> {
        self.get_nodes_by_type(NodeKind::Start)
    }

    /// Returns references to all end nodes.
    #[must_use]
    pub fn get_end_nodes(&self) -> Vec<&NodeDefinition> {
        self.get_nodes_by_type(NodeKind::End)
    }

    /// Validate the structural integrity of the workflow definition.
    ///
    /// Checks:
    /// - At least one start node and one end node exist.
    /// - No duplicate node IDs.
    /// - All edges reference existing node IDs.
    /// - Start nodes have no incoming edges.
    /// - End nodes have no outgoing edges.
    pub fn validate(&self) -> WorkflowResult<()> {
        let mut errors: Vec<String> = Vec::new();

        // Must have at least one start node.
        let start_nodes = self.get_start_nodes();
        if start_nodes.is_empty() {
            errors.push("workflow must have at least one start node".to_string());
        }

        // Must have at least one end node.
        let end_nodes = self.get_end_nodes();
        if end_nodes.is_empty() {
            errors.push("workflow must have at least one end node".to_string());
        }

        // No duplicate node IDs.
        {
            let mut seen = HashMap::new();
            for node in &self.nodes {
                let id = node.node_id();
                if let Some(prev) = seen.insert(id, node.name()) {
                    errors.push(format!(
                        "duplicate node ID {} (found in \"{}\" and \"{}\")",
                        id,
                        prev,
                        node.name()
                    ));
                }
            }
        }

        // Build a set of valid node IDs for edge validation.
        let valid_ids: HashMap<NodeId, &NodeDefinition> =
            self.nodes.iter().map(|n| (n.node_id(), n)).collect();

        // Collect start and end node IDs for edge checks.
        let start_ids: std::collections::HashSet<NodeId> =
            start_nodes.iter().map(|n| n.node_id()).collect();
        let end_ids: std::collections::HashSet<NodeId> =
            end_nodes.iter().map(|n| n.node_id()).collect();

        // Validate all edges.
        for edge in &self.edges {
            // Edge must reference existing nodes.
            if !valid_ids.contains_key(&edge.from) {
                errors.push(format!(
                    "edge {} references non-existent source node {}",
                    edge.id, edge.from
                ));
            }
            if !valid_ids.contains_key(&edge.to) {
                errors.push(format!(
                    "edge {} references non-existent destination node {}",
                    edge.id, edge.to
                ));
            }

            // Start nodes must have no incoming edges.
            if edge.to != edge.from && start_ids.contains(&edge.to) {
                errors.push(format!(
                    "start node {} has an incoming edge from {}",
                    edge.to, edge.from
                ));
            }

            // End nodes must have no outgoing edges.
            if edge.to != edge.from && end_ids.contains(&edge.from) {
                errors.push(format!(
                    "end node {} has an outgoing edge to {}",
                    edge.from, edge.to
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(WorkflowError::invalid_definition(errors.join("; ")))
        }
    }
}

// ===========================================================================
// Display impls
// ===========================================================================

impl fmt::Display for NodeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Start => write!(f, "Start"),
            Self::End => write!(f, "End"),
            Self::Capability => write!(f, "Capability"),
            Self::Decision => write!(f, "Decision"),
            Self::Parallel => write!(f, "Parallel"),
            Self::Merge => write!(f, "Merge"),
            Self::Loop => write!(f, "Loop"),
            Self::Delay => write!(f, "Delay"),
            Self::Wait => write!(f, "Wait"),
            Self::HumanApproval => write!(f, "HumanApproval"),
            Self::SubWorkflow => write!(f, "SubWorkflow"),
        }
    }
}

impl fmt::Display for MergeStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::All => write!(f, "All"),
            Self::Any => write!(f, "Any"),
            Self::First => write!(f, "First"),
        }
    }
}

impl fmt::Display for RetryStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fixed => write!(f, "Fixed"),
            Self::ExponentialBackoff => write!(f, "ExponentialBackoff"),
            Self::LinearBackoff => write!(f, "LinearBackoff"),
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // EdgeId
    // -----------------------------------------------------------------------

    #[test]
    fn edge_id_new_is_unique() {
        let a = EdgeId::new();
        let b = EdgeId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn edge_id_default_is_unique() {
        let a = EdgeId::default();
        let b = EdgeId::default();
        assert_ne!(a, b);
    }

    #[test]
    fn edge_id_display() {
        let id = EdgeId(Uuid::nil());
        assert_eq!(id.to_string(), "00000000-0000-0000-0000-000000000000");
    }

    #[test]
    fn edge_id_roundtrip_uuid() {
        let id = EdgeId::new();
        let uuid: Uuid = id.into();
        let id2: EdgeId = uuid.into();
        assert_eq!(id, id2);
    }

    // -----------------------------------------------------------------------
    // RetryPolicy
    // -----------------------------------------------------------------------

    #[test]
    fn retry_policy_none() {
        let p = RetryPolicy::none();
        assert_eq!(p.max_attempts, 0);
        assert_eq!(p.base_delay_ms, 0);
    }

    #[test]
    fn retry_policy_default() {
        let p = RetryPolicy::default();
        assert_eq!(p.max_attempts, 3);
        assert!(p.jitter);
    }

    #[test]
    fn retry_policy_fixed_delay() {
        let p = RetryPolicy {
            max_attempts: 3,
            strategy: RetryStrategy::Fixed,
            base_delay_ms: 1_000,
            max_delay_ms: 10_000,
            jitter: false,
        };
        assert_eq!(p.delay_for_attempt(0), 1_000);
        assert_eq!(p.delay_for_attempt(5), 1_000);
    }

    #[test]
    fn retry_policy_exponential_backoff() {
        let p = RetryPolicy {
            max_attempts: 5,
            strategy: RetryStrategy::ExponentialBackoff,
            base_delay_ms: 100,
            max_delay_ms: 5_000,
            jitter: false,
        };
        assert_eq!(p.delay_for_attempt(0), 100);
        assert_eq!(p.delay_for_attempt(1), 200);
        assert_eq!(p.delay_for_attempt(2), 400);
        assert_eq!(p.delay_for_attempt(3), 800);
    }

    #[test]
    fn retry_policy_exponential_capped() {
        let p = RetryPolicy {
            max_attempts: 10,
            strategy: RetryStrategy::ExponentialBackoff,
            base_delay_ms: 1_000,
            max_delay_ms: 5_000,
            jitter: false,
        };
        // 1000 * 2^3 = 8000, capped to 5000
        assert_eq!(p.delay_for_attempt(3), 5_000);
    }

    #[test]
    fn retry_policy_linear_backoff() {
        let p = RetryPolicy {
            max_attempts: 5,
            strategy: RetryStrategy::LinearBackoff,
            base_delay_ms: 100,
            max_delay_ms: 10_000,
            jitter: false,
        };
        assert_eq!(p.delay_for_attempt(0), 100);
        assert_eq!(p.delay_for_attempt(1), 200);
        assert_eq!(p.delay_for_attempt(2), 300);
    }

    #[test]
    fn retry_policy_clone() {
        let p = RetryPolicy::default();
        let p2 = p.clone();
        assert_eq!(p.max_attempts, p2.max_attempts);
        assert_eq!(p.strategy, p2.strategy);
    }

    // -----------------------------------------------------------------------
    // Condition
    // -----------------------------------------------------------------------

    #[test]
    fn condition_always() {
        let c = Condition::Always;
        assert_eq!(c, Condition::Always);
    }

    #[test]
    fn condition_variable_equals() {
        let c = Condition::VariableEquals {
            key: "status".to_string(),
            value: serde_json::json!("approved"),
        };
        match &c {
            Condition::VariableEquals { key, value } => {
                assert_eq!(key, "status");
                assert_eq!(value, &serde_json::json!("approved"));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn condition_variable_greater_than() {
        let c = Condition::VariableGreaterThan {
            key: "score".to_string(),
            value: 90.0,
        };
        match &c {
            Condition::VariableGreaterThan { key, value } => {
                assert_eq!(key, "score");
                assert_eq!(*value, 90.0);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn condition_expression() {
        let c = Condition::Expression("ctx.get('x') > 10".to_string());
        match &c {
            Condition::Expression(e) => assert_eq!(e, "ctx.get('x') > 10"),
            _ => panic!("wrong variant"),
        }
    }

    // -----------------------------------------------------------------------
    // ConditionBranch
    // -----------------------------------------------------------------------

    #[test]
    fn condition_branch_construction() {
        let target = NodeId::new();
        let cb = ConditionBranch::new(Condition::Always, target, "default");
        assert_eq!(cb.target_node_id, target);
        assert_eq!(cb.label, "default");
        assert_eq!(cb.condition, Condition::Always);
    }

    // -----------------------------------------------------------------------
    // BranchDef
    // -----------------------------------------------------------------------

    #[test]
    fn branch_def_construction() {
        let target = NodeId::new();
        let bd = BranchDef::new("branch-a", target);
        assert_eq!(bd.name, "branch-a");
        assert_eq!(bd.target_node_id, target);
    }

    // -----------------------------------------------------------------------
    // MergeStrategy
    // -----------------------------------------------------------------------

    #[test]
    fn merge_strategy_default_is_all() {
        assert_eq!(MergeStrategy::default(), MergeStrategy::All);
    }

    #[test]
    fn merge_strategy_display() {
        assert_eq!(MergeStrategy::All.to_string(), "All");
        assert_eq!(MergeStrategy::Any.to_string(), "Any");
        assert_eq!(MergeStrategy::First.to_string(), "First");
    }

    // -----------------------------------------------------------------------
    // NodeDefinition
    // -----------------------------------------------------------------------

    #[test]
    fn node_start() {
        let n = StartNodeDef::new("begin");
        let def = NodeDefinition::Start(n);
        assert_eq!(def.kind(), NodeKind::Start);
        assert_eq!(def.name(), "begin");
    }

    #[test]
    fn node_end() {
        let n = EndNodeDef::new("finish");
        let def = NodeDefinition::End(n);
        assert_eq!(def.kind(), NodeKind::End);
        assert_eq!(def.name(), "finish");
    }

    #[test]
    fn node_capability() {
        let cap_id = neo_capabilities::core::CapabilityId::new();
        let mut n = CapabilityNodeDef::new("call-api", cap_id);
        n.input_mapping
            .insert("url".to_string(), "input.url".to_string());
        n.output_mapping
            .insert("output.result".to_string(), "result".to_string());
        assert_eq!(n.input_mapping.len(), 1);
        assert_eq!(n.output_mapping.len(), 1);
        assert!(n.is_critical);
    }

    #[test]
    fn node_decision() {
        let target = NodeId::new();
        let mut n = DecisionNodeDef::new("branch");
        n.conditions
            .push(ConditionBranch::new(Condition::Always, target, "yes"));
        assert_eq!(n.conditions.len(), 1);
        let def = NodeDefinition::Decision(n);
        assert_eq!(def.kind(), NodeKind::Decision);
    }

    #[test]
    fn node_parallel() {
        let target = NodeId::new();
        let mut n = ParallelNodeDef::new("fan-out");
        n.branches.push(BranchDef::new("b1", target));
        assert_eq!(n.branches.len(), 1);
        let def = NodeDefinition::Parallel(n);
        assert_eq!(def.kind(), NodeKind::Parallel);
    }

    #[test]
    fn node_merge() {
        let n = MergeNodeDef::with_strategy("fan-in", MergeStrategy::Any);
        assert_eq!(n.merge_strategy, MergeStrategy::Any);
        let def = NodeDefinition::Merge(n);
        assert_eq!(def.kind(), NodeKind::Merge);
    }

    #[test]
    fn node_loop() {
        let n = LoopNodeDef::new("iterate", "i");
        assert_eq!(n.loop_variable, "i");
        assert_eq!(n.max_iterations, 100);
        assert!(n.break_condition.is_none());
        let def = NodeDefinition::Loop(n);
        assert_eq!(def.kind(), NodeKind::Loop);
    }

    #[test]
    fn node_delay() {
        let n = DelayNodeDef::new("wait-5s", 5_000);
        assert_eq!(n.delay_ms, 5_000);
        let def = NodeDefinition::Delay(n);
        assert_eq!(def.kind(), NodeKind::Delay);
    }

    #[test]
    fn node_wait_event() {
        let n = WaitNodeDef::for_event("wait-deploy", "deploy.complete");
        assert_eq!(
            n.wait_for,
            WaitFor::Event {
                event_type: "deploy.complete".to_string()
            }
        );
    }

    #[test]
    fn node_wait_duration() {
        let n = WaitNodeDef::for_duration("cool-down", 10_000);
        assert_eq!(n.wait_for, WaitFor::Duration { ms: 10_000 });
    }

    #[test]
    fn node_wait_condition() {
        let n = WaitNodeDef::for_condition("wait-ready", "service.status == 'ready'");
        assert_eq!(
            n.wait_for,
            WaitFor::Condition {
                expression: "service.status == 'ready'".to_string()
            }
        );
    }

    #[test]
    fn node_human_approval() {
        let n =
            HumanApprovalNodeDef::new("approve-spend", "cfo@corp.com", "Please approve $50k spend");
        assert_eq!(n.assignee, "cfo@corp.com");
        assert_eq!(n.message, "Please approve $50k spend");
        assert_eq!(n.timeout_ms, 86_400_000);
    }

    #[test]
    fn node_sub_workflow() {
        let sub_id = WorkflowId::new();
        let mut n = SubWorkflowNodeDef::new("run-onboarding", sub_id);
        n.input_mapping
            .insert("user_id".to_string(), "input.user".to_string());
        assert_eq!(n.sub_workflow_id, sub_id);
        assert_eq!(n.input_mapping.len(), 1);
    }

    #[test]
    fn node_definition_node_id_is_unique_per_instance() {
        let a = NodeDefinition::Start(StartNodeDef::new("a"));
        let b = NodeDefinition::Start(StartNodeDef::new("b"));
        assert_ne!(a.node_id(), b.node_id());
    }

    #[test]
    fn node_definition_all_kinds() {
        let cap_id = neo_capabilities::core::CapabilityId::new();
        let sub_id = WorkflowId::new();
        let nodes = vec![
            NodeDefinition::Start(StartNodeDef::new("s")),
            NodeDefinition::End(EndNodeDef::new("e")),
            NodeDefinition::Capability(CapabilityNodeDef::new("c", cap_id)),
            NodeDefinition::Decision(DecisionNodeDef::new("d")),
            NodeDefinition::Parallel(ParallelNodeDef::new("p")),
            NodeDefinition::Merge(MergeNodeDef::new("m")),
            NodeDefinition::Loop(LoopNodeDef::new("l", "i")),
            NodeDefinition::Delay(DelayNodeDef::new("del", 1000)),
            NodeDefinition::Wait(WaitNodeDef::for_event("w", "evt")),
            NodeDefinition::HumanApproval(HumanApprovalNodeDef::new("ha", "a@b.c", "ok?")),
            NodeDefinition::SubWorkflow(SubWorkflowNodeDef::new("sw", sub_id)),
        ];
        assert_eq!(nodes.len(), 11);
        let kinds: Vec<NodeKind> = nodes.iter().map(|n| n.kind()).collect();
        assert_eq!(
            kinds,
            vec![
                NodeKind::Start,
                NodeKind::End,
                NodeKind::Capability,
                NodeKind::Decision,
                NodeKind::Parallel,
                NodeKind::Merge,
                NodeKind::Loop,
                NodeKind::Delay,
                NodeKind::Wait,
                NodeKind::HumanApproval,
                NodeKind::SubWorkflow,
            ]
        );
    }

    // -----------------------------------------------------------------------
    // EdgeDefinition
    // -----------------------------------------------------------------------

    #[test]
    fn edge_new_unconditional() {
        let from = NodeId::new();
        let to = NodeId::new();
        let e = EdgeDefinition::new(from, to);
        assert_eq!(e.from, from);
        assert_eq!(e.to, to);
        assert!(e.condition.is_none());
        assert!(e.label.is_none());
        assert!(!e.is_critical);
    }

    #[test]
    fn edge_conditional() {
        let from = NodeId::new();
        let to = NodeId::new();
        let e = EdgeDefinition::conditional(from, to, Condition::Always);
        assert_eq!(e.condition, Some(Condition::Always));
    }

    #[test]
    fn edge_builder_pattern() {
        let from = NodeId::new();
        let to = NodeId::new();
        let e = EdgeDefinition::new(from, to)
            .with_label("on success")
            .critical();
        assert_eq!(e.label.as_deref(), Some("on success"));
        assert!(e.is_critical);
    }

    // -----------------------------------------------------------------------
    // WorkflowDefinition - Construction
    // -----------------------------------------------------------------------

    #[test]
    fn wf_def_new() {
        let wf = WorkflowDefinition::new("test-workflow");
        assert_eq!(wf.name, "test-workflow");
        assert_eq!(wf.nodes.len(), 0);
        assert_eq!(wf.edges.len(), 0);
        assert_eq!(wf.version, WorkflowVersion::initial());
    }

    #[test]
    fn wf_def_add_node() {
        let mut wf = WorkflowDefinition::new("test");
        let id = wf.add_node(NodeDefinition::Start(StartNodeDef::new("s")));
        assert_eq!(wf.node_count(), 1);
        assert!(wf.find_node(id).is_some());
    }

    #[test]
    fn wf_def_add_edge() {
        let mut wf = WorkflowDefinition::new("test");
        let from = wf.add_node(NodeDefinition::Start(StartNodeDef::new("s")));
        let to = wf.add_node(NodeDefinition::End(EndNodeDef::new("e")));
        let edge_id = wf.add_edge(EdgeDefinition::new(from, to));
        assert_eq!(wf.edge_count(), 1);
        assert_eq!(wf.edges[0].id, edge_id);
    }

    #[test]
    fn wf_def_find_node_not_found() {
        let wf = WorkflowDefinition::new("test");
        assert!(wf.find_node(NodeId::new()).is_none());
    }

    #[test]
    fn wf_def_get_nodes_by_type() {
        let mut wf = WorkflowDefinition::new("test");
        wf.add_node(NodeDefinition::Start(StartNodeDef::new("s1")));
        wf.add_node(NodeDefinition::Start(StartNodeDef::new("s2")));
        wf.add_node(NodeDefinition::End(EndNodeDef::new("e1")));
        let starts = wf.get_start_nodes();
        assert_eq!(starts.len(), 2);
        let ends = wf.get_end_nodes();
        assert_eq!(ends.len(), 1);
    }

    #[test]
    fn wf_def_modified_at_updates() {
        let mut wf = WorkflowDefinition::new("test");
        let initial_modified = wf.modified_at;
        // Small delay to ensure timestamp changes.
        std::thread::sleep(std::time::Duration::from_millis(10));
        wf.add_node(NodeDefinition::Start(StartNodeDef::new("s")));
        assert!(wf.modified_at >= initial_modified);
    }

    // -----------------------------------------------------------------------
    // WorkflowDefinition - Validation
    // -----------------------------------------------------------------------

    #[test]
    fn validate_valid_simple_workflow() {
        let mut wf = WorkflowDefinition::new("test");
        let s = wf.add_node(NodeDefinition::Start(StartNodeDef::new("start")));
        let e = wf.add_node(NodeDefinition::End(EndNodeDef::new("end")));
        wf.add_edge(EdgeDefinition::new(s, e));
        assert!(wf.validate().is_ok());
    }

    #[test]
    fn validate_no_start_node() {
        let mut wf = WorkflowDefinition::new("test");
        let e = wf.add_node(NodeDefinition::End(EndNodeDef::new("end")));
        let _ = e;
        let err = wf.validate().unwrap_err();
        assert!(err.to_string().contains("start node"));
    }

    #[test]
    fn validate_no_end_node() {
        let mut wf = WorkflowDefinition::new("test");
        let s = wf.add_node(NodeDefinition::Start(StartNodeDef::new("start")));
        let _ = s;
        let err = wf.validate().unwrap_err();
        assert!(err.to_string().contains("end node"));
    }

    #[test]
    fn validate_no_nodes() {
        let wf = WorkflowDefinition::new("test");
        let err = wf.validate().unwrap_err();
        assert!(err.to_string().contains("start node"));
        assert!(err.to_string().contains("end node"));
    }

    #[test]
    fn validate_duplicate_node_ids() {
        let mut wf = WorkflowDefinition::new("test");
        let shared_id = NodeId::new();
        wf.nodes.push(NodeDefinition::Start(StartNodeDef {
            node_id: shared_id,
            name: "first".to_string(),
        }));
        wf.nodes.push(NodeDefinition::End(EndNodeDef {
            node_id: shared_id,
            name: "second".to_string(),
        }));
        let err = wf.validate().unwrap_err();
        assert!(err.to_string().contains("duplicate node ID"));
    }

    #[test]
    fn validate_edge_references_nonexistent_source() {
        let mut wf = WorkflowDefinition::new("test");
        let real = wf.add_node(NodeDefinition::End(EndNodeDef::new("e")));
        let ghost = NodeId::new();
        wf.add_edge(EdgeDefinition::new(ghost, real));
        let err = wf.validate().unwrap_err();
        assert!(err.to_string().contains("non-existent source"));
    }

    #[test]
    fn validate_edge_references_nonexistent_destination() {
        let mut wf = WorkflowDefinition::new("test");
        let real = wf.add_node(NodeDefinition::Start(StartNodeDef::new("s")));
        let ghost = NodeId::new();
        wf.add_edge(EdgeDefinition::new(real, ghost));
        let err = wf.validate().unwrap_err();
        assert!(err.to_string().contains("non-existent destination"));
    }

    #[test]
    fn validate_start_node_has_incoming_edge() {
        let mut wf = WorkflowDefinition::new("test");
        let s = wf.add_node(NodeDefinition::Start(StartNodeDef::new("s")));
        let e = wf.add_node(NodeDefinition::End(EndNodeDef::new("e")));
        // Edge from end back to start (invalid).
        wf.add_edge(EdgeDefinition::new(e, s));
        // Also add valid edge s->e.
        wf.add_edge(EdgeDefinition::new(s, e));
        let err = wf.validate().unwrap_err();
        assert!(err.to_string().contains("incoming edge"));
    }

    #[test]
    fn validate_end_node_has_outgoing_edge() {
        let mut wf = WorkflowDefinition::new("test");
        let s = wf.add_node(NodeDefinition::Start(StartNodeDef::new("s")));
        let e = wf.add_node(NodeDefinition::End(EndNodeDef::new("e")));
        let e2 = wf.add_node(NodeDefinition::End(EndNodeDef::new("e2")));
        // Valid: s -> e
        wf.add_edge(EdgeDefinition::new(s, e));
        // Invalid: e -> e2 (end node has outgoing edge)
        wf.add_edge(EdgeDefinition::new(e, e2));
        let err = wf.validate().unwrap_err();
        assert!(err.to_string().contains("outgoing edge"));
    }

    #[test]
    fn validate_empty_self_loop_on_non_start_end_is_ok() {
        // A self-loop on a non-start, non-end node is structurally valid per our checks.
        let mut wf = WorkflowDefinition::new("test");
        let s = wf.add_node(NodeDefinition::Start(StartNodeDef::new("s")));
        let mid = wf.add_node(NodeDefinition::Delay(DelayNodeDef::new("delay", 1000)));
        let e = wf.add_node(NodeDefinition::End(EndNodeDef::new("e")));
        wf.add_edge(EdgeDefinition::new(s, mid));
        wf.add_edge(EdgeDefinition::new(mid, mid)); // self-loop
        wf.add_edge(EdgeDefinition::new(mid, e));
        assert!(wf.validate().is_ok());
    }

    #[test]
    fn validate_multiple_errors_aggregated() {
        let wf = WorkflowDefinition::new("test");
        // No start, no end -> two errors.
        let err = wf.validate().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("start node"));
        assert!(msg.contains("end node"));
    }

    #[test]
    fn validate_complex_valid_workflow() {
        let mut wf = WorkflowDefinition::new("complex");

        let s = wf.add_node(NodeDefinition::Start(StartNodeDef::new("start")));
        let cap_id = neo_capabilities::core::CapabilityId::new();
        let cap = wf.add_node(NodeDefinition::Capability(CapabilityNodeDef::new(
            "cap", cap_id,
        )));
        let decision = wf.add_node(NodeDefinition::Decision(DecisionNodeDef::new("dec")));
        let end_ok = wf.add_node(NodeDefinition::End(EndNodeDef::new("end-ok")));
        let end_fail = wf.add_node(NodeDefinition::End(EndNodeDef::new("end-fail")));

        wf.add_edge(EdgeDefinition::new(s, cap));
        wf.add_edge(EdgeDefinition::new(cap, decision));
        wf.add_edge(EdgeDefinition::conditional(
            decision,
            end_ok,
            Condition::Always,
        ));
        wf.add_edge(EdgeDefinition::conditional(
            decision,
            end_fail,
            Condition::Always,
        ));

        assert!(wf.validate().is_ok());
        assert_eq!(wf.node_count(), 5);
        assert_eq!(wf.edge_count(), 4);
    }

    // -----------------------------------------------------------------------
    // Serialization roundtrips
    // -----------------------------------------------------------------------

    #[test]
    fn condition_serde_roundtrip() {
        let c = Condition::VariableEquals {
            key: "x".to_string(),
            value: serde_json::json!(42),
        };
        let json = serde_json::to_string(&c).unwrap();
        let parsed: Condition = serde_json::from_str(&json).unwrap();
        assert_eq!(c, parsed);
    }

    #[test]
    fn node_definition_serde_roundtrip() {
        let cap_id = neo_capabilities::core::CapabilityId::new();
        let original = NodeDefinition::Capability(CapabilityNodeDef::new("test-cap", cap_id));
        let json = serde_json::to_string(&original).unwrap();
        let parsed: NodeDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(original.kind(), parsed.kind());
        assert_eq!(original.name(), parsed.name());
    }

    #[test]
    fn edge_definition_serde_roundtrip() {
        let from = NodeId::new();
        let to = NodeId::new();
        let original = EdgeDefinition::new(from, to).with_label("test").critical();
        let json = serde_json::to_string(&original).unwrap();
        let parsed: EdgeDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.from, from);
        assert_eq!(parsed.to, to);
        assert_eq!(parsed.label.as_deref(), Some("test"));
        assert!(parsed.is_critical);
    }

    #[test]
    fn workflow_definition_serde_roundtrip() {
        let mut wf = WorkflowDefinition::new("roundtrip");
        wf.description = "test description".to_string();
        let s = wf.add_node(NodeDefinition::Start(StartNodeDef::new("start")));
        let e = wf.add_node(NodeDefinition::End(EndNodeDef::new("end")));
        wf.add_edge(EdgeDefinition::new(s, e));

        let json = serde_json::to_string(&wf).unwrap();
        let parsed: WorkflowDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "roundtrip");
        assert_eq!(parsed.description, "test description");
        assert_eq!(parsed.node_count(), 2);
        assert_eq!(parsed.edge_count(), 1);
    }

    #[test]
    fn retry_policy_serde_roundtrip() {
        let original = RetryPolicy::default();
        let json = serde_json::to_string(&original).unwrap();
        let parsed: RetryPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.max_attempts, original.max_attempts);
        assert_eq!(parsed.strategy, original.strategy);
    }

    // -----------------------------------------------------------------------
    // NodeKind
    // -----------------------------------------------------------------------

    #[test]
    fn node_kind_display() {
        assert_eq!(NodeKind::Start.to_string(), "Start");
        assert_eq!(NodeKind::End.to_string(), "End");
        assert_eq!(NodeKind::Capability.to_string(), "Capability");
        assert_eq!(NodeKind::Decision.to_string(), "Decision");
        assert_eq!(NodeKind::Parallel.to_string(), "Parallel");
        assert_eq!(NodeKind::Merge.to_string(), "Merge");
        assert_eq!(NodeKind::Loop.to_string(), "Loop");
        assert_eq!(NodeKind::Delay.to_string(), "Delay");
        assert_eq!(NodeKind::Wait.to_string(), "Wait");
        assert_eq!(NodeKind::HumanApproval.to_string(), "HumanApproval");
        assert_eq!(NodeKind::SubWorkflow.to_string(), "SubWorkflow");
    }

    #[test]
    fn node_kind_eq_hash() {
        let mut seen = std::collections::HashSet::new();
        let all = [
            NodeKind::Start,
            NodeKind::End,
            NodeKind::Capability,
            NodeKind::Decision,
            NodeKind::Parallel,
            NodeKind::Merge,
            NodeKind::Loop,
            NodeKind::Delay,
            NodeKind::Wait,
            NodeKind::HumanApproval,
            NodeKind::SubWorkflow,
        ];
        for kind in &all {
            assert!(seen.insert(*kind), "duplicate NodeKind: {}", kind);
        }
    }

    // -----------------------------------------------------------------------
    // Default impls
    // -----------------------------------------------------------------------

    #[test]
    fn default_impls() {
        let _ = EdgeId::default();
        let _ = RetryPolicy::default();
        let _ = RetryStrategy::default();
        let _ = MergeStrategy::default();
    }
}
