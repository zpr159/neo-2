use std::collections::HashMap;
use parking_lot::RwLock;
use uuid::Uuid;

use crate::error::{InferenceError, InferenceResult};
use crate::context::{ContextId, ConversationContext, ContextConfig, Message};

pub struct ContextEngine {
    config: ContextConfig,
    contexts: RwLock<HashMap<ContextId, ConversationContext>>,
}

impl std::fmt::Debug for ContextEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContextEngine")
            .field("config", &self.config)
            .field("context_count", &self.contexts.read().len())
            .finish()
    }
}

impl ContextEngine {
    pub fn new(config: ContextConfig) -> Self {
        Self {
            config,
            contexts: RwLock::new(HashMap::new()),
        }
    }

    pub fn create_context(&self, max_tokens: Option<usize>) -> ContextId {
        let max = max_tokens.unwrap_or(self.config.max_context_tokens);
        let ctx = ConversationContext::new(max)
            .with_sliding_window(self.config.sliding_window_size);
        let id = ctx.id;
        self.contexts.write().insert(id, ctx);
        id
    }

    pub fn create_with_system_prompt(&self, system_prompt: &str, max_tokens: Option<usize>) -> ContextId {
        let max = max_tokens.unwrap_or(self.config.max_context_tokens);
        let ctx = ConversationContext::new(max)
            .with_system_prompt(system_prompt)
            .with_sliding_window(self.config.sliding_window_size);
        let id = ctx.id;
        self.contexts.write().insert(id, ctx);
        id
    }

    pub fn add_message(&self, context_id: ContextId, message: Message) -> InferenceResult<()> {
        let mut contexts = self.contexts.write();
        let ctx = contexts.get_mut(&context_id)
            .ok_or_else(|| InferenceError::ContextError {
                reason: format!("context {} not found", context_id.0),
            })?;
        ctx.add_message(message);
        Ok(())
    }

    pub fn get_messages(&self, context_id: ContextId) -> InferenceResult<Vec<Message>> {
        let contexts = self.contexts.read();
        let ctx = contexts.get(&context_id)
            .ok_or_else(|| InferenceError::ContextError {
                reason: format!("context {} not found", context_id.0),
            })?;
        Ok(ctx.messages.clone())
    }

    pub fn get_context(&self, context_id: ContextId) -> InferenceResult<ConversationContext> {
        let contexts = self.contexts.read();
        contexts.get(&context_id).cloned()
            .ok_or_else(|| InferenceError::ContextError {
                reason: format!("context {} not found", context_id.0),
            })
    }

    pub fn delete_context(&self, context_id: ContextId) -> InferenceResult<()> {
        self.contexts.write().remove(&context_id)
            .ok_or_else(|| InferenceError::ContextError {
                reason: format!("context {} not found", context_id.0),
            })?;
        Ok(())
    }

    pub fn clear_context(&self, context_id: ContextId) -> InferenceResult<()> {
        let mut contexts = self.contexts.write();
        let ctx = contexts.get_mut(&context_id)
            .ok_or_else(|| InferenceError::ContextError {
                reason: format!("context {} not found", context_id.0),
            })?;
        ctx.clear();
        Ok(())
    }

    pub fn total_contexts(&self) -> usize {
        self.contexts.read().len()
    }

    pub fn compress_context(&self, context_id: ContextId) -> InferenceResult<Vec<Message>> {
        let mut contexts = self.contexts.write();
        let ctx = contexts.get_mut(&context_id)
            .ok_or_else(|| InferenceError::ContextError {
                reason: format!("context {} not found", context_id.0),
            })?;
        let total_tokens = ctx.total_tokens();
        if total_tokens <= self.config.compression_threshold {
            return Ok(ctx.messages.clone());
        }
        let mut messages = ctx.messages.clone();
        let system_msgs: Vec<Message> = messages.iter().filter(|m| matches!(m.role, crate::context::MessageRole::System)).cloned().collect();
        let recent_count = messages.len() / 2;
        let compressed: Vec<Message> = if messages.len() > recent_count {
            let mut result = Vec::new();
            result.extend(system_msgs);
            let summary = Message {
                role: crate::context::MessageRole::System,
                content: format!("Previous context summary: {} messages with {} total tokens. Most recent messages follow.", messages.len() - recent_count, total_tokens),
                name: None,
                timestamp: chrono::Utc::now(),
                token_count: None,
                metadata: HashMap::new(),
            };
            result.push(summary);
            let recent: Vec<Message> = messages.drain(messages.len() - recent_count..).collect();
            result.extend(recent);
            result
        } else {
            messages
        };
        ctx.messages = compressed.clone();
        Ok(compressed)
    }

    pub fn merge_contexts(&self, context_ids: &[ContextId]) -> InferenceResult<ContextId> {
        let mut merged = ConversationContext::new(self.config.max_context_tokens);
        let mut contexts = self.contexts.write();
        for &cid in context_ids {
            if let Some(ctx) = contexts.get(&cid) {
                for msg in &ctx.messages {
                    merged.add_message(msg.clone());
                }
            }
        }
        let new_id = merged.id;
        contexts.insert(new_id, merged);
        Ok(new_id)
    }

    pub fn sliding_window_trim(&self, context_id: ContextId) -> InferenceResult<()> {
        let mut contexts = self.contexts.write();
        let ctx = contexts.get_mut(&context_id)
            .ok_or_else(|| InferenceError::ContextError {
                reason: format!("context {} not found", context_id.0),
            })?;
        if let Some(window) = ctx.sliding_window_size {
            while ctx.messages.len() > window {
                ctx.messages.remove(0);
            }
        }
        Ok(())
    }

    pub fn list_contexts(&self) -> Vec<ContextId> {
        self.contexts.read().keys().copied().collect()
    }
}
