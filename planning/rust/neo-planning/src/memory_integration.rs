//! Memory integration for the planning system.
//!
//! Provides a thread-safe store for plan-related memory records, enabling
//! the planner to recall past planning decisions, outcomes, and context
//! for improved future planning.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::error::{PlanningError, PlanningResult};
use crate::id::{PlanId, PlanningGoalId};

// ---------------------------------------------------------------------------
// MemoryRecordType
// ---------------------------------------------------------------------------

/// The kind of memory record stored.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryRecordType {
    PlanCreated,
    PlanCompleted,
    PlanFailed,
    GoalDecomposed,
    StrategySelected,
    ReplanningEvent,
    LessonLearned,
    UserFeedback,
    Custom(String),
}

impl std::fmt::Display for MemoryRecordType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PlanCreated => write!(f, "plan_created"),
            Self::PlanCompleted => write!(f, "plan_completed"),
            Self::PlanFailed => write!(f, "plan_failed"),
            Self::GoalDecomposed => write!(f, "goal_decomposed"),
            Self::StrategySelected => write!(f, "strategy_selected"),
            Self::ReplanningEvent => write!(f, "replanning_event"),
            Self::LessonLearned => write!(f, "lesson_learned"),
            Self::UserFeedback => write!(f, "user_feedback"),
            Self::Custom(name) => write!(f, "custom({})", name),
        }
    }
}

// ---------------------------------------------------------------------------
// PlanMemoryRecord
// ---------------------------------------------------------------------------

/// A single memory record associated with a planning event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanMemoryRecord {
    pub id: String,
    pub record_type: MemoryRecordType,
    pub plan_id: Option<PlanId>,
    pub goal_id: Option<PlanningGoalId>,
    pub summary: String,
    pub details: HashMap<String, serde_json::Value>,
    pub tags: Vec<String>,
    pub importance: f64,
    pub created_at: DateTime<Utc>,
}

impl PlanMemoryRecord {
    /// Create a new memory record.
    pub fn new(
        id: impl Into<String>,
        record_type: MemoryRecordType,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            record_type,
            plan_id: None,
            goal_id: None,
            summary: summary.into(),
            details: HashMap::new(),
            tags: Vec::new(),
            importance: 0.5,
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

    /// Set the summary.
    #[must_use]
    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = summary.into();
        self
    }

    /// Add a detail entry.
    #[must_use]
    pub fn with_detail(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.details.insert(key.into(), value);
        self
    }

    /// Add a tag.
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set the importance score.
    #[must_use]
    pub fn with_importance(mut self, importance: f64) -> Self {
        self.importance = importance.clamp(0.0, 1.0);
        self
    }
}

// ---------------------------------------------------------------------------
// PlanMemoryStore
// ---------------------------------------------------------------------------

/// Thread-safe in-memory store for [`PlanMemoryRecord`]s.
#[derive(Clone)]
pub struct PlanMemoryStore {
    inner: Arc<PlanMemoryStoreInner>,
}

struct PlanMemoryStoreInner {
    records: RwLock<Vec<PlanMemoryRecord>>,
    index: RwLock<HashMap<String, usize>>,
}

