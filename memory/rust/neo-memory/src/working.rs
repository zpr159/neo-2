use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::types::{MemoryEntry, MemoryId, MemoryPriority};

/// Configuration for working memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingMemoryConfig {
    /// Maximum total capacity (number of entries).
    pub max_capacity: usize,
    /// Number of attention slots (high-priority permanent slots).
    pub attention_slots: usize,
    /// Default expiration time in seconds (0 = no expiration).
    pub default_expiration_secs: u64,
    /// Whether to evict lowest priority first when at capacity.
    pub priority_eviction: bool,
    /// Maximum combined estimated tokens.
    pub max_tokens: usize,
}

impl Default for WorkingMemoryConfig {
    fn default() -> Self {
        Self {
            max_capacity: 50,
            attention_slots: 5,
            default_expiration_secs: 1800, // 30 minutes
            priority_eviction: true,
            max_tokens: 8192,
        }
    }
}

/// Statistics about working memory usage.
#[derive(Debug, Clone, Default)]
pub struct WorkingMemoryStats {
    /// Current number of entries.
    pub current_size: usize,
    /// Maximum capacity.
    pub max_capacity: usize,
    /// Number of attention slot entries.
    pub attention_used: usize,
    /// Total attention slots.
    pub attention_slots: usize,
    /// Total evictions performed.
    pub total_evictions: u64,
    /// Total entries inserted.
    pub total_insertions: u64,
    /// Total lookups.
    pub total_lookups: u64,
    /// Total cache hits (lookups that found the entry).
    pub cache_hits: u64,
    /// Total tokens currently used.
    pub current_tokens: usize,
    /// Maximum tokens.
    pub max_tokens: usize,
}

impl WorkingMemoryStats {
    /// Cache hit rate.
    #[must_use]
    pub fn hit_rate(&self) -> f64 {
        if self.total_lookups == 0 {
            0.0
        } else {
            self.cache_hits as f64 / self.total_lookups as f64
        }
    }

    /// Capacity utilization.
    #[must_use]
    pub fn utilization(&self) -> f64 {
        if self.max_capacity == 0 {
            0.0
        } else {
            self.current_size as f64 / self.max_capacity as f64
        }
    }
}

/// Fixed-size, priority-aware working memory with attention slots.
///
/// Operates as a bounded queue with priority-based eviction. Attention slots
/// provide reserved capacity for critical information that should not be evicted.
#[derive(Debug)]
pub struct WorkingMemory {
    /// Regular entries ordered by insertion time (oldest first).
    entries: RwLock<VecDeque<MemoryEntry>>,
    /// Attention slot entries that are protected from eviction.
    attention: RwLock<VecDeque<MemoryEntry>>,
    /// Configuration.
    config: WorkingMemoryConfig,
    /// Statistics.
    stats: RwLock<WorkingMemoryStats>,
    /// Whether the working memory is cleared on shutdown.
    clear_on_shutdown: AtomicBool,
}

impl WorkingMemory {
    /// Create a new working memory with the given configuration.
    #[must_use]
    pub fn new(config: WorkingMemoryConfig) -> Self {
        let max_tokens = config.max_tokens;
        let max_capacity = config.max_capacity;
        let attention_slots = config.attention_slots;
        Self {
            entries: RwLock::new(VecDeque::with_capacity(max_capacity)),
            attention: RwLock::new(VecDeque::with_capacity(attention_slots)),
            config,
            stats: RwLock::new(WorkingMemoryStats {
                max_capacity,
                attention_slots,
                max_tokens,
                ..WorkingMemoryStats::default()
            }),
            clear_on_shutdown: AtomicBool::new(true),
        }
    }

    /// Insert an entry into working memory.
    ///
    /// If the entry has Critical priority and attention slots are available,
    /// it is placed in an attention slot. Otherwise, it is appended to the
    /// regular queue. If at capacity, the lowest-priority oldest entry is evicted.
    ///
    /// Returns the evicted entry if one was removed.
    pub fn push(&self, mut entry: MemoryEntry) -> Option<MemoryEntry> {
        let mut evicted = None;

        if entry.priority == MemoryPriority::Critical {
            let mut attention = self.attention.write();
            if attention.len() < self.config.attention_slots {
                let mut stats = self.stats.write();
                stats.attention_used = attention.len() + 1;
                stats.total_insertions += 1;
                stats.current_size += 1;
                stats.current_tokens += entry.estimated_tokens;
                entry.access();
                attention.push_back(entry);
                return None;
            }
        }

        let mut entries = self.entries.write();
        let mut stats = self.stats.write();

        // Check capacity and evict if necessary.
        let total = entries.len() + self.attention.read().len();
        if total >= self.config.max_capacity {
            if self.config.priority_eviction {
                evicted = self.evict_lowest_priority(&mut entries, &mut stats);
            } else if let Some(old) = entries.pop_front() {
                stats.total_evictions += 1;
                stats.current_tokens = stats.current_tokens.saturating_sub(old.estimated_tokens);
                evicted = Some(old);
            }
        } else {
            stats.current_size = total + 1;
        }

        stats.total_insertions += 1;
        stats.current_tokens += entry.estimated_tokens;
        entry.access();
        entries.push_back(entry);
        evicted
    }

