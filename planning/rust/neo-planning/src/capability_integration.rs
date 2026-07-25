//! Capability integration for the planning system.
//!
//! Maps planning tasks to available capabilities provided by agents or
//! services, enabling the planner to make informed decisions about resource
//! allocation.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{PlanningError, PlanningResult};
use crate::types::ResourceRequirements;

// ---------------------------------------------------------------------------
// CapabilityStatus
// ---------------------------------------------------------------------------

/// Whether a capability is currently available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CapabilityStatus {
    Available,
    InUse,
    Unavailable,
    Deprecated,
}

impl Default for CapabilityStatus {
    fn default() -> Self {
        Self::Available
    }
}

// ---------------------------------------------------------------------------
// CapabilityInfo
// ---------------------------------------------------------------------------

/// Describes a single capability offered by a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub provider: String,
    pub version: String,
    pub status: CapabilityStatus,
    pub tags: Vec<String>,
    pub resource_cost: ResourceRequirements,
    pub estimated_latency_ms: u64,
    pub reliability: f64,
    pub metadata: HashMap<String, serde_json::Value>,
    pub registered_at: DateTime<Utc>,
}

impl CapabilityInfo {
    /// Create a new capability.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        provider: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            provider: provider.into(),
            version: "1.0.0".to_string(),
            status: CapabilityStatus::Available,
            tags: Vec::new(),
            resource_cost: ResourceRequirements::default(),
            estimated_latency_ms: 0,
            reliability: 1.0,
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
    pub fn with_status(mut self, status: CapabilityStatus) -> Self {
        self.status = status;
        self
    }

    /// Add a tag.
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set resource cost.
    #[must_use]
    pub fn with_resource_cost(mut self, cost: ResourceRequirements) -> Self {
        self.resource_cost = cost;
        self
    }

    /// Set estimated latency.
    #[must_use]
    pub fn with_latency(mut self, ms: u64) -> Self {
        self.estimated_latency_ms = ms;
        self
    }

    /// Set reliability.
    #[must_use]
    pub fn with_reliability(mut self, r: f64) -> Self {
        self.reliability = r.clamp(0.0, 1.0);
        self
    }

    /// Add metadata.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Check whether the capability is available for use.
    pub fn is_available(&self) -> bool {
        self.status == CapabilityStatus::Available
    }
}

// ---------------------------------------------------------------------------
// CapabilitySelector
// ---------------------------------------------------------------------------

/// Selects the best capability for a given task requirement from a pool
/// of available capabilities.
#[derive(Debug, Clone)]
pub struct CapabilitySelector {
    /// Weight for reliability in scoring.
    pub reliability_weight: f64,
    /// Weight for latency (lower is better).
    pub latency_weight: f64,
    /// Weight for resource cost (lower is better).
    pub cost_weight: f64,
}

impl Default for CapabilitySelector {
    fn default() -> Self {
        Self {
            reliability_weight: 0.5,
            latency_weight: 0.25,
            cost_weight: 0.25,
        }
    }
}

impl CapabilitySelector {
    /// Create a new selector with default weights.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the reliability weight.
    #[must_use]
    pub fn with_reliability_weight(mut self, w: f64) -> Self {
        self.reliability_weight = w;
        self
    }

    /// Set the latency weight.
    #[must_use]
    pub fn with_latency_weight(mut self, w: f64) -> Self {
        self.latency_weight = w;
        self
    }

    /// Set the cost weight.
    #[must_use]
    pub fn with_cost_weight(mut self, w: f64) -> Self {
        self.cost_weight = w;
        self
    }

