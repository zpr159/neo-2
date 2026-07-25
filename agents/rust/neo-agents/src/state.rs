use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentState {
    Initializing,
    Idle,
    Thinking,
    Acting,
    Communicating,
    Learning,
    Suspended,
    Terminated,
    Error,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentMetrics {
    pub uptime_secs: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub goals_completed: u64,
    pub goals_failed: u64,
    pub memory_used_bytes: usize,
    pub cpu_time_ms: u64,
}
