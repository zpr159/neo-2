use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::conversation::error::ConversationResult;
use crate::conversation::types::*;

/// Result of planning decomposition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningContext {
    pub subtasks: Vec<Subtask>,
    pub estimated_cost: f64,
    pub estimated_duration_ms: u64,
    pub dependencies: Vec<Dependency>,
    pub clarification_needed: bool,
    pub clarification_questions: Vec<String>,
    pub execution_graph: ExecutionGraph,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subtask {
    pub id: String,
    pub description: String,
    pub required_capabilities: Vec<String>,
    pub estimated_tokens: usize,
    pub dependencies: Vec<String>,
    pub priority: u32,
    pub can_parallel: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub from: String,
    pub to: String,
    pub kind: DependencyKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyKind {
    DataFlow,
    ExecutionOrder,
    ResourceLock,
    Optional,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionGraph {
    pub layers: Vec<Vec<String>>,
    pub parallel_groups: Vec<Vec<String>>,
    pub sequential_chain: Vec<String>,
}

/// Bridge between the Planning subsystem and the Conversation layer.
#[async_trait]
pub trait PlanningConversationBridge: Send + Sync {
    /// Decompose a complex request into subtasks.
    async fn decompose(
        &self,
        context: &ConversationContext,
        objective: &str,
    ) -> ConversationResult<PlanningContext>;

    /// Estimate the execution cost of a request.
    async fn estimate_cost(
        &self,
        context: &ConversationContext,
        objective: &str,
    ) -> ConversationResult<f64>;

    /// Identify dependencies between operations.
    async fn identify_dependencies(
        &self,
        context: &ConversationContext,
        subtasks: &[Subtask],
    ) -> ConversationResult<Vec<Dependency>>;

    /// Schedule subtasks for execution.
    async fn schedule(
        &self,
        context: &ConversationContext,
        subtasks: &[Subtask],
        dependencies: &[Dependency],
    ) -> ConversationResult<ExecutionGraph>;

    /// Request clarification from the user if the objective is ambiguous.
    async fn request_clarification(
        &self,
        context: &ConversationContext,
        objective: &str,
    ) -> ConversationResult<Vec<String>>;

    /// Generate a full execution plan.
    async fn generate_plan(
        &self,
        context: &ConversationContext,
        objective: &str,
    ) -> ConversationResult<PlanningContext>;
}

/// Mock implementation for testing.
pub struct MockPlanningBridge;

#[async_trait]
impl PlanningConversationBridge for MockPlanningBridge {
    async fn decompose(
        &self,
        _context: &ConversationContext,
        objective: &str,
    ) -> ConversationResult<PlanningContext> {
        Ok(PlanningContext {
            subtasks: vec![Subtask {
                id: "task-1".to_string(),
                description: objective.to_string(),
                required_capabilities: Vec::new(),
                estimated_tokens: 500,
                dependencies: Vec::new(),
                priority: 50,
                can_parallel: false,
            }],
            estimated_cost: 1.0,
            estimated_duration_ms: 1000,
            dependencies: Vec::new(),
            clarification_needed: false,
            clarification_questions: Vec::new(),
            execution_graph: ExecutionGraph {
                layers: vec![vec!["task-1".to_string()]],
                parallel_groups: Vec::new(),
                sequential_chain: vec!["task-1".to_string()],
            },
        })
    }

    async fn estimate_cost(
        &self,
        _context: &ConversationContext,
        _objective: &str,
    ) -> ConversationResult<f64> {
        Ok(1.0)
    }

    async fn identify_dependencies(
        &self,
        _context: &ConversationContext,
        _subtasks: &[Subtask],
    ) -> ConversationResult<Vec<Dependency>> {
        Ok(Vec::new())
    }

    async fn schedule(
        &self,
        _context: &ConversationContext,
        subtasks: &[Subtask],
        _dependencies: &[Dependency],
    ) -> ConversationResult<ExecutionGraph> {
        Ok(ExecutionGraph {
            layers: subtasks.iter().map(|t| t.id.clone()).collect::<Vec<_>>().into_iter().map(|id| vec![id]).collect(),
            parallel_groups: Vec::new(),
            sequential_chain: subtasks.iter().map(|t| t.id.clone()).collect(),
        })
    }

    async fn request_clarification(
        &self,
        _context: &ConversationContext,
        _objective: &str,
    ) -> ConversationResult<Vec<String>> {
        Ok(Vec::new())
    }

    async fn generate_plan(
        &self,
        context: &ConversationContext,
        objective: &str,
    ) -> ConversationResult<PlanningContext> {
        self.decompose(context, objective).await
    }
}
