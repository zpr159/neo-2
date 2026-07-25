//! Reasoning integration for the planning system.
//!
//! Provides types and structures for interfacing with the reasoning engine,
//! enabling the planner to request reasoning assistance and incorporate
//! reasoning results into planning decisions.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::id::{PlanId, PlanningGoalId, StrategyId};

// ---------------------------------------------------------------------------
// ReasoningMode
// ---------------------------------------------------------------------------

/// The mode of reasoning to use for a request.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReasoningMode {
    Deductive,
    Inductive,
    Abductive,
    Analogical,
    Causal,
    Strategic,
}

impl Default for ReasoningMode {
    fn default() -> Self {
        Self::Deductive
    }
}

impl std::fmt::Display for ReasoningMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Deductive => write!(f, "deductive"),
            Self::Inductive => write!(f, "inductive"),
            Self::Abductive => write!(f, "abductive"),
            Self::Analogical => write!(f, "analogical"),
            Self::Causal => write!(f, "causal"),
            Self::Strategic => write!(f, "strategic"),
        }
    }
}

// ---------------------------------------------------------------------------
// ReasoningAssistRequest
// ---------------------------------------------------------------------------

/// A request for reasoning assistance from the planning system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningAssistRequest {
    pub id: String,
    pub query: String,
    pub mode: ReasoningMode,
    pub context: HashMap<String, serde_json::Value>,
    pub plan_id: Option<PlanId>,
    pub goal_id: Option<PlanningGoalId>,
    pub strategy_id: Option<StrategyId>,
    pub max_results: usize,
    pub confidence_threshold: f64,
    pub created_at: DateTime<Utc>,
}

impl ReasoningAssistRequest {
    /// Create a new request.
    pub fn new(id: impl Into<String>, query: impl Into<String>, mode: ReasoningMode) -> Self {
        Self {
            id: id.into(),
            query: query.into(),
            mode,
            context: HashMap::new(),
            plan_id: None,
            goal_id: None,
            strategy_id: None,
            max_results: 10,
            confidence_threshold: 0.5,
            created_at: Utc::now(),
        }
    }

    /// Attach a plan id.
    #[must_use]
    pub fn with_plan_id(mut self, plan_id: PlanId) -> Self {
        self.plan_id = Some(plan_id);
        self
    }

    /// Attach a goal id.
    #[must_use]
    pub fn with_goal_id(mut self, goal_id: PlanningGoalId) -> Self {
        self.goal_id = Some(goal_id);
        self
    }

    /// Attach a strategy id.
    #[must_use]
    pub fn with_strategy_id(mut self, strategy_id: StrategyId) -> Self {
        self.strategy_id = Some(strategy_id);
        self
    }

    /// Set the max results.
    #[must_use]
    pub fn with_max_results(mut self, n: usize) -> Self {
        self.max_results = n;
        self
    }

