use chrono::{DateTime, Utc};

use crate::core::entity::Entity;

/// Searches entities by temporal attributes.
pub struct TemporalSearch;

impl TemporalSearch {
    /// Create a new temporal search.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Search entities created after a given timestamp.
    #[must_use]
    pub fn created_after(&self, entities: &[Entity], after: DateTime<Utc>) -> Vec<Entity> {
        entities
            .iter()
            .filter(|e| e.active && e.created_at > after)
            .cloned()
            .collect()
    }

    /// Search entities created before a given timestamp.
    #[must_use]
    pub fn created_before(&self, entities: &[Entity], before: DateTime<Utc>) -> Vec<Entity> {
        entities
            .iter()
            .filter(|e| e.active && e.created_at < before)
            .cloned()
            .collect()
    }

    /// Search entities updated within the last N seconds.
    #[must_use]
    pub fn updated_within(&self, entities: &[Entity], seconds: i64) -> Vec<Entity> {
        let cutoff = Utc::now() - chrono::Duration::seconds(seconds);
        entities
            .iter()
            .filter(|e| e.active && e.updated_at > cutoff)
            .cloned()
            .collect()
    }

    /// Search entities created within a time range.
    #[must_use]
    pub fn created_between(
        &self,
        entities: &[Entity],
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<Entity> {
        entities
            .iter()
            .filter(|e| e.active && e.created_at >= start && e.created_at <= end)
            .cloned()
            .collect()
    }

    /// Search entities updated in the last hour.
    #[must_use]
    pub fn recently_updated(&self, entities: &[Entity]) -> Vec<Entity> {
        self.updated_within(entities, 3600)
    }

    /// Search entities created today.
    #[must_use]
    pub fn created_today(&self, entities: &[Entity]) -> Vec<Entity> {
        let today_start = Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap())
            .and_local_timezone(Utc)
            .unwrap();
        self.created_after(entities, today_start)
    }
}

impl Default for TemporalSearch {
    fn default() -> Self {
        Self::new()
    }
}