    /// Remove and return the most recently inserted entry from regular slots.
    #[must_use]
    pub fn pop(&self) -> Option<MemoryEntry> {
        let mut entries = self.entries.write();
        let entry = entries.pop_back();
        if let Some(ref e) = entry {
            let mut stats = self.stats.write();
            stats.current_size = stats.current_size.saturating_sub(1);
            stats.current_tokens = stats.current_tokens.saturating_sub(e.estimated_tokens);
        }
        entry
    }

    /// Peek at the most recently inserted entry without removing it.
    #[must_use]
    pub fn peek(&self) -> Option<MemoryEntry> {
        // Check attention slots first (most recently inserted critical).
        {
            let attention = self.attention.read();
            if let Some(back) = attention.back() {
                return Some(back.clone());
            }
        }
        self.entries.read().back().cloned()
    }

    /// Retrieve an entry by id, searching attention slots first, then regular entries.
    ///
    /// Increments access count on hit.
    pub fn get(&self, id: MemoryId) -> Option<MemoryEntry> {
        self.stats.write().total_lookups += 1;

        // Search attention slots first.
        {
            let attention = self.attention.read();
            if let Some(entry) = attention.iter().find(|e| e.id == id) {
                entry.access();
                self.stats.write().cache_hits += 1;
                return Some(entry.clone());
            }
        }

        // Search regular entries.
        {
            let entries = self.entries.read();
            if let Some(entry) = entries.iter().find(|e| e.id == id) {
                entry.access();
                self.stats.write().cache_hits += 1;
                return Some(entry.clone());
            }
        }

        None
    }

    /// Check whether an entry with the given id exists.
    #[must_use]
    pub fn contains(&self, id: MemoryId) -> bool {
        if self.attention.read().iter().any(|e| e.id == id) {
            return true;
        }
        self.entries.read().iter().any(|e| e.id == id)
    }

    /// Remove a specific entry by id.
    pub fn remove(&self, id: MemoryId) -> Option<MemoryEntry> {
        // Try attention slots.
        {
            let mut attention = self.attention.write();
            if let Some(pos) = attention.iter().position(|e| e.id == id) {
                let removed = attention.remove(pos);
                if let Some(ref e) = removed {
                    let mut stats = self.stats.write();
                    stats.current_size = stats.current_size.saturating_sub(1);
                    stats.current_tokens = stats.current_tokens.saturating_sub(e.estimated_tokens);
                }
                return removed;
            }
        }

        // Try regular entries.
        {
            let mut entries = self.entries.write();
            if let Some(pos) = entries.iter().position(|e| e.id == id) {
                let removed = entries.remove(pos);
                if let Some(ref e) = removed {
                    let mut stats = self.stats.write();
                    stats.current_size = stats.current_size.saturating_sub(1);
                    stats.current_tokens = stats.current_tokens.saturating_sub(e.estimated_tokens);
                }
                return removed;
            }
        }

        None
    }

    /// Remove all expired entries.
    pub fn evict_expired(&self) -> Vec<MemoryEntry> {
        let mut evicted = Vec::new();

        {
            let mut attention = self.attention.write();
            let before = attention.len();
            attention.retain(|e| {
                if e.is_expired() {
                    evicted.push(e.clone());
                    false
                } else {
                    true
                }
            });
            if attention.len() < before {
                let mut stats = self.stats.write();
                stats.current_size -= before - attention.len();
                stats.attention_used = attention.len();
            }
        }

        {
            let mut entries = self.entries.write();
            let before = entries.len();
            entries.retain(|e| {
                if e.is_expired() {
                    evicted.push(e.clone());
                    false
                } else {
                    true
                }
            });
            if entries.len() < before {
                let mut stats = self.stats.write();
                stats.current_size -= before - entries.len();
            }
        }

        {
            let mut stats = self.stats.write();
            for e in &evicted {
                stats.current_tokens = stats.current_tokens.saturating_sub(e.estimated_tokens);
            }
        }

        evicted
    }

    /// Return the current number of entries (attention + regular).
    #[must_use]
    pub fn len(&self) -> usize {
        self.attention.read().len() + self.entries.read().len()
    }