    /// Set the confidence threshold.
    #[must_use]
    pub fn with_confidence_threshold(mut self, threshold: f64) -> Self {
        self.confidence_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Add context.
    #[must_use]
    pub fn with_context(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.context.insert(key.into(), value);
        self
    }
}

// ---------------------------------------------------------------------------
// ReasoningConclusion
// ---------------------------------------------------------------------------

/// A single conclusion produced by the reasoning engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningConclusion {
    pub statement: String,
    pub confidence: f64,
    pub supporting_evidence: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ReasoningConclusion {
    /// Create a new conclusion.
    pub fn new(statement: impl Into<String>, confidence: f64) -> Self {
        Self {
            statement: statement.into(),
            confidence: confidence.clamp(0.0, 1.0),
            supporting_evidence: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add supporting evidence.
    #[must_use]
    pub fn with_evidence(mut self, evidence: impl Into<String>) -> Self {
        self.supporting_evidence.push(evidence.into());
        self
    }

    /// Add metadata.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

// ---------------------------------------------------------------------------
// ReasoningSuggestion
// ---------------------------------------------------------------------------

/// A suggestion produced by the reasoning engine for the planner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningSuggestion {
    pub action: String,
    pub rationale: String,
    pub priority: f64,
    pub estimated_impact: f64,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ReasoningSuggestion {
    /// Create a new suggestion.
    pub fn new(action: impl Into<String>, rationale: impl Into<String>, priority: f64) -> Self {
        Self {
            action: action.into(),
            rationale: rationale.into(),
            priority: priority.clamp(0.0, 1.0),
            estimated_impact: 0.0,
            metadata: HashMap::new(),
        }
    }

    /// Set the estimated impact.
    #[must_use]
    pub fn with_impact(mut self, impact: f64) -> Self {
        self.estimated_impact = impact.clamp(0.0, 1.0);
        self
    }

    /// Add metadata.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

// ---------------------------------------------------------------------------
// ReasoningAssistResult
// ---------------------------------------------------------------------------

/// The result of a reasoning assist request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningAssistResult {
    pub request_id: String,
    pub conclusions: Vec<ReasoningConclusion>,
    pub suggestions: Vec<ReasoningSuggestion>,
    pub overall_confidence: f64,
    pub reasoning_time_ms: u64,
    pub metadata: HashMap<String, serde_json::Value>,
    pub completed_at: DateTime<Utc>,
}

impl ReasoningAssistResult {
    /// Create a new result.
    pub fn new(request_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            conclusions: Vec::new(),
            suggestions: Vec::new(),
            overall_confidence: 0.0,
            reasoning_time_ms: 0,
            metadata: HashMap::new(),
            completed_at: Utc::now(),
        }
    }

    /// Add a conclusion.
    #[must_use]
    pub fn with_conclusion(mut self, conclusion: ReasoningConclusion) -> Self {
        self.conclusions.push(conclusion);
        self
    }

    /// Add a suggestion.
    #[must_use]
    pub fn with_suggestion(mut self, suggestion: ReasoningSuggestion) -> Self {
        self.suggestions.push(suggestion);
        self
    }

    /// Set the overall confidence.
    #[must_use]
    pub fn with_overall_confidence(mut self, confidence: f64) -> Self {
        self.overall_confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Set the reasoning time.
    #[must_use]
    pub fn with_reasoning_time_ms(mut self, ms: u64) -> Self {
        self.reasoning_time_ms = ms;
        self
    }

    /// Add metadata.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Get conclusions with confidence above the threshold.
    pub fn high_confidence_conclusions(&self, threshold: f64) -> Vec<&ReasoningConclusion> {
        self.conclusions
            .iter()
            .filter(|c| c.confidence >= threshold)
            .collect()
    }

    /// Get suggestions sorted by priority descending.
    pub fn sorted_suggestions(&self) -> Vec<&ReasoningSuggestion> {
        let mut suggestions: Vec<&ReasoningSuggestion> = self.suggestions.iter().collect();
        suggestions.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        suggestions
    }
}

// ---------------------------------------------------------------------------
// ReasoningIntegrator
// ---------------------------------------------------------------------------

/// Integrates reasoning results with planning decisions. Maintains a
/// history of reasoning requests and results for future reference.
#[derive(Debug, Clone)]
pub struct ReasoningIntegrator {
    history: Vec<(ReasoningAssistRequest, ReasoningAssistResult)>,
}

impl ReasoningIntegrator {
    /// Create a new integrator.
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
        }
    }

    /// Record a request-result pair.
    pub fn record(&mut self, request: ReasoningAssistRequest, result: ReasoningAssistResult) {
        self.history.push((request, result));
    }

    /// Get the number of recorded pairs.
    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// Check if the history is empty.
    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }

    /// Get the most recent result, if any.
    pub fn latest_result(&self) -> Option<&ReasoningAssistResult> {
        self.history.last().map(|(_, r)| r)
    }

    /// Get all results.
    pub fn all_results(&self) -> Vec<&ReasoningAssistResult> {
        self.history.iter().map(|(_, r)| r).collect()
    }

    /// Get the average overall confidence across all recorded results.
    pub fn average_confidence(&self) -> f64 {
        if self.history.is_empty() {
            return 0.0;
        }
        let total: f64 = self.history.iter().map(|(_, r)| r.overall_confidence).sum();
        total / self.history.len() as f64
    }

    /// Get all high-confidence conclusions from the history.
    pub fn high_confidence_conclusions(&self, threshold: f64) -> Vec<&ReasoningConclusion> {
        self.history
            .iter()
            .flat_map(|(_, r)| r.high_confidence_conclusions(threshold))
            .collect()
    }

    /// Get all suggestions across history, sorted by priority descending.
    pub fn all_suggestions(&self) -> Vec<&ReasoningSuggestion> {
        let mut suggestions: Vec<&ReasoningSuggestion> = self
            .history
            .iter()
            .flat_map(|(_, r)| r.suggestions.iter())
            .collect();
        suggestions.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        suggestions
    }

    /// Merge a new result with existing knowledge by filtering out
    /// duplicate conclusions (by statement).
    pub fn merge_conclusions(
        &self,
        new_conclusions: &[ReasoningConclusion],
    ) -> Vec<ReasoningConclusion> {
        let existing: Vec<&str> = self
            .history
            .iter()
            .flat_map(|(_, r)| r.conclusions.iter())
            .map(|c| c.statement.as_str())
            .collect();
        new_conclusions
            .iter()
            .filter(|c| !existing.contains(&c.statement.as_str()))
            .cloned()
            .collect()
    }
}

impl Default for ReasoningIntegrator {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ReasoningMode tests