    /// Score a capability for the given requirements.
    ///
    /// Returns a value in `[0.0, 1.0]` where higher is better.
    pub fn score_capability(
        &self,
        cap: &CapabilityInfo,
        requirements: &ResourceRequirements,
    ) -> f64 {
        let rel_score = cap.reliability;

        let lat_score = if cap.estimated_latency_ms > 0 {
            (1000.0 / cap.estimated_latency_ms as f64).min(1.0)
        } else {
            1.0
        };

        let total_req = requirements.cpu_units as f64
            + requirements.memory_mb as f64 / 256.0
            + requirements.storage_mb as f64 / 1024.0;
        let total_cost = cap.resource_cost.cpu_units as f64
            + cap.resource_cost.memory_mb as f64 / 256.0
            + cap.resource_cost.storage_mb as f64 / 1024.0;
        let cost_score = if total_req > 0.0 {
            (1.0 - (total_cost / total_req)).clamp(0.0, 1.0)
        } else {
            0.5
        };

        let total_weight = self.reliability_weight + self.latency_weight + self.cost_weight;
        if total_weight <= 0.0 {
            return 0.5;
        }

        (self.reliability_weight * rel_score
            + self.latency_weight * lat_score
            + self.cost_weight * cost_score)
            / total_weight
    }

