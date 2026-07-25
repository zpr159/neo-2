use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuditLevel {
    Trace,
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

impl std::fmt::Display for AuditLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditLevel::Trace => write!(f, "TRACE"),
            AuditLevel::Debug => write!(f, "DEBUG"),
            AuditLevel::Info => write!(f, "INFO"),
            AuditLevel::Warning => write!(f, "WARNING"),
            AuditLevel::Error => write!(f, "ERROR"),
            AuditLevel::Critical => write!(f, "CRITICAL"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub level: AuditLevel,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub action: String,
    pub principal: Option<String>,
    pub resource: Option<String>,
    pub result: String,
    pub metadata: std::collections::HashMap<String, String>,
    pub ip_address: Option<String>,
}

impl AuditEvent {
    pub fn new(source: &str, action: &str, result: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            level: AuditLevel::Info,
            timestamp: Utc::now(),
            source: source.to_string(),
            action: action.to_string(),
            principal: None,
            resource: None,
            result: result.to_string(),
            metadata: std::collections::HashMap::new(),
            ip_address: None,
        }
    }

    pub fn with_level(mut self, level: AuditLevel) -> Self {
        self.level = level;
        self
    }

    pub fn with_principal(mut self, principal: &str) -> Self {
        self.principal = Some(principal.to_string());
        self
    }

    pub fn with_resource(mut self, resource: &str) -> Self {
        self.resource = Some(resource.to_string());
        self
    }

    pub fn with_ip(mut self, ip: &str) -> Self {
        self.ip_address = Some(ip.to_string());
        self
    }
}

#[derive(Debug, Clone)]
pub struct AuditFilter {
    pub min_level: Option<AuditLevel>,
    pub source: Option<String>,
    pub principal: Option<String>,
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
}

impl Default for AuditFilter {
    fn default() -> Self {
        Self {
            min_level: None,
            source: None,
            principal: None,
            time_range: None,
        }
    }
}

#[derive(Debug)]
pub struct AuditLogger {
    events: Vec<AuditEvent>,
    min_level: AuditLevel,
}

impl AuditLogger {
    pub fn new(min_level: AuditLevel) -> Self {
        tracing::info!(min_level = %min_level, "audit logger initialized");
        Self {
            events: Vec::new(),
            min_level,
        }
    }

    pub fn log_event(&mut self, event: AuditEvent) {
        if event.level >= self.min_level {
            tracing::debug!(
                level = %event.level,
                source = %event.source,
                action = %event.action,
                "audit event logged"
            );
            self.events.push(event);
        }
    }

    pub fn query(&self, filter: AuditFilter) -> Vec<&AuditEvent> {
        self.events
            .iter()
            .filter(|e| {
                if let Some(ref min_level) = filter.min_level {
                    if e.level < *min_level {
                        return false;
                    }
                }
                if let Some(ref source) = filter.source {
                    if e.source != *source {
                        return false;
                    }
                }
                if let Some(ref principal) = filter.principal {
                    if e.principal.as_deref() != Some(principal.as_str()) {
                        return false;
                    }
                }
                if let Some((start, end)) = filter.time_range {
                    if e.timestamp < start || e.timestamp > end {
                        return false;
                    }
                }
                true
            })
            .collect()
    }

    pub fn recent(&self, count: usize) -> Vec<&AuditEvent> {
        let start = self.events.len().saturating_sub(count);
        self.events[start..].iter().collect()
    }

    pub fn count(&self) -> usize {
        self.events.len()
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new(AuditLevel::Info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_logging() {
        let mut logger = AuditLogger::new(AuditLevel::Trace);
        let event = AuditEvent::new("auth", "login", "success")
            .with_level(AuditLevel::Info)
            .with_principal("admin");

        logger.log_event(event);
        assert_eq!(logger.count(), 1);
    }

    #[test]
    fn test_query_by_level() {
        let mut logger = AuditLogger::new(AuditLevel::Trace);
        logger.log_event(AuditEvent::new("auth", "login", "ok").with_level(AuditLevel::Info));
        logger.log_event(
            AuditEvent::new("auth", "fail", "denied").with_level(AuditLevel::Warning),
        );
        logger.log_event(
            AuditEvent::new("auth", "error", "crash").with_level(AuditLevel::Critical),
        );

        let filter = AuditFilter {
            min_level: Some(AuditLevel::Warning),
            ..Default::default()
        };
        let results = logger.query(filter);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_recent() {
        let mut logger = AuditLogger::new(AuditLevel::Trace);
        for i in 0..10 {
            logger.log_event(AuditEvent::new("src", &format!("action_{}", i), "ok"));
        }
        let recent = logger.recent(3);
        assert_eq!(recent.len(), 3);
    }
}
