//! Heartbeat service — periodic health reporting, latency measurement,
//! clock drift estimation, and node liveness tracking.

use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::error::NeoResult;
use crate::types::{NodeId, NodeResources, NodeState};

// ---------------------------------------------------------------------------
// HeartbeatMessage
// ---------------------------------------------------------------------------

/// A heartbeat message sent by a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatMessage {
    /// Sender node ID.
    pub node_id: NodeId,
    /// Sender state.
    pub state: NodeState,
    /// Current resource utilization.
    pub resources: NodeResources,
    /// Monotonic sequence number.
    pub sequence: u64,
    /// Timestamp when the heartbeat was created.
    pub timestamp: DateTime<Utc>,
    /// Local monotonic time (for clock drift estimation).
    pub monotonic_ms: u64,
}

// ---------------------------------------------------------------------------
// HealthReport
// ---------------------------------------------------------------------------

/// Comprehensive health report for a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    /// Node ID.
    pub node_id: NodeId,
    /// Overall health score 0.0 – 1.0.
    pub score: f32,
    /// Current state.
    pub state: NodeState,
    /// Timestamp.
    pub timestamp: DateTime<Utc>,
    /// Measured latency in milliseconds.
    pub latency_ms: f64,
    /// Estimated clock drift in microseconds.
    pub clock_drift_us: i64,
    /// Whether the node is responsive.
    pub responsive: bool,
    /// Health warnings.
    pub warnings: Vec<String>,
    /// Resource utilization.
    pub resources: NodeResources,
}

// ---------------------------------------------------------------------------
// HeartbeatService
// ---------------------------------------------------------------------------

/// Manages heartbeat sending, receiving, and liveness tracking.
pub struct HeartbeatService {
    /// Our own node ID.
    node_id: NodeId,
    /// Heartbeat interval.
    interval: Duration,
    /// Heartbeat timeout (how long before marking suspect).
    timeout: Duration,
    /// Last heartbeat sent time.
    last_sent: RwLock<Option<Instant>>,
    /// Last heartbeat received from each node.
    last_received: RwLock<std::collections::HashMap<NodeId, Instant>>,
    /// Heartbeat sequence counter.
    sequence: std::sync::atomic::AtomicU64,
    /// Latency measurements per node.
    latencies: RwLock<std::collections::HashMap<NodeId, LatencyTracker>>,
    /// Clock drift estimates per node.
    clock_drifts: RwLock<std::collections::HashMap<NodeId, i64>>,
    /// Total heartbeats sent.
    sent_count: std::sync::atomic::AtomicU64,
    /// Total heartbeats received.
    received_count: std::sync::atomic::AtomicU64,
    /// Total timeouts detected.
    timeout_count: std::sync::atomic::AtomicU64,
}