impl PlanMemoryStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(PlanMemoryStoreInner {
                records: RwLock::new(Vec::new()),
                index: RwLock::new(HashMap::new()),
            }),
        }
    }

    /// Insert a record.
    pub fn insert(&self, record: PlanMemoryRecord) {
        let id = record.id.clone();
        let mut records = self.inner.records.write();
        let idx = records.len();
        records.push(record);
        drop(records);
        self.inner.index.write().insert(id, idx);
    }

    /// Get a record by id.
    pub fn get(&self, id: &str) -> PlanningResult<PlanMemoryRecord> {
        let index = self.inner.index.read();
        let idx = index.get(id).ok_or_else(|| {
            PlanningError::new(
                crate::error::PlanningErrorCode::PlanNotFound,
                format!("memory record '{}' not found", id),
            )
        })?;
        let records = self.inner.records.read();
        Ok(records[*idx].clone())
    }

    /// Get all records.
    pub fn all_records(&self) -> Vec<PlanMemoryRecord> {
        self.inner.records.read().clone()
    }

    /// Get records for a specific plan.
    pub fn records_for_plan(&self, plan_id: PlanId) -> Vec<PlanMemoryRecord> {
        self.inner
            .records
            .read()
            .iter()
            .filter(|r| r.plan_id == Some(plan_id))
            .cloned()
            .collect()
    }

    /// Get records matching a type.
    pub fn records_by_type(&self, record_type: &MemoryRecordType) -> Vec<PlanMemoryRecord> {
        self.inner
            .records
            .read()
            .iter()
            .filter(|r| r.record_type == *record_type)
            .cloned()
            .collect()
    }

    /// Get records for a specific goal.
    pub fn records_for_goal(&self, goal_id: PlanningGoalId) -> Vec<PlanMemoryRecord> {
        self.inner
            .records
            .read()
            .iter()
            .filter(|r| r.goal_id == Some(goal_id))
            .cloned()
            .collect()
    }

    /// Get records matching a tag.
    pub fn records_with_tag(&self, tag: &str) -> Vec<PlanMemoryRecord> {
        self.inner
            .records
            .read()
            .iter()
            .filter(|r| r.tags.contains(&tag.to_string()))
            .cloned()
            .collect()
    }

    /// Get records with importance >= threshold, sorted by importance descending.
    pub fn important_records(&self, threshold: f64) -> Vec<PlanMemoryRecord> {
        let mut records: Vec<PlanMemoryRecord> = self
            .inner
            .records
            .read()
            .iter()
            .filter(|r| r.importance >= threshold)
            .cloned()
            .collect();
        records.sort_by(|a, b| {
            b.importance
                .partial_cmp(&a.importance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        records
    }

    /// Search records whose summary contains the query (case-insensitive).
    pub fn search(&self, query: &str) -> Vec<PlanMemoryRecord> {
        let q = query.to_lowercase();
        self.inner
            .records
            .read()
            .iter()
            .filter(|r| r.summary.to_lowercase().contains(&q))
            .cloned()
            .collect()
    }

    /// Remove a record by id.
    pub fn remove(&self, id: &str) -> PlanningResult<PlanMemoryRecord> {
        let mut index = self.inner.index.write();
        let idx = index.remove(id).ok_or_else(|| {
            PlanningError::new(
                crate::error::PlanningErrorCode::PlanNotFound,
                format!("memory record '{}' not found", id),
            )
        })?;
        let mut records = self.inner.records.write();
        Ok(records.remove(idx))
    }

    /// Return the number of stored records.
    pub fn len(&self) -> usize {
        self.inner.records.read().len()
    }

    /// Check if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.records.read().is_empty()
    }

    /// Clear all records.
    pub fn clear(&self) {
        self.inner.records.write().clear();
        self.inner.index.write().clear();
    }
}

impl Default for PlanMemoryStore {
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

    fn make_record(id: &str, rtype: MemoryRecordType) -> PlanMemoryRecord {
        PlanMemoryRecord::new(id, rtype, format!("summary for {}", id))
    }

    // MemoryRecordType tests

    #[test]
    fn memory_record_type_display() {
        assert_eq!(MemoryRecordType::PlanCreated.to_string(), "plan_created");
        assert_eq!(
            MemoryRecordType::LessonLearned.to_string(),
            "lesson_learned"
        );
        assert_eq!(
            MemoryRecordType::Custom("x".to_string()).to_string(),
            "custom(x)"
        );
    }

    // PlanMemoryRecord tests

    #[test]
    fn record_creation() {
        let r = make_record("r1", MemoryRecordType::PlanCreated);
        assert_eq!(r.id, "r1");
        assert_eq!(r.record_type, MemoryRecordType::PlanCreated);
        assert!(r.plan_id.is_none());
        assert!((r.importance - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn record_builder() {
        let plan_id = PlanId::new();
        let goal_id = PlanningGoalId::new();
        let r = PlanMemoryRecord::new("r", MemoryRecordType::PlanCompleted, "done")
            .with_plan_id(plan_id)
            .with_goal_id(goal_id)
            .with_detail("score", serde_json::json!(95))
            .with_tag("success")
            .with_importance(0.9);
        assert_eq!(r.plan_id, Some(plan_id));
        assert_eq!(r.goal_id, Some(goal_id));
        assert_eq!(r.details.get("score").unwrap(), 95);
        assert!(r.tags.contains(&"success".to_string()));
        assert!((r.importance - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn record_importance_clamped() {
        let r = PlanMemoryRecord::new("r", MemoryRecordType::PlanCreated, "s").with_importance(5.0);
        assert!((r.importance - 1.0).abs() < f64::EPSILON);
    }

    // PlanMemoryStore tests

    #[test]
    fn store_new_is_empty() {
        let store = PlanMemoryStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn store_insert_and_get() {
        let store = PlanMemoryStore::new();
        let r = make_record("r1", MemoryRecordType::PlanCreated);
        store.insert(r);
        let retrieved = store.get("r1").unwrap();
        assert_eq!(retrieved.id, "r1");
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn store_get_missing() {
        let store = PlanMemoryStore::new();
        assert!(store.get("missing").is_err());
    }

    #[test]
    fn store_remove() {
        let store = PlanMemoryStore::new();
        store.insert(make_record("r1", MemoryRecordType::PlanCreated));
        let removed = store.remove("r1").unwrap();
        assert_eq!(removed.id, "r1");
        assert!(store.is_empty());
    }

    #[test]
    fn store_remove_missing() {
        let store = PlanMemoryStore::new();
        assert!(store.remove("nope").is_err());
    }

    #[test]
    fn store_all_records() {
        let store = PlanMemoryStore::new();
        store.insert(make_record("a", MemoryRecordType::PlanCreated));
        store.insert(make_record("b", MemoryRecordType::PlanCompleted));
        assert_eq!(store.all_records().len(), 2);
    }

    #[test]
    fn store_records_for_plan() {
        let store = PlanMemoryStore::new();
        let plan_id = PlanId::new();
        store.insert(make_record("r1", MemoryRecordType::PlanCreated).with_plan_id(plan_id));
        store.insert(make_record("r2", MemoryRecordType::PlanCreated));
        assert_eq!(store.records_for_plan(plan_id).len(), 1);
    }

    #[test]
    fn store_records_by_type() {
        let store = PlanMemoryStore::new();
        store.insert(make_record("a", MemoryRecordType::PlanCreated));
        store.insert(make_record("b", MemoryRecordType::PlanCompleted));
        store.insert(make_record("c", MemoryRecordType::PlanCreated));
        assert_eq!(
            store.records_by_type(&MemoryRecordType::PlanCreated).len(),
            2
        );
    }

    #[test]
    fn store_records_for_goal() {
        let store = PlanMemoryStore::new();
        let gid = PlanningGoalId::new();
        store.insert(make_record("r1", MemoryRecordType::GoalDecomposed).with_goal_id(gid));
        store.insert(make_record("r2", MemoryRecordType::GoalDecomposed));
        assert_eq!(store.records_for_goal(gid).len(), 1);
    }

    #[test]
    fn store_records_with_tag() {
        let store = PlanMemoryStore::new();
        store.insert(make_record("a", MemoryRecordType::LessonLearned).with_tag("perf"));
        store.insert(make_record("b", MemoryRecordType::LessonLearned));
        assert_eq!(store.records_with_tag("perf").len(), 1);
    }

    #[test]
    fn store_important_records() {
        let store = PlanMemoryStore::new();
        store.insert(make_record("a", MemoryRecordType::LessonLearned).with_importance(0.3));
        store.insert(make_record("b", MemoryRecordType::LessonLearned).with_importance(0.8));
        store.insert(make_record("c", MemoryRecordType::LessonLearned).with_importance(0.9));
        let important = store.important_records(0.5);
        assert_eq!(important.len(), 2);
        assert!(important[0].importance >= important[1].importance);
    }

    #[test]
    fn store_search() {
        let store = PlanMemoryStore::new();
        store.insert(
            make_record("r1", MemoryRecordType::LessonLearned).with_summary("fast algorithm"),
        );
        store.insert(
            make_record("r2", MemoryRecordType::LessonLearned).with_summary("slow approach"),
        );
        let results = store.search("fast");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "r1");
    }

    #[test]
    fn store_clear() {
        let store = PlanMemoryStore::new();
        store.insert(make_record("a", MemoryRecordType::PlanCreated));
        store.insert(make_record("b", MemoryRecordType::PlanCompleted));
        store.clear();
        assert!(store.is_empty());
    }

    #[test]
    fn store_clone_shares_state() {
        let store1 = PlanMemoryStore::new();
        let store2 = store1.clone();
        store1.insert(make_record("r1", MemoryRecordType::PlanCreated));
        assert_eq!(store2.len(), 1);
    }

    // Serialization tests

    #[test]
    fn record_serialization_roundtrip() {
        let r = make_record("r1", MemoryRecordType::PlanCompleted)
            .with_importance(0.75)
            .with_tag("test");
        let json = serde_json::to_string(&r).unwrap();
        let back: PlanMemoryRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "r1");
        assert_eq!(back.record_type, MemoryRecordType::PlanCompleted);
        assert!((back.importance - 0.75).abs() < f64::EPSILON);
    }
}
