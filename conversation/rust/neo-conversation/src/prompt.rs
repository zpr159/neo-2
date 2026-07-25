use std::collections::HashMap;

use crate::types::{CognitiveContext, LlmMessage, MessageRole, SessionConfig};

/// Template for constructing prompts.
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    pub name: String,
    pub system: String,
    pub cognitive_injection: String,
    pub user_wrapper: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl PromptTemplate {
    pub fn new(name: impl Into<String>, system: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            system: system.into(),
            cognitive_injection: String::new(),
            user_wrapper: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_cognitive_injection(mut self, injection: impl Into<String>) -> Self {
        self.cognitive_injection = injection.into();
        self
    }

    pub fn with_user_wrapper(mut self, wrapper: impl Into<String>) -> Self {
        self.user_wrapper = Some(wrapper.into());
        self
    }

    /// Build-in templates for common conversation modes.
    pub fn conversation() -> Self {
        Self::new(
            "conversation",
            "You are Neo, an AI operating system assistant. You are thoughtful, precise, and helpful.",
        )
    }

    pub fn coding() -> Self {
        Self::new(
            "coding",
            "You are Neo, an expert software engineering assistant. You write clean, efficient, well-tested code. You follow best practices and existing conventions.",
        )
    }

    pub fn planning() -> Self {
        Self::new(
            "planning",
            "You are Neo, a strategic planning assistant. You break down complex tasks into actionable steps, consider dependencies, and create clear plans.",
        )
    }

    pub fn reasoning() -> Self {
        Self::new(
            "reasoning",
            "You are Neo, a logical reasoning assistant. You think step by step, consider evidence carefully, and reach well-justified conclusions.",
        )
    }

    pub fn research() -> Self {
        Self::new(
            "research",
            "You are Neo, a research assistant. You gather information systematically, analyze sources critically, and present findings clearly.",
        )
    }

    pub fn debugging() -> Self {
        Self::new(
            "debugging",
            "You are Neo, a debugging specialist. You systematically identify issues, trace root causes, and provide precise fixes.",
        )
    }

    pub fn creative() -> Self {
        Self::new(
            "creative",
            "You are Neo, a creative assistant. You generate original ideas, write engaging content, and approach problems with imagination.",
        )
    }

    pub fn summarization() -> Self {
        Self::new(
            "summarization",
            "You are Neo, a summarization assistant. You distill complex information into clear, concise summaries while preserving key details.",
        )
    }

    pub fn explanation() -> Self {
        Self::new(
            "explanation",
            "You are Neo, an explanation assistant. You make complex topics understandable with clear, well-structured explanations and examples.",
        )
    }

    pub fn tool_usage() -> Self {
        Self::new(
            "tool_usage",
            "You are Neo, an AI assistant with tool access. Use the provided tools when needed. Format tool calls as JSON in code blocks.",
        )
    }
}

/// Builds prompts by assembling system instructions, cognitive context, and conversation history.
pub struct PromptBuilder {
    templates: HashMap<String, PromptTemplate>,
    active_template: String,
}

impl PromptBuilder {
    pub fn new() -> Self {
        let mut templates = HashMap::new();

        let built_in = [
            PromptTemplate::conversation(),
            PromptTemplate::coding(),
            PromptTemplate::planning(),
            PromptTemplate::reasoning(),
            PromptTemplate::research(),
            PromptTemplate::debugging(),
            PromptTemplate::creative(),
            PromptTemplate::summarization(),
            PromptTemplate::explanation(),
            PromptTemplate::tool_usage(),
        ];

        for t in built_in {
            templates.insert(t.name.clone(), t);
        }

        Self {
            templates,
            active_template: "conversation".into(),
        }
    }

    /// Register a custom template.
    pub fn register_template(&mut self, template: PromptTemplate) {
        self.templates.insert(template.name.clone(), template);
    }

    /// Set the active template.
    pub fn set_template(&mut self, name: &str) -> bool {
        if self.templates.contains_key(name) {
            self.active_template = name.to_string();
            true
        } else {
            false
        }
    }

    /// Get the active template.
    #[must_use]
    pub fn active_template(&self) -> Option<&PromptTemplate> {
        self.templates.get(&self.active_template)
    }

    /// Build the system prompt from the active template and cognitive context.
    #[must_use]
    pub fn build_system_prompt(&self, context: &CognitiveContext) -> String {
        let template = self
            .templates
            .get(&self.active_template)
            .expect("active template must exist");

        let mut parts = vec![template.system.clone()];

        if !template.cognitive_injection.is_empty() {
            parts.push(template.cognitive_injection.clone());
        }

        let context_block = context.build_context_block();
        if !context_block.is_empty() {
            parts.push(format!(
                "## Cognitive Context\nThe following context has been gathered from Neo's cognitive subsystems to inform your response.\n\n{context_block}"
            ));
        }

        parts.join("\n\n")
    }

    /// Build the full prompt message list.
    pub fn build_messages(
        &self,
        config: &SessionConfig,
        context: &CognitiveContext,
        history: &[LlmMessage],
    ) -> Vec<LlmMessage> {
        let mut messages = Vec::new();

        let system_content = self.build_system_prompt(context);
        messages.push(LlmMessage {
            role: MessageRole::System,
            content: system_content,
        });

        for msg in history.iter().skip(1) {
            messages.push(msg.clone());
        }

        let _ = config;
        messages
    }

    /// Build a prompt for a specific query with context.
    #[must_use]
    pub fn build_query_prompt(&self, query: &str, context: &CognitiveContext) -> Vec<LlmMessage> {
        let mut messages = Vec::new();

        let system_content = self.build_system_prompt(context);
        messages.push(LlmMessage {
            role: MessageRole::System,
            content: system_content,
        });

        messages.push(LlmMessage {
            role: MessageRole::User,
            content: query.to_string(),
        });

        messages
    }

    /// Estimate token count for a set of messages.
    #[must_use]
    pub fn estimate_tokens(messages: &[LlmMessage]) -> usize {
        messages
            .iter()
            .map(|m| m.content.len() / 4 + 4)
            .sum()
    }

    /// Truncate messages to fit within a token budget.
    pub fn truncate_to_budget(messages: &mut Vec<LlmMessage>, budget: usize) {
        let total = Self::estimate_tokens(messages);
        if total <= budget || messages.is_empty() {
            return;
        }

        let system = messages[0].clone();
        let mut remaining: Vec<LlmMessage> = messages[1..].to_vec();

        while !remaining.is_empty() {
            let current =
                Self::estimate_tokens(&remaining) + Self::estimate_tokens(&[system.clone()]);
            if current <= budget {
                break;
            }
            remaining.remove(0);
        }

        messages.clear();
        messages.push(system);
        messages.extend(remaining);
    }
}

impl Default for PromptBuilder {
    fn default() -> Self {
        Self::new()
    }
}
