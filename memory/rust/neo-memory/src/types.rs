use std::collections::HashSet;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a memory entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryId(pub Uuid);

impl MemoryId {
    /// Create a new random MemoryId.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for MemoryId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for MemoryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for MemoryId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<MemoryId> for Uuid {
    fn from(id: MemoryId) -> Self {
        id.0
    }
}

/// Namespace for memory isolation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryNamespace(pub String);

impl MemoryNamespace {
    /// Create a new namespace.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Default global namespace.
    #[must_use]
    pub fn global() -> Self {
        Self("global".to_string())
    }
}

impl Default for MemoryNamespace {
    fn default() -> Self {
        Self::global()
    }
}

impl fmt::Display for MemoryNamespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Tier of memory indicating storage and retrieval characteristics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MemoryTier {
    /// Short-term, limited-capacity working memory.
    Working = 0,
    /// Experience-based episodic memory with temporal context.
    Episodic = 1,
    /// Fact-based semantic memory with relationships.
    Semantic = 2,
    /// Skill and procedure-based procedural memory.
    Procedural = 3,
    /// Long-term persistent storage.
    LongTerm = 4,
}

impl fmt::Display for MemoryTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Working => write!(f, "working"),
            Self::Episodic => write!(f, "episodic"),
            Self::Semantic => write!(f, "semantic"),
            Self::Procedural => write!(f, "procedural"),
            Self::LongTerm => write!(f, "long_term"),
        }
    }
}

/// Priority level for memory entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MemoryPriority {
    /// Background priority, evicted first.
    Background = 0,
    /// Low priority.
    Low = 1,
    /// Normal priority.
    Normal = 2,
    /// High priority.
    High = 3,
    /// Critical priority, never evicted unless explicit.
    Critical = 4,
}

impl Default for MemoryPriority {
    fn default() -> Self {
        Self::Normal
    }
}

impl fmt::Display for MemoryPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Background => write!(f, "background"),
            Self::Low => write!(f, "low"),
            Self::Normal => write!(f, "normal"),
            Self::High => write!(f, "high"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

/// Status of a memory entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryStatus {
    /// Memory is active and available for retrieval.
    Active,
    /// Memory has been compressed.
    Compressed,
    /// Memory is archived but still accessible.
    Archived,
    /// Memory has been deleted.
    Deleted,
    /// Memory is pinned and cannot be evicted.
    Pinned,
}

impl Default for MemoryStatus {
    fn default() -> Self {
        Self::Active
    }
}

impl fmt::Display for MemoryStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Compressed => write!(f, "compressed"),
            Self::Archived => write!(f, "archived"),
            Self::Deleted => write!(f, "deleted"),
            Self::Pinned => write!(f, "pinned"),
        }
    }
}

/// A single memory entry stored across tiers.
#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Unique identifier for this memory.
    pub id: MemoryId,
    /// The memory tier this entry belongs to.
    pub tier: MemoryTier,
    /// Namespace for isolation.
    pub namespace: MemoryNamespace,
    /// The stored content as arbitrary JSON.
    pub content: serde_json::Value,
    /// Optional embedding vector for similarity search.
    pub embedding: Option<Vec<f32>>,
    /// Tags for categorization and retrieval.
    pub tags: HashSet<String>,
    /// Number of times this memory has been accessed.
    pub access_count: AtomicU64,
    /// Timestamp when this memory was created.
    pub created_at: DateTime<Utc>,
    /// Timestamp when this memory was last accessed.
    pub last_accessed: Mutex<DateTime<Utc>>,
    /// Timestamp when this memory was last modified.
    pub last_modified: Mutex<DateTime<Utc>>,
    /// Importance score between 0.0 and 1.0.
    pub importance: f32,
    /// Novelty score between 0.0 and 1.0.
    pub novelty: f32,
    /// Confidence score between 0.0 and 1.0.
    pub confidence: f32,
    /// Priority level.
    pub priority: MemoryPriority,
    /// Current status.
    pub status: MemoryStatus,
    /// Optional time-to-live after which the memory is considered expired.
    pub ttl: Option<Duration>,
    /// Optional parent id for hierarchical memory.
    pub parent_id: Option<MemoryId>,
    /// Source attribution.
    pub source: Option<String>,
    /// Estimated token count for context budgeting.
    pub estimated_tokens: usize,
    /// Whether this entry has been consolidated from a lower tier.
    pub consolidated: bool,
    /// Version number for optimistic concurrency.
    pub version: u64,
}

