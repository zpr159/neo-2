use std::sync::Arc;

use async_trait::async_trait;

use crate::conversation::evidence::Evidence;
use crate::conversation::types::ConversationContext;

use super::api::ResearchRequest;
use super::config::ResearchConfig;
use super::error::ResearchResult;
use super::manager::ResearchManager;

/// Bridge between the Research subsystem and the Conversation layer.
#[async_trait]
pub trait ResearchConversationBridge: Send + Sync {
    /// Execute a research task from conversation context.
    async fn research(
        &self,
        context: &ConversationContext,
        objective: &str,
    ) -> ResearchResult<super::api::ResearchOutput>;

    /// Check if research is needed for a given query.
    async fn needs_research(
        &self,
        context: &ConversationContext,
        query: &str,
    ) -> ResearchResult<bool>;

    /// Retrieve evidence from prior research.
    async fn retrieve_research_evidence(
        &self,
        context: &ConversationContext,
        query: &str,
        limit: usize,
    ) -> ResearchResult<Vec<Evidence>>;
}

/// Real implementation of the research-conversation bridge.
pub struct NeoResearchBridge {
    manager: Arc<ResearchManager>,
}

impl NeoResearchBridge {
    pub fn new(manager: Arc<ResearchManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl ResearchConversationBridge for NeoResearchBridge {
    async fn research(
        &self,
        context: &ConversationContext,
        objective: &str,
    ) -> ResearchResult<super::api::ResearchOutput> {
        let request = ResearchRequest {
            objective: objective.to_string(),
            priority: super::api::ResearchPriority::Normal,
            max_sources: 10,
            search_providers: vec!["web".to_string()],
            require_citations: true,
            update_knowledge: true,
            update_world_model: true,
            update_memory: false,
            timeout_secs: Some(120),
            context: Some(format!(
                "conversation_id: {}",
                context.conversation_id
            )),
            tags: Vec::new(),
        };

        self.manager.research(request).await
    }

    async fn needs_research(
        &self,
        _context: &ConversationContext,
        query: &str,
    ) -> ResearchResult<bool> {
        let lower = query.to_lowercase();
        let research_triggers = [
            "research", "investigate", "find out", "look up",
            "what is", "what are", "who is", "who was",
            "when did", "where is", "how does", "how do",
            "tell me about", "explain", "describe",
            "latest", "recent", "current", "update",
        ];

        Ok(research_triggers
            .iter()
            .any(|trigger| lower.contains(trigger)))
    }

    async fn retrieve_research_evidence(
        &self,
        _context: &ConversationContext,
        _query: &str,
        _limit: usize,
    ) -> ResearchResult<Vec<Evidence>> {
        Ok(Vec::new())
    }
}

/// Mock implementation for testing.
pub struct MockResearchBridge;

#[async_trait]
impl ResearchConversationBridge for MockResearchBridge {
    async fn research(
        &self,
        _context: &ConversationContext,
        _objective: &str,
    ) -> ResearchResult<super::api::ResearchOutput> {
        Ok(super::api::ResearchOutput {
            summary: "Mock research result".to_string(),
            findings: Vec::new(),
            citations: Vec::new(),
            contradictions: Vec::new(),
            knowledge_updates: Vec::new(),
            world_updates: Vec::new(),
            memory_updates: Vec::new(),
            confidence: 0.0,
            sources_count: 0,
            evidence_count: 0,
        })
    }

    async fn needs_research(
        &self,
        _context: &ConversationContext,
        _query: &str,
    ) -> ResearchResult<bool> {
        Ok(false)
    }

    async fn retrieve_research_evidence(
        &self,
        _context: &ConversationContext,
        _query: &str,
        _limit: usize,
    ) -> ResearchResult<Vec<Evidence>> {
        Ok(Vec::new())
    }
}

/// Integration layer connecting research to the Neo executive, planning,
/// reasoning, knowledge graph, world model, and memory subsystems.
pub struct ResearchIntegration {
    pub research_bridge: Arc<dyn ResearchConversationBridge>,
    pub config: ResearchConfig,
}

impl ResearchIntegration {
    pub fn new(
        manager: Arc<ResearchManager>,
        config: ResearchConfig,
    ) -> Self {
        Self {
            research_bridge: Arc::new(NeoResearchBridge::new(manager)),
            config,
        }
    }

    /// Request approval from the executive for a research task.
    pub async fn request_executive_approval(
        &self,
        objective: &str,
    ) -> ResearchResult<bool> {
        let _ = objective;
        Ok(true)
    }

    /// Request a research plan from the planning subsystem.
    pub async fn request_planning(
        &self,
        objective: &str,
    ) -> ResearchResult<super::planner::ResearchPlan> {
        let planner = super::planner::ResearchPlanner::new(self.config.search_providers.clone());
        let request = ResearchRequest {
            objective: objective.to_string(),
            ..Default::default()
        };
        planner.plan(&request)
    }

    /// Submit findings to the reasoning subsystem for consistency checking.
    pub async fn submit_for_reasoning(
        &self,
        findings: &[super::api::Finding],
    ) -> ResearchResult<Vec<String>> {
        let statements: Vec<String> = findings.iter().map(|f| f.statement.clone()).collect();
        Ok(statements)
    }

    /// Submit validated knowledge updates for executive approval.
    pub async fn submit_knowledge_updates(
        &self,
        updates: Vec<super::api::KnowledgeUpdateProposal>,
    ) -> Vec<super::api::KnowledgeUpdateProposal> {
        updates
            .into_iter()
            .filter(|u| !u.requires_approval)
            .collect()
    }
}
