/// Metrics collection types for the Neo AGI Operating System.
///
/// Provides thread-safe metric collection using atomic counters and
/// comprehensive metric types covering all system subsystems.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// System-level metrics describing resource utilization.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemMetrics {
    /// CPU usage as a percentage (0.0 - 100.0).
    pub cpu_usage: f64,
    /// Memory usage in bytes.
    pub memory_usage_bytes: u64,
    /// GPU usage as a percentage (0.0 - 100.0).
    pub gpu_usage: f64,
    /// Disk usage in bytes.
    pub disk_usage_bytes: u64,
}

/// Metrics related to conversation processing.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConversationMetrics {
    /// Number of currently active conversation sessions.
    pub active_sessions: u64,
    /// Total number of conversations processed.
    pub total_conversations: u64,
    /// Current message throughput per second.
    pub messages_per_second: f64,
    /// Average end-to-end latency in milliseconds.
    pub avg_latency_ms: f64,
    /// Total number of tool executions performed.
    pub tool_executions: u64,
}

/// Metrics related to language model operations.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LanguageMetrics {
    /// Health status of each language provider (provider_name, healthy).
    pub provider_health: Vec<(String, bool)>,
    /// Current token generation throughput.
    pub tokens_per_second: f64,
    /// Total API requests made to language providers.
    pub total_requests: u64,
    /// Total failed requests to language providers.
    pub failed_requests: u64,
    /// Average time to first token in milliseconds.
    pub avg_first_token_ms: f64,
}

/// Metrics related to memory retrieval and knowledge operations.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RetrievalMetrics {
    /// Average memory retrieval latency in milliseconds.
    pub memory_retrieval_ms: f64,
    /// Average knowledge base lookup latency in milliseconds.
    pub knowledge_lookup_ms: f64,
    /// Average world model query latency in milliseconds.
    pub world_model_query_ms: f64,
    /// Average context assembly latency in milliseconds.
    pub context_assembly_ms: f64,
}

/// Metrics related to reasoning and inference processes.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReasoningMetrics {
    /// Average reasoning latency in milliseconds.
    pub reasoning_latency_ms: f64,
    /// Average planning latency in milliseconds.
    pub planning_latency_ms: f64,
    /// Number of detected contradictions.
    pub contradictions_detected: u64,
    /// Average number of inference steps per reasoning cycle.
    pub inference_steps: f64,
}

/// Metrics related to workflow execution.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowMetrics {
    /// Number of currently active workflows.
    pub active_workflows: u64,
    /// Total number of successfully completed workflows.
    pub completed_workflows: u64,
    /// Total number of failed workflows.
    pub failed_workflows: u64,
    /// Average workflow execution time in milliseconds.
    pub avg_execution_ms: f64,
}

/// Metrics related to agent operations.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentMetrics {
    /// Number of currently active agents.
    pub active_agents: u64,
    /// Total tasks completed across all agents.
    pub total_tasks_completed: u64,
    /// Agent utilization as a percentage (0.0 - 100.0).
    pub agent_utilization: f64,
}

/// An aggregated snapshot of all metrics at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedMetrics {
    /// Timestamp when these metrics were collected (Unix epoch seconds).
    pub timestamp: u64,
    /// Identifier of the node that produced these metrics.
    pub node_id: String,
    /// System resource metrics.
    pub system: SystemMetrics,
    /// Conversation processing metrics.
    pub conversation: ConversationMetrics,
    /// Language model operation metrics.
    pub language: LanguageMetrics,
    /// Memory retrieval and knowledge metrics.
    pub retrieval: RetrievalMetrics,
    /// Reasoning and inference metrics.
    pub reasoning: ReasoningMetrics,
    /// Workflow execution metrics.
    pub workflow: WorkflowMetrics,
    /// Agent operation metrics.
    pub agent: AgentMetrics,
}