impl Clone for MemoryEntry {
    fn clone(&self) -> Self {
        let last_accessed = self
            .last_accessed
            .lock()
            .map_or(self.created_at, |l| *l);
        let last_modified = self
            .last_modified
            .lock()
            .map_or(self.created_at, |l| *l);
        Self {
            id: self.id,
            tier: self.tier,
            namespace: self.namespace.clone(),
            content: self.content.clone(),
            embedding: self.embedding.clone(),
            tags: self.tags.clone(),
            access_count: AtomicU64::new(self.access_count.load(Ordering::SeqCst)),
            created_at: self.created_at,
            last_accessed: Mutex::new(last_accessed),
            last_modified: Mutex::new(last_modified),
            importance: self.importance,
            novelty: self.novelty,
            confidence: self.confidence,
            priority: self.priority,
            status: self.status,
            ttl: self.ttl,
            parent_id: self.parent_id,
            source: self.source.clone(),
            estimated_tokens: self.estimated_tokens,
            consolidated: self.consolidated,
            version: self.version,
        }
    }
}

impl MemoryEntry {
    /// Create a new memory entry.
    pub fn new(tier: MemoryTier, content: serde_json::Value, tags: HashSet<String>) -> Self {
        let now = Utc::now();
        Self {
            id: MemoryId::new(),
            tier,
            namespace: MemoryNamespace::global(),
            content,
            embedding: None,
            tags,
            access_count: AtomicU64::new(0),
            created_at: now,
            last_accessed: Mutex::new(now),
            last_modified: Mutex::new(now),
            importance: 0.5,
            novelty: 0.5,
            confidence: 0.5,
            priority: MemoryPriority::Normal,
            status: MemoryStatus::Active,
            ttl: None,
            parent_id: None,
            source: None,
            estimated_tokens: 0,
            consolidated: false,
            version: 1,
        }
    }

    /// Set the namespace.
    #[must_use]
    pub fn with_namespace(mut self, ns: MemoryNamespace) -> Self {
        self.namespace = ns;
        self
    }

    /// Set the importance.
    #[must_use]
    pub fn with_importance(mut self, importance: f32) -> Self {
        self.importance = importance.clamp(0.0, 1.0);
        self
    }

    /// Set the priority.
    #[must_use]
    pub fn with_priority(mut self, priority: MemoryPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set the TTL.
    #[must_use]
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = Some(ttl);
        self
    }

    /// Set the source.
    #[must_use]
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Set estimated tokens.
    #[must_use]
    pub fn with_tokens(mut self, tokens: usize) -> Self {
        self.estimated_tokens = tokens;
        self
    }

    /// Record an access to this memory.
    pub fn access(&self) {
        self.access_count.fetch_add(1, Ordering::SeqCst);
        if let Ok(mut last) = self.last_accessed.lock() {
            *last = Utc::now();
        }
    }

    /// Check whether this memory has expired based on its TTL.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        match self.ttl {
            Some(ttl) => {
                let elapsed = Utc::now().signed_duration_since(self.created_at);
                let elapsed_duration =
                    Duration::from_millis(elapsed.num_milliseconds().max(0) as u64);
                elapsed_duration >= ttl
            }
            None => false,
        }
    }

    /// Check whether this memory is deletable (not pinned and not deleted).
    #[must_use]
    pub fn is_deletable(&self) -> bool {
        self.status != MemoryStatus::Pinned && self.status != MemoryStatus::Deleted
    }

    /// Mark as deleted (soft delete).
    pub fn mark_deleted(&mut self) {
        self.status = MemoryStatus::Deleted;
        self.touch_modified();
    }

    /// Mark as archived.
    pub fn mark_archived(&mut self) {
        self.status = MemoryStatus::Archived;
        self.touch_modified();
    }

    /// Mark as compressed.
    pub fn mark_compressed(&mut self) {
        self.status = MemoryStatus::Compressed;
        self.touch_modified();
    }

    /// Touch the last_modified timestamp.
    pub fn touch_modified(&self) {
        if let Ok(mut last) = self.last_modified.lock() {
            *last = Utc::now();
        }
    }

    /// Compute a relevance score combining importance, access count, recency, and novelty.
    #[must_use]
    pub fn score(&self) -> f64 {
        let access = self.access_count.load(Ordering::SeqCst) as f64;
        let recency = {
            let last = self
                .last_accessed
                .lock()
                .map_or(self.created_at, |l| *l);
            let elapsed = Utc::now().signed_duration_since(last);
            let minutes = elapsed.num_minutes().max(0) as f64;
            1.0 / (1.0 + minutes / 60.0)
        };
        let importance = self.importance as f64;
        let novelty = self.novelty as f64;
        let confidence = self.confidence as f64;
        let access_score = (access / (access + 10.0)) * 0.15;
        (importance * 0.35) + (recency * 0.25) + (novelty * 0.15) + (confidence * 0.10) + access_score
    }

    /// Check if the entry is active (not deleted or expired).
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.status == MemoryStatus::Active && !self.is_expired()
    }
}

