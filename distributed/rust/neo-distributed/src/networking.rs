//! Networking layer — transport abstraction, RPC protocol, streaming,
//! compression, and encryption for inter-node communication.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::config::{NetworkingConfiguration, TransportProtocol};
use crate::error::{DistributedError, NeoResult};
use crate::types::NodeId;

// ---------------------------------------------------------------------------
// TransportLayer
// ---------------------------------------------------------------------------

/// Abstraction over network transports.
#[derive(Debug)]
pub struct TransportLayer {
    config: RwLock<NetworkingConfiguration>,
    connections: RwLock<HashMap<NodeId, ConnectionInfo>>,
    /// Active streams.
    active_streams: std::sync::atomic::AtomicUsize,
    /// Total bytes sent.
    bytes_sent: std::sync::atomic::AtomicU64,
    /// Total bytes received.
    bytes_received: std::sync::atomic::AtomicU64,
}

impl TransportLayer {
    /// Create a new transport layer.
    pub fn new(config: NetworkingConfiguration) -> Self {
        tracing::info!(
            protocol = %config.transport,
            max_message_size = config.max_message_size,
            "transport layer created"
        );
        Self {
            config: RwLock::new(config),
            connections: RwLock::new(HashMap::new()),
            active_streams: std::sync::atomic::AtomicUsize::new(0),
            bytes_sent: std::sync::atomic::AtomicU64::new(0),
            bytes_received: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Get the transport protocol.
    pub fn protocol(&self) -> TransportProtocol {
        self.config.read().transport
    }

    /// Connect to a remote node.
    pub async fn connect(&self, node_id: NodeId, addr: SocketAddr) -> NeoResult<()> {
        let config = self.config.read();
        let conn_info = ConnectionInfo {
            addr,
            connected_at: chrono::Utc::now(),
            protocol: config.transport,
            compression: config.compression_enabled,
        };

        tracing::info!(
            node_id = %node_id,
            addr = %addr,
            protocol = %config.transport,
            "connecting to remote node"
        );

        self.connections.write().insert(node_id, conn_info);
        Ok(())
    }

    /// Disconnect from a remote node.
    pub fn disconnect(&self, node_id: NodeId) {
        self.connections.write().remove(&node_id);
        tracing::debug!(node_id = %node_id, "disconnected from node");
    }

    /// Check if connected to a node.
    pub fn is_connected(&self, node_id: NodeId) -> bool {
        self.connections.read().contains_key(&node_id)
    }

    /// Get connection info for a node.
    pub fn connection_info(&self, node_id: NodeId) -> Option<ConnectionInfo> {
        self.connections.read().get(&node_id).cloned()
    }

    /// Get all connected nodes.
    pub fn connected_nodes(&self) -> Vec<NodeId> {
        self.connections.read().keys().copied().collect()
    }

    /// Number of active connections.
    pub fn connection_count(&self) -> usize {
        self.connections.read().len()
    }

    /// Send bytes to a node (stub — real implementation would use TCP/QUIC).
    pub async fn send(&self, node_id: NodeId, data: &[u8]) -> NeoResult<()> {
        if !self.is_connected(node_id) {
            return Err(DistributedError::network(format!(
                "not connected to node {node_id}"
            )));
        }

        self.bytes_sent
            .fetch_add(data.len() as u64, std::sync::atomic::Ordering::Relaxed);

        tracing::trace!(
            node_id = %node_id,
            bytes = data.len(),
            "data sent"
        );

        Ok(())
    }

    /// Receive bytes from a node (stub — real implementation would read from socket).
    pub async fn receive(&self, node_id: NodeId, max_size: usize) -> NeoResult<Vec<u8>> {
        if !self.is_connected(node_id) {
            return Err(DistributedError::network(format!(
                "not connected to node {node_id}"
            )));
        }

        // Stub: return empty buffer.
        self.bytes_received
            .fetch_add(0, std::sync::atomic::Ordering::Relaxed);

        Ok(Vec::new())
    }

    /// Get transport statistics.
    pub fn stats(&self) -> TransportStats {
        TransportStats {
            connections: self.connection_count(),
            active_streams: self
                .active_streams
                .load(std::sync::atomic::Ordering::Relaxed),
            bytes_sent: self.bytes_sent.load(std::sync::atomic::Ordering::Relaxed),
            bytes_received: self
                .bytes_received
                .load(std::sync::atomic::Ordering::Relaxed),
        }
    }

    /// Shutdown the transport layer.
    pub async fn shutdown(&self) -> NeoResult<()> {
        let nodes: Vec<NodeId> = self.connections.read().keys().copied().collect();
        for node_id in nodes {
            self.disconnect(node_id);
        }
        tracing::info!("transport layer shutdown");
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// ConnectionInfo
// ---------------------------------------------------------------------------

/// Information about an active connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    /// Remote address.
    pub addr: SocketAddr,
    /// When the connection was established.
    pub connected_at: chrono::DateTime<chrono::Utc>,
    /// Transport protocol.
    pub protocol: TransportProtocol,
    /// Whether compression is enabled.
    pub compression: bool,
}

// ---------------------------------------------------------------------------
// TransportStats
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportStats {
    pub connections: usize,
    pub active_streams: usize,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

// ---------------------------------------------------------------------------
// RpcProtocol
// ---------------------------------------------------------------------------

/// RPC protocol for request/reply messaging.
#[derive(Debug)]
pub struct RpcProtocol {
    transport: Arc<TransportLayer>,
    /// Pending requests awaiting response.
    pending: RwLock<HashMap<uuid::Uuid, PendingRequest>>,
    /// Request timeout.
    timeout: Duration,
}

struct PendingRequest {
    sender: tokio::sync::oneshot::Sender<Vec<u8>>,
    created_at: std::time::Instant,
}

impl std::fmt::Debug for PendingRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingRequest")
            .field("created_at", &self.created_at)
            .finish_non_exhaustive()
    }
}

impl RpcProtocol {
    pub fn new(transport: Arc<TransportLayer>, timeout: Duration) -> Self {
        Self {
            transport,
            pending: RwLock::new(HashMap::new()),
            timeout,
        }
    }

    /// Send an RPC request and wait for response.
    pub async fn request(&self, node_id: NodeId, data: Vec<u8>) -> NeoResult<Vec<u8>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let request_id = uuid::Uuid::new_v4();

        self.pending.write().insert(
            request_id,
            PendingRequest {
                sender: tx,
                created_at: std::time::Instant::now(),
            },
        );

        // Send the request.
        self.transport.send(node_id, &data).await?;

        // Wait for response with timeout.
        match tokio::time::timeout(self.timeout, rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Err(DistributedError::network("response channel closed")),
            Err(_) => {
                self.pending.write().remove(&request_id);
                Err(DistributedError::timeout("rpc request timed out"))
            }
        }
    }

    /// Handle an incoming response.
    pub fn handle_response(&self, request_id: uuid::Uuid, data: Vec<u8>) -> bool {
        if let Some(pending) = self.pending.write().remove(&request_id) {
            let _ = pending.sender.send(data);
            true
        } else {
            false
        }
    }

    /// Clean up stale pending requests.
    pub fn cleanup_stale(&self) -> usize {
        let now = std::time::Instant::now();
        let mut stale = Vec::new();
        self.pending.read().iter().for_each(|(id, req)| {
            if now.duration_since(req.created_at) > self.timeout {
                stale.push(*id);
            }
        });
        let count = stale.len();
        for id in stale {
            self.pending.write().remove(&id);
        }
        count
    }
}

// ---------------------------------------------------------------------------
// StreamingTransport
// ---------------------------------------------------------------------------

/// Bidirectional streaming transport for large data transfers.
#[derive(Debug)]
pub struct StreamingTransport {
    transport: Arc<TransportLayer>,
    /// Maximum chunk size.
    chunk_size: usize,
    /// Active streams.
    streams: RwLock<HashMap<uuid::Uuid, StreamInfo>>,
}

#[derive(Debug, Clone)]
struct StreamInfo {
    node_id: NodeId,
    total_bytes: u64,
    chunks_sent: u64,
    started_at: std::time::Instant,
}

impl StreamingTransport {
    pub fn new(transport: Arc<TransportLayer>, chunk_size: usize) -> Self {
        Self {
            transport,
            chunk_size,
            streams: RwLock::new(HashMap::new()),
        }
    }

