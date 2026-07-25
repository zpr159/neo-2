use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::conversation::error::ConversationResult;
use crate::conversation::types::*;

/// Executive decisions that become part of conversation context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutiveDecision {
    pub intent: Intent,
    pub urgency: Urgency,
    pub classification: RequestClassification,
    pub execution_policy: ExecutionPolicy,
    pub reasoning_depth: ReasoningDepth,
    pub tool_authorizations: Vec<ToolAuthorization>,
    pub allowed_workflows: Vec<String>,
    pub allowed_agents: Vec<String>,
    pub estimated_cost: f64,
    pub priority: u32,
    pub rationale: Option<String>,
}

/// Bridge between the Executive subsystem and the Conversation layer.
///
/// The Executive is responsible for high-level decision-making about how
/// a conversation request should be handled. It determines intent, urgency,
/// authorization, and execution policy.
#[async_trait]
pub trait ExecutiveConversationBridge: Send + Sync {
    /// Receive a conversation objective and produce an Executive decision.
    async fn process_objective(
        &self,
        context: &ConversationContext,
        objective: &str,
    ) -> ConversationResult<ExecutiveDecision>;

    /// Determine the intent of a user message.
    async fn classify_intent(
        &self,
        context: &ConversationContext,
        message: &str,
    ) -> ConversationResult<Intent>;

    /// Determine the urgency level of a request.
    async fn assess_urgency(
        &self,
        context: &ConversationContext,
        intent: &Intent,
        message: &str,
    ) -> ConversationResult<Urgency>;

    /// Classify the request complexity and required approach.
    async fn classify_request(
        &self,
        context: &ConversationContext,
        intent: &Intent,
        message: &str,
    ) -> ConversationResult<RequestClassification>;

    /// Select the execution policy for a classified request.
    async fn select_execution_policy(
        &self,
        context: &ConversationContext,
        classification: &RequestClassification,
    ) -> ConversationResult<ExecutionPolicy>;

    /// Authorize specific tool execution.
    async fn authorize_tool(
        &self,
        context: &ConversationContext,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> ConversationResult<ToolAuthorization>;

    /// Determine the required reasoning depth.
    async fn prioritize_reasoning_depth(
        &self,
        context: &ConversationContext,
        intent: &Intent,
        classification: &RequestClassification,
    ) -> ConversationResult<ReasoningDepth>;

    /// Approve or deny workflow execution.
    async fn approve_workflow(
        &self,
        context: &ConversationContext,
        workflow_id: &str,
    ) -> ConversationResult<bool>;

    /// Approve or deny agent delegation.
    async fn approve_agent_delegation(
        &self,
        context: &ConversationContext,
        agent_id: &str,
        objective: &str,
    ) -> ConversationResult<bool>;
}

/// Mock implementation for testing.
pub struct MockExecutiveBridge;

#[async_trait]
impl ExecutiveConversationBridge for MockExecutiveBridge {
    async fn process_objective(
        &self,
        _context: &ConversationContext,
        _objective: &str,
    ) -> ConversationResult<ExecutiveDecision> {
        Ok(ExecutiveDecision {
            intent: Intent::Conversation,
            urgency: Urgency::Normal,
            classification: RequestClassification::SimpleQuery,
            execution_policy: ExecutionPolicy::Immediate,
            reasoning_depth: ReasoningDepth::Normal,
            tool_authorizations: vec![ToolAuthorization::Auto],
            allowed_workflows: Vec::new(),
            allowed_agents: Vec::new(),
            estimated_cost: 0.0,
            priority: 50,
            rationale: None,
        })
    }

    async fn classify_intent(
        &self,
        _context: &ConversationContext,
        _message: &str,
    ) -> ConversationResult<Intent> {
        Ok(Intent::Conversation)
    }

    async fn assess_urgency(
        &self,
        _context: &ConversationContext,
        _intent: &Intent,
        _message: &str,
    ) -> ConversationResult<Urgency> {
        Ok(Urgency::Normal)
    }

    async fn classify_request(
        &self,
        _context: &ConversationContext,
        _intent: &Intent,
        _message: &str,
    ) -> ConversationResult<RequestClassification> {
        Ok(RequestClassification::SimpleQuery)
    }

    async fn select_execution_policy(
        &self,
        _context: &ConversationContext,
        _classification: &RequestClassification,
    ) -> ConversationResult<ExecutionPolicy> {
        Ok(ExecutionPolicy::Immediate)
    }

    async fn authorize_tool(
        &self,
        _context: &ConversationContext,
        _tool_name: &str,
        _arguments: &serde_json::Value,
    ) -> ConversationResult<ToolAuthorization> {
        Ok(ToolAuthorization::Auto)
    }

    async fn prioritize_reasoning_depth(
        &self,
        _context: &ConversationContext,
        _intent: &Intent,
        _classification: &RequestClassification,
    ) -> ConversationResult<ReasoningDepth> {
        Ok(ReasoningDepth::Normal)
    }

    async fn approve_workflow(
        &self,
        _context: &ConversationContext,
        _workflow_id: &str,
    ) -> ConversationResult<bool> {
        Ok(true)
    }

    async fn approve_agent_delegation(
        &self,
        _context: &ConversationContext,
        _agent_id: &str,
        _objective: &str,
    ) -> ConversationResult<bool> {
        Ok(true)
    }
}
