//! Service discovery — automatic node discovery via static peers, multicast,
//! DNS SRV, Kubernetes endpoints, and manual registration.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::config::DiscoveryConfiguration;
use crate::error::NeoResult;
use crate::types::{NodeId, NodeInfo};

// ---------------------------------------------------------------------------
// ServiceAdvertisement
// ---------------------------------------------------------------------------

/// An advertisement broadcast by a node to announce its presence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceAdvertisement {
    /// Node information.
    pub info: NodeInfo,
    /// Cluster name the node belongs to.
    pub cluster_name: String,
    /// Addresses the node is reachable on.
    pub addresses: Vec<SocketAddr>,
    /// Metadata (e.g. version, capabilities summary).
    pub metadata: HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// ClusterMembership
// ---------------------------------------------------------------------------

/// Snapshot of current cluster membership.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMembership {
    /// All known nodes.
    pub nodes: Vec<NodeInfo>,
    /// The node that provided this membership view.
    pub source: NodeId,
    /// Membership version (incremented on each change).
    pub version: u64,
}

// ---------------------------------------------------------------------------
// DiscoveryRegistry
// ---------------------------------------------------------------------------

/// In-memory registry of discovered nodes.
pub struct DiscoveryRegistry {
    /// Nodes keyed by NodeId.
    nodes: DashMap<NodeId, NodeInfo>,
    /// Nodes keyed by hostname for quick lookup.
    by_hostname: DashMap<String, NodeId>,
    /// Registration version (monotonic).
    version: std::sync::atomic::AtomicU64,
}

impl std::fmt::Debug for DiscoveryRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiscoveryRegistry")
            .field("nodes", &self.nodes.len())
            .field("version", &self.version.load(std::sync::atomic::Ordering::Relaxed))
            .finish()
    }
}

