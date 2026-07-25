use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering as AtomicOrdering};
use std::time::Instant;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InferencePriority {
    Critical = 0,
    High = 1,
    Normal = 2,
    Low = 3,
    Background = 4,
}

impl InferencePriority {
    #[must_use]
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Critical,
            1 => Self::High,
            2 => Self::Normal,
            3 => Self::Low,
            _ => Self::Background,
        }
    }
}

impl Ord for InferencePriority {
    fn cmp(&self, other: &Self) -> Ordering {
        (*other as u8).cmp(&(*self as u8))
    }
}

impl PartialOrd for InferencePriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledRequest {
    pub request_id: String,
    pub model_id: String,
    pub priority: InferencePriority,
    pub submitted_at: chrono::DateTime<chrono::Utc>,
    pub deadline_ms: Option<u64>,
    pub estimated_tokens: Option<usize>,
    pub device_preference: Option<String>,
    pub payload_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub max_queue_size: usize,
    pub max_concurrent: usize,
    pub gpu_max_concurrent: usize,
    pub batch_timeout_ms: u64,
    pub max_batch_size: usize,
    pub enable_dynamic_batching: bool,
    pub enable_priority_scheduling: bool,
    pub worker_threads: usize,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 4096,
            max_concurrent: 64,
            gpu_max_concurrent: 8,
            batch_timeout_ms: 100,
            max_batch_size: 32,
            enable_dynamic_batching: true,
            enable_priority_scheduling: true,
            worker_threads: 4,
        }
    }
}

struct PriorityEntry {
    request: ScheduledRequest,
    sequence: u64,
}

impl PartialEq for PriorityEntry {
    fn eq(&self, other: &Self) -> bool {
        self.request.priority == other.request.priority && self.sequence == other.sequence
    }
}

impl Eq for PriorityEntry {}

impl Ord for PriorityEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.request
            .priority
            .cmp(&other.request.priority)
            .then_with(|| self.sequence.cmp(&other.sequence).reverse())
    }
}

impl PartialOrd for PriorityEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct InferenceScheduler {
    config: SchedulerConfig,
    queue: parking_lot::Mutex<BinaryHeap<PriorityEntry>>,
    sequence_counter: AtomicU64,
    active_count: AtomicUsize,
    total_submitted: AtomicU64,
    total_completed: AtomicU64,
    total_dropped: AtomicU64,
}

impl std::fmt::Debug for InferenceScheduler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InferenceScheduler")
            .field("config", &self.config)
            .field("queue_len", &self.queue_len())
            .field("active_count", &self.active_count.load(AtomicOrdering::Relaxed))
            .finish()
    }
}

impl InferenceScheduler {
    pub fn new(config: SchedulerConfig) -> Self {
        Self {
            config,
            queue: parking_lot::Mutex::new(BinaryHeap::new()),
            sequence_counter: AtomicU64::new(0),
            active_count: AtomicUsize::new(0),
            total_submitted: AtomicU64::new(0),
            total_completed: AtomicU64::new(0),
            total_dropped: AtomicU64::new(0),
        }
    }

    pub fn submit(&self, request: ScheduledRequest) -> bool {
        let queue_len = {
            let q = self.queue.lock();
            q.len()
        };
        if queue_len >= self.config.max_queue_size {
            self.total_dropped.fetch_add(1, AtomicOrdering::Relaxed);
            return false;
        }
        let seq = self.sequence_counter.fetch_add(1, AtomicOrdering::SeqCst);
        let entry = PriorityEntry {
            request,
            sequence: seq,
        };
        self.queue.lock().push(entry);
        self.total_submitted.fetch_add(1, AtomicOrdering::Relaxed);
        true
    }

    pub fn dequeue(&self) -> Option<ScheduledRequest> {
        if self.active_count.load(AtomicOrdering::Relaxed) >= self.config.max_concurrent {
            return None;
        }
        let entry = self.queue.lock().pop()?;
        self.active_count.fetch_add(1, AtomicOrdering::SeqCst);
        Some(entry.request)
    }

    pub fn dequeue_batch(&self, max_size: usize) -> Vec<ScheduledRequest> {
        let mut batch = Vec::with_capacity(max_size);
        let available = self
            .config
            .max_concurrent
            .saturating_sub(self.active_count.load(AtomicOrdering::Relaxed));
        let batch_size = max_size.min(available);
        for _ in 0..batch_size {
            if let Some(entry) = self.dequeue() {
                batch.push(entry);
            } else {
                break;
            }
        }
        batch
    }

    pub fn complete(&self) {
        self.active_count.fetch_sub(1, AtomicOrdering::SeqCst);
        self.total_completed.fetch_add(1, AtomicOrdering::Relaxed);
    }

    pub fn cancel(&self, request_id: &str) -> bool {
        let mut queue = self.queue.lock();
        let len_before = queue.len();
        let remaining: BinaryHeap<PriorityEntry> = queue
            .drain()
            .filter(|e| e.request.request_id != request_id)
            .collect();
        let cancelled = len_before - remaining.len();
        *queue = remaining;
        if cancelled > 0 {
            self.total_dropped.fetch_add(cancelled as u64, AtomicOrdering::Relaxed);
            true
        } else {
            false
        }
    }

    pub fn peek_deadline(&self) -> Option<Instant> {
        let queue = self.queue.lock();
        queue.iter().filter_map(|e| {
            e.request.deadline_ms?;
            Some(e.request.submitted_at)
        }).min()
        .and_then(|_| None)
    }

    #[must_use]
    pub fn queue_len(&self) -> usize {
        self.queue.lock().len()
    }

    #[must_use]
    pub fn active_count(&self) -> usize {
        self.active_count.load(AtomicOrdering::Relaxed)
    }

    #[must_use]
    pub fn available_slots(&self) -> usize {
        self.config
            .max_concurrent
            .saturating_sub(self.active_count.load(AtomicOrdering::Relaxed))
    }

    #[must_use]
    pub fn statistics(&self) -> SchedulerStatistics {
        SchedulerStatistics {
            queue_length: self.queue_len(),
            active_count: self.active_count(),
            total_submitted: self.total_submitted.load(AtomicOrdering::Relaxed),
            total_completed: self.total_completed.load(AtomicOrdering::Relaxed),
            total_dropped: self.total_dropped.load(AtomicOrdering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerStatistics {
    pub queue_length: usize,
    pub active_count: usize,
    pub total_submitted: u64,
    pub total_completed: u64,
    pub total_dropped: u64,
}