    #[test]
    fn reasoning_mode_display() {
        assert_eq!(ReasoningMode::Deductive.to_string(), "deductive");
        assert_eq!(ReasoningMode::Inductive.to_string(), "inductive");
        assert_eq!(ReasoningMode::Abductive.to_string(), "abductive");
        assert_eq!(ReasoningMode::Analogical.to_string(), "analogical");
        assert_eq!(ReasoningMode::Causal.to_string(), "causal");
        assert_eq!(ReasoningMode::Strategic.to_string(), "strategic");
    }

    #[test]
    fn reasoning_mode_default() {
        assert_eq!(ReasoningMode::default(), ReasoningMode::Deductive);
    }

    // ReasoningAssistRequest tests

    #[test]
    fn request_creation() {
        let r = ReasoningAssistRequest::new("r1", "why?", ReasoningMode::Deductive);
        assert_eq!(r.id, "r1");
        assert_eq!(r.query, "why?");
        assert_eq!(r.mode, ReasoningMode::Deductive);
        assert_eq!(r.max_results, 10);
        assert!((r.confidence_threshold - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn request_builder() {
        let plan_id = PlanId::new();
        let goal_id = PlanningGoalId::new();
        let strategy_id = StrategyId::new();
        let r = ReasoningAssistRequest::new("r", "q", ReasoningMode::Strategic)
            .with_plan_id(plan_id)
            .with_goal_id(goal_id)
            .with_strategy_id(strategy_id)
            .with_max_results(5)
            .with_confidence_threshold(0.8)
            .with_context("key", serde_json::json!(42));
        assert_eq!(r.plan_id, Some(plan_id));
        assert_eq!(r.goal_id, Some(goal_id));
        assert_eq!(r.strategy_id, Some(strategy_id));
        assert_eq!(r.max_results, 5);
        assert!((r.confidence_threshold - 0.8).abs() < f64::EPSILON);
        assert_eq!(r.context.get("key").unwrap(), 42);
    }

    #[test]
    fn request_confidence_clamped() {
        let r = ReasoningAssistRequest::new("r", "q", ReasoningMode::Deductive)
            .with_confidence_threshold(5.0);
        assert!((r.confidence_threshold - 1.0).abs() < f64::EPSILON);
    }

    // ReasoningConclusion tests

    #[test]
    fn conclusion_creation() {
        let c = ReasoningConclusion::new("A implies B", 0.9);
        assert_eq!(c.statement, "A implies B");
        assert!((c.confidence - 0.9).abs() < f64::EPSILON);
        assert!(c.supporting_evidence.is_empty());
    }

    #[test]
    fn conclusion_builder() {
        let c = ReasoningConclusion::new("x", 0.8)
            .with_evidence("evidence1")
            .with_evidence("evidence2")
            .with_metadata("k", serde_json::json!("v"));
        assert_eq!(c.supporting_evidence.len(), 2);
        assert_eq!(c.metadata.get("k").unwrap(), "v");
    }

    #[test]
    fn conclusion_confidence_clamped() {
        let c = ReasoningConclusion::new("s", 5.0);
        assert!((c.confidence - 1.0).abs() < f64::EPSILON);
    }

    // ReasoningSuggestion tests

    #[test]
    fn suggestion_creation() {
        let s = ReasoningSuggestion::new("do X", "because Y", 0.7);
        assert_eq!(s.action, "do X");
        assert_eq!(s.rationale, "because Y");
        assert!((s.priority - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn suggestion_builder() {
        let s = ReasoningSuggestion::new("a", "r", 0.5)
            .with_impact(0.8)
            .with_metadata("k", serde_json::json!(42));
        assert!((s.estimated_impact - 0.8).abs() < f64::EPSILON);
        assert_eq!(s.metadata.get("k").unwrap(), 42);
    }

    #[test]
    fn suggestion_priority_clamped() {
        let s = ReasoningSuggestion::new("a", "r", 5.0);
        assert!((s.priority - 1.0).abs() < f64::EPSILON);
    }

    // ReasoningAssistResult tests

    #[test]
    fn result_creation() {
        let r = ReasoningAssistResult::new("req1");
        assert_eq!(r.request_id, "req1");
        assert!(r.conclusions.is_empty());
        assert!(r.suggestions.is_empty());
        assert_eq!(r.overall_confidence, 0.0);
    }

    #[test]
    fn result_builder() {
        let r = ReasoningAssistResult::new("r")
            .with_conclusion(ReasoningConclusion::new("c1", 0.9))
            .with_conclusion(ReasoningConclusion::new("c2", 0.7))
            .with_suggestion(ReasoningSuggestion::new("a1", "r1", 0.8))
            .with_overall_confidence(0.85)
            .with_reasoning_time_ms(150)
            .with_metadata("key", serde_json::json!("val"));
        assert_eq!(r.conclusions.len(), 2);
        assert_eq!(r.suggestions.len(), 1);
        assert!((r.overall_confidence - 0.85).abs() < f64::EPSILON);
        assert_eq!(r.reasoning_time_ms, 150);
    }

    #[test]
    fn result_high_confidence_conclusions() {
        let r = ReasoningAssistResult::new("r")
            .with_conclusion(ReasoningConclusion::new("c1", 0.9))
            .with_conclusion(ReasoningConclusion::new("c2", 0.3));
        let high = r.high_confidence_conclusions(0.5);
        assert_eq!(high.len(), 1);
        assert_eq!(high[0].statement, "c1");
    }

    #[test]
    fn result_sorted_suggestions() {
        let r = ReasoningAssistResult::new("r")
            .with_suggestion(ReasoningSuggestion::new("a1", "r1", 0.3))
            .with_suggestion(ReasoningSuggestion::new("a2", "r2", 0.9))
            .with_suggestion(ReasoningSuggestion::new("a3", "r3", 0.6));
        let sorted = r.sorted_suggestions();
        assert_eq!(sorted.len(), 3);
        assert!((sorted[0].priority - 0.9).abs() < f64::EPSILON);
        assert!((sorted[1].priority - 0.6).abs() < f64::EPSILON);
        assert!((sorted[2].priority - 0.3).abs() < f64::EPSILON);
    }

    // ReasoningIntegrator tests

    #[test]
    fn integrator_new_is_empty() {
        let i = ReasoningIntegrator::new();
        assert!(i.is_empty());
        assert_eq!(i.history_len(), 0);
    }

    #[test]
    fn integrator_record_and_latest() {
        let mut i = ReasoningIntegrator::new();
        let req1 = ReasoningAssistRequest::new("r1", "q1", ReasoningMode::Deductive);
        let res1 = ReasoningAssistResult::new("r1").with_overall_confidence(0.7);
        i.record(req1, res1);
        assert_eq!(i.history_len(), 1);
        assert!((i.latest_result().unwrap().overall_confidence - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn integrator_average_confidence() {
        let mut i = ReasoningIntegrator::new();
        i.record(
            ReasoningAssistRequest::new("r1", "q", ReasoningMode::Deductive),
            ReasoningAssistResult::new("r1").with_overall_confidence(0.6),
        );
        i.record(
            ReasoningAssistRequest::new("r2", "q", ReasoningMode::Inductive),
            ReasoningAssistResult::new("r2").with_overall_confidence(0.8),
        );
        let avg = i.average_confidence();
        assert!((avg - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn integrator_average_confidence_empty() {
        let i = ReasoningIntegrator::new();
        assert!((i.average_confidence() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn integrator_high_confidence_conclusions() {
        let mut i = ReasoningIntegrator::new();
        i.record(
            ReasoningAssistRequest::new("r1", "q", ReasoningMode::Deductive),
            ReasoningAssistResult::new("r1")
                .with_conclusion(ReasoningConclusion::new("c1", 0.9))
                .with_conclusion(ReasoningConclusion::new("c2", 0.3)),
        );
        let high = i.high_confidence_conclusions(0.5);
        assert_eq!(high.len(), 1);
    }

    #[test]
    fn integrator_all_suggestions() {
        let mut i = ReasoningIntegrator::new();
        i.record(
            ReasoningAssistRequest::new("r1", "q", ReasoningMode::Deductive),
            ReasoningAssistResult::new("r1")
                .with_suggestion(ReasoningSuggestion::new("a1", "r", 0.3)),
        );
        i.record(
            ReasoningAssistRequest::new("r2", "q", ReasoningMode::Inductive),
            ReasoningAssistResult::new("r2")
                .with_suggestion(ReasoningSuggestion::new("a2", "r", 0.9)),
        );
        let all = i.all_suggestions();
        assert_eq!(all.len(), 2);
        assert!((all[0].priority - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn integrator_merge_conclusions_deduplicates() {
        let mut i = ReasoningIntegrator::new();
        i.record(
            ReasoningAssistRequest::new("r1", "q", ReasoningMode::Deductive),
            ReasoningAssistResult::new("r1").with_conclusion(ReasoningConclusion::new("c1", 0.9)),
        );
        let new_conclusions = vec![
            ReasoningConclusion::new("c1", 0.8),
            ReasoningConclusion::new("c2", 0.7),
        ];
        let merged = i.merge_conclusions(&new_conclusions);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].statement, "c2");
    }

    // Serialization tests

    #[test]
    fn request_serialization_roundtrip() {
        let r = ReasoningAssistRequest::new("r1", "query", ReasoningMode::Strategic);
        let json = serde_json::to_string(&r).unwrap();
        let back: ReasoningAssistRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "r1");
        assert_eq!(back.mode, ReasoningMode::Strategic);
    }

    #[test]
    fn result_serialization_roundtrip() {
        let r = ReasoningAssistResult::new("r1")
            .with_conclusion(ReasoningConclusion::new("c", 0.8))
            .with_suggestion(ReasoningSuggestion::new("a", "r", 0.5));
        let json = serde_json::to_string(&r).unwrap();
        let back: ReasoningAssistResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.request_id, "r1");
        assert_eq!(back.conclusions.len(), 1);
        assert_eq!(back.suggestions.len(), 1);
    }

    #[test]
    fn conclusion_serialization_roundtrip() {
        let c = ReasoningConclusion::new("stmt", 0.75).with_evidence("ev");
        let json = serde_json::to_string(&c).unwrap();
        let back: ReasoningConclusion = serde_json::from_str(&json).unwrap();
        assert_eq!(back.statement, "stmt");
    }

    #[test]
    fn suggestion_serialization_roundtrip() {
        let s = ReasoningSuggestion::new("act", "rat", 0.6);
        let json = serde_json::to_string(&s).unwrap();
        let back: ReasoningSuggestion = serde_json::from_str(&json).unwrap();
        assert_eq!(back.action, "act");
    }
}
