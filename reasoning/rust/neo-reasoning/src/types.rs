use std::collections::HashMap;
use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ReasoningResult;
pub use crate::strategy::ReasoningStrategy;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SessionState {
    Created,
    Planning,
    Reasoning,
    Reflecting,
    Deciding,
    Verifying,
    Completed,
    Failed,
    Cancelled,
}

impl SessionState {
    pub fn can_transition_to(self, target: SessionState) -> bool {
        matches!(
            (self, target),
            (Self::Created, Self::Planning)
                | (Self::Planning, Self::Reasoning)
                | (Self::Reasoning, Self::Reflecting)
                | (Self::Reasoning, Self::Deciding)
                | (Self::Reasoning, Self::Verifying)
                | (Self::Reflecting, Self::Reasoning)
                | (Self::Reflecting, Self::Deciding)
                | (Self::Reflecting, Self::Completed)
                | (Self::Deciding, Self::Verifying)
                | (Self::Deciding, Self::Completed)
                | (Self::Verifying, Self::Completed)
                | (Self::Verifying, Self::Reasoning)
                | (_, Self::Failed)
                | (_, Self::Cancelled)
        )
    }
}

impl fmt::Display for SessionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::Planning => write!(f, "planning"),
            Self::Reasoning => write!(f, "reasoning"),
            Self::Reflecting => write!(f, "reflecting"),
            Self::Deciding => write!(f, "deciding"),
            Self::Verifying => write!(f, "verifying"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReasoningPhase {
    KnowledgeRetrieval,
    HypothesisGeneration,
    Planning,
    StrategyExecution,
    ChainOfThought,
    Reflection,
    Decision,
    Verification,
    Explanation,
}

impl fmt::Display for ReasoningPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KnowledgeRetrieval => write!(f, "knowledge_retrieval"),
            Self::HypothesisGeneration => write!(f, "hypothesis_generation"),
            Self::Planning => write!(f, "planning"),
            Self::StrategyExecution => write!(f, "strategy_execution"),
            Self::ChainOfThought => write!(f, "chain_of_thought"),
            Self::Reflection => write!(f, "reflection"),
            Self::Decision => write!(f, "decision"),
            Self::Verification => write!(f, "verification"),
            Self::Explanation => write!(f, "explanation"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningSession {
    pub id: Uuid,
    pub query: String,
    pub state: SessionState,
    pub strategy: ReasoningStrategy,
    pub context: HashMap<String, serde_json::Value>,
    pub max_depth: u32,
    pub timeout_ms: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub phase_history: Vec<PhaseTransition>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ReasoningSession {
    pub fn new(query: String, strategy: ReasoningStrategy, timeout_ms: u64) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            query,
            state: SessionState::Created,
            strategy,
            context: HashMap::new(),
            max_depth: 128,
            timeout_ms,
            created_at: now,
            updated_at: now,
            phase_history: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_depth(mut self, max_depth: u32) -> Self {
        self.max_depth = max_depth;
        self
    }

    pub fn with_context(mut self, key: String, value: serde_json::Value) -> Self {
        self.context.insert(key, value);
        self
    }

    pub fn transition(&mut self, target: SessionState) -> ReasoningResult<()> {
        if !self.state.can_transition_to(target) {
            return Err(crate::error::ReasoningError::InvalidState(format!(
                "cannot transition from {} to {}",
                self.state, target
            )));
        }
        self.phase_history.push(PhaseTransition {
            from: self.state,
            to: target,
            timestamp: Utc::now(),
        });
        self.state = target;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn elapsed_ms(&self) -> u64 {
        let now = Utc::now();
        let duration = now.signed_duration_since(self.created_at);
        duration.num_milliseconds() as u64
    }

    pub fn is_expired(&self) -> bool {
        self.elapsed_ms() >= self.timeout_ms
    }

    pub fn record_phase(&mut self, phase: ReasoningPhase) {
        self.metadata
            .insert("last_phase".to_string(), serde_json::json!(phase.to_string()));
        self.updated_at = Utc::now();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseTransition {
    pub from: SessionState,
    pub to: SessionState,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionNode {
    pub id: Uuid,
    pub phase: ReasoningPhase,
    pub strategy: Option<ReasoningStrategy>,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub dependencies: Vec<Uuid>,
    pub status: NodeStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
    Cancelled,
}

impl fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Skipped => write!(f, "skipped"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionGraph {
    pub nodes: Vec<ExecutionNode>,
}

impl ExecutionGraph {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn add_node(&mut self, node: ExecutionNode) {
        self.nodes.push(node);
    }

    pub fn get_node(&self, id: Uuid) -> Option<&ExecutionNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn get_node_mut(&mut self, id: Uuid) -> Option<&mut ExecutionNode> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    pub fn roots(&self) -> Vec<&ExecutionNode> {
        self.nodes
            .iter()
            .filter(|n| n.dependencies.is_empty())
            .collect()
    }

    pub fn dependents_of(&self, id: Uuid) -> Vec<&ExecutionNode> {
        self.nodes
            .iter()
            .filter(|n| n.dependencies.contains(&id))
            .collect()
    }

    pub fn execution_order(&self) -> Vec<Uuid> {
        let mut visited = std::collections::HashSet::new();
        let mut order = Vec::new();

        for root in self.roots() {
            self.topological_visit(root.id, &mut visited, &mut order);
        }

        order
    }

    fn topological_visit(
        &self,
        id: Uuid,
        visited: &mut std::collections::HashSet<Uuid>,
        order: &mut Vec<Uuid>,
    ) {
        if visited.contains(&id) {
            return;
        }
        visited.insert(id);

        for dep in &self.dependents_of(id) {
            let all_deps_met = dep
                .dependencies
                .iter()
                .all(|d| visited.contains(d));
            if all_deps_met {
                self.topological_visit(dep.id, visited, order);
            }
        }

        order.push(id);
    }

    pub fn all_completed(&self) -> bool {
        self.nodes.iter().all(|n| {
            matches!(
                n.status,
                NodeStatus::Completed | NodeStatus::Skipped | NodeStatus::Cancelled
            )
        })
    }

    pub fn has_failures(&self) -> bool {
        self.nodes.iter().any(|n| n.status == NodeStatus::Failed)
    }

    pub fn pending_nodes(&self) -> Vec<&ExecutionNode> {
        self.nodes
            .iter()
            .filter(|n| n.status == NodeStatus::Pending)
            .collect()
    }

    pub fn ready_nodes(&self) -> Vec<&ExecutionNode> {
        let completed: std::collections::HashSet<Uuid> = self
            .nodes
            .iter()
            .filter(|n| matches!(n.status, NodeStatus::Completed | NodeStatus::Skipped))
            .map(|n| n.id)
            .collect();

        self.nodes
            .iter()
            .filter(|n| {
                n.status == NodeStatus::Pending
                    && n.dependencies.iter().all(|d| completed.contains(d))
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodePriority {
    Critical,
    High,
    Normal,
    Low,
    Background,
}

impl Default for NodePriority {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningConfig {
    pub max_depth: u32,
    pub timeout_ms: u64,
    pub max_hypotheses: usize,
    pub min_confidence: f32,
    pub enable_reflection: bool,
    pub enable_caching: bool,
    pub cache_ttl_secs: u64,
    pub max_cache_entries: usize,
    pub enable_tool_reasoning: bool,
    pub enable_multi_model: bool,
    pub consensus_threshold: f32,
    pub max_alternatives: usize,
}

impl Default for ReasoningConfig {
    fn default() -> Self {
        Self {
            max_depth: 128,
            timeout_ms: 30_000,
            max_hypotheses: 10,
            min_confidence: 0.3,
            enable_reflection: true,
            enable_caching: true,
            cache_ttl_secs: 3600,
            max_cache_entries: 10_000,
            enable_tool_reasoning: true,
            enable_multi_model: false,
            consensus_threshold: 0.6,
            max_alternatives: 5,
        }
    }
}
