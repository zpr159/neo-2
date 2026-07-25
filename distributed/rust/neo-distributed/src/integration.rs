//! Integration bridge connecting the distributed runtime with all existing
//! Neo AGI OS subsystems: Runtime, Executive, Planning, Autonomous Learning,
//! Multimodal Intelligence, Workflow Engine, Agent Framework, Capability
//! Framework, Tool Ecosystem, Memory, Knowledge Graph, and Reasoning Engine.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::cluster::Cluster;
use crate::config::ClusterConfiguration;
use crate::error::NeoResult;
use crate::execution::{ExecutionRequest, ExecutionResponse, ExecutionType};
use crate::memory::DistributedMemory;
use crate::node::NodeManager;
use crate::scheduler::DistributedScheduler;
use crate::types::{NodeId, TaskPriority};

// ---------------------------------------------------------------------------
// SubsystemId
// ---------------------------------------------------------------------------

/// Identifies a Neo subsystem for integration routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubsystemId {
    Runtime,
    Executive,
    Planning,
    AutonomousLearning,
    MultimodalIntelligence,
    WorkflowEngine,
    AgentFramework,
    CapabilityFramework,
    ToolEcosystem,
    Memory,
    KnowledgeGraph,
    ReasoningEngine,
    Distributed,
}

impl std::fmt::Display for SubsystemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Runtime => write!(f, "runtime"),
            Self::Executive => write!(f, "executive"),
            Self::Planning => write!(f, "planning"),
            Self::AutonomousLearning => write!(f, "autonomous_learning"),
            Self::MultimodalIntelligence => write!(f, "multimodal_intelligence"),
            Self::WorkflowEngine => write!(f, "workflow_engine"),
            Self::AgentFramework => write!(f, "agent_framework"),
            Self::CapabilityFramework => write!(f, "capability_framework"),
            Self::ToolEcosystem => write!(f, "tool_ecosystem"),
            Self::Memory => write!(f, "memory"),
            Self::KnowledgeGraph => write!(f, "knowledge_graph"),
            Self::ReasoningEngine => write!(f, "reasoning_engine"),
            Self::Distributed => write!(f, "distributed"),
        }
    }
}

// ---------------------------------------------------------------------------
// SubsystemBridge
// ---------------------------------------------------------------------------

/// Bridge connecting a single subsystem to the distributed runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsystemBridge {
    /// Subsystem identifier.
    pub subsystem: SubsystemId,
    /// Whether the subsystem is cluster-aware.
    pub cluster_aware: bool,
    /// Preferred execution node types.
    pub preferred_nodes: Vec<String>,
    /// Required capabilities.
    pub required_capabilities: Vec<String>,
    /// Whether the subsystem supports remote execution.
    pub supports_remote: bool,
    /// Whether the subsystem supports migration.
    pub supports_migration: bool,
}

impl SubsystemBridge {
    /// Create a new bridge definition.
    pub fn new(subsystem: SubsystemId) -> Self {
        Self {
            subsystem,
            cluster_aware: false,
            preferred_nodes: Vec::new(),
            required_capabilities: Vec::new(),
            supports_remote: false,
            supports_migration: false,
        }
    }

    /// Mark as cluster-aware.
    #[must_use]
    pub fn cluster_aware(mut self) -> Self {
        self.cluster_aware = true;
        self
    }

    /// Set preferred nodes.
    #[must_use]
    pub fn with_preferred_nodes(mut self, nodes: Vec<String>) -> Self {
        self.preferred_nodes = nodes;
        self
    }

    /// Set required capabilities.
    #[must_use]
    pub fn with_capabilities(mut self, caps: Vec<String>) -> Self {
        self.required_capabilities = caps;
        self
    }

    /// Enable remote execution.
    #[must_use]
    pub fn remote_executable(mut self) -> Self {
        self.supports_remote = true;
        self
    }

    /// Enable migration.
    #[must_use]
    pub fn migratable(mut self) -> Self {
        self.supports_migration = true;
        self
    }
}

// ---------------------------------------------------------------------------
// SubsystemRegistry
// ---------------------------------------------------------------------------

/// Registry of all subsystem integrations.
pub struct SubsystemRegistry {
    bridges: parking_lot::RwLock<std::collections::HashMap<SubsystemId, SubsystemBridge>>,
}

