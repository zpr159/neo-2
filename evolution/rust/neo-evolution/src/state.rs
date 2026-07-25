use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::{EvolutionId, EvolutionPhase, EvolutionStatus, SubsystemTarget};

/// Persistent state for the evolution subsystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionState {
    /// Current phase of the evolution lifecycle.
    pub current_phase: EvolutionPhase,
    /// Status of the evolution subsystem.
    pub status: EvolutionStatus,
    /// ID of the active evolution cycle, if any.
    pub active_cycle_id: Option<EvolutionId>,
    /// Timestamp of the last analysis pass.
    pub last_analysis: Option<DateTime<Utc>>,
    /// Timestamp of the last improvement applied.
    pub last_improvement: Option<DateTime<Utc>>,
    /// Number of completed evolution cycles.
    pub completed_cycles: u64,
    /// Number of failed evolution cycles.
    pub failed_cycles: u64,
    /// Subsystems currently undergoing analysis.
    pub active_subsystems: Vec<SubsystemTarget>,
}

impl Default for EvolutionState {
    fn default() -> Self {
        Self {
            current_phase: EvolutionPhase::Analysis,
            status: EvolutionStatus::Pending,
            active_cycle_id: None,
            last_analysis: None,
            last_improvement: None,
            completed_cycles: 0,
            failed_cycles: 0,
            active_subsystems: Vec::new(),
        }
    }
}

/// A point-in-time snapshot of [`EvolutionState`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionSnapshot {
    /// The captured state.
    pub state: EvolutionState,
    /// When this snapshot was taken.
    pub timestamp: DateTime<Utc>,
}

impl EvolutionSnapshot {
    /// Create a new snapshot from the given state.
    pub fn capture(state: &EvolutionState) -> Self {
        Self {
            state: state.clone(),
            timestamp: Utc::now(),
        }
    }
}