    /// Select the best capability from a list of candidates.
    pub fn select<'a>(
        &self,
        capabilities: &'a [CapabilityInfo],
        requirements: &ResourceRequirements,
    ) -> PlanningResult<&'a CapabilityInfo> {
        let available: Vec<&CapabilityInfo> =
            capabilities.iter().filter(|c| c.is_available()).collect();

        if available.is_empty() {
            return Err(PlanningError::new(
                crate::error::PlanningErrorCode::CapabilitySelectionFailed,
                "no available capabilities matching requirements",
            ));
        }

        let best = available
            .iter()
            .max_by(|a, b| {
                let sa = self.score_capability(a, requirements);
                let sb = self.score_capability(b, requirements);
                sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();

        Ok(best)
    }

    /// Rank all capabilities by score (highest first).
    pub fn rank<'a>(
        &self,
        capabilities: &'a [CapabilityInfo],
        requirements: &ResourceRequirements,
    ) -> Vec<(&'a CapabilityInfo, f64)> {
        let mut scored: Vec<(&CapabilityInfo, f64)> = capabilities
            .iter()
            .filter(|c| c.is_available())
            .map(|c| (c, self.score_capability(c, requirements)))
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

    fn make_capability(name: &str) -> CapabilityInfo {
        CapabilityInfo::new(name, name, "provider-a")
            .with_reliability(0.9)
            .with_latency(50)
    }

    // CapabilityStatus tests

    #[test]
    fn capability_status_default() {
        assert_eq!(CapabilityStatus::default(), CapabilityStatus::Available);
    }

    // CapabilityInfo tests

    #[test]
    fn capability_info_creation() {
        let c = make_capability("cap1");
        assert_eq!(c.id, "cap1");
        assert_eq!(c.name, "cap1");
        assert_eq!(c.provider, "provider-a");
        assert_eq!(c.status, CapabilityStatus::Available);
        assert_eq!(c.version, "1.0.0");
        assert!(c.is_available());
    }

    #[test]
    fn capability_info_builder() {
        let c = CapabilityInfo::new("c", "n", "p")
            .with_description("desc")
            .with_version("2.0.0")
            .with_status(CapabilityStatus::InUse)
            .with_tag("ml")
            .with_latency(100)
            .with_reliability(0.95)
            .with_metadata("k", serde_json::json!(42));
        assert_eq!(c.description, "desc");
        assert_eq!(c.version, "2.0.0");
        assert_eq!(c.status, CapabilityStatus::InUse);
        assert!(c.tags.contains(&"ml".to_string()));
        assert_eq!(c.estimated_latency_ms, 100);
        assert!((c.reliability - 0.95).abs() < f64::EPSILON);
        assert_eq!(c.metadata.get("k").unwrap(), 42);
    }

    #[test]
    fn capability_info_not_available_when_in_use() {
        let c = make_capability("c").with_status(CapabilityStatus::InUse);
        assert!(!c.is_available());
    }

    #[test]
    fn capability_info_not_available_when_deprecated() {
        let c = make_capability("c").with_status(CapabilityStatus::Deprecated);
        assert!(!c.is_available());
    }

    #[test]
    fn capability_info_reliability_clamped() {
        let c = CapabilityInfo::new("c", "n", "p").with_reliability(5.0);
        assert!((c.reliability - 1.0).abs() < f64::EPSILON);
        let c2 = CapabilityInfo::new("c", "n", "p").with_reliability(-1.0);
        assert!(c2.reliability.abs() < f64::EPSILON);
    }

    // CapabilitySelector tests

    #[test]
    fn selector_default_weights() {
        let s = CapabilitySelector::default();
        assert!((s.reliability_weight - 0.5).abs() < f64::EPSILON);
        assert!((s.latency_weight - 0.25).abs() < f64::EPSILON);
        assert!((s.cost_weight - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn selector_builder() {
        let s = CapabilitySelector::new()
            .with_reliability_weight(0.6)
            .with_latency_weight(0.2)
            .with_cost_weight(0.2);
        assert!((s.reliability_weight - 0.6).abs() < f64::EPSILON);
    }

    #[test]
    fn score_capability_in_range() {
        let sel = CapabilitySelector::new();
        let cap = make_capability("c");
        let req = ResourceRequirements {
            cpu_units: 4,
            memory_mb: 1024,
            ..Default::default()
        };
        let score = sel.score_capability(&cap, &req);
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn score_higher_reliability_is_better() {
        let sel = CapabilitySelector::new().with_reliability_weight(1.0);
        let cap_low = make_capability("low").with_reliability(0.3);
        let cap_high = make_capability("high").with_reliability(0.9);
        let req = ResourceRequirements::default();
        assert!(sel.score_capability(&cap_high, &req) > sel.score_capability(&cap_low, &req));
    }

    #[test]
    fn score_lower_latency_is_better() {
        let sel = CapabilitySelector::new().with_latency_weight(1.0);
        let cap_slow = make_capability("slow").with_latency(500);
        let cap_fast = make_capability("fast").with_latency(10);
        let req = ResourceRequirements::default();
        assert!(sel.score_capability(&cap_fast, &req) > sel.score_capability(&cap_slow, &req));
    }

    #[test]
    fn select_picks_best() {
        let sel = CapabilitySelector::new();
        let caps = vec![
            make_capability("a").with_reliability(0.5),
            make_capability("b").with_reliability(0.9),
            make_capability("c").with_reliability(0.7),
        ];
        let req = ResourceRequirements::default();
        let best = sel.select(&caps, &req).unwrap();
        assert_eq!(best.id, "b");
    }

    #[test]
    fn select_skips_unavailable() {
        let sel = CapabilitySelector::new();
        let caps = vec![
            make_capability("a").with_status(CapabilityStatus::Unavailable),
            make_capability("b"),
        ];
        let req = ResourceRequirements::default();
        let best = sel.select(&caps, &req).unwrap();
        assert_eq!(best.id, "b");
    }

    #[test]
    fn select_all_unavailable_errors() {
        let sel = CapabilitySelector::new();
        let caps: Vec<CapabilityInfo> = (0..3)
            .map(|i| make_capability(&format!("c{}", i)).with_status(CapabilityStatus::Unavailable))
            .collect();
        let req = ResourceRequirements::default();
        assert!(sel.select(&caps, &req).is_err());
    }

    #[test]
    fn select_empty_errors() {
        let sel = CapabilitySelector::new();
        let req = ResourceRequirements::default();
        assert!(sel.select(&[], &req).is_err());
    }

    #[test]
    fn rank_returns_descending_order() {
        let sel = CapabilitySelector::new();
        let caps = vec![
            make_capability("a").with_reliability(0.3),
            make_capability("b").with_reliability(0.9),
            make_capability("c").with_reliability(0.6),
        ];
        let req = ResourceRequirements::default();
        let ranked = sel.rank(&caps, &req);
        assert_eq!(ranked.len(), 3);
        for i in 1..ranked.len() {
            assert!(ranked[i - 1].1 >= ranked[i].1);
        }
    }

    #[test]
    fn rank_excludes_unavailable() {
        let sel = CapabilitySelector::new();
        let caps = vec![
            make_capability("a"),
            make_capability("b").with_status(CapabilityStatus::Deprecated),
        ];
        let req = ResourceRequirements::default();
        let ranked = sel.rank(&caps, &req);
        assert_eq!(ranked.len(), 1);
    }

    // Serialization tests

    #[test]
    fn capability_serialization_roundtrip() {
        let c = make_capability("c1");
        let json = serde_json::to_string(&c).unwrap();
        let back: CapabilityInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "c1");
        assert_eq!(back.status, CapabilityStatus::Available);
    }
}