impl SubsystemRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            bridges: parking_lot::RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Register a subsystem bridge.
    pub fn register(&self, bridge: SubsystemBridge) {
        tracing::info!(
            subsystem = %bridge.subsystem,
            cluster_aware = bridge.cluster_aware,
            "subsystem registered"
        );
        self.bridges
            .write()
            .insert(bridge.subsystem, bridge);
    }

    /// Get a subsystem bridge.
    pub fn get(&self, subsystem: SubsystemId) -> Option<SubsystemBridge> {
        self.bridges.read().get(&subsystem).cloned()
    }

    /// Get all registered subsystems.
    pub fn all(&self) -> Vec<SubsystemBridge> {
        self.bridges.read().values().cloned().collect()
    }

    /// Get cluster-aware subsystems.
    pub fn cluster_aware(&self) -> Vec<SubsystemBridge> {
        self.bridges
            .read()
            .values()
            .filter(|b| b.cluster_aware)
            .cloned()
            .collect()
    }

    /// Get remote-executable subsystems.
    pub fn remote_executable(&self) -> Vec<SubsystemBridge> {
        self.bridges
            .read()
            .values()
            .filter(|b| b.supports_remote)
            .cloned()
            .collect()
    }

    /// Number of registered subsystems.
    pub fn count(&self) -> usize {
        self.bridges.read().len()
    }
}

impl Default for SubsystemRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// DistributedIntegration
// ---------------------------------------------------------------------------

/// High-level integration manager connecting all Neo subsystems to the
/// distributed runtime.
pub struct DistributedIntegration {
    /// Subsystem registry.
    pub registry: Arc<SubsystemRegistry>,
    /// Cluster reference.
    cluster: Arc<Cluster>,
    /// Scheduler reference.
    scheduler: Arc<DistributedScheduler>,
    /// Distributed memory reference.
    memory: Arc<DistributedMemory>,
}

impl DistributedIntegration {
    /// Create a new integration manager with default subsystem registrations.
    pub fn new(
        cluster: Arc<Cluster>,
        scheduler: Arc<DistributedScheduler>,
        memory: Arc<DistributedMemory>,
    ) -> Self {
        let registry = Arc::new(SubsystemRegistry::new());

        // Register default subsystem bridges.
        let defaults = vec![
            SubsystemBridge::new(SubsystemId::Runtime)
                .cluster_aware()
                .remote_executable()
                .migratable(),
            SubsystemBridge::new(SubsystemId::Executive)
                .cluster_aware()
                .remote_executable()
                .with_capabilities(vec!["planning".to_string()]),
            SubsystemBridge::new(SubsystemId::Planning)
                .cluster_aware()
                .remote_executable()
                .with_capabilities(vec!["planning".to_string()]),
            SubsystemBridge::new(SubsystemId::AutonomousLearning)
                .cluster_aware()
                .remote_executable()
                .with_capabilities(vec!["learning".to_string()]),
            SubsystemBridge::new(SubsystemId::MultimodalIntelligence)
                .cluster_aware()
                .remote_executable()
                .with_capabilities(vec!["ocr".to_string(), "speech".to_string(), "vision".to_string()]),
            SubsystemBridge::new(SubsystemId::WorkflowEngine)
                .cluster_aware()
                .remote_executable()
                .migratable(),
            SubsystemBridge::new(SubsystemId::AgentFramework)
                .cluster_aware()
                .remote_executable()
                .migratable(),
            SubsystemBridge::new(SubsystemId::CapabilityFramework)
                .cluster_aware()
                .remote_executable(),
            SubsystemBridge::new(SubsystemId::ToolEcosystem)
                .cluster_aware()
                .remote_executable()
                .with_preferred_nodes(vec!["edge".to_string(), "cloud".to_string()]),
            SubsystemBridge::new(SubsystemId::Memory)
                .cluster_aware()
                .with_capabilities(vec!["memory".to_string()]),
            SubsystemBridge::new(SubsystemId::KnowledgeGraph)
                .cluster_aware()
                .with_capabilities(vec!["knowledge".to_string()]),
            SubsystemBridge::new(SubsystemId::ReasoningEngine)
                .cluster_aware()
                .remote_executable()
                .with_capabilities(vec!["reasoning".to_string()]),
        ];

        for bridge in defaults {
            registry.register(bridge);
        }

        tracing::info!(
            subsystem_count = registry.count(),
            "distributed integration created"
        );

        Self {
            registry,
            cluster,
            scheduler,
            memory,
        }
    }

