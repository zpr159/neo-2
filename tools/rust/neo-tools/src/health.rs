//! Tool health monitoring.

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::types::HealthStatus;

// ---------------------------------------------------------------------------
// HealthCheckRecord
// ---------------------------------------------------------------------------

/// Record of a health check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckRecord {
    pub tool_name: String,
    pub status: HealthStatus,
    pub message: String,
    pub latency_ms: f64,
    pub checked_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// HealthMonitor
// ---------------------------------------------------------------------------

/// Monitors health of all registered tools.
pub struct HealthMonitor {
    records: DashMap<String, Vec<HealthCheckRecord>>,
    last_check: DashMap<String, DateTime<Utc>>,
    check_interval_secs: u64,
}

impl std::fmt::Debug for HealthMonitor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HealthMonitor")
            .field("check_interval_secs", &self.check_interval_secs)
            .field("tracked_tools", &self.records.len())
            .finish()
    }
}

impl HealthMonitor {
    pub fn new(check_interval_secs: u64) -> Self {
        Self {
            records: DashMap::new(),
            last_check: DashMap::new(),
            check_interval_secs,
        }
    }

    /// Record a health check result.
    pub fn record(&self, record: HealthCheckRecord) {
        self.last_check
            .insert(record.tool_name.clone(), record.checked_at);
        self.records
            .entry(record.tool_name.clone())
            .or_default()
            .push(record);
    }

    /// Get the last health check for a tool.
    pub fn last_check(&self, tool_name: &str) -> Option<HealthCheckRecord> {
        self.records
            .get(tool_name)
            .and_then(|entry| entry.value().last().cloned())
    }

    /// Get all health records for a tool.
    pub fn history(&self, tool_name: &str) -> Vec<HealthCheckRecord> {
        self.records
            .get(tool_name)
            .map(|entry| entry.value().clone())
            .unwrap_or_default()
    }

    /// Get tools that are currently unhealthy.
    pub fn unhealthy_tools(&self) -> Vec<String> {
        self.records
            .iter()
            .filter(|entry| {
                entry
                    .value()
                    .last()
                    .map(|r| r.status == HealthStatus::Unhealthy)
                    .unwrap_or(false)
            })
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Get tools that are degraded.
    pub fn degraded_tools(&self) -> Vec<String> {
        self.records
            .iter()
            .filter(|entry| {
                entry
                    .value()
                    .last()
                    .map(|r| r.status == HealthStatus::Degraded)
                    .unwrap_or(false)
            })
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Get overall health summary.
    pub fn summary(&self) -> HealthSummary {
        let mut healthy = 0;
        let mut degraded = 0;
        let mut unhealthy = 0;
        let mut unknown = 0;

        for entry in self.records.iter() {
            if let Some(last) = entry.value().last() {
                match last.status {
                    HealthStatus::Healthy => healthy += 1,
                    HealthStatus::Degraded => degraded += 1,
                    HealthStatus::Unhealthy => unhealthy += 1,
                    HealthStatus::Unknown => unknown += 1,
                }
            } else {
                unknown += 1;
            }
        }

        HealthSummary {
            total: healthy + degraded + unhealthy + unknown,
            healthy,
            degraded,
            unhealthy,
            unknown,
            check_interval_secs: self.check_interval_secs,
        }
    }

    /// Check if a tool needs a health check based on interval.
    pub fn needs_check(&self, tool_name: &str) -> bool {
        match self.last_check.get(tool_name) {
            Some(last) => {
                let elapsed = Utc::now().signed_duration_since(*last);
                elapsed.num_seconds() >= self.check_interval_secs as i64
            }
            None => true,
        }
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new(60)
    }
}

// ---------------------------------------------------------------------------
// HealthSummary
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSummary {
    pub total: usize,
    pub healthy: usize,
    pub degraded: usize,
    pub unhealthy: usize,
    pub unknown: usize,
    pub check_interval_secs: u64,
}

impl HealthSummary {
    pub fn health_pct(&self) -> f64 {
        if self.total == 0 {
            return 100.0;
        }
        self.healthy as f64 / self.total as f64 * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_monitor() {
        let monitor = HealthMonitor::new(30);
        monitor.record(HealthCheckRecord {
            tool_name: "tool_a".into(),
            status: HealthStatus::Healthy,
            message: "OK".into(),
            latency_ms: 5.0,
            checked_at: Utc::now(),
        });

        let summary = monitor.summary();
        assert_eq!(summary.total, 1);
        assert_eq!(summary.healthy, 1);
        assert!(!monitor.needs_check("tool_a"));
    }
}
