use crate::types::{WorldVersion, WorldSnapshot};

/// Maintains the current world state.
pub struct WorldStateManager {
    version: WorldVersion,
    snapshots: Vec<WorldSnapshot>,
    max_snapshots: usize,
}

impl WorldStateManager {
    pub fn new(max_snapshots: usize) -> Self {
        Self {
            version: WorldVersion::initial(),
            snapshots: Vec::new(),
            max_snapshots,
        }
    }

    pub fn advance(&mut self) -> WorldVersion {
        self.version = self.version.next();
        self.version
    }

    pub fn current_version(&self) -> WorldVersion {
        self.version
    }

    pub fn snapshot(&mut self, summary: impl Into<String>) -> WorldSnapshot {
        self.advance();
        let snap = WorldSnapshot::new(self.version, summary);
        self.snapshots.push(snap.clone());
        if self.snapshots.len() > self.max_snapshots {
            self.snapshots.drain(..self.snapshots.len() - self.max_snapshots);
        }
        snap
    }

    pub fn latest_snapshot(&self) -> Option<&WorldSnapshot> {
        self.snapshots.last()
    }

    pub fn snapshots(&self) -> &[WorldSnapshot] {
        &self.snapshots
    }

    pub fn rollback(&mut self, to_version: WorldVersion) -> Option<&WorldSnapshot> {
        self.snapshots.iter().rev().find(|s| s.version == to_version)
    }

    pub fn diff(&self, from: WorldVersion, to: WorldVersion) -> Option<(WorldSnapshot, WorldSnapshot)> {
        let from_snap = self.snapshots.iter().find(|s| s.version == from)?;
        let to_snap = self.snapshots.iter().find(|s| s.version == to)?;
        Some((from_snap.clone(), to_snap.clone()))
    }
}

impl Default for WorldStateManager {
    fn default() -> Self {
        Self::new(1000)
    }
}
