use crate::conversation::error::ConversationResult;
use crate::conversation::retrieval_coordinator::CognitiveContext;
use crate::conversation::types::*;
use crate::language::types::{Message, ToolDefinition};

/// A built prompt ready for language engine consumption.
#[derive(Debug, Clone)]
pub struct BuiltPrompt {
    pub system_message: Message,
    pub context_messages: Vec<Message>,
    pub tools: Vec<ToolDefinition>,
    pub metadata: std::collections::HashMap<String, String>,
}

/// Builds structured prompts from the unified cognitive context.
pub struct PromptBuilder;

impl PromptBuilder {
    pub fn new() -> Self {
        Self
    }

    /// Build a complete prompt from conversation context and cognitive context.
    pub fn build(
        &self,
        context: &ConversationContext,
        cognitive: &CognitiveContext,
        tool_definitions: &[ToolDefinition],
        _available_functions: &[String],
    ) -> ConversationResult<BuiltPrompt> {
        let system_prompt = self.build_system_prompt(context, cognitive)?;
        let context_messages = self.build_context_messages(context, cognitive)?;
        let tools = tool_definitions.to_vec();

        let mut metadata = std::collections::HashMap::new();
        metadata.insert(
            "conversation_id".to_string(),
            context.conversation_id.to_string(),
        );
        metadata.insert(
            "session_id".to_string(),
            context.session_id.to_string(),
        );
        if let Some(ref intent) = context.intent {
            metadata.insert("intent".to_string(), intent.to_string());
        }
        metadata.insert(
            "evidence_count".to_string(),
            cognitive.unified.evidence_count.to_string(),
        );
        metadata.insert(
            "cognitive_confidence".to_string(),
            cognitive.confidence.to_string(),
        );

        Ok(BuiltPrompt {
            system_message: Message::system(system_prompt),
            context_messages,
            tools,
            metadata,
        })
    }

    fn build_system_prompt(
        &self,
        context: &ConversationContext,
        cognitive: &CognitiveContext,
    ) -> ConversationResult<String> {
        let mut parts = Vec::new();
        parts.push("You are Neo, an AGI operating system assistant.".to_string());

        if let Some(ref intent) = context.intent {
            parts.push(format!("Current intent: {}", intent));
        }
        parts.push(format!("Urgency: {:?}", context.urgency));
        parts.push(format!("Reasoning depth: {:?}", context.reasoning_depth));

        if !cognitive.ranked_evidence.is_empty() {
            parts.push(format!(
                "Available context: {} evidence items (confidence: {:.2})",
                cognitive.ranked_evidence.len(),
                cognitive.confidence
            ));
        }

        if let Some(ref executive) = cognitive.executive_context {
            parts.push(format!(
                "Request classification: {:?}",
                executive.classification
            ));
        }

        if cognitive.unified.contradictions_detected > 0 {
            parts.push(format!(
                "Warning: {} contradictions detected in retrieved context",
                cognitive.unified.contradictions_detected
            ));
        }

        Ok(parts.join("\n"))
    }

    fn build_context_messages(
        &self,
        context: &ConversationContext,
        cognitive: &CognitiveContext,
    ) -> ConversationResult<Vec<Message>> {
        let mut messages = Vec::new();

        // Include relevant evidence as context
        for ranked in cognitive.ranked_evidence.iter().take(10) {
            if ranked.final_score > 0.3 {
                messages.push(Message::system(format!(
                    "[Evidence from {:?}] {}",
                    ranked.evidence.source, ranked.evidence.content
                )));
            }
        }

        // Include recent conversation history
        let skip = context.messages.len().saturating_sub(20);
        for msg in context.messages.iter().skip(skip) {
            messages.push(msg.to_language_message());
        }

        Ok(messages)
    }
}

impl Default for PromptBuilder {
    fn default() -> Self {
        Self::new()
    }
}