impl Default for AggregatedMetrics {
    fn default() -> Self {
        Self {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            node_id: String::new(),
            system: SystemMetrics::default(),
            conversation: ConversationMetrics::default(),
            language: LanguageMetrics::default(),
            retrieval: RetrievalMetrics::default(),
            reasoning: ReasoningMetrics::default(),
            workflow: WorkflowMetrics::default(),
            agent: AgentMetrics::default(),
        }
    }
}

/// A thread-safe metrics collector using atomic counters.
///
/// All counter fields use atomic types to allow safe concurrent updates
/// from multiple threads without external synchronization.
pub struct MetricsCollector {
    // System metrics
    cpu_usage: AtomicU64,
    memory_usage_bytes: AtomicU64,
    gpu_usage: AtomicU64,
    disk_usage_bytes: AtomicU64,

    // Conversation metrics
    active_sessions: AtomicU64,
    total_conversations: AtomicU64,
    messages_per_second_numerator: AtomicU64,
    messages_per_second_denominator: AtomicU64,
    total_latency_ms: AtomicU64,
    latency_count: AtomicU64,
    tool_executions: AtomicU64,

    // Language metrics
    tokens_per_second_numerator: AtomicU64,
    tokens_per_second_denominator: AtomicU64,
    language_total_requests: AtomicU64,
    language_failed_requests: AtomicU64,
    total_first_token_ms: AtomicU64,
    first_token_count: AtomicU64,

    // Retrieval metrics
    total_memory_retrieval_ms: AtomicU64,
    memory_retrieval_count: AtomicU64,
    total_knowledge_lookup_ms: AtomicU64,
    knowledge_lookup_count: AtomicU64,
    total_world_model_query_ms: AtomicU64,
    world_model_query_count: AtomicU64,
    total_context_assembly_ms: AtomicU64,
    context_assembly_count: AtomicU64,

    // Reasoning metrics
    total_reasoning_latency_ms: AtomicU64,
    reasoning_latency_count: AtomicU64,
    total_planning_latency_ms: AtomicU64,
    planning_latency_count: AtomicU64,
    contradictions_detected: AtomicU64,
    total_inference_steps: AtomicU64,
    inference_steps_count: AtomicU64,

    // Workflow metrics
    active_workflows: AtomicU64,
    completed_workflows: AtomicU64,
    failed_workflows: AtomicU64,
    total_workflow_execution_ms: AtomicU64,
    workflow_execution_count: AtomicU64,

    // Agent metrics
    active_agents: AtomicU64,
    total_tasks_completed: AtomicU64,
    agent_utilization_numerator: AtomicU64,
    agent_utilization_denominator: AtomicU64,

    // State
    enabled: AtomicBool,
}

impl MetricsCollector {
    /// Creates a new `MetricsCollector` with all counters initialized to zero.
    pub fn new() -> Self {
        Self {
            cpu_usage: AtomicU64::new(0),
            memory_usage_bytes: AtomicU64::new(0),
            gpu_usage: AtomicU64::new(0),
            disk_usage_bytes: AtomicU64::new(0),
            active_sessions: AtomicU64::new(0),
            total_conversations: AtomicU64::new(0),
            messages_per_second_numerator: AtomicU64::new(0),
            messages_per_second_denominator: AtomicU64::new(0),
            total_latency_ms: AtomicU64::new(0),
            latency_count: AtomicU64::new(0),
            tool_executions: AtomicU64::new(0),
            tokens_per_second_numerator: AtomicU64::new(0),
            tokens_per_second_denominator: AtomicU64::new(0),
            language_total_requests: AtomicU64::new(0),
            language_failed_requests: AtomicU64::new(0),
            total_first_token_ms: AtomicU64::new(0),
            first_token_count: AtomicU64::new(0),
            total_memory_retrieval_ms: AtomicU64::new(0),
            memory_retrieval_count: AtomicU64::new(0),
            total_knowledge_lookup_ms: AtomicU64::new(0),
            knowledge_lookup_count: AtomicU64::new(0),
            total_world_model_query_ms: AtomicU64::new(0),
            world_model_query_count: AtomicU64::new(0),
            total_context_assembly_ms: AtomicU64::new(0),
            context_assembly_count: AtomicU64::new(0),
            total_reasoning_latency_ms: AtomicU64::new(0),
            reasoning_latency_count: AtomicU64::new(0),
            total_planning_latency_ms: AtomicU64::new(0),
            planning_latency_count: AtomicU64::new(0),
            contradictions_detected: AtomicU64::new(0),
            total_inference_steps: AtomicU64::new(0),
            inference_steps_count: AtomicU64::new(0),
            active_workflows: AtomicU64::new(0),
            completed_workflows: AtomicU64::new(0),
            failed_workflows: AtomicU64::new(0),
            total_workflow_execution_ms: AtomicU64::new(0),
            workflow_execution_count: AtomicU64::new(0),
            active_agents: AtomicU64::new(0),
            total_tasks_completed: AtomicU64::new(0),
            agent_utilization_numerator: AtomicU64::new(0),
            agent_utilization_denominator: AtomicU64::new(0),
            enabled: AtomicBool::new(true),
        }
    }

