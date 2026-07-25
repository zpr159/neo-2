use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::error::{NeoError, NeoResult};
use crate::id::ComponentId;
use crate::time::Timestamp;

/// Unique event identifier.
pub type EventId = uuid::Uuid;

/// An event that flows through the Neo event system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub source: ComponentId,
    pub timestamp: Timestamp,
    pub payload: serde_json::Value,
}

impl Event {
    pub fn new(source: ComponentId, payload: serde_json::Value) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            source,
            timestamp: Timestamp::now(),
            payload,
        }
    }
}

/// Filter for selecting events by source and/or type.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventFilter {
    pub source: Option<ComponentId>,
    pub event_type: Option<String>,
}

impl EventFilter {
    pub fn matches(&self, event: &Event) -> bool {
        if let Some(ref source) = self.source {
            if &event.source != source {
                return false;
            }
        }
        if let Some(ref event_type) = self.event_type {
            if let Some(t) = event.payload.get("type").and_then(|v| v.as_str()) {
                if t != event_type {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    }
}

/// Broadcast event bus for inter-component communication.
pub struct EventBus {
    sender: broadcast::Sender<Event>,
}

impl EventBus {
    /// Creates a new event bus with a default channel capacity of 256.
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(256);
        Self { sender }
    }

    /// Publish an event to all subscribers.
    pub async fn publish(&self, event: Event) -> NeoResult<()> {
        self.sender
            .send(event)
            .map_err(|e| NeoError::Internal(format!("event publish failed: {}", e)))?;
        Ok(())
    }

    /// Subscribe to events matching the given filter.
    pub fn subscribe(&self, filter: EventFilter) -> broadcast::Receiver<Event> {
        let rx = self.sender.subscribe();
        FilteredReceiver { rx, filter }.into_receiver()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Wrapper that filters incoming events before delivering them.
struct FilteredReceiver {
    rx: broadcast::Receiver<Event>,
    filter: EventFilter,
}

impl FilteredReceiver {
    fn into_receiver(self) -> broadcast::Receiver<Event> {
        let (tx, rx) = broadcast::channel(256);
        let mut source_rx = self.rx;
        let filter = self.filter;

        tokio::spawn(async move {
            loop {
                match source_rx.recv().await {
                    Ok(event) => {
                        if filter.matches(&event) {
                            let _ = tx.send(event);
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        });

        rx
    }
}
