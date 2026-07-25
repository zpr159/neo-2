use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::{HistoryEntryId, HistoryEntryType, WorldVersion};

/// A single history entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: HistoryEntryId,
    pub entry_type: HistoryEntryType,
    pub entity_id: Option<String>,
    pub description: String,
    pub version: WorldVersion,
    pub timestamp: DateTime<Utc>,
    pub details: serde_json::Value,
}

impl HistoryEntry {
    pub fn new(
        entry_type: HistoryEntryType,
        description: impl Into<String>,
        version: WorldVersion,
    ) -> Self {
        Self {
            id: HistoryEntryId::random(),
            entry_type,
            entity_id: None,
            description: description.into(),
            version,
            timestamp: Utc::now(),
            details: serde_json::Value::Null,
        }
    }

    pub fn for_entity(mut self, entity_id: impl Into<String>) -> Self {
        self.entity_id = Some(entity_id.into());
        self
    }
}

/// Manages world history.
pub struct HistoryManager {
    entries: Vec<HistoryEntry>,
    max_entries: usize,
}

impl HistoryManager {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_entries,
        }
    }

    pub fn record(&mut self, entry: HistoryEntry) {
        self.entries.push(entry);
        if self.entries.len() > self.max_entries {
            self.entries.drain(..self.entries.len() - self.max_entries);
        }
    }

    pub fn recent(&self, count: usize) -> Vec<&HistoryEntry> {
        self.entries.iter().rev().take(count).collect()
    }

    pub fn for_entity(&self, entity_id: &str) -> Vec<&HistoryEntry> {
        self.entries
            .iter()
            .filter(|e| e.entity_id.as_deref() == Some(entity_id))
            .collect()
    }

    pub fn by_type(&self, entry_type: &HistoryEntryType) -> Vec<&HistoryEntry> {
        self.entries
            .iter()
            .filter(|e| &e.entry_type == entry_type)
            .collect()
    }

    pub fn in_range(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<&HistoryEntry> {
        self.entries
            .iter()
            .filter(|e| e.timestamp >= from && e.timestamp <= to)
            .collect()
    }

    pub fn total_count(&self) -> usize {
        self.entries.len()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

impl Default for HistoryManager {
    fn default() -> Self {
        Self::new(100_000)
    }
}