/// Episode outcome result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EpisodeOutcome {
    /// The episode was successful.
    Success,
    /// The episode failed.
    Failure,
    /// The episode was partial or ambiguous.
    Partial,
    /// The outcome is unknown.
    Unknown,
}

impl fmt::Display for EpisodeOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success => write!(f, "success"),
            Self::Failure => write!(f, "failure"),
            Self::Partial => write!(f, "partial"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Retention policy for memory decay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RetentionPolicy {
    /// Retain forever.
    Permanent,
    /// Retain for a specific duration.
    TimeBased,
    /// Retain based on access count.
    AccessBased,
    /// Retain based on importance threshold.
    ImportanceBased,
    /// Retain based on a composite score.
    Composite,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self::Composite
    }
}

/// Permission level for memory access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MemoryPermission {
    /// Read-only access.
    Read = 0,
    /// Read and write access.
    Write = 1,
    /// Full access including delete and admin operations.
    Admin = 2,
}

impl Default for MemoryPermission {
    fn default() -> Self {
        Self::Read
    }
}

impl fmt::Display for MemoryPermission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read => write!(f, "read"),
            Self::Write => write!(f, "write"),
            Self::Admin => write!(f, "admin"),
        }
    }
}

/// Audit log entry for security tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Timestamp of the action.
    pub timestamp: DateTime<Utc>,
    /// The action performed.
    pub action: String,
    /// The memory id affected.
    pub memory_id: MemoryId,
    /// The namespace.
    pub namespace: MemoryNamespace,
    /// The actor performing the action.
    pub actor: String,
    /// Whether the action was permitted.
    pub permitted: bool,
    /// Additional details.
    pub details: Option<String>,
}

/// Retention configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionConfig {
    /// The retention policy to use.
    pub policy: RetentionPolicy,
    /// Maximum age before decay in seconds.
    pub max_age_secs: u64,
    /// Minimum access count before consideration for deletion.
    pub min_access_count: u64,
    /// Minimum importance before consideration for deletion.
    pub min_importance: f32,
    /// Maximum entries per tier before GC triggers.
    pub max_entries_per_tier: usize,
    /// GC interval in seconds.
    pub gc_interval_secs: u64,
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            policy: RetentionPolicy::Composite,
            max_age_secs: 86_400 * 30, // 30 days
            min_access_count: 1,
            min_importance: 0.1,
            max_entries_per_tier: 100_000,
            gc_interval_secs: 3_600, // 1 hour
        }
    }
}

/// Security configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Whether encryption is enabled.
    pub enabled: bool,
    /// Encryption key (hex-encoded).
    pub encryption_key: Option<String>,
    /// Default permission for new memories.
    pub default_permission: MemoryPermission,
    /// Whether audit logging is enabled.
    pub audit_logging: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            encryption_key: None,
            default_permission: MemoryPermission::Read,
            audit_logging: true,
        }
    }
}

/// Analytics snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalyticsSnapshot {
    /// Total memories stored.
    pub total_memories: u64,
    /// Memories per tier.
    pub per_tier: std::collections::HashMap<String, u64>,
    /// Total bytes stored (approximate).
    pub total_bytes: u64,
    /// Total recall attempts.
    pub recall_attempts: u64,
    /// Successful recalls.
    pub recall_hits: u64,
    /// Recall hit rate (hits / attempts).
    pub recall_rate: f64,
    /// Average importance across all memories.
    pub avg_importance: f64,
    /// Memories created in the last hour.
    pub created_last_hour: u64,
    /// Memories accessed in the last hour.
    pub accessed_last_hour: u64,
    /// Compression ratio (compressed / original).
    pub compression_ratio: f64,
    /// Memory health score (0.0 to 1.0).
    pub health_score: f64,
    /// Number of archived memories.
    pub archived_count: u64,
    /// Number of deleted memories pending GC.
    pub deleted_pending_gc: u64,
    /// Namespace counts.
    pub per_namespace: std::collections::HashMap<String, u64>,
}

/// Consolidation status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConsolidationStatus {
    /// Not yet consolidated.
    Pending,
    /// Currently being consolidated.
    InProgress,
    /// Consolidation complete.
    Completed,
    /// Consolidation failed.
    Failed,
}
