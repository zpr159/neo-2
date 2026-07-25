#!\[forbid(unsafe_code)\]
#![deny(
    missing_docs,
    warnings,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_extern_crates
)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use async_trait::async_trait;
use super::types::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LearningEventType {
    LearningStarted,
    LearningCompleted,
    ExperienceRecorded,
    EpisodeCreated,
    PatternDiscovered,
    SkillExtracted,
    ReflectionCompleted,
    KnowledgeConsolidated,
    HeuristicUpdated,
    OptimizationSuggested,
    FailureAnalyzed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningEvent {
    pub event_type: LearningEventType,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub payload: serde_json::Value,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl LearningEvent {
    pub fn new(event_type: LearningEventType, source: impl Into<String>) -> Self {
        Self {
            event_type,
            timestamp: Utc::now(),
            source: source.into(),
            payload: serde_json::Value::Null,
            metadata: HashMap::new(),
        }
    }

    pub fn with_payload(mut self, payload: serde_json::Value) -> Self {
        self.payload = payload;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

pub struct EventBus {
    sender: tokio::sync::broadcast::Sender<LearningEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = tokio::sync::broadcast::channel(capacity);
        Self { sender }
    }

    pub fn publish(&self, event: LearningEvent) -> Result<(), LearningError> {
        self.sender
            .send(event)
            .map_err(|_| LearningError::EventBroadcastError("No receivers available".to_string()))
    }

    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<LearningEvent> {
        self.sender.subscribe()
    }
}

#[async_trait]
pub trait LearningEventHandler: Send + Sync {
    async fn handle_event(&self, event: LearningEvent) -> Result<(), LearningError>;
}

pub struct EventProcessor {
    handlers: Vec<Box<dyn LearningEventHandler + Send + Sync>>,
}

impl EventProcessor {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    pub fn add_handler<H: LearningEventHandler + 'static>(&mut self, handler: H) {
        self.handlers.push(Box::new(handler));
    }

    pub async fn process(&self, event: LearningEvent) -> Result<(), LearningError> {
        for handler in &self.handlers {
            handler.handle_event(event.clone()).await?;
        }
        Ok(())
    }
}

pub struct EventLogger {
    log_level: LogLevel,
    log_file: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl EventLogger {
    pub fn new(log_level: LogLevel, log_file: impl Into<String>) -> Self {
        Self {
            log_level,
            log_file: log_file.into(),
        }
    }

    pub fn log_event(&self, event: LearningEvent) -> Result<(), LearningError> {
        let json = serde_json::to_string(&event)
            .map_err(|e| LearningError::SerializationError(e.to_string()))?;
        
        match self.log_level {
            LogLevel::Trace | LogLevel::Debug | LogLevel::Info | LogLevel::Warn => {
                tracing::info!("LearningEvent: {}", json);
            }
            LogLevel::Error | LogLevel::Fatal => {
                tracing::error!("LearningEvent: {}", json);
            }
        }
        
        Ok(())
    }
}

pub struct EventAnalytics {
    events_processed: u64,
    event_types_processed: HashMap<LearningEventType, u64>,
    sources_processed: HashMap<String, u64>,
    processing_errors: u64,
    last_updated: DateTime<Utc>,
}

impl EventAnalytics {
    pub fn new() -> Self {
        Self {
            events_processed: 0,
            event_types_processed: HashMap::new(),
            sources_processed: HashMap::new(),
            processing_errors: 0,
            last_updated: Utc::now(),
        }
    }

    pub fn record_event(&mut self, event: &LearningEvent) {
        self.events_processed += 1;
        *self.event_types_processed.entry(event.event_type.clone()).or_insert(0) += 1;
        *self.sources_processed.entry(event.source.clone()).or_insert(0) += 1;
        self.last_updated = Utc::now();
    }

    pub fn record_error(&mut self) {
        self.processing_errors += 1;
        self.last_updated = Utc::now();
    }
}
