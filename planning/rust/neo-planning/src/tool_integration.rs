//! Tool integration for the planning system.
//!
//! Maps planning tasks to available tools, enabling the planner to
//! determine which tools are needed for each task and select the best
//! tool when multiple options exist.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{PlanningError, PlanningResult};
use crate::types::ResourceRequirements;

// ---------------------------------------------------------------------------
// ToolStatus
// ---------------------------------------------------------------------------

/// Whether a tool is currently operational.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolStatus {
    Ready,
    Busy,
    Offline,
    Maintenance,
}

impl Default for ToolStatus {
    fn default() -> Self {
        Self::Ready
    }
}

// ---------------------------------------------------------------------------
// ToolInfo
// ---------------------------------------------------------------------------

/// Describes a single tool available to the planning system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub status: ToolStatus,
    pub tags: Vec<String>,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub resource_cost: ResourceRequirements,
    pub estimated_duration_ms: u64,
    pub success_rate: f64,
    pub invocation_count: u64,
    pub metadata: HashMap<String, serde_json::Value>,
    pub registered_at: DateTime<Utc>,
}

impl ToolInfo {
    /// Create a new tool.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            version: "1.0.0".to_string(),
            status: ToolStatus::Ready,
            tags: Vec::new(),
            input_schema: None,
            output_schema: None,
            resource_cost: ResourceRequirements::default(),
            estimated_duration_ms: 0,
            success_rate: 1.0,
            invocation_count: 0,
            metadata: HashMap::new(),
            registered_at: Utc::now(),
        }
    }

    /// Set the description.
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set the version.
    #[must_use]
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Set the status.
    #[must_use]
    pub fn with_status(mut self, status: ToolStatus) -> Self {
        self.status = status;
        self
    }

    /// Add a tag.
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set the input schema.
    #[must_use]
    pub fn with_input_schema(mut self, schema: serde_json::Value) -> Self {
        self.input_schema = Some(schema);
        self
    }

    /// Set the output schema.
    #[must_use]
    pub fn with_output_schema(mut self, schema: serde_json::Value) -> Self {
        self.output_schema = Some(schema);
        self
    }

    /// Set resource cost.
    #[must_use]
    pub fn with_resource_cost(mut self, cost: ResourceRequirements) -> Self {
        self.resource_cost = cost;
        self
    }

    /// Set estimated duration.
    #[must_use]
    pub fn with_duration(mut self, ms: u64) -> Self {
        self.estimated_duration_ms = ms;
        self
    }

    /// Set success rate.
    #[must_use]
    pub fn with_success_rate(mut self, rate: f64) -> Self {
        self.success_rate = rate.clamp(0.0, 1.0);
        self
    }

    /// Add metadata.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Check whether the tool is ready for use.
    pub fn is_ready(&self) -> bool {
        self.status == ToolStatus::Ready
    }
}

// ---------------------------------------------------------------------------
// ToolSelector
// ---------------------------------------------------------------------------

/// Selects the best tool for a given task from a pool of available tools.
#[derive(Debug, Clone)]
pub struct ToolSelector {
    /// Weight for success rate in scoring.
    pub success_rate_weight: f64,
    /// Weight for speed (lower duration is better).
    pub speed_weight: f64,
    /// Weight for resource cost (lower is better).
    pub cost_weight: f64,
}

impl Default for ToolSelector {
    fn default() -> Self {
        Self {
            success_rate_weight: 0.5,
            speed_weight: 0.3,
            cost_weight: 0.2,
        }
    }
}

impl ToolSelector {
    /// Create a new selector with default weights.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the success rate weight.
    #[must_use]
    pub fn with_success_rate_weight(mut self, w: f64) -> Self {
        self.success_rate_weight = w;
        self
    }

    /// Set the speed weight.
    #[must_use]
    pub fn with_speed_weight(mut self, w: f64) -> Self {
        self.speed_weight = w;
        self
    }

    /// Set the cost weight.
    #[must_use]
    pub fn with_cost_weight(mut self, w: f64) -> Self {
        self.cost_weight = w;
        self
    }