impl HeartbeatService {
    /// Create a new heartbeat service.
    pub fn new(node_id: NodeId, interval: Duration, timeout: Duration) -> Self {
        tracing::info!(
            node_id = %node_id,
            interval_ms = interval.as_millis() as u64,
            timeout_ms = timeout.as_millis() as u64,
            "heartbeat service created"
        );
        Self {
            node_id,
            interval,
            timeout,
            last_sent: RwLock::new(None),
            last_received: RwLock::new(std::collections::HashMap::new()),
            sequence: std::sync::atomic::AtomicU64::new(0),
            latencies: RwLock::new(std::collections::HashMap::new()),
            clock_drifts: RwLock::new(std::collections::HashMap::new()),
            sent_count: std::sync::atomic::AtomicU64::new(0),
            received_count: std::sync::atomic::AtomicU64::new(0),
            timeout_count: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Create a heartbeat message.
    pub fn create_heartbeat(&self, state: NodeState, resources: NodeResources) -> HeartbeatMessage {
        let seq = self
            .sequence
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let now = Instant::now();
        *self.last_sent.write() = Some(now);

        self.sent_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        HeartbeatMessage {
            node_id: self.node_id,
            state,
            resources,
            sequence: seq,
            timestamp: Utc::now(),
            monotonic_ms: now.elapsed().as_millis() as u64,
        }
    }

    /// Record receipt of a heartbeat from another node.
    pub fn record_heartbeat(
        &self,
        from: NodeId,
        message: &HeartbeatMessage,
    ) -> NeoResult<()> {
        self.last_received
            .write()
            .insert(from, Instant::now());

        self.received_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Estimate latency (simplified — in production use round-trip).
        let now = Instant::now();
        {
            let mut latencies = self.latencies.write();
            let entry = latencies
                .entry(from)
                .or_insert_with(LatencyTracker::default);
            entry.record(1.0); // Placeholder for actual RTT measurement.
        }

        // Estimate clock drift.
        let local_monotonic = now.elapsed().as_millis() as u64;
        let drift = message.monotonic_ms as i64 - local_monotonic as i64;
        self.clock_drifts.write().insert(from, drift);

        Ok(())
    }

    /// Check if a node has timed out (no heartbeat within timeout window).
    pub fn is_node_timed_out(&self, node_id: NodeId) -> bool {
        if let Some(last) = self.last_received.read().get(&node_id) {
            last.elapsed() > self.timeout
        } else {
            // Never received a heartbeat — consider timed out.
            true
        }
    }

    /// Get all timed-out node IDs.
    pub fn timed_out_nodes(&self) -> Vec<NodeId> {
        let received = self.last_received.read();
        let mut timed_out = Vec::new();
        for (&node_id, last_time) in received.iter() {
            if last_time.elapsed() > self.timeout {
                timed_out.push(node_id);
                self.timeout_count
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
        timed_out
    }

    /// Check if the heartbeat service should send now.
    pub fn should_send(&self) -> bool {
        match *self.last_sent.read() {
            Some(last) => last.elapsed() >= self.interval,
            None => true,
        }
    }

    /// Get the heartbeat interval.
    pub fn interval(&self) -> Duration {
        self.interval
    }

    /// Get the heartbeat timeout.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Get measured latency for a node.
    pub fn latency(&self, node_id: NodeId) -> Option<f64> {
        self.latencies.read().get(&node_id).map(|l| l.average())
    }

    /// Get clock drift estimate for a node.
    pub fn clock_drift(&self, node_id: NodeId) -> Option<i64> {
        self.clock_drifts.read().get(&node_id).copied()
    }

    /// Build a health report for a node.
    pub fn build_health_report(
        &self,
        node_id: NodeId,
        state: NodeState,
        resources: NodeResources,
    ) -> HealthReport {
        let score = self.calculate_health_score(node_id);
        let latency = self.latency(node_id).unwrap_or(0.0);
        let drift = self.clock_drift(node_id).unwrap_or(0);
        let responsive = !self.is_node_timed_out(node_id);

        let mut warnings = Vec::new();
        if latency > 100.0 {
            warnings.push(format!("high latency: {latency:.1}ms"));
        }
        if drift.abs() > 1000 {
            warnings.push(format!("clock drift: {drift}us"));
        }
        if !responsive {
            warnings.push("not responsive".to_string());
        }

        HealthReport {
            node_id,
            score,
            state,
            timestamp: Utc::now(),
            latency_ms: latency,
            clock_drift_us: drift,
            responsive,
            warnings,
            resources,
        }
    }

    /// Calculate health score based on various factors.
    fn calculate_health_score(&self, node_id: NodeId) -> f32 {
        let mut score: f32 = 1.0;

        // Latency penalty.
        if let Some(latency) = self.latency(node_id) {
            let latency_penalty = ((latency / 1000.0) as f32).min(0.5);
            score -= latency_penalty;
        }

        // Timeout penalty.
        if self.is_node_timed_out(node_id) {
            score *= 0.1;
        }

        score.max(0.0).min(1.0)
    }

    // -- Statistics --

    /// Total heartbeats sent.
    pub fn sent_count(&self) -> u64 {
        self.sent_count.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Total heartbeats received.
    pub fn received_count(&self) -> u64 {
        self.received_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Total timeouts detected.
    pub fn timeout_count(&self) -> u64 {
        self.timeout_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Get heartbeat statistics.
    pub fn stats(&self) -> HeartbeatStats {
        HeartbeatStats {
            sent: self.sent_count(),
            received: self.received_count(),
            timeouts: self.timeout_count(),
            tracked_nodes: self.last_received.read().len(),
        }
    }
}

// ---------------------------------------------------------------------------
// LatencyTracker
// ---------------------------------------------------------------------------

/// Tracks latency measurements with exponential moving average.
#[derive(Debug, Clone, Default)]
struct LatencyTracker {
    /// Exponential moving average.
    ema: f64,
    /// Sample count.
    count: u64,
    /// Minimum observed latency.
    min: f64,
    /// Maximum observed latency.
    max: f64,
}

impl LatencyTracker {
    const ALPHA: f64 = 0.2; // EMA smoothing factor.

    fn record(&mut self, latency_ms: f64) {
        if self.count == 0 {
            self.ema = latency_ms;
            self.min = latency_ms;
            self.max = latency_ms;
        } else {
            self.ema = Self::ALPHA * latency_ms + (1.0 - Self::ALPHA) * self.ema;
            self.min = self.min.min(latency_ms);
            self.max = self.max.max(latency_ms);
        }
        self.count += 1;
    }

    fn average(&self) -> f64 {
        self.ema
    }
}

// ---------------------------------------------------------------------------
// HeartbeatStats
// ---------------------------------------------------------------------------

/// Heartbeat service statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatStats {
    pub sent: u64,
    pub received: u64,
    pub timeouts: u64,
    pub tracked_nodes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::NodeResources;

    #[test]
    fn heartbeat_creation() {
        let node_id = NodeId::new();
        let svc = HeartbeatService::new(node_id, Duration::from_secs(1), Duration::from_secs(5));
        let msg = svc.create_heartbeat(NodeState::Ready, NodeResources::default());
        assert_eq!(msg.node_id, node_id);
        assert_eq!(msg.sequence, 0);
    }

    #[test]
    fn heartbeat_sequence() {
        let svc = HeartbeatService::new(NodeId::new(), Duration::from_secs(1), Duration::from_secs(5));
        let m1 = svc.create_heartbeat(NodeState::Ready, NodeResources::default());
        let m2 = svc.create_heartbeat(NodeState::Busy, NodeResources::default());
        assert_eq!(m1.sequence, 0);
        assert_eq!(m2.sequence, 1);
    }

    #[test]
    fn record_heartbeat() {
        let svc = HeartbeatService::new(NodeId::new(), Duration::from_secs(1), Duration::from_secs(5));
        let from = NodeId::new();
        let msg = HeartbeatMessage {
            node_id: from,
            state: NodeState::Ready,
            resources: NodeResources::default(),
            sequence: 1,
            timestamp: Utc::now(),
            monotonic_ms: 100,
        };
        svc.record_heartbeat(from, &msg).unwrap();
        assert_eq!(svc.received_count(), 1);
    }

    #[test]
    fn timeout_detection() {
        let svc = HeartbeatService::new(
            NodeId::new(),
            Duration::from_millis(10),
            Duration::from_millis(50),
        );
        let from = NodeId::new();
        let msg = HeartbeatMessage {
            node_id: from,
            state: NodeState::Ready,
            resources: NodeResources::default(),
            sequence: 1,
            timestamp: Utc::now(),
            monotonic_ms: 0,
        };
        svc.record_heartbeat(from, &msg).unwrap();
        // Not timed out immediately.
        assert!(!svc.is_node_timed_out(from));
    }

    #[test]
    fn health_report() {
        let svc = HeartbeatService::new(NodeId::new(), Duration::from_secs(1), Duration::from_secs(5));
        let node_id = NodeId::new();
        let report = svc.build_health_report(node_id, NodeState::Ready, NodeResources::default());
        assert!(report.score > 0.0);
        assert!(report.responsive);
    }

    #[test]
    fn stats() {
        let svc = HeartbeatService::new(NodeId::new(), Duration::from_secs(1), Duration::from_secs(5));
        let stats = svc.stats();
        assert_eq!(stats.sent, 0);
        assert_eq!(stats.received, 0);
    }

    #[test]
    fn should_send() {
        let svc = HeartbeatService::new(
            NodeId::new(),
            Duration::from_millis(10),
            Duration::from_secs(5),
        );
        // First call should send.
        assert!(svc.should_send());
    }
}
