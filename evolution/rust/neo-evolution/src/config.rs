use serde::{Deserialize, Serialize};

use crate::types::RiskLevel;

/// Top-level configuration for the evolution subsystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionConfiguration {
    /// Maximum number of evolution cycles to run concurrently.
    pub max_concurrent_cycles: usize,
    /// Global risk threshold — evolutions above this level require approval.
    pub risk_threshold: RiskLevel,
    /// Maximum number of historical analysis records to retain.
    pub analysis_history_limit: usize,
    /// Interval in seconds between automated analysis sweeps.
    pub analysis_interval_secs: u64,
    /// Enable verbose tracing for evolution internals.
    pub verbose_logging: bool,
    /// Sandbox mode — simulate changes without applying them.
    pub sandbox_mode: bool,
    /// Maximum number of improvement proposals per cycle.
    pub max_improvements_per_cycle: usize,
    /// Timeout in seconds for a single analysis pass.
    pub analysis_timeout_secs: u64,
}

impl Default for EvolutionConfiguration {
    fn default() -> Self {
        Self {
            max_concurrent_cycles: 4,
            risk_threshold: RiskLevel::High,
            analysis_history_limit: 1000,
            analysis_interval_secs: 300,
            verbose_logging: false,
            sandbox_mode: true,
            max_improvements_per_cycle: 10,
            analysis_timeout_secs: 60,
        }
    }
}