    /// Start a new stream.
    pub fn start_stream(&self, node_id: NodeId) -> uuid::Uuid {
        let stream_id = uuid::Uuid::new_v4();
        self.streams.write().insert(
            stream_id,
            StreamInfo {
                node_id,
                total_bytes: 0,
                chunks_sent: 0,
                started_at: std::time::Instant::now(),
            },
        );
        tracing::debug!(
            stream_id = %stream_id,
            node_id = %node_id,
            "stream started"
        );
        stream_id
    }

    /// Send a chunk in a stream.
    pub async fn send_chunk(&self, stream_id: uuid::Uuid, data: Bytes) -> NeoResult<()> {
        let mut streams = self.streams.write();
        let stream = streams
            .get_mut(&stream_id)
            .ok_or_else(|| DistributedError::network(format!("stream not found: {stream_id}")))?;

        self.transport.send(stream.node_id, &data).await?;

        stream.total_bytes += data.len() as u64;
        stream.chunks_sent += 1;

        Ok(())
    }

    /// Close a stream.
    pub fn close_stream(&self, stream_id: uuid::Uuid) -> Option<StreamInfo> {
        self.streams.write().remove(&stream_id)
    }

    /// Get active stream count.
    pub fn active_streams(&self) -> usize {
        self.streams.read().len()
    }
}

// ---------------------------------------------------------------------------
// Compression utilities
// ---------------------------------------------------------------------------

/// Compression utilities for message payloads.
pub struct Compression;

impl Compression {
    /// Compress data using LZ4-style (stub — would use lz4 crate).
    pub fn compress(data: &[u8]) -> Vec<u8> {
        // Stub: return data as-is. Real impl would use lz4/flate2.
        data.to_vec()
    }

