use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A snapshot of graph growth metrics at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthSnapshot {
    pub timestamp: DateTime<Utc>,
    pub entity_count: usize,
    pub relation_count: usize,
}

/// Tracks knowledge graph growth over time.
pub struct GrowthTracker {
    snapshots: parking_lot::RwLock<Vec<GrowthSnapshot>>,
}

impl GrowthTracker {
    /// Create a new growth tracker.
    #[must_use]
    pub fn new() -> Self {
        Self {
            snapshots: parking_lot::RwLock::new(Vec::new()),
        }
    }

    /// Record a growth snapshot.
    pub fn record(&self, entity_count: usize, relation_count: usize) {
        self.snapshots.write().push(GrowthSnapshot {
            timestamp: Utc::now(),
            entity_count,
            relation_count,
        });
    }

    /// Get all snapshots.
    #[must_use]
    pub fn snapshots(&self) -> Vec<GrowthSnapshot> {
        self.snapshots.read().clone()
    }

    /// Compute growth rate (entities per hour) over the tracking period.
    #[must_use]
    pub fn growth_rate(&self) -> f64 {
        let snapshots = self.snapshots.read();
        if snapshots.len() < 2 {
            return 0.0;
        }

        let first = &snapshots[0];
        let last = &snapshots[snapshots.len() - 1];

        let entity_diff = last.entity_count as f64 - first.entity_count as f64;
        let time_diff_hours = last
            .timestamp
            .signed_duration_since(first.timestamp)
            .num_minutes() as f64
            / 60.0;

        if time_diff_hours > 0.0 {
            entity_diff / time_diff_hours
        } else {
            0.0
        }
    }

    /// Get the latest entity count.
    #[must_use]
    pub fn current_count(&self) -> Option<(usize, usize)> {
        self.snapshots
            .read()
            .last()
            .map(|s| (s.entity_count, s.relation_count))
    }
}

impl Default for GrowthTracker {
    fn default() -> Self {
        Self::new()
    }
}
