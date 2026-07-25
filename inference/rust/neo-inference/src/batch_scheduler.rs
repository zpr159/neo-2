use std::sync::Mutex;
use std::time::{Duration, Instant};
use uuid::Uuid;
use serde::{Deserialize, Serialize};

use crate::request::RequestId;

/// A single item queued for batch processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchItem {
    pub request_id: RequestId,
    pub model_id: Uuid,
    pub input: serde_json::Value,
    pub added_at: Instant,
}

impl BatchItem {
    /// Creates a new batch item.
    pub fn new(request_id: RequestId, model_id: Uuid, input: serde_json::Value) -> Self {
        Self {
            request_id,
            model_id,
            input,
            added_at: Instant::now(),
        }
    }

    /// Returns how long this item has been waiting.
    pub fn age(&self) -> Duration {
        self.added_at.elapsed()
    }
}

/// Manages batching of inference requests for efficient GPU utilisation.
#[derive(Debug)]
pub struct BatchScheduler {
    max_batch_size: usize,
    pending: Mutex<Vec<BatchItem>>,
    timeout: Duration,
}

impl BatchScheduler {
    /// Creates a new batch scheduler.
    pub fn new(max_batch_size: usize, timeout: Duration) -> Self {
        Self {
            max_batch_size,
            pending: Mutex::new(Vec::new()),
            timeout,
        }
    }

    /// Adds an item to the pending queue.
    pub fn add(&self, item: BatchItem) {
        if let Ok(mut pending) = self.pending.lock() {
            pending.push(item);
        }
    }

    /// Drains all pending items and returns them as a batch.
    pub fn drain_batch(&self) -> Vec<BatchItem> {
        self.pending
            .lock()
            .map(|mut pending| {
                std::mem::take(&mut *pending)
            })
            .unwrap_or_default()
    }

    /// Returns the number of items currently pending.
    pub fn pending_count(&self) -> usize {
        self.pending.lock().map(|p| p.len()).unwrap_or(0)
    }

    /// Returns true if the batch should be flushed (full or oldest item timed out).
    pub fn should_flush(&self) -> bool {
        let pending = match self.pending.lock() {
            Ok(p) => p,
            Err(_) => return false,
        };

        if pending.len() >= self.max_batch_size {
            return true;
        }

        if let Some(oldest) = pending.first() {
            return oldest.age() >= self.timeout;
        }

        false
    }

    /// Returns the maximum batch size.
    pub fn max_batch_size(&self) -> usize {
        self.max_batch_size
    }

    /// Returns the flush timeout duration.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}
