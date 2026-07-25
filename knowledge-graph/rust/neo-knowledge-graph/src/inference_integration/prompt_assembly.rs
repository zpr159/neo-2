use crate::core::entity::Entity;
use crate::inference_integration::fact_retrieval::RetrievedFact;

/// Assembles final prompts with knowledge context.
pub struct PromptAssembler;

impl PromptAssembler {
    /// Create a new assembler.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Assemble a complete prompt with system context and knowledge facts.
    #[must_use]
    pub fn assemble(
        &self,
        system_prompt: &str,
        user_query: &str,
        facts: &[RetrievedFact],
        max_tokens: usize,
    ) -> AssembledPrompt {
        let mut knowledge_section = String::new();
        let mut token_budget = max_tokens;

        // Reserve tokens for system prompt and query
        let system_tokens = system_prompt.len() / 4;
        let query_tokens = user_query.len() / 4;
        token_budget = token_budget.saturating_sub(system_tokens + query_tokens + 20);

        for fact in facts {
            let fact_tokens = fact.text.len() / 4;
            if fact_tokens > token_budget {
                break;
            }
            knowledge_section.push_str(&format!("- {} (confidence: {:.0}%)\n", fact.text, fact.confidence * 100.0));
            token_budget -= fact_tokens;
        }

        let mut messages = Vec::new();

        if !system_prompt.is_empty() {
            messages.push(PromptMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            });
        }

        if !knowledge_section.is_empty() {
            messages.push(PromptMessage {
                role: "system".to_string(),
                content: format!("Knowledge base:\n{}", knowledge_section),
            });
        }

        messages.push(PromptMessage {
            role: "user".to_string(),
            content: user_query.to_string(),
        });

        AssembledPrompt {
            messages,
            facts_used: facts.len(),
            estimated_tokens: max_tokens.saturating_sub(token_budget),
        }
    }
}

/// A prompt message.
#[derive(Debug, Clone)]
pub struct PromptMessage {
    /// Role (system, user, assistant).
    pub role: String,
    /// Content.
    pub content: String,
}

/// An assembled prompt ready for inference.
#[derive(Debug, Clone)]
pub struct AssembledPrompt {
    /// The messages.
    pub messages: Vec<PromptMessage>,
    /// Number of facts incorporated.
    pub facts_used: usize,
    /// Estimated total tokens.
    pub estimated_tokens: usize,
}

impl Default for PromptAssembler {
    fn default() -> Self {
        Self::new()
    }
}