    /// Score a tool for the given requirements.
    ///
    /// Returns a value in `[0.0, 1.0]` where higher is better.
    pub fn score_tool(&self, tool: &ToolInfo, requirements: &ResourceRequirements) -> f64 {
        let sr_score = tool.success_rate;

        let speed_score = if tool.estimated_duration_ms > 0 {
            (10000.0 / tool.estimated_duration_ms as f64).min(1.0)
        } else {
            1.0
        };

        let total_req = requirements.cpu_units as f64
            + requirements.memory_mb as f64 / 256.0
            + requirements.storage_mb as f64 / 1024.0;
        let total_cost = tool.resource_cost.cpu_units as f64
            + tool.resource_cost.memory_mb as f64 / 256.0
            + tool.resource_cost.storage_mb as f64 / 1024.0;
        let cost_score = if total_req > 0.0 {
            (1.0 - (total_cost / total_req)).clamp(0.0, 1.0)
        } else {
            0.5
        };

        let total_weight = self.success_rate_weight + self.speed_weight + self.cost_weight;
        if total_weight <= 0.0 {
            return 0.5;
        }

        (self.success_rate_weight * sr_score
            + self.speed_weight * speed_score
            + self.cost_weight * cost_score)
            / total_weight
    }

    /// Select the best tool from a list of candidates.
    pub fn select<'a>(
        &self,
        tools: &'a [ToolInfo],
        requirements: &ResourceRequirements,
    ) -> PlanningResult<&'a ToolInfo> {
        let ready: Vec<&ToolInfo> = tools.iter().filter(|t| t.is_ready()).collect();

        if ready.is_empty() {
            return Err(PlanningError::new(
                crate::error::PlanningErrorCode::ToolSelectionFailed,
                "no available tools matching requirements",
            ));
        }

        let best = ready
            .iter()
            .max_by(|a, b| {
                let sa = self.score_tool(a, requirements);
                let sb = self.score_tool(b, requirements);
                sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();

        Ok(best)
    }

    /// Rank all tools by score (highest first).
    pub fn rank<'a>(
        &self,
        tools: &'a [ToolInfo],
        requirements: &ResourceRequirements,
    ) -> Vec<(&'a ToolInfo, f64)> {
        let mut scored: Vec<(&ToolInfo, f64)> = tools
            .iter()
            .filter(|t| t.is_ready())
            .map(|t| (t, self.score_tool(t, requirements)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tool(name: &str) -> ToolInfo {
        ToolInfo::new(name, name)
            .with_success_rate(0.9)
            .with_duration(100)
    }

    // ToolStatus tests

    #[test]
    fn tool_status_default() {
        assert_eq!(ToolStatus::default(), ToolStatus::Ready);
    }

    // ToolInfo tests

    #[test]
    fn tool_info_creation() {
        let t = make_tool("tool1");
        assert_eq!(t.id, "tool1");
        assert_eq!(t.name, "tool1");
        assert_eq!(t.status, ToolStatus::Ready);
        assert!(t.is_ready());
    }

    #[test]
    fn tool_info_builder() {
        let t = ToolInfo::new("t", "n")
            .with_description("desc")
            .with_version("2.0.0")
            .with_status(ToolStatus::Busy)
            .with_tag("api")
            .with_duration(200)
            .with_success_rate(0.95)
            .with_input_schema(serde_json::json!({"type": "object"}))
            .with_output_schema(serde_json::json!({"type": "string"}))
            .with_metadata("k", serde_json::json!(42));
        assert_eq!(t.description, "desc");
        assert_eq!(t.version, "2.0.0");
        assert!(t.tags.contains(&"api".to_string()));
        assert_eq!(t.estimated_duration_ms, 200);
        assert!(t.input_schema.is_some());
        assert!(t.output_schema.is_some());
    }

    #[test]
    fn tool_info_not_ready_when_busy() {
        let t = make_tool("t").with_status(ToolStatus::Busy);
        assert!(!t.is_ready());
    }

    #[test]
    fn tool_info_not_ready_when_offline() {
        let t = make_tool("t").with_status(ToolStatus::Offline);
        assert!(!t.is_ready());
    }

    #[test]
    fn tool_info_success_rate_clamped() {
        let t = ToolInfo::new("t", "n").with_success_rate(5.0);
        assert!((t.success_rate - 1.0).abs() < f64::EPSILON);
    }

    // ToolSelector tests

    #[test]
    fn selector_default_weights() {
        let s = ToolSelector::default();
        assert!((s.success_rate_weight - 0.5).abs() < f64::EPSILON);
        assert!((s.speed_weight - 0.3).abs() < f64::EPSILON);
        assert!((s.cost_weight - 0.2).abs() < f64::EPSILON);
    }

    #[test]
    fn selector_builder() {
        let s = ToolSelector::new()
            .with_success_rate_weight(0.6)
            .with_speed_weight(0.3)
            .with_cost_weight(0.1);
        assert!((s.success_rate_weight - 0.6).abs() < f64::EPSILON);
    }

    #[test]
    fn score_tool_in_range() {
        let sel = ToolSelector::new();
        let tool = make_tool("t");
        let req = ResourceRequirements {
            cpu_units: 4,
            memory_mb: 1024,
            ..Default::default()
        };
        let score = sel.score_tool(&tool, &req);
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn score_higher_success_rate_is_better() {
        let sel = ToolSelector::new().with_success_rate_weight(1.0);
        let low = make_tool("low").with_success_rate(0.3);
        let high = make_tool("high").with_success_rate(0.95);
        let req = ResourceRequirements::default();
        assert!(sel.score_tool(&high, &req) > sel.score_tool(&low, &req));
    }

    #[test]
    fn score_faster_is_better() {
        let sel = ToolSelector::new().with_speed_weight(1.0);
        let slow = make_tool("slow").with_duration(1000);
        let fast = make_tool("fast").with_duration(10);
        let req = ResourceRequirements::default();
        assert!(sel.score_tool(&fast, &req) > sel.score_tool(&slow, &req));
    }

    #[test]
    fn select_picks_best() {
        let sel = ToolSelector::new();
        let tools = vec![
            make_tool("a").with_success_rate(0.5),
            make_tool("b").with_success_rate(0.95),
            make_tool("c").with_success_rate(0.7),
        ];
        let req = ResourceRequirements::default();
        let best = sel.select(&tools, &req).unwrap();
        assert_eq!(best.id, "b");
    }

    #[test]
    fn select_skips_non_ready() {
        let sel = ToolSelector::new();
        let tools = vec![make_tool("a").with_status(ToolStatus::Busy), make_tool("b")];
        let req = ResourceRequirements::default();
        let best = sel.select(&tools, &req).unwrap();
        assert_eq!(best.id, "b");
    }

    #[test]
    fn select_all_non_ready_errors() {
        let sel = ToolSelector::new();
        let tools: Vec<ToolInfo> = (0..3)
            .map(|i| make_tool(&format!("t{}", i)).with_status(ToolStatus::Offline))
            .collect();
        let req = ResourceRequirements::default();
        assert!(sel.select(&tools, &req).is_err());
    }

    #[test]
    fn select_empty_errors() {
        let sel = ToolSelector::new();
        let req = ResourceRequirements::default();
        assert!(sel.select(&[], &req).is_err());
    }

    #[test]
    fn rank_returns_descending_order() {
        let sel = ToolSelector::new();
        let tools = vec![
            make_tool("a").with_success_rate(0.3),
            make_tool("b").with_success_rate(0.9),
            make_tool("c").with_success_rate(0.6),
        ];
        let req = ResourceRequirements::default();
        let ranked = sel.rank(&tools, &req);
        assert_eq!(ranked.len(), 3);
        for i in 1..ranked.len() {
            assert!(ranked[i - 1].1 >= ranked[i].1);
        }
    }

    #[test]
    fn rank_excludes_non_ready() {
        let sel = ToolSelector::new();
        let tools = vec![
            make_tool("a"),
            make_tool("b").with_status(ToolStatus::Maintenance),
        ];
        let req = ResourceRequirements::default();
        let ranked = sel.rank(&tools, &req);
        assert_eq!(ranked.len(), 1);
    }

    // Serialization tests

    #[test]
    fn tool_serialization_roundtrip() {
        let t = make_tool("t1");
        let json = serde_json::to_string(&t).unwrap();
        let back: ToolInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "t1");
        assert_eq!(back.status, ToolStatus::Ready);
    }
}
