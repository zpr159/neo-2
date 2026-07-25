use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::{ScheduleId, WorkflowId};
use crate::error::WorkflowResult;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScheduleIdWrapper(pub ScheduleId);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScheduleType {
    /// Run once at a specific time.
    Once,
    /// Run on a cron expression (e.g., "0 9 * * *").
    Cron(String),
    /// Run at a fixed interval in milliseconds.
    Interval(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    pub id: ScheduleId,
    pub workflow_id: WorkflowId,
    pub schedule_type: ScheduleType,
    pub enabled: bool,
    pub max_executions: Option<u32>,
    pub execution_count: u32,
    pub next_execution: Option<DateTime<Utc>>,
    pub last_execution: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub payload: serde_json::Value,
}

impl ScheduleConfig {
    #[must_use]
    pub fn new(workflow_id: WorkflowId, schedule_type: ScheduleType) -> Self {
        Self {
            id: ScheduleId::new(),
            workflow_id,
            schedule_type,
            enabled: true,
            max_executions: None,
            execution_count: 0,
            next_execution: None,
            last_execution: None,
            created_at: Utc::now(),
            payload: serde_json::Value::Null,
        }
    }

    /// Check if the schedule should trigger now.
    #[must_use]
    pub fn should_trigger(&self, now: DateTime<Utc>) -> bool {
        if !self.enabled {
            return false;
        }
        if let Some(max) = self.max_executions {
            if self.execution_count >= max {
                return false;
            }
        }
        if let Some(next) = self.next_execution {
            return now >= next;
        }
        // No next_execution set: trigger for Once or Interval (first time)
        matches!(
            self.schedule_type,
            ScheduleType::Once | ScheduleType::Interval(_)
        ) && self.execution_count == 0
    }

    /// Record that the schedule was triggered.
    pub fn record_trigger(&mut self, now: DateTime<Utc>) {
        self.execution_count += 1;
        self.last_execution = Some(now);
        match &self.schedule_type {
            ScheduleType::Once => {
                self.enabled = false;
            }
            ScheduleType::Interval(ms) => {
                self.next_execution = Some(now + chrono::Duration::milliseconds(*ms as i64));
            }
            ScheduleType::Cron(_) => {
                // Simplified: schedule 1 hour from now
                self.next_execution = Some(now + chrono::Duration::hours(1));
            }
        }
    }
}

/// Manages workflow schedules.
#[derive(Debug)]
pub struct ScheduleManager {
    schedules: Vec<ScheduleConfig>,
}

impl ScheduleManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            schedules: Vec::new(),
        }
    }

    pub fn add(&mut self, config: ScheduleConfig) {
        self.schedules.push(config);
    }

    pub fn remove(&mut self, schedule_id: &ScheduleId) -> bool {
        let len = self.schedules.len();
        self.schedules.retain(|s| s.id != *schedule_id);
        self.schedules.len() < len
    }

    /// Get all schedules that should trigger at the given time.
    #[must_use]
    pub fn get_due(&self, now: DateTime<Utc>) -> Vec<&ScheduleConfig> {
        self.schedules
            .iter()
            .filter(|s| s.should_trigger(now))
            .collect()
    }

    /// Get all schedules for a workflow.
    #[must_use]
    pub fn for_workflow(&self, workflow_id: &WorkflowId) -> Vec<&ScheduleConfig> {
        self.schedules
            .iter()
            .filter(|s| s.workflow_id == *workflow_id)
            .collect()
    }

    /// Get schedule by ID.
    #[must_use]
    pub fn get(&self, schedule_id: &ScheduleId) -> Option<&ScheduleConfig> {
        self.schedules.iter().find(|s| s.id == *schedule_id)
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.schedules.len()
    }

    #[must_use]
    pub fn active_count(&self) -> usize {
        self.schedules.iter().filter(|s| s.enabled).count()
    }
}

impl Default for ScheduleManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn once_schedule_triggers_once() {
        let wf_id = WorkflowId::new();
        let mut sched = ScheduleConfig::new(wf_id, ScheduleType::Once);
        let now = Utc::now();
        assert!(sched.should_trigger(now));
        sched.record_trigger(now);
        assert!(!sched.should_trigger(now));
        assert_eq!(sched.execution_count, 1);
    }

    #[test]
    fn interval_schedule() {
        let wf_id = WorkflowId::new();
        let mut sched = ScheduleConfig::new(wf_id, ScheduleType::Interval(60000));
        let now = Utc::now();
        assert!(sched.should_trigger(now));
        sched.record_trigger(now);
        let future = now + chrono::Duration::seconds(30);
        assert!(!sched.should_trigger(future));
        let later = now + chrono::Duration::seconds(61);
        assert!(sched.should_trigger(later));
    }

    #[test]
    fn disabled_schedule() {
        let wf_id = WorkflowId::new();
        let mut sched = ScheduleConfig::new(wf_id, ScheduleType::Once);
        sched.enabled = false;
        assert!(!sched.should_trigger(Utc::now()));
    }

    #[test]
    fn max_executions() {
        let wf_id = WorkflowId::new();
        let mut sched = ScheduleConfig::new(wf_id, ScheduleType::Interval(1000));
        sched.max_executions = Some(2);
        let now = Utc::now();
        sched.record_trigger(now);
        sched.record_trigger(now);
        assert!(!sched.should_trigger(now));
    }

    #[test]
    fn schedule_manager() {
        let mut mgr = ScheduleManager::new();
        let wf_id = WorkflowId::new();
        let sched = ScheduleConfig::new(wf_id, ScheduleType::Once);
        let id = sched.id;
        mgr.add(sched);
        assert_eq!(mgr.count(), 1);
        assert_eq!(mgr.active_count(), 1);
        assert!(mgr.get(&id).is_some());
        mgr.remove(&id);
        assert_eq!(mgr.count(), 0);
    }

    #[test]
    fn get_due() {
        let mut mgr = ScheduleManager::new();
        let wf_id = WorkflowId::new();
        let mut sched = ScheduleConfig::new(wf_id, ScheduleType::Once);
        sched.next_execution = Some(Utc::now() - chrono::Duration::hours(1));
        mgr.add(sched);
        let due = mgr.get_due(Utc::now());
        assert_eq!(due.len(), 1);
    }
}
