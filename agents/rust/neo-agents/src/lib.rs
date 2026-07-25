//! # Neo Agents
//!
//! Enterprise-grade autonomous agent framework for the Neo AGI Operating System.
//!
//! This crate provides a complete agent lifecycle management system including:
//! - Agent creation, configuration, and lifecycle management
//! - Inter-agent communication (messaging, broadcasts, conversations)
//! - Task scheduling and execution
//! - Supervisor system with health monitoring and failure recovery
//! - Shared context and collaborative workspaces
//! - Agent memory (working, episodic, long-term, procedural)
//! - Resource management and quota enforcement
//! - Event system integration
//! - Persistence and analytics
//! - SDK builders and REST API surface
//!
//! # Architecture
//!
//! The Agent Framework is Neo's primary execution layer. The Executive issues
//! goals to agents, which decide how to plan, select capabilities, generate
//! workflows, and execute strategies.
//!
//! ## Core Components
//!
//! - **Agent**: The fundamental execution unit with a full lifecycle state machine
//! - **AgentManager**: Central orchestrator for creating, managing, and coordinating agents
//! - **AgentRegistry**: Thread-safe registry of all agents with indexing
//! - **TaskScheduler**: Priority-aware task scheduling with dependency resolution
//! - **SupervisorAgent**: Monitors agent health and handles failure recovery
//! - **SharedWorkspace**: Collaborative workspace for inter-agent context sharing
//!
//! # Example
//!
//! ```no_run
//! use neo_agents::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a manager
//!     let manager = AgentManager::builder()
//!         .with_max_agents(100)
//!         .build();
//!
//!     // Create an agent
//!     let agent = Agent::builder()
//!         .name("Research Agent")
//!         .role(AgentRole::Researcher)
//!         .agent_type(AgentType::Deliberative)
//!         .description("Gathers information from external sources")
//!         .max_concurrent_tasks(4)
//!         .build();
//!
//!     // Register and start the agent
//!     let id = manager.create_agent(agent.config().clone()).await?;
//!     manager.start_agent(id).await?;
//!
//!     // Create and submit a task
//!     let task = Task::builder()
//!         .name("Research AI Safety")
//!         .description("Gather latest papers on AI safety")
//!         .priority(TaskPriority::High)
//!         .build();
//!
//!     Ok(())
//! }
//! ```

pub mod agent;
pub mod analytics;
pub mod api;
pub mod cli;
pub mod communication;
pub mod error;
pub mod events;
pub mod manager;
pub mod memory;
pub mod persistence;
pub mod resources;
pub mod sdk;
pub mod shared_context;
pub mod supervisor;
pub mod task;
pub mod types;

// Re-export primary types at crate root
pub use agent::{Agent, AgentCommand, AgentContext, AgentRuntimeHandle};
pub use analytics::{
    AgentAnalytics, CommunicationAnalytics, PerformanceAnalytics, ResourceAnalytics,
    SystemAnalytics, TaskAnalytics,
};
pub use api::{AgentApi, CreateAgentRequest, CreateAgentResponse, ListAgentsResponse};
pub use cli::AgentCli;
pub use communication::{
    AgentMessage, ConversationManager, MessageChannel, MessageChannelRegistry, MessageEnvelope,
    MessageQueue, MessageType,
};
pub use error::{AgentError, AgentErrorCode, AgentResult};
pub use events::{AgentEvent, AgentEventBus, EventRecorder};
pub use manager::{AgentManager, AgentManagerBuilder, AgentRegistry, AgentRuntime};
pub use memory::{AgentMemory, MemoryEntry, MemoryId, MemoryTier};
pub use persistence::{AgentPersistence, PersistenceConfig};
pub use resources::{AgentLimits, AgentQuota, ResourceManager, ResourceReservation, ResourceType};
pub use sdk::{AgentBuilder, TaskBuilder};
pub use shared_context::{
    ContextSnapshot, ContextVersion, SharedBlackboard, SharedContext, SharedWorkspace,
    WorkingMemory,
};
pub use supervisor::{
    FailureDetector, HealthCheck, HealthManager, LoadBalancer, RecoveryManager, RecoveryStrategy,
    SupervisorAgent, SupervisorAlert,
};
pub use task::{Task, TaskId, TaskQueue, TaskResult, TaskScheduler, TaskStatus};
pub use types::{
    AgentConfiguration, AgentHealth, AgentId, AgentMetadata, AgentMetrics, AgentRole,
    AgentSnapshot, AgentStatistics, AgentStatus, AgentType, AgentVersion, Conversation,
    ConversationId, MessagePriority, TaskPriority,
};

/// Convenience prelude module with all commonly used types.
pub mod prelude {
    pub use crate::agent::{Agent, AgentCommand, AgentContext, AgentRuntimeHandle};
    pub use crate::communication::{
        AgentMessage, MessageChannel, MessageEnvelope, MessageQueue, MessageType,
    };
    pub use crate::error::{AgentError, AgentResult};
    pub use crate::events::{AgentEvent, AgentEventBus, EventRecorder};
    pub use crate::manager::{AgentManager, AgentRegistry};
    pub use crate::memory::{AgentMemory, MemoryEntry, MemoryTier};
    pub use crate::resources::{ResourceManager, ResourceType};
    pub use crate::sdk::{AgentBuilder, TaskBuilder};
    pub use crate::shared_context::{SharedContext, SharedWorkspace, WorkingMemory};
    pub use crate::supervisor::{HealthManager, LoadBalancer, RecoveryManager, SupervisorAgent};
    pub use crate::task::{Task, TaskId, TaskResult, TaskScheduler, TaskStatus};
    pub use crate::types::{
        AgentConfiguration, AgentHealth, AgentId, AgentMetadata, AgentMetrics, AgentRole,
        AgentSnapshot, AgentStatistics, AgentStatus, AgentType, AgentVersion, Conversation,
        ConversationId, MessagePriority, TaskPriority,
    };
}