    /// Decompress data.
    pub fn decompress(data: &[u8]) -> Vec<u8> {
        // Stub: return data as-is.
        data.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    fn test_config() -> NetworkingConfiguration {
        NetworkingConfiguration::default()
    }

    #[test]
    fn transport_creation() {
        let transport = TransportLayer::new(test_config());
        assert_eq!(transport.protocol(), TransportProtocol::Tcp);
        assert_eq!(transport.connection_count(), 0);
    }

    #[tokio::test]
    async fn transport_connect_disconnect() {
        let transport = TransportLayer::new(test_config());
        let addr = SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 7400);
        let node_id = NodeId::new();
        transport.connect(node_id, addr).await.unwrap();
        assert!(transport.is_connected(node_id));
        transport.disconnect(node_id);
        assert!(!transport.is_connected(node_id));
    }

    #[test]
    fn rpc_cleanup() {
        let transport = Arc::new(TransportLayer::new(test_config()));
        let rpc = RpcProtocol::new(transport, Duration::from_millis(1));
        let cleaned = rpc.cleanup_stale();
        assert_eq!(cleaned, 0);
    }

    #[test]
    fn streaming_transport() {
        let transport = Arc::new(TransportLayer::new(test_config()));
        let streaming = StreamingTransport::new(transport, 1024);
        let stream_id = streaming.start_stream(NodeId::new());
        assert_eq!(streaming.active_streams(), 1);
        streaming.close_stream(stream_id);
        assert_eq!(streaming.active_streams(), 0);
    }

    #[test]
    fn compression_roundtrip() {
        let data = b"hello, compression!";
        let compressed = Compression::compress(data);
        let decompressed = Compression::decompress(&compressed);
        assert_eq!(data.to_vec(), decompressed);
    }

    #[test]
    fn transport_stats() {
        let transport = TransportLayer::new(test_config());
        let stats = transport.stats();
        assert_eq!(stats.connections, 0);
        assert_eq!(stats.bytes_sent, 0);
    }
}
