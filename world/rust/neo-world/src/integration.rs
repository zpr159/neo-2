use crate::types::EntityId;

/// Integration points with other Neo subsystems.
///
/// This module defines the interfaces through which external subsystems
/// (Memory, Knowledge Graph, Reasoning, Executive, Planning, Conversation)
/// interact with the World Model.
pub struct IntegrationLayer;

impl IntegrationLayer {
    /// Build a context summary from the world model for the reasoning engine.
    pub fn reasoning_context(
        entity_ids: &[EntityId],
        entity_descriptions: &[String],
    ) -> String {
        if entity_ids.is_empty() {
            return "No entities in context".into();
        }
        let mut parts = Vec::new();
        for (id, desc) in entity_ids.iter().zip(entity_descriptions.iter()) {
            parts.push(format!("- {id}: {desc}"));
        }
        format!("World entities:\n{}", parts.join("\n"))
    }

    /// Build a context summary for the conversation system.
    pub fn conversation_context(
        entity_names: &[String],
        recent_events: &[String],
    ) -> String {
        let mut parts = Vec::new();
        if !entity_names.is_empty() {
            parts.push(format!(
                "Known entities: {}",
                entity_names.join(", ")
            ));
        }
        if !recent_events.is_empty() {
            parts.push(format!(
                "Recent events:\n{}",
                recent_events
                    .iter()
                    .map(|e| format!("  - {e}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        if parts.is_empty() {
            "World model is empty".into()
        } else {
            parts.join("\n\n")
        }
    }

    /// Build a context summary for the planning system.
    pub fn planning_context(
        active_goals: &[String],
        entity_summary: &str,
    ) -> String {
        let mut parts = Vec::new();
        if !active_goals.is_empty() {
            parts.push(format!(
                "Active goals:\n{}",
                active_goals
                    .iter()
                    .map(|g| format!("  - {g}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        if !entity_summary.is_empty() {
            parts.push(format!("Entity summary: {entity_summary}"));
        }
        if parts.is_empty() {
            "No planning context available".into()
        } else {
            parts.join("\n\n")
        }
    }

    /// Build a context summary for the memory system.
    pub fn memory_context(
        recent_changes: &[String],
        version: u64,
    ) -> String {
        format!(
            "World model version: v{version}\nRecent changes:\n{}",
            if recent_changes.is_empty() {
                "  none".to_string()
            } else {
                recent_changes
                    .iter()
                    .map(|c| format!("  - {c}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        )
    }
}