    /// Get the subsystem registry.
    pub fn registry(&self) -> &Arc<SubsystemRegistry> {
        &self.registry
    }

    /// Route an execution request to the appropriate subsystem and node.
    pub fn route_execution(&self, request: ExecutionRequest) -> NeoResult<()> {
        // Determine subsystem from execution type.
        let _subsystem = match request.execution_type {
            ExecutionType::Capability => SubsystemId::CapabilityFramework,
            ExecutionType::Workflow => SubsystemId::WorkflowEngine,
            ExecutionType::Planning => SubsystemId::Planning,
            ExecutionType::Inference => SubsystemId::MultimodalIntelligence,
            ExecutionType::Multimodal => SubsystemId::MultimodalIntelligence,
            ExecutionType::Task => SubsystemId::Runtime,
        };

        // Submit to scheduler.
        let task = crate::scheduler::SchedulingTask {
            id: request.id,
            task_type: request.execution_type.to_string(),
            priority: request.priority,
            estimated_duration_ms: request.timeout.as_millis() as u64,
            required_capabilities: request.required_capabilities,
            required_labels: std::collections::HashMap::new(),
            requires_gpu: false,
            min_memory_bytes: 0,
            data: request.payload,
            submitted_at: chrono::Utc::now(),
            deadline: request.deadline,
        };

        self.scheduler.submit_task(task)
    }

    /// Check if a subsystem is cluster-aware.
    pub fn is_cluster_aware(&self, subsystem: SubsystemId) -> bool {
        self.registry
            .get(subsystem)
            .map_or(false, |b| b.cluster_aware)
    }

    /// Get integration statistics.
    pub fn stats(&self) -> IntegrationStats {
        let all = self.registry.all();
        IntegrationStats {
            registered_subsystems: all.len(),
            cluster_aware: all.iter().filter(|b| b.cluster_aware).count(),
            remote_executable: all.iter().filter(|b| b.supports_remote).count(),
            migratable: all.iter().filter(|b| b.supports_migration).count(),
            cluster_nodes: self.cluster.node_count(),
            healthy_nodes: self.cluster.healthy_node_count(),
        }
    }
}

// ---------------------------------------------------------------------------
// IntegrationStats
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationStats {
    pub registered_subsystems: usize,
    pub cluster_aware: usize,
    pub remote_executable: usize,
    pub migratable: usize,
    pub cluster_nodes: usize,
    pub healthy_nodes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ClusterConfiguration;

    fn make_integration() -> DistributedIntegration {
        let config = ClusterConfiguration::testing();
        let cluster = Arc::new(Cluster::new(config));
        let scheduler = Arc::new(DistributedScheduler::new(
            crate::config::SchedulerConfiguration::default(),
        ));
        let memory = Arc::new(DistributedMemory::new(
            crate::config::MemoryConfiguration::default(),
        ));
        DistributedIntegration::new(cluster, scheduler, memory)
    }

    #[test]
    fn register_subsystems() {
        let integration = make_integration();
        assert_eq!(integration.registry().count(), 12);
    }

    #[test]
    fn cluster_aware_check() {
        let integration = make_integration();
        assert!(integration.is_cluster_aware(SubsystemId::Runtime));
        assert!(integration.is_cluster_aware(SubsystemId::AgentFramework));
    }

    #[test]
    fn integration_stats() {
        let integration = make_integration();
        let stats = integration.stats();
        assert_eq!(stats.registered_subsystems, 12);
        assert!(stats.cluster_aware > 0);
    }

    #[test]
    fn subsystem_bridge_builder() {
        let bridge = SubsystemBridge::new(SubsystemId::Planning)
            .cluster_aware()
            .remote_executable()
            .migratable()
            .with_capabilities(vec!["planning".to_string()]);
        assert!(bridge.cluster_aware);
        assert!(bridge.supports_remote);
        assert!(bridge.supports_migration);
    }
}