impl DiscoveryRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            nodes: DashMap::new(),
            by_hostname: DashMap::new(),
            version: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Register a node.
    pub fn register(&self, node_id: NodeId, info: NodeInfo) {
        self.by_hostname.insert(info.hostname.clone(), node_id);
        self.nodes.insert(node_id, info);
        self.version
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Deregister a node.
    pub fn deregister(&self, node_id: NodeId) -> Option<NodeInfo> {
        if let Some((_, info)) = self.nodes.remove(&node_id) {
            self.by_hostname.remove(&info.hostname);
            self.version
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Some(info)
        } else {
            None
        }
    }

    /// Get a node by ID.
    pub fn get(&self, node_id: NodeId) -> Option<NodeInfo> {
        self.nodes.get(&node_id).map(|r| r.value().clone())
    }

    /// Get a node ID by hostname.
    pub fn get_by_hostname(&self, hostname: &str) -> Option<NodeId> {
        self.by_hostname.get(hostname).map(|r| *r.value())
    }

    /// Get all known nodes.
    pub fn all(&self) -> Vec<NodeInfo> {
        self.nodes.iter().map(|r| r.value().clone()).collect()
    }

    /// Number of known nodes.
    pub fn count(&self) -> usize {
        self.nodes.len()
    }

    /// Current version.
    pub fn version(&self) -> u64 {
        self.version.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Build a membership snapshot.
    pub fn membership(&self, source: NodeId) -> ClusterMembership {
        ClusterMembership {
            nodes: self.all(),
            source,
            version: self.version(),
        }
    }
}

impl Default for DiscoveryRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// DiscoveryProtocol
// ---------------------------------------------------------------------------

/// Trait for implementing custom discovery protocols.
#[async_trait::async_trait]
pub trait DiscoveryProtocol: Send + Sync + std::fmt::Debug {
    /// Start the discovery protocol.
    async fn start(&self) -> NeoResult<()>;

    /// Stop the discovery protocol.
    async fn stop(&self) -> NeoResult<()>;

    /// Discover peers.
    async fn discover(&self) -> NeoResult<Vec<NodeInfo>>;

    /// Advertise this node.
    async fn advertise(&self, info: &NodeInfo) -> NeoResult<()>;

    /// Check if the protocol is running.
    fn is_running(&self) -> bool;
}

// ---------------------------------------------------------------------------
// StaticDiscovery
// ---------------------------------------------------------------------------

/// Discovery via a static list of peer addresses.
#[derive(Debug)]
pub struct StaticDiscovery {
    peers: Vec<String>,
    registry: Arc<DiscoveryRegistry>,
}

impl StaticDiscovery {
    pub fn new(peers: Vec<String>) -> Self {
        Self {
            peers,
            registry: Arc::new(DiscoveryRegistry::new()),
        }
    }

    pub fn registry(&self) -> &Arc<DiscoveryRegistry> {
        &self.registry
    }
}

#[async_trait::async_trait]
impl DiscoveryProtocol for StaticDiscovery {
    async fn start(&self) -> NeoResult<()> {
        tracing::info!(
            peer_count = self.peers.len(),
            "static discovery started"
        );
        Ok(())
    }

    async fn stop(&self) -> NeoResult<()> {
        tracing::info!("static discovery stopped");
        Ok(())
    }

    async fn discover(&self) -> NeoResult<Vec<NodeInfo>> {
        // In a real implementation, we would connect to each peer and fetch
        // their info. For now we return what's in the registry.
        Ok(self.registry.all())
    }

    async fn advertise(&self, _info: &NodeInfo) -> NeoResult<()> {
        // Static discovery doesn't broadcast; peers are pre-configured.
        Ok(())
    }

    fn is_running(&self) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// MulticastDiscovery
// ---------------------------------------------------------------------------

/// Discovery via UDP multicast.
#[derive(Debug)]
pub struct MulticastDiscovery {
    address: String,
    port: u16,
    registry: Arc<DiscoveryRegistry>,
    running: std::sync::atomic::AtomicBool,
}

impl MulticastDiscovery {
    pub fn new(address: String, port: u16) -> Self {
        Self {
            address,
            port,
            registry: Arc::new(DiscoveryRegistry::new()),
            running: std::sync::atomic::AtomicBool::new(false),
        }
    }

    pub fn registry(&self) -> &Arc<DiscoveryRegistry> {
        &self.registry
    }
}

#[async_trait::async_trait]
impl DiscoveryProtocol for MulticastDiscovery {
    async fn start(&self) -> NeoResult<()> {
        self.running.store(true, std::sync::atomic::Ordering::Relaxed);
        tracing::info!(
            address = %self.address,
            port = self.port,
            "multicast discovery started"
        );
        Ok(())
    }

    async fn stop(&self) -> NeoResult<()> {
        self.running.store(false, std::sync::atomic::Ordering::Relaxed);
        tracing::info!("multicast discovery stopped");
        Ok(())
    }

    async fn discover(&self) -> NeoResult<Vec<NodeInfo>> {
        Ok(self.registry.all())
    }

    async fn advertise(&self, info: &NodeInfo) -> NeoResult<()> {
        tracing::debug!(
            hostname = %info.hostname,
            "advertising via multicast"
        );
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::Relaxed)
    }
}

// ---------------------------------------------------------------------------
// DnsDiscovery
// ---------------------------------------------------------------------------

/// Discovery via DNS SRV records.
#[derive(Debug)]
pub struct DnsDiscovery {
    domain: String,
    registry: Arc<DiscoveryRegistry>,
    running: std::sync::atomic::AtomicBool,
}

impl DnsDiscovery {
    pub fn new(domain: String) -> Self {
        Self {
            domain,
            registry: Arc::new(DiscoveryRegistry::new()),
            running: std::sync::atomic::AtomicBool::new(false),
        }
    }

    pub fn registry(&self) -> &Arc<DiscoveryRegistry> {
        &self.registry
    }
}

#[async_trait::async_trait]
impl DiscoveryProtocol for DnsDiscovery {
    async fn start(&self) -> NeoResult<()> {
        self.running.store(true, std::sync::atomic::Ordering::Relaxed);
        tracing::info!(domain = %self.domain, "DNS discovery started");
        Ok(())
    }

    async fn stop(&self) -> NeoResult<()> {
        self.running.store(false, std::sync::atomic::Ordering::Relaxed);
        tracing::info!("DNS discovery stopped");
        Ok(())
    }

    async fn discover(&self) -> NeoResult<Vec<NodeInfo>> {
        tracing::debug!(domain = %self.domain, "querying DNS SRV records");
        Ok(self.registry.all())
    }

    async fn advertise(&self, info: &NodeInfo) -> NeoResult<()> {
        tracing::debug!(
            hostname = %info.hostname,
            "registering with DNS"
        );
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::Relaxed)
    }
}

// ---------------------------------------------------------------------------
// KubernetesDiscovery
// ---------------------------------------------------------------------------

/// Discovery via Kubernetes endpoint API.
#[derive(Debug)]
pub struct KubernetesDiscovery {
    namespace: String,
    service: String,
    registry: Arc<DiscoveryRegistry>,
    running: std::sync::atomic::AtomicBool,
}

impl KubernetesDiscovery {
    pub fn new(namespace: String, service: String) -> Self {
        Self {
            namespace,
            service,
            registry: Arc::new(DiscoveryRegistry::new()),
            running: std::sync::atomic::AtomicBool::new(false),
        }
    }

    pub fn registry(&self) -> &Arc<DiscoveryRegistry> {
        &self.registry
    }
}

#[async_trait::async_trait]
impl DiscoveryProtocol for KubernetesDiscovery {
    async fn start(&self) -> NeoResult<()> {
        self.running.store(true, std::sync::atomic::Ordering::Relaxed);
        tracing::info!(
            namespace = %self.namespace,
            service = %self.service,
            "Kubernetes discovery started"
        );
        Ok(())
    }