    /// Enables metrics collection.
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::Relaxed);
    }

    /// Disables metrics collection. All update methods become no-ops.
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Relaxed);
    }

    /// Returns whether metrics collection is currently enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    // -- System metrics --

    /// Records the current CPU usage percentage.
    pub fn set_cpu_usage(&self, usage: f64) {
        if self.is_enabled() {
            self.cpu_usage
                .store((usage * 100.0) as u64, Ordering::Relaxed);
        }
    }

    /// Records the current memory usage in bytes.
    pub fn set_memory_usage(&self, bytes: u64) {
        if self.is_enabled() {
            self.memory_usage_bytes.store(bytes, Ordering::Relaxed);
        }
    }

    /// Records the current GPU usage percentage.
    pub fn set_gpu_usage(&self, usage: f64) {
        if self.is_enabled() {
            self.gpu_usage
                .store((usage * 100.0) as u64, Ordering::Relaxed);
        }
    }

    /// Records the current disk usage in bytes.
    pub fn set_disk_usage(&self, bytes: u64) {
        if self.is_enabled() {
            self.disk_usage_bytes.store(bytes, Ordering::Relaxed);
        }
    }

    // -- Conversation metrics --

    /// Sets the number of active conversation sessions.
    pub fn set_active_sessions(&self, count: u64) {
        if self.is_enabled() {
            self.active_sessions.store(count, Ordering::Relaxed);
        }
    }

    /// Increments the total conversation count by one.
    pub fn record_conversation(&self) {
        if self.is_enabled() {
            self.total_conversations.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Records a message processing latency.
    pub fn record_message_latency(&self, latency_ms: f64) {
        if self.is_enabled() {
            self.total_latency_ms
                .fetch_add(latency_ms as u64, Ordering::Relaxed);
            self.latency_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Sets the current messages-per-second throughput.
    pub fn set_messages_per_second(&self, mps: f64) {
        if self.is_enabled() {
            let scaled = (mps * 1000.0) as u64;
            self.messages_per_second_numerator
                .store(scaled, Ordering::Relaxed);
            self.messages_per_second_denominator
                .store(1000, Ordering::Relaxed);
        }
    }

    /// Increments the tool execution count by one.
    pub fn record_tool_execution(&self) {
        if self.is_enabled() {
            self.tool_executions.fetch_add(1, Ordering::Relaxed);
        }
    }

    // -- Language metrics --

    /// Records a language provider request.
    pub fn record_language_request(&self, succeeded: bool) {
        if self.is_enabled() {
            self.language_total_requests
                .fetch_add(1, Ordering::Relaxed);
            if !succeeded {
                self.language_failed_requests
                    .fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Records the time-to-first-token for a language request.
    pub fn record_first_token_latency(&self, latency_ms: f64) {
        if self.is_enabled() {
            self.total_first_token_ms
                .fetch_add(latency_ms as u64, Ordering::Relaxed);
            self.first_token_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Sets the current token generation throughput.
    pub fn set_tokens_per_second(&self, tps: f64) {
        if self.is_enabled() {
            let scaled = (tps * 1000.0) as u64;
            self.tokens_per_second_numerator
                .store(scaled, Ordering::Relaxed);
            self.tokens_per_second_denominator
                .store(1000, Ordering::Relaxed);
        }
    }

    // -- Retrieval metrics --

    /// Records a memory retrieval operation latency.
    pub fn record_memory_retrieval(&self, latency_ms: f64) {
        if self.is_enabled() {
            self.total_memory_retrieval_ms
                .fetch_add(latency_ms as u64, Ordering::Relaxed);
            self.memory_retrieval_count
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Records a knowledge base lookup latency.
    pub fn record_knowledge_lookup(&self, latency_ms: f64) {
        if self.is_enabled() {
            self.total_knowledge_lookup_ms
                .fetch_add(latency_ms as u64, Ordering::Relaxed);
            self.knowledge_lookup_count
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Records a world model query latency.
    pub fn record_world_model_query(&self, latency_ms: f64) {
        if self.is_enabled() {
            self.total_world_model_query_ms
                .fetch_add(latency_ms as u64, Ordering::Relaxed);
            self.world_model_query_count
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Records a context assembly latency.
    pub fn record_context_assembly(&self, latency_ms: f64) {
        if self.is_enabled() {
            self.total_context_assembly_ms
                .fetch_add(latency_ms as u64, Ordering::Relaxed);
            self.context_assembly_count
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    // -- Reasoning metrics --

    /// Records a reasoning cycle latency.
    pub fn record_reasoning_latency(&self, latency_ms: f64) {
        if self.is_enabled() {
            self.total_reasoning_latency_ms
                .fetch_add(latency_ms as u64, Ordering::Relaxed);
            self.reasoning_latency_count
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Records a planning step latency.
    pub fn record_planning_latency(&self, latency_ms: f64) {
        if self.is_enabled() {
            self.total_planning_latency_ms
                .fetch_add(latency_ms as u64, Ordering::Relaxed);
            self.planning_latency_count
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Increments the contradiction detection counter.
    pub fn record_contradiction(&self) {
        if self.is_enabled() {
            self.contradictions_detected
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Records the number of inference steps for a reasoning cycle.
    pub fn record_inference_steps(&self, steps: u64) {
        if self.is_enabled() {
            self.total_inference_steps
                .fetch_add(steps, Ordering::Relaxed);
            self.inference_steps_count
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    // -- Workflow metrics --

    /// Sets the number of currently active workflows.
    pub fn set_active_workflows(&self, count: u64) {
        if self.is_enabled() {
            self.active_workflows.store(count, Ordering::Relaxed);
        }
    }

    /// Records a completed workflow execution.
    pub fn record_workflow_completion(&self, execution_ms: f64) {
        if self.is_enabled() {
            self.completed_workflows.fetch_add(1, Ordering::Relaxed);
            self.total_workflow_execution_ms
                .fetch_add(execution_ms as u64, Ordering::Relaxed);
            self.workflow_execution_count
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Records a failed workflow execution.
    pub fn record_workflow_failure(&self) {
        if self.is_enabled() {
            self.failed_workflows.fetch_add(1, Ordering::Relaxed);
        }
    }

    // -- Agent metrics --

    /// Sets the number of currently active agents.
    pub fn set_active_agents(&self, count: u64) {
        if self.is_enabled() {
            self.active_agents.store(count, Ordering::Relaxed);
        }
    }

    /// Increments the total completed tasks counter.
    pub fn record_task_completion(&self) {
        if self.is_enabled() {
            self.total_tasks_completed
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Sets the current agent utilization percentage.
    pub fn set_agent_utilization(&self, utilization: f64) {
        if self.is_enabled() {
            let scaled = (utilization * 1000.0) as u64;
            self.agent_utilization_numerator
                .store(scaled, Ordering::Relaxed);
            self.agent_utilization_denominator
                .store(1000, Ordering::Relaxed);
        }
    }

    /// Collects a snapshot of all current metrics into an `AggregatedMetrics`.
    pub fn collect(&self, node_id: String) -> AggregatedMetrics {
        let load = |a: &AtomicU64| a.load(Ordering::Relaxed);
        let load_f64_div = |num: &AtomicU64, den: &AtomicU64| -> f64 {
            let d = den.load(Ordering::Relaxed);
            if d == 0 {
                0.0
            } else {
                load(num) as f64 / d as f64
            }
        };
        let load_avg = |total: &AtomicU64, count: &AtomicU64| -> f64 {
            let c = count.load(Ordering::Relaxed);
            if c == 0 {
                0.0
            } else {
                load(total) as f64 / c as f64
            }
        };

        AggregatedMetrics {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            node_id,
            system: SystemMetrics {
                cpu_usage: load(&self.cpu_usage) as f64 / 100.0,
                memory_usage_bytes: load(&self.memory_usage_bytes),
                gpu_usage: load(&self.gpu_usage) as f64 / 100.0,
                disk_usage_bytes: load(&self.disk_usage_bytes),
            },
            conversation: ConversationMetrics {
                active_sessions: load(&self.active_sessions),
                total_conversations: load(&self.total_conversations),
                messages_per_second: load_f64_div(
                    &self.messages_per_second_numerator,
                    &self.messages_per_second_denominator,
                ),
                avg_latency_ms: load_avg(&self.total_latency_ms, &self.latency_count),
                tool_executions: load(&self.tool_executions),
            },
            language: LanguageMetrics {
                provider_health: Vec::new(),
                tokens_per_second: load_f64_div(
                    &self.tokens_per_second_numerator,
                    &self.tokens_per_second_denominator,
                ),
                total_requests: load(&self.language_total_requests),
                failed_requests: load(&self.language_failed_requests),
                avg_first_token_ms: load_avg(&self.total_first_token_ms, &self.first_token_count),
            },
            retrieval: RetrievalMetrics {
                memory_retrieval_ms: load_avg(
                    &self.total_memory_retrieval_ms,
                    &self.memory_retrieval_count,
                ),
                knowledge_lookup_ms: load_avg(
                    &self.total_knowledge_lookup_ms,
                    &self.knowledge_lookup_count,
                ),
                world_model_query_ms: load_avg(
                    &self.total_world_model_query_ms,
                    &self.world_model_query_count,
                ),
                context_assembly_ms: load_avg(
                    &self.total_context_assembly_ms,
                    &self.context_assembly_count,
                ),
            },
            reasoning: ReasoningMetrics {
                reasoning_latency_ms: load_avg(
                    &self.total_reasoning_latency_ms,
                    &self.reasoning_latency_count,
                ),
                planning_latency_ms: load_avg(
                    &self.total_planning_latency_ms,
                    &self.planning_latency_count,
                ),
                contradictions_detected: load(&self.contradictions_detected),
                inference_steps: load_avg(
                    &self.total_inference_steps,
                    &self.inference_steps_count,
                ),
            },
            workflow: WorkflowMetrics {
                active_workflows: load(&self.active_workflows),
                completed_workflows: load(&self.completed_workflows),
                failed_workflows: load(&self.failed_workflows),
                avg_execution_ms: load_avg(
                    &self.total_workflow_execution_ms,
                    &self.workflow_execution_count,
                ),
            },
            agent: AgentMetrics {
                active_agents: load(&self.active_agents),
                total_tasks_completed: load(&self.total_tasks_completed),
                agent_utilization: load_f64_div(
                    &self.agent_utilization_numerator,
                    &self.agent_utilization_denominator,
                ),
            },
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}
