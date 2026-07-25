//! Tool analytics: execution, failure, performance, and aggregate metrics.

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::ToolVersion;

// ---------------------------------------------------------------------------
// ExecutionRecord
// ---------------------------------------------------------------------------

/// Record of a single tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub execution_id: String,
    pub tool_name: String,
    pub operation: String,
    pub success: bool,
    pub duration_ms: u64,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub error: Option<String>,
    pub retry_count: u32,
    pub caller_id: String,
    pub input_size_bytes: Option<u64>,
    pub output_size_bytes: Option<u64>,
}

// ---------------------------------------------------------------------------
// ToolAnalytics
// ---------------------------------------------------------------------------

/// Analytics engine for tool execution data.
pub struct ToolAnalytics {
    records: DashMap<String, Vec<ExecutionRecord>>,
    global_records: DashVec,
    tool_versions: DashMap<String, ToolVersion>,
}

impl std::fmt::Debug for ToolAnalytics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolAnalytics")
            .field("tool_count", &self.records.len())
            .finish()
    }
}

struct DashVec {
    records: parking_lot::RwLock<Vec<ExecutionRecord>>,
}

impl DashVec {
    fn new() -> Self {
        Self {
            records: parking_lot::RwLock::new(Vec::new()),
        }
    }

    fn push(&self, record: ExecutionRecord) {
        self.records.write().push(record);
    }

    fn len(&self) -> usize {
        self.records.read().len()
    }

    fn snapshot(&self) -> Vec<ExecutionRecord> {
        self.records.read().clone()
    }

    fn last_n(&self, n: usize) -> Vec<ExecutionRecord> {
        let r = self.records.read();
        r.iter().rev().take(n).cloned().collect()
    }
}

impl ToolAnalytics {
    pub fn new() -> Self {
        Self {
            records: DashMap::new(),
            global_records: DashVec::new(),
            tool_versions: DashMap::new(),
        }
    }

    /// Record an execution.
    pub fn record(&self, record: ExecutionRecord) {
        self.global_records.push(record.clone());
        self.records
            .entry(record.tool_name.clone())
            .or_default()
            .push(record);
    }

    /// Get all records for a tool.
    pub fn records_for(&self, tool_name: &str) -> Vec<ExecutionRecord> {
        self.records
            .get(tool_name)
            .map(|entry| entry.value().clone())
            .unwrap_or_default()
    }

    /// Get the last N global records.
    pub fn recent(&self, n: usize) -> Vec<ExecutionRecord> {
        self.global_records.last_n(n)
    }

    /// Get total execution count.
    pub fn total_executions(&self) -> usize {
        self.global_records.len()
    }

    /// Get failure count for a tool.
    pub fn failure_count(&self, tool_name: &str) -> usize {
        self.records
            .get(tool_name)
            .map(|entry| entry.value().iter().filter(|r| !r.success).count())
            .unwrap_or(0)
    }

    /// Get success rate for a tool.
    pub fn success_rate(&self, tool_name: &str) -> f64 {
        let records = self.records_for(tool_name);
        if records.is_empty() {
            return 0.0;
        }
        let successes = records.iter().filter(|r| r.success).count();
        successes as f64 / records.len() as f64
    }

    /// Get average latency for a tool.
    pub fn avg_latency_ms(&self, tool_name: &str) -> f64 {
        let records = self.records_for(tool_name);
        if records.is_empty() {
            return 0.0;
        }
        let total: u64 = records.iter().map(|r| r.duration_ms).sum();
        total as f64 / records.len() as f64
    }

    /// Get p95 latency for a tool.
    pub fn p95_latency_ms(&self, tool_name: &str) -> f64 {
        let mut records = self.records_for(tool_name);
        if records.is_empty() {
            return 0.0;
        }
        records.sort_by_key(|r| r.duration_ms);
        let idx = (records.len() as f64 * 0.95) as usize;
        records[idx.min(records.len() - 1)].duration_ms as f64
    }