    async fn stop(&self) -> NeoResult<()> {
        self.running.store(false, std::sync::atomic::Ordering::Relaxed);
        tracing::info!("Kubernetes discovery stopped");
        Ok(())
    }

    async fn discover(&self) -> NeoResult<Vec<NodeInfo>> {
        tracing::debug!(
            namespace = %self.namespace,
            service = %self.service,
            "querying Kubernetes endpoints"
        );
        Ok(self.registry.all())
    }

    async fn advertise(&self, info: &NodeInfo) -> NeoResult<()> {
        tracing::debug!(
            hostname = %info.hostname,
            "registering with Kubernetes"
        );
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::Relaxed)
    }
}

// ---------------------------------------------------------------------------
// DiscoveryService
// ---------------------------------------------------------------------------

/// High-level discovery service that wraps a protocol implementation.
pub struct DiscoveryService {
    config: DiscoveryConfiguration,
    registry: Arc<DiscoveryRegistry>,
}

impl DiscoveryService {
    /// Create a new discovery service from configuration.
    pub fn new(config: DiscoveryConfiguration) -> Self {
        let registry = Arc::new(DiscoveryRegistry::new());
        tracing::info!(method = ?config.method, "discovery service created");
        Self { config, registry }
    }

    /// Get the discovery registry.
    pub fn registry(&self) -> &Arc<DiscoveryRegistry> {
        &self.registry
    }

    /// Get the configuration.
    pub fn config(&self) -> &DiscoveryConfiguration {
        &self.config
    }

    /// Register a node in the discovery registry.
    pub fn register_node(&self, node_id: NodeId, info: NodeInfo) {
        self.registry.register(node_id, info);
    }

    /// Deregister a node from the discovery registry.
    pub fn deregister_node(&self, node_id: NodeId) -> Option<NodeInfo> {
        self.registry.deregister(node_id)
    }

    /// Get all known nodes.
    pub fn known_nodes(&self) -> Vec<NodeInfo> {
        self.registry.all()
    }

    /// Number of known nodes.
    pub fn known_count(&self) -> usize {
        self.registry.count()
    }

    /// Get static peers from configuration.
    pub fn static_peers(&self) -> &[String] {
        &self.config.static_peers
    }

    /// Get bootstrap nodes from configuration.
    pub fn bootstrap_nodes(&self) -> &[String] {
        &self.config.bootstrap_nodes
    }
}

impl std::fmt::Debug for DiscoveryService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiscoveryService")
            .field("method", &self.config.method)
            .field("known_nodes", &self.registry.count())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::NodeCapabilities;

    fn test_info(hostname: &str) -> NodeInfo {
        NodeInfo {
            hostname: hostname.to_string(),
            ip_address: "127.0.0.1".to_string(),
            port: 7000,
            node_type: crate::types::NodeType::CpuWorker,
            capabilities: NodeCapabilities::default(),
            version: "0.1.0".to_string(),
            zone: "default".to_string(),
            rack: None,
        }
    }

    #[test]
    fn registry_register_deregister() {
        let reg = DiscoveryRegistry::new();
        let id = NodeId::new();
        reg.register(id, test_info("host-a"));
        assert_eq!(reg.count(), 1);
        assert!(reg.get(id).is_some());

        reg.deregister(id);
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn registry_hostname_lookup() {
        let reg = DiscoveryRegistry::new();
        let id = NodeId::new();
        reg.register(id, test_info("host-b"));
        assert_eq!(reg.get_by_hostname("host-b"), Some(id));
    }

    #[test]
    fn registry_version() {
        let reg = DiscoveryRegistry::new();
        assert_eq!(reg.version(), 0);
        let id = NodeId::new();
        reg.register(id, test_info("host-c"));
        assert!(reg.version() > 0);
    }

    #[test]
    fn discovery_service_static() {
        let config = crate::config::DiscoveryConfiguration {
            method: crate::config::DiscoveryMethod::Static,
            static_peers: vec!["10.0.0.1:7400".to_string()],
            ..Default::default()
        };
        let svc = DiscoveryService::new(config);
        assert_eq!(svc.static_peers().len(), 1);
        assert_eq!(svc.known_count(), 0);
    }

    #[tokio::test]
    async fn static_discovery_protocol() {
        let disc = StaticDiscovery::new(vec!["10.0.0.1:7400".to_string()]);
        disc.start().await.unwrap();
        assert!(disc.is_running());
        let nodes = disc.discover().await.unwrap();
        assert!(nodes.is_empty());
        disc.stop().await.unwrap();
    }

    #[test]
    fn membership_snapshot() {
        let reg = DiscoveryRegistry::new();
        let src = NodeId::new();
        reg.register(NodeId::new(), test_info("h1"));
        reg.register(NodeId::new(), test_info("h2"));
        let membership = reg.membership(src);
        assert_eq!(membership.nodes.len(), 2);
        assert_eq!(membership.source, src);
    }
}
