use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::MemoryResult;
use crate::types::{
    MemoryEntry, MemoryId, MemoryNamespace, MemoryPriority, MemoryTier,
};

/// Request to store a memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreRequest {
    /// Memory tier.
    pub tier: MemoryTier,
    /// Content to store.
    pub content: serde_json::Value,
    /// Tags.
    pub tags: Vec<String>,
    /// Importance score.
    pub importance: Option<f32>,
    /// Priority.
    pub priority: Option<MemoryPriority>,
    /// Namespace.
    pub namespace: Option<String>,
    /// TTL in seconds.
    pub ttl_secs: Option<u64>,
    /// Source attribution.
    pub source: Option<String>,
}

/// Response from storing a memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreResponse {
    /// The id of the stored memory.
    pub id: String,
    /// When it was created.
    pub created_at: DateTime<Utc>,
}

/// Request to update a memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRequest {
    /// New content.
    pub content: Option<serde_json::Value>,
    /// New tags.
    pub tags: Option<Vec<String>>,
    /// New importance.
    pub importance: Option<f32>,
    /// New priority.
    pub priority: Option<MemoryPriority>,
}

/// Request to search memories.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchRequest {
    /// Text query.
    pub query: Option<String>,
    /// Tiers to search.
    pub tiers: Option<Vec<String>>,
    /// Limit.
    pub limit: Option<usize>,
    /// Filter by tags.
    pub tags: Option<Vec<String>>,
    /// Filter by namespace.
    pub namespace: Option<String>,
    /// Minimum importance.
    pub min_importance: Option<f32>,
}

/// Response from searching memories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    /// Matching entries.
    pub results: Vec<MemoryEntrySummary>,
    /// Total matches found.
    pub total: usize,
}

/// Summary of a memory entry for API responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntrySummary {
    /// Memory id.
    pub id: String,
    /// Tier.
    pub tier: String,
    /// Content preview.
    pub content_preview: String,
    /// Tags.
    pub tags: Vec<String>,
    /// Importance.
    pub importance: f32,
    /// Created at.
    pub created_at: String,
    /// Access count.
    pub access_count: u64,
}

impl From<&MemoryEntry> for MemoryEntrySummary {
    fn from(entry: &MemoryEntry) -> Self {
        let content_str = entry.content.to_string();
        let preview = if content_str.len() > 100 {
            format!("{}...", &content_str[..100])
        } else {
            content_str
        };

        Self {
            id: entry.id.to_string(),
            tier: entry.tier.to_string(),
            content_preview: preview,
            tags: entry.tags.iter().cloned().collect(),
            importance: entry.importance,
            created_at: entry.created_at.to_rfc3339(),
            access_count: entry
                .access_count
                .load(std::sync::atomic::Ordering::SeqCst),
        }
    }
}

/// Request to merge memories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeRequest {
    /// Memory ids to merge.
    pub ids: Vec<String>,
    /// Strategy for merging content.
    pub strategy: MergeStrategy,
}

/// Merge strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MergeStrategy {
    /// Keep the most recent version.
    MostRecent,
    /// Concatenate all content.
    Concatenate,
    /// Keep the highest importance entry.
    HighestImportance,
}

/// Response from merging memories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeResponse {
    /// The merged memory id.
    pub merged_id: String,
    /// Number of entries merged.
    pub entries_merged: usize,
}

/// Request to export memories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRequest {
    /// Tiers to export.
    pub tiers: Option<Vec<String>>,
    /// Namespace filter.
    pub namespace: Option<String>,
    /// Export format.
    pub format: ExportFormat,
}

/// Export format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExportFormat {
    /// JSON format.
    Json,
    /// CSV format.
    Csv,
}

/// Response from exporting memories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResponse {
    /// Serialized data.
    pub data: String,
    /// Number of entries exported.
    pub count: usize,
    /// Format used.
    pub format: String,
}

/// Request to import memories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRequest {
    /// Data to import.
    pub data: String,
    /// Format of the data.
    pub format: ExportFormat,
    /// Target namespace.
    pub namespace: Option<String>,
}

/// Response from importing memories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResponse {
    /// Number of entries imported.
    pub count: usize,
    /// Any errors encountered.
    pub errors: Vec<String>,
}

/// Split request: split a large memory into smaller ones.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitRequest {
    /// Memory id to split.
    pub id: String,
    /// Maximum size per chunk.
    pub max_chunk_size: usize,
}

/// Response from splitting a memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitResponse {
    /// New memory ids created.
    pub new_ids: Vec<String>,
    /// Original memory was removed.
    pub original_removed: bool,
}

/// Summarize request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummarizeRequest {
    /// Memory ids to summarize together.
    pub ids: Vec<String>,
    /// Maximum summary length in characters.
    pub max_length: Option<usize>,
}

/// Response from summarization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummarizeResponse {
    /// Generated summary.
    pub summary: String,
    /// Number of entries summarized.
    pub entries_summarized: usize,
}

/// Health check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Overall health status.
    pub status: String,
    /// Total memories stored.
    pub total_memories: u64,
    /// Memories per tier.
    pub per_tier: std::collections::HashMap<String, u64>,
    /// Total bytes stored.
    pub total_bytes: u64,
    /// Cache hit rate.
    pub cache_hit_rate: f64,
    /// Uptime in seconds.
    pub uptime_secs: u64,
}
