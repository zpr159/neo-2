//! # Neo Distributed
//!
//! Distributed computing and cluster management for the Neo AGI Operating System.
//!
//! This crate provides a production-grade distributed runtime capable of:
//!
//! - **Multi-node execution** — coordinate workloads across a cluster
//! - **Remote agents** — migrate and replicate agents across nodes
//! - **Distributed workflows** — execute DAG workflows across nodes
//! - **Distributed planning** — coordinate planning across the cluster
//! - **Distributed memory** — shared memory with replication and sharding
//! - **Distributed knowledge** — graph partitioning and distributed traversal
//! - **Distributed inference** — route inference to GPU/capability nodes
//! - **High availability** — leader election, failover, and recovery
//! - **Horizontal scaling** — add/remove nodes dynamically
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use neo_distributed::sdk::DistributedRuntime;
//! use neo_distributed::config::ClusterConfiguration;
//!
//! # async fn example() -> neo_distributed::error::NeoResult<()> {
//! let runtime = DistributedRuntime::builder()
//!     .config(ClusterConfiguration::testing())
//!     .enable_discovery(true)
//!     .enable_replication(true)
//!     .enable_failover(true)
//!     .build()?;
//!
//! runtime.cluster.add_node(neo_distributed::types::NodeInfo {
//!     hostname: "node-1".to_string(),
//!     ip_address: "127.0.0.1".to_string(),
//!     port: 7400,
//!     node_type: neo_distributed::types::NodeType::CpuWorker,
//!     capabilities: neo_distributed::types::NodeCapabilities::default(),
//!     version: "0.1.0".to_string(),
//!     zone: "default".to_string(),
//!     rack: None,
//! })?;
//!
//! # Ok(())
//! # }
//! ```

pub mod cluster;
pub mod config;
pub mod discovery;
pub mod error;
pub mod event_bus;
pub mod execution;
pub mod failure;
pub mod heartbeat;
pub mod integration;
pub mod knowledge;
pub mod memory;
pub mod message;
pub mod monitoring;
pub mod networking;
pub mod node;
pub mod scheduler;
pub mod security;
pub mod storage;
pub mod types;
pub mod agents;
pub mod api;
pub mod consensus;
pub mod sdk;

// Re-export primary types for convenience.
pub use cluster::Cluster;
pub use config::ClusterConfiguration;
pub use error::{DistributedError, NeoResult, RecoveryAction};
pub use types::{
    ClusterMetadata, ClusterState, NodeCapabilities, NodeHealth, NodeId, NodeInfo,
    NodeResources, NodeState, NodeType, TaskPriority,
};
pub use node::NodeManager;
pub use scheduler::DistributedScheduler;
pub use discovery::DiscoveryService;
pub use consensus::ConsensusEngine;
pub use heartbeat::HeartbeatService;
pub use failure::FailureDetector;
pub use execution::RemoteExecutionEngine;
pub use memory::DistributedMemory;
pub use knowledge::DistributedKnowledgeGraph;
pub use event_bus::DistributedEventBus;
pub use agents::DistributedAgentManager;
pub use security::ClusterSecurity;
pub use networking::TransportLayer;
pub use storage::DistributedRepository;
pub use monitoring::ResourceMonitor;
pub use api::DistributedApi;
pub use sdk::DistributedRuntime;
pub use integration::DistributedIntegration;