    /// Aggregate analytics for all tools.
    pub fn aggregate(&self) -> AggregateAnalytics {
        let total = self.total_executions();
        let all_records = self.global_records.snapshot();
        let successes = all_records.iter().filter(|r| r.success).count();
        let total_duration: u64 = all_records.iter().map(|r| r.duration_ms).sum();

        let mut tool_count: HashMap<String, usize> = HashMap::new();
        for r in &all_records {
            *tool_count.entry(r.tool_name.clone()).or_default() += 1;
        }

        AggregateAnalytics {
            total_executions: total,
            total_successes: successes,
            total_failures: total - successes,
            avg_latency_ms: if total > 0 {
                total_duration as f64 / total as f64
            } else {
                0.0
            },
            executions_by_tool: tool_count,
        }
    }

    /// Get tool version tracking.
    pub fn set_version(&self, tool_name: impl Into<String>, version: ToolVersion) {
        self.tool_versions.insert(tool_name.into(), version);
    }

    pub fn version(&self, tool_name: &str) -> Option<ToolVersion> {
        self.tool_versions
            .get(tool_name)
            .map(|entry| entry.value().clone())
    }
}

impl Default for ToolAnalytics {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// AggregateAnalytics
// ---------------------------------------------------------------------------

/// Aggregate analytics across all tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateAnalytics {
    pub total_executions: usize,
    pub total_successes: usize,
    pub total_failures: usize,
    pub avg_latency_ms: f64,
    pub executions_by_tool: HashMap<String, usize>,
}

impl AggregateAnalytics {
    pub fn success_rate(&self) -> f64 {
        if self.total_executions == 0 {
            return 0.0;
        }
        self.total_successes as f64 / self.total_executions as f64
    }
}

// ---------------------------------------------------------------------------
// FailureAnalyzer
// ---------------------------------------------------------------------------

/// Analyzes failure patterns across tool executions.
pub struct FailureAnalyzer {
    analytics: std::sync::Arc<ToolAnalytics>,
}

impl std::fmt::Debug for FailureAnalyzer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FailureAnalyzer").finish()
    }
}

impl FailureAnalyzer {
    pub fn new(analytics: std::sync::Arc<ToolAnalytics>) -> Self {
        Self { analytics }
    }

    /// Get most frequent failure types for a tool.
    pub fn top_failures(&self, tool_name: &str, limit: usize) -> Vec<(String, usize)> {
        let records = self.analytics.records_for(tool_name);
        let mut error_counts: HashMap<String, usize> = HashMap::new();
        for r in &records {
            if !r.success {
                if let Some(ref err) = r.error {
                    *error_counts.entry(err.clone()).or_default() += 1;
                }
            }
        }
        let mut failures: Vec<(String, usize)> = error_counts.into_iter().collect();
        failures.sort_by(|a, b| b.1.cmp(&a.1));
        failures.into_iter().take(limit).collect()
    }

    /// Check if a tool has degraded reliability (success rate below threshold).
    pub fn is_degraded(&self, tool_name: &str, threshold: f64) -> bool {
        let records = self.analytics.records_for(tool_name);
        if records.is_empty() {
            return false;
        }
        let rate = self.analytics.success_rate(tool_name);
        rate < threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record(tool: &str, success: bool, duration: u64) -> ExecutionRecord {
        ExecutionRecord {
            execution_id: uuid::Uuid::new_v4().to_string(),
            tool_name: tool.to_string(),
            operation: "test".into(),
            success,
            duration_ms: duration,
            started_at: Utc::now(),
            finished_at: Utc::now(),
            error: if success { None } else { Some("error".into()) },
            retry_count: 0,
            caller_id: "test".into(),
            input_size_bytes: None,
            output_size_bytes: None,
        }
    }

    #[test]
    fn test_analytics_record() {
        let analytics = ToolAnalytics::new();
        analytics.record(make_record("tool_a", true, 100));
        analytics.record(make_record("tool_a", true, 200));
        analytics.record(make_record("tool_a", false, 300));

        assert_eq!(analytics.total_executions(), 3);
        assert!((analytics.success_rate("tool_a") - 0.6667).abs() < 0.01);
    }

    #[test]
    fn test_failure_analyzer() {
        let analytics = std::sync::Arc::new(ToolAnalytics::new());
        analytics.record(make_record("tool_a", false, 100));
        analytics.record(make_record("tool_a", false, 100));

        let analyzer = FailureAnalyzer::new(analytics);
        assert!(analyzer.is_degraded("tool_a", 0.9));
        assert!(!analyzer.is_degraded("tool_b", 0.9));
    }
}
