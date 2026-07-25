use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::WorldVersion;

/// State of a synchronization peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPeer {
    pub peer_id: String,
    pub last_known_version: WorldVersion,
    pub last_seen: DateTime<Utc>,
    pub is_online: bool,
    pub metadata: HashMap<String, String>,
}

/// A synchronization delta between versions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncDelta {
    pub from_version: WorldVersion,
    pub to_version: WorldVersion,
    pub entity_updates: Vec<serde_json::Value>,
    pub relationship_updates: Vec<serde_json::Value>,
    pub event_updates: Vec<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

/// Manages state synchronization between distributed nodes.
pub struct SynchronizationManager {
    peers: dashmap::DashMap<String, SyncPeer>,
    pending_deltas: Vec<SyncDelta>,
    last_sync: Option<DateTime<Utc>>,
}

impl SynchronizationManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            peers: dashmap::DashMap::new(),
            pending_deltas: Vec::new(),
            last_sync: None,
        }
    }

    pub fn register_peer(&self, peer_id: impl Into<String>) {
        let peer = SyncPeer {
            peer_id: peer_id.into(),
            last_known_version: WorldVersion::initial(),
            last_seen: Utc::now(),
            is_online: true,
            metadata: HashMap::new(),
        };
        self.peers.insert(peer.peer_id.clone(), peer);
    }

    pub fn update_peer(&self, peer_id: &str, version: WorldVersion) {
        if let Some(mut peer) = self.peers.get_mut(peer_id) {
            peer.last_known_version = version;
            peer.last_seen = Utc::now();
            peer.is_online = true;
        }
    }

    pub fn peer_offline(&self, peer_id: &str) {
        if let Some(mut peer) = self.peers.get_mut(peer_id) {
            peer.is_online = false;
        }
    }

    pub fn online_peers(&self) -> Vec<SyncPeer> {
        self.peers
            .iter()
            .filter(|p| p.value().is_online)
            .map(|p| p.value().clone())
            .collect()
    }

    pub fn pending_deltas(&self) -> &[SyncDelta] {
        &self.pending_deltas
    }

    pub fn queue_delta(&mut self, delta: SyncDelta) {
        self.pending_deltas.push(delta);
    }

    pub fn clear_deltas(&mut self) {
        self.pending_deltas.clear();
        self.last_sync = Some(Utc::now());
    }

    pub fn last_sync(&self) -> Option<DateTime<Utc>> {
        self.last_sync
    }

    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }
}

impl Default for SynchronizationManager {
    fn default() -> Self {
        Self::new()
    }
}
