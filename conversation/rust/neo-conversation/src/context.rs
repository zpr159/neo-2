use crate::session::ConversationSession;
use crate::types::{CognitiveContext, LlmMessage, SessionConfig};

/// Constructs the prompt context from all available cognitive systems.
///
/// Assembles the full prompt by combining system instructions, cognitive context,
/// conversation history, and tool definitions.
pub struct ContextManager {
    system_template: String,
}

impl ContextManager {
    #[must_use]
    pub fn new(config: &SessionConfig) -> Self {
        Self {
            system_template: config.system_prompt.clone(),
        }
    }

    pub fn build_prompt(&self, session: &ConversationSession) -> Vec<LlmMessage> {
        let mut messages = Vec::new();

        let system_content = self.build_system_prompt(&session.cognitive_context);
        messages.push(LlmMessage {
            role: crate::types::MessageRole::System,
            content: system_content,
        });

        for msg in session.messages.iter().skip(1) {
            messages.push(LlmMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
            });
        }

        messages
    }

    fn build_system_prompt(&self, context: &CognitiveContext) -> String {
        let context_block = context.build_context_block();

        if context_block.is_empty() {
            self.system_template.clone()
        } else {
            format!(
                "{system}\n\n## Cognitive Context\nThe following context has been gathered from Neo's cognitive subsystems to inform your response.\n\n{context}",
                system = self.system_template,
                context = context_block,
            )
        }
    }

    #[must_use]
    pub fn estimate_tokens(&self, messages: &[LlmMessage]) -> usize {
        messages
            .iter()
            .map(|m| m.content.len() / 4 + 4)
            .sum()
    }

    pub fn truncate_to_budget(&self, messages: &mut Vec<LlmMessage>, budget: usize) {
        let total = self.estimate_tokens(messages);
        if total <= budget {
            return;
        }

        if messages.is_empty() {
            return;
        }
        let system = messages[0].clone();
        let mut remaining = messages[1..].to_vec();

        while !remaining.is_empty() {
            let current_tokens =
                self.estimate_tokens(&remaining) + self.estimate_tokens(&[system.clone()]);
            if current_tokens <= budget {
                break;
            }
            remaining.remove(0);
        }

        messages.clear();
        messages.push(system);
        messages.extend(remaining);
    }

    #[must_use]
    pub fn build_query_context(&self, query: &str, context: &CognitiveContext) -> String {
        let mut parts = Vec::new();
        parts.push(format!("User query: {query}"));

        let context_block = context.build_context_block();
        if !context_block.is_empty() {
            parts.push(format!("Available context:\n{context_block}"));
        }

        parts.join("\n\n")
    }
}

/// Assembles context from all cognitive subsystems.
///
/// Responsible for retrieving memory, knowledge, plans, reasoning, world state,
/// workflows, agent outputs — then merging, ranking, deduplicating, and
/// respecting a token budget.
pub struct ContextAssembler {
    token_budget: usize,
    max_memories: usize,
    max_knowledge: usize,
    max_reasoning: usize,
    max_world: usize,
}

impl ContextAssembler {
    pub fn new(token_budget: usize) -> Self {
        Self {
            token_budget,
            max_memories: 10,
            max_knowledge: 10,
            max_reasoning: 5,
            max_world: 5,
        }
    }

    pub fn with_limits(
        token_budget: usize,
        max_memories: usize,
        max_knowledge: usize,
        max_reasoning: usize,
        max_world: usize,
    ) -> Self {
        Self {
            token_budget,
            max_memories,
            max_knowledge,
            max_reasoning,
            max_world,
        }
    }

    /// Assemble a CognitiveContext from all available sources.
    pub fn assemble(
        &self,
        memory_results: Vec<String>,
        knowledge_results: Vec<String>,
        reasoning_results: Vec<String>,
        world_results: Vec<String>,
        plan_context: Option<String>,
        agent_outputs: Vec<String>,
        workflow_outputs: Vec<String>,
        executive_decisions: Vec<String>,
    ) -> CognitiveContext {
        let mut context = CognitiveContext::empty();

        context.memories = self.rank_and_limit(memory_results, self.max_memories);
        context.knowledge = self.rank_and_limit(knowledge_results, self.max_knowledge);
        context.reasoning = self.rank_and_limit(reasoning_results, self.max_reasoning);
        context.world_state = self.rank_and_limit(world_results, self.max_world);
        context.plan_context = plan_context;
        context.agent_outputs = agent_outputs;
        context.workflow_outputs = workflow_outputs;
        context.executive_decisions = executive_decisions;

        if !context.memories.is_empty() {
            context.sources.push(crate::types::CognitiveSource::Memory);
        }
        if !context.knowledge.is_empty() {
            context
                .sources
                .push(crate::types::CognitiveSource::KnowledgeGraph);
        }
        if !context.reasoning.is_empty() {
            context
                .sources
                .push(crate::types::CognitiveSource::Reasoning);
        }
        if !context.world_state.is_empty() {
            context
                .sources
                .push(crate::types::CognitiveSource::WorldModel);
        }
        if context.plan_context.is_some() {
            context.sources.push(crate::types::CognitiveSource::Planning);
        }

        context
    }

    fn rank_and_limit(&self, items: Vec<String>, limit: usize) -> Vec<String> {
        let mut unique: Vec<String> = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for item in items {
            let normalized = item.trim().to_lowercase();
            if !seen.contains(&normalized) {
                seen.insert(normalized);
                unique.push(item);
            }
        }

        unique.truncate(limit);
        unique
    }

    /// Estimate the token cost of a context block.
    #[must_use]
    pub fn estimate_context_tokens(context: &CognitiveContext) -> usize {
        let block = context.build_context_block();
        block.len() / 4 + 4
    }

    /// Check if context fits within the token budget.
    #[must_use]
    pub fn fits_budget(&self, context: &CognitiveContext) -> bool {
        Self::estimate_context_tokens(context) <= self.token_budget
    }
}

impl Default for ContextAssembler {
    fn default() -> Self {
        Self::new(8192)
    }
}
