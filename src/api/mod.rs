pub mod conversation;
pub mod world_model;
pub mod memory;
pub mod knowledge;
pub mod planning;
pub mod reasoning;
pub mod workflow;
pub mod agent;
pub mod language;

use serde::{Deserialize, Serialize};

pub use conversation::*;
pub use world_model::*;
pub use memory::*;
pub use knowledge::*;
pub use planning::*;
pub use reasoning::*;
pub use workflow::*;
pub use agent::*;
pub use language::*;

/// API version identifier.
pub const API_VERSION: &str = "v1";

/// Common pagination parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    pub offset: usize,
    pub limit: usize,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self { offset: 0, limit: 50 }
    }
}

/// Paginated response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub items: Vec<T>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
}

/// Common API error response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: u16,
    pub message: String,
    pub details: Option<String>,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for ApiError {}

/// Health status for any subsystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
    pub subsystems: std::collections::HashMap<String, SubsystemHealth>,
}

/// Health of an individual subsystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsystemHealth {
    pub healthy: bool,
    pub latency_ms: Option<f64>,
    pub message: Option<String>,
}

/// The unified Neo API that aggregates all subsystem APIs.
pub struct NeoApi<C, W, M, K, P, R, WF, A, L> {
    pub conversation: C,
    pub world_model: W,
    pub memory: M,
    pub knowledge: K,
    pub planning: P,
    pub reasoning: R,
    pub workflow: WF,
    pub agent: A,
    pub language: L,
}

impl<C, W, M, K, P, R, WF, A, L> NeoApi<C, W, M, K, P, R, WF, A, L> {
    pub fn new(
        conversation: C,
        world_model: W,
        memory: M,
        knowledge: K,
        planning: P,
        reasoning: R,
        workflow: WF,
        agent: A,
        language: L,
    ) -> Self {
        Self { conversation, world_model, memory, knowledge, planning, reasoning, workflow, agent, language }
    }
}
