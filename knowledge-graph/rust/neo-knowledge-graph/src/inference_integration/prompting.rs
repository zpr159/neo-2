use crate::core::entity::Entity;
use crate::storage::graph_store::GraphStore;

/// Generates knowledge-aware prompts for the inference engine.
pub struct KnowledgeAwarePrompter;

impl KnowledgeAwarePrompter {
    /// Create a new prompter.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Build a knowledge-augmented prompt given a query and relevant knowledge.
    #[must_use]
    pub fn build_prompt(
        &self,
        query: &str,
        relevant_entities: &[Entity],
        max_context_tokens: usize,
    ) -> String {
        let mut context_parts = Vec::new();
        let mut estimated_tokens = 0;

        for entity in relevant_entities {
            let entry = format!(
                "Knowledge: {} ({}): {}",
                entity.label,
                entity.entity_type,
                entity.description
            );
            let token_estimate = entry.len() / 4; // rough estimate
            if estimated_tokens + token_estimate > max_context_tokens {
                break;
            }
            context_parts.push(entry);
            estimated_tokens += token_estimate;
        }

        if context_parts.is_empty() {
            format!("Query: {}", query)
        } else {
            format!(
                "Based on the following knowledge:\n{}\n\nQuery: {}",
                context_parts.join("\n"),
                query
            )
        }
    }

    /// Build a fact-grounded prompt with source attribution.
    #[must_use]
    pub fn build_fact_grounded_prompt(
        &self,
        query: &str,
        facts: &[(String, f32)], // (fact text, confidence)
    ) -> String {
        let mut prompt = String::from("Use the following verified facts to answer:\n\n");

        for (i, (fact, confidence)) in facts.iter().enumerate() {
            prompt.push_str(&format!(
                "[Fact {} (confidence: {:.0}%)] {}\n",
                i + 1,
                confidence * 100.0,
                fact
            ));
        }

        prompt.push_str(&format!("\nQuestion: {}", query));
        prompt
    }
}

impl Default for KnowledgeAwarePrompter {
    fn default() -> Self {
        Self::new()
    }
}