    /// Return whether the memory is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return whether all capacity is used.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.len() >= self.config.max_capacity
    }

    /// Return current token usage.
    #[must_use]
    pub fn current_tokens(&self) -> usize {
        self.stats.read().current_tokens
    }

    /// Clear all entries.
    pub fn clear(&self) {
        self.attention.write().clear();
        self.entries.write().clear();
        let mut stats = self.stats.write();
        stats.current_size = 0;
        stats.attention_used = 0;
        stats.current_tokens = 0;
    }

    /// Return a snapshot of all current entries (attention + regular).
    #[must_use]
    pub fn entries(&self) -> Vec<MemoryEntry> {
        let attention = self.attention.read();
        let entries = self.entries.read();
        let mut result: Vec<MemoryEntry> = Vec::with_capacity(attention.len() + entries.len());
        result.extend(attention.iter().cloned());
        result.extend(entries.iter().cloned());
        result
    }

    /// Return entries sorted by score (descending).
    #[must_use]
    pub fn entries_by_score(&self) -> Vec<MemoryEntry> {
        let mut all = self.entries();
        all.sort_by(|a, b| {
            b.score()
                .partial_cmp(&a.score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        all
    }

    /// Get current statistics snapshot.
    #[must_use]
    pub fn stats(&self) -> WorkingMemoryStats {
        let mut stats = self.stats.read().clone();
        stats.current_size = self.len();
        stats.attention_used = self.attention.read().len();
        stats
    }

    /// Promote an entry from regular to attention slot if slot is available.
    /// Returns true if promoted.
    pub fn promote_to_attention(&self, id: MemoryId) -> bool {
        let mut entries = self.entries.write();
        if let Some(pos) = entries.iter().position(|e| e.id == id) {
            let mut attention = self.attention.write();
            if attention.len() < self.config.attention_slots {
                let entry = entries.remove(pos);
                if let Some(e) = entry {
                    attention.push_back(e);
                    self.stats.write().attention_used = attention.len();
                    return true;
                }
            }
        }
        false
    }

    /// Demote an entry from attention slot to regular queue.
    pub fn demote_from_attention(&self, id: MemoryId) -> bool {
        let mut attention = self.attention.write();
        if let Some(pos) = attention.iter().position(|e| e.id == id) {
            let entry = attention.remove(pos);
            if let Some(e) = entry {
                self.entries.write().push_back(e);
                self.stats.write().attention_used = attention.len();
                return true;
            }
        }
        false
    }

    /// Whether to clear on shutdown.
    #[must_use]
    pub fn should_clear_on_shutdown(&self) -> bool {
        self.clear_on_shutdown.load(Ordering::SeqCst)
    }

    /// Set whether to clear on shutdown.
    pub fn set_clear_on_shutdown(&self, clear: bool) {
        self.clear_on_shutdown.store(clear, Ordering::SeqCst);
    }

    /// Evict the lowest-priority, oldest entry from the regular queue.
    fn evict_lowest_priority(
        &self,
        entries: &mut VecDeque<MemoryEntry>,
        stats: &mut WorkingMemoryStats,
    ) -> Option<MemoryEntry> {
        if entries.is_empty() {
            return None;
        }

        // Find the entry with lowest priority (and among those, the oldest).
        let mut min_idx = 0;
        let mut min_priority = entries[0].priority;

        for (i, entry) in entries.iter().enumerate().skip(1) {
            if entry.priority < min_priority
                || (entry.priority == min_priority && entry.created_at < entries[min_idx].created_at)
            {
                min_idx = i;
                min_priority = entry.priority;
            }
        }

        let evicted = entries.remove(min_idx);
        if let Some(ref e) = evicted {
            stats.total_evictions += 1;
            stats.current_size = stats.current_size.saturating_sub(1);
            stats.current_tokens = stats.current_tokens.saturating_sub(e.estimated_tokens);
        }
        evicted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn make_entry(priority: MemoryPriority, tokens: usize) -> MemoryEntry {
        let mut entry = MemoryEntry::new(
            crate::types::MemoryTier::Working,
            serde_json::json!("test"),
            HashSet::new(),
        );
        entry.priority = priority;
        entry.estimated_tokens = tokens;
        entry
    }

    #[test]
    fn push_and_get() {
        let wm = WorkingMemory::new(WorkingMemoryConfig {
            max_capacity: 10,
            attention_slots: 2,
            ..WorkingMemoryConfig::default()
        });

        let entry = make_entry(MemoryPriority::Normal, 100);
        let id = entry.id;
        wm.push(entry);

        assert!(wm.contains(id));
        assert_eq!(wm.len(), 1);

        let retrieved = wm.get(id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, id);
    }

    #[test]
    fn capacity_eviction() {
        let wm = WorkingMemory::new(WorkingMemoryConfig {
            max_capacity: 3,
            attention_slots: 0,
            ..WorkingMemoryConfig::default()
        });

        let e1 = make_entry(MemoryPriority::Low, 10);
        let e2 = make_entry(MemoryPriority::Low, 10);
        let e3 = make_entry(MemoryPriority::Low, 10);
        let e4 = make_entry(MemoryPriority::Normal, 10);

        let id1 = e1.id;
        wm.push(e1);
        wm.push(e2);
        wm.push(e3);
        let evicted = wm.push(e4);

        assert!(evicted.is_some());
        assert_eq!(evicted.unwrap().id, id1);
        assert_eq!(wm.len(), 3);
    }

    #[test]
    fn attention_slot_protection() {
        let wm = WorkingMemory::new(WorkingMemoryConfig {
            max_capacity: 3,
            attention_slots: 1,
            ..WorkingMemoryConfig::default()
        });

        let critical = make_entry(MemoryPriority::Critical, 10);
        let critical_id = critical.id;
        wm.push(critical);

        let normal = make_entry(MemoryPriority::Normal, 10);
        wm.push(normal);

        // Critical should be in attention slot, safe from eviction.
        assert!(wm.contains(critical_id));
        assert_eq!(wm.len(), 2);
    }

    #[test]
    fn expiration() {
        let wm = WorkingMemory::new(WorkingMemoryConfig::default());

        let mut entry = make_entry(MemoryPriority::Normal, 10);
        entry.ttl = Some(std::time::Duration::from_secs(0)); // Expire immediately

        wm.push(entry);
        assert_eq!(wm.len(), 1);

        let evicted = wm.evict_expired();
        assert_eq!(evicted.len(), 1);
        assert_eq!(wm.len(), 0);
    }

    #[test]
    fn priority_eviction() {
        let wm = WorkingMemory::new(WorkingMemoryConfig {
            max_capacity: 3,
            attention_slots: 0,
            priority_eviction: true,
            ..WorkingMemoryConfig::default()
        });

        let bg = make_entry(MemoryPriority::Background, 10);
        let normal = make_entry(MemoryPriority::Normal, 10);
        let high = make_entry(MemoryPriority::High, 10);
        let new = make_entry(MemoryPriority::Normal, 10);

        let bg_id = bg.id;
        wm.push(bg);
        wm.push(normal);
        wm.push(high);
        let evicted = wm.push(new);

        // Background should be evicted first.
        assert!(evicted.is_some());
        assert_eq!(evicted.unwrap().id, bg_id);
    }

    #[test]
    fn stats_tracking() {
        let wm = WorkingMemory::new(WorkingMemoryConfig {
            max_capacity: 10,
            attention_slots: 2,
            ..WorkingMemoryConfig::default()
        });

        let e1 = make_entry(MemoryPriority::Normal, 100);
        let id1 = e1.id;
        wm.push(e1);

        let e2 = make_entry(MemoryPriority::High, 200);
        wm.push(e2);

        let stats = wm.stats();
        assert_eq!(stats.total_insertions, 2);
        assert_eq!(stats.current_tokens, 300);
        assert_eq!(stats.current_size, 2);

        wm.get(id1);
        let stats = wm.stats();
        assert_eq!(stats.total_lookups, 1);
        assert_eq!(stats.cache_hits, 1);
    }

    #[test]
    fn clear() {
        let wm = WorkingMemory::new(WorkingMemoryConfig::default());
        wm.push(make_entry(MemoryPriority::Normal, 10));
        wm.push(make_entry(MemoryPriority::Normal, 10));
        assert_eq!(wm.len(), 2);

        wm.clear();
        assert_eq!(wm.len(), 0);
        assert!(wm.is_empty());
    }

    #[test]
    fn remove_by_id() {
        let wm = WorkingMemory::new(WorkingMemoryConfig::default());
        let entry = make_entry(MemoryPriority::Normal, 10);
        let id = entry.id;
        wm.push(entry);

        let removed = wm.remove(id);
        assert!(removed.is_some());
        assert!(!wm.contains(id));
    }

    #[test]
    fn entries_by_score() {
        let wm = WorkingMemory::new(WorkingMemoryConfig::default());

        let mut low = make_entry(MemoryPriority::Low, 10);
        low.importance = 0.1;
        let mut high = make_entry(MemoryPriority::High, 10);
        high.importance = 0.9;

        let high_id = high.id;
        wm.push(low);
        wm.push(high);

        let sorted = wm.entries_by_score();
        assert!(!sorted.is_empty());
        assert_eq!(sorted[0].id, high_id);
    }
}
