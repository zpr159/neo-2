use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationConfig {
    pub max_context_messages: usize,
    pub max_tokens_per_response: usize,
    pub timeout_ms: u64,
    pub max_tool_executions_per_turn: usize,
    pub enable_memory_retrieval: bool,
    pub enable_knowledge_retrieval: bool,
    pub enable_world_model: bool,
    pub enable_workflow_execution: bool,
    pub enable_agent_delegation: bool,
    pub auto_consolidate_memory: bool,
    pub enable_response_validation: bool,
    pub max_evidence_items: usize,
    pub confidence_threshold: f32,
    pub ranking_config: RankingConfig,
    pub tool_config: ToolConfig,
    pub distributed_config: DistributedConversationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingConfig {
    pub semantic_weight: f32,
    pub recency_weight: f32,
    pub importance_weight: f32,
    pub confidence_weight: f32,
    pub source_reliability_weight: f32,
    pub user_relevance_weight: f32,
    pub task_relevance_weight: f32,
    pub max_items: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    pub auto_approve_safe_tools: bool,
    pub require_approval_patterns: Vec<String>,
    pub deny_patterns: Vec<String>,
    pub max_concurrent_executions: usize,
    pub default_timeout_ms: u64,
    pub max_retries: usize,
    pub enable_tool_chains: bool,
    pub max_chain_length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedConversationConfig {
    pub enabled: bool,
    pub session_migration_enabled: bool,
    pub shared_memory_read_consistent: bool,
    pub world_state_sync_interval_ms: u64,
    pub node_failover_enabled: bool,
}

impl Default for ConversationConfig {
    fn default() -> Self {
        Self {
            max_context_messages: 50,
            max_tokens_per_response: 4096,
            timeout_ms: 30_000,
            max_tool_executions_per_turn: 10,
            enable_memory_retrieval: true,
            enable_knowledge_retrieval: true,
            enable_world_model: true,
            enable_workflow_execution: true,
            enable_agent_delegation: true,
            auto_consolidate_memory: true,
            enable_response_validation: true,
            max_evidence_items: 20,
            confidence_threshold: 0.3,
            ranking_config: RankingConfig::default(),
            tool_config: ToolConfig::default(),
            distributed_config: DistributedConversationConfig::default(),
        }
    }
}

impl Default for RankingConfig {
    fn default() -> Self {
        Self {
            semantic_weight: 0.3,
            recency_weight: 0.15,
            importance_weight: 0.15,
            confidence_weight: 0.15,
            source_reliability_weight: 0.1,
            user_relevance_weight: 0.05,
            task_relevance_weight: 0.1,
            max_items: 100,
        }
    }
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            auto_approve_safe_tools: true,
            require_approval_patterns: Vec::new(),
            deny_patterns: Vec::new(),
            max_concurrent_executions: 5,
            default_timeout_ms: 30_000,
            max_retries: 3,
            enable_tool_chains: true,
            max_chain_length: 10,
        }
    }
}

impl Default for DistributedConversationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            session_migration_enabled: false,
            shared_memory_read_consistent: true,
            world_state_sync_interval_ms: 5_000,
            node_failover_enabled: false,
        }
    }
}
