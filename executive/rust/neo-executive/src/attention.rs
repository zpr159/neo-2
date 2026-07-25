use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::goal::GoalId;
use crate::task::TaskId;
use crate::error::{ExecutiveError, ExecutiveResult};

/// Unique identifier for an attention context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AttentionId(pub Uuid);

impl AttentionId {
    /// Create a new attention identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for AttentionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Type of interrupt that can steal attention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InterruptType {
    Critical,
    High,
    Normal,
    Low,
}

/// An interrupt that requests attention.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interrupt {
    pub id: AttentionId,
    pub interrupt_type: InterruptType,
    pub source: String,
    pub description: String,
    pub timestamp: DateTime<Utc>,
    pub goal_id: Option<GoalId>,
    pub task_id: Option<TaskId>,
}

impl Interrupt {
    /// Create a new interrupt.
    pub fn new(
        interrupt_type: InterruptType,
        source: String,
        description: String,
    ) -> Self {
        Self {
            id: AttentionId::new(),
            interrupt_type,
            source,
            description,
            timestamp: Utc::now(),
            goal_id: None,
            task_id: None,
        }
    }

    /// Attach a goal to the interrupt.
    pub fn with_goal(mut self, goal_id: GoalId) -> Self {
        self.goal_id = Some(goal_id);
        self
    }

    /// Attach a task to the interrupt.
    pub fn with_task(mut self, task_id: TaskId) -> Self {
        self.task_id = Some(task_id);
        self
    }
}

/// Focus represents the current attention focus of the executive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Focus {
    pub goal_id: Option<GoalId>,
    pub task_id: Option<TaskId>,
    pub description: String,
    pub started_at: DateTime<Utc>,
    pub attention_cost: f64,
}

/// Context switch event for tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSwitchEvent {
    pub timestamp: DateTime<Utc>,
    pub from: Option<String>,
    pub to: String,
    pub reason: String,
    pub duration_ms: u64,
}

/// Attention budget tracks how much attention has been consumed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionBudget {
    pub total_budget: f64,
    pub consumed: f64,
    pub reserved: f64,
}

impl AttentionBudget {
    /// Create a new attention budget.
    pub fn new(total: f64) -> Self {
        Self {
            total_budget: total,
            consumed: 0.0,
            reserved: 0.0,
        }
    }

    /// Check if budget allows an allocation.
    pub fn can_allocate(&self, amount: f64) -> bool {
        self.consumed + self.reserved + amount <= self.total_budget
    }

    /// Reserve budget.
    pub fn reserve(&mut self, amount: f64) -> bool {
        if self.can_allocate(amount) {
            self.reserved += amount;
            true
        } else {
            false
        }
    }

    /// Commit reserved budget.
    pub fn commit(&mut self, amount: f64) {
        self.reserved = (self.reserved - amount).max(0.0);
        self.consumed += amount;
    }

    /// Release consumed budget.
    pub fn release(&mut self, amount: f64) {
        self.consumed = (self.consumed - amount).max(0.0);
    }

    /// Get remaining budget.
    pub fn remaining(&self) -> f64 {
        (self.total_budget - self.consumed - self.reserved).max(0.0)
    }

    /// Get utilization ratio.
    pub fn utilization(&self) -> f64 {
        if self.total_budget == 0.0 {
            0.0
        } else {
            self.consumed / self.total_budget
        }
    }
}

/// Attention manager controls focus selection, context switching, interrupt handling, and attention budget.
#[derive(Clone)]
pub struct AttentionManager {
    inner: Arc<AttentionManagerInner>,
}

struct AttentionManagerInner {
    current_focus: RwLock<Option<Focus>>,
    focus_history: RwLock<Vec<Focus>>,
    interrupt_queue: RwLock<VecDeque<Interrupt>>,
    processed_interrupts: RwLock<Vec<Interrupt>>,
    budget: RwLock<AttentionBudget>,
    context_switches: RwLock<Vec<ContextSwitchEvent>>,
    max_history: RwLock<usize>,
    focus_counts: RwLock<HashMap<String, u64>>,
}

impl AttentionManager {
    /// Create a new attention manager.
    pub fn new(budget_total: f64) -> Self {
        Self {
            inner: Arc::new(AttentionManagerInner {
                current_focus: RwLock::new(None),
                focus_history: RwLock::new(Vec::new()),
                interrupt_queue: RwLock::new(VecDeque::new()),
                processed_interrupts: RwLock::new(Vec::new()),
                budget: RwLock::new(AttentionBudget::new(budget_total)),
                context_switches: RwLock::new(Vec::new()),
                max_history: RwLock::new(1000),
                focus_counts: RwLock::new(HashMap::new()),
            }),
        }
    }

    /// Get the current focus.
    pub fn current_focus(&self) -> Option<Focus> {
        self.inner.current_focus.read().clone()
    }

    /// Set the current focus.
    pub fn set_focus(&self, focus: Focus) -> bool {
        {
            let mut budget = self.inner.budget.write();
            if !budget.can_allocate(focus.attention_cost) {
                return false;
            }
            budget.reserve(focus.attention_cost);
        }

        let old_focus = self.inner.current_focus.write().replace(focus.clone());
        {
            let mut budget = self.inner.budget.write();
            budget.commit(focus.attention_cost);
        }

        if let Some(old) = &old_focus {
            let old_desc = old.description.clone();
            let elapsed = Utc::now()
                .signed_duration_since(old.started_at)
                .num_milliseconds() as u64;

            self.inner.context_switches.write().push(ContextSwitchEvent {
                timestamp: Utc::now(),
                from: Some(old_desc),
                to: focus.description.clone(),
                reason: "focus change".to_string(),
                duration_ms: elapsed,
            });

            {
                let mut budget = self.inner.budget.write();
                budget.release(old.attention_cost);
            }
        }

        *self.inner.current_focus.write() = Some(focus.clone());

        let mut counts = self.inner.focus_counts.write();
        *counts.entry(focus.description).or_insert(0) += 1;

        true
    }

    /// Focus on a goal.
    pub fn focus_on_goal(&self, goal_id: GoalId, description: String, cost: f64) -> bool {
        self.set_focus(Focus {
            goal_id: Some(goal_id),
            task_id: None,
            description,
            started_at: Utc::now(),
            attention_cost: cost,
        })
    }

    /// Focus on a task.
    pub fn focus_on_task(&self, task_id: TaskId, description: String, cost: f64) -> bool {
        self.set_focus(Focus {
            goal_id: None,
            task_id: Some(task_id),
            description,
            started_at: Utc::now(),
            attention_cost: cost,
        })
    }

    /// Clear the current focus.
    pub fn clear_focus(&self) {
        let old = self.inner.current_focus.write().take();
        if let Some(old) = old {
            let cost = old.attention_cost;
            self.inner.focus_history.write().push(old);
            let mut budget = self.inner.budget.write();
            budget.release(cost);

            let max = *self.inner.max_history.read();
            let mut history = self.inner.focus_history.write();
            if history.len() > max {
                let drain_count = history.len() - max;
                history.drain(..drain_count);
            }
        }
    }

    /// Queue an interrupt.
    pub fn queue_interrupt(&self, interrupt: Interrupt) {
        self.inner.interrupt_queue.write().push_back(interrupt);
    }

    /// Process the next interrupt if possible.
    pub fn process_next_interrupt(&self) -> Option<Interrupt> {
        let interrupt = self.inner.interrupt_queue.write().pop_front()?;
        self.inner.processed_interrupts.write().push(interrupt.clone());
        Some(interrupt)
    }

    /// Peek at the next interrupt.
    pub fn peek_next_interrupt(&self) -> Option<Interrupt> {
        self.inner.interrupt_queue.read().front().cloned()
    }

    /// Check if there are pending interrupts.
    pub fn has_pending_interrupts(&self) -> bool {
        !self.inner.interrupt_queue.read().is_empty()
    }

    /// Get pending interrupt count.
    pub fn pending_interrupt_count(&self) -> usize {
        self.inner.interrupt_queue.read().len()
    }

    /// Get the attention budget.
    pub fn budget(&self) -> AttentionBudget {
        self.inner.budget.read().clone()
    }

    /// Get the remaining attention budget.
    pub fn remaining_budget(&self) -> f64 {
        self.inner.budget.read().remaining()
    }

    /// Get the context switch history.
    pub fn context_switch_history(&self) -> Vec<ContextSwitchEvent> {
        self.inner.context_switches.read().clone()
    }

    /// Get the focus history.
    pub fn focus_history(&self) -> Vec<Focus> {
        self.inner.focus_history.read().clone()
    }

    /// Get focus statistics.
    pub fn focus_stats(&self) -> HashMap<String, u64> {
        self.inner.focus_counts.read().clone()
    }

    /// Get the number of context switches.
    pub fn context_switch_count(&self) -> usize {
        self.inner.context_switches.read().len()
    }

    /// Set the maximum history size.
    pub fn set_max_history(&self, max: usize) {
        *self.inner.max_history.write() = max;
    }

    /// Get processed interrupts.
    pub fn processed_interrupts(&self) -> Vec<Interrupt> {
        self.inner.processed_interrupts.read().clone()
    }
}

impl Default for AttentionManager {
    fn default() -> Self {
        Self::new(100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attention_budget() {
        let mut budget = AttentionBudget::new(10.0);
        assert!(budget.can_allocate(5.0));
        assert!(!budget.can_allocate(15.0));

        budget.reserve(3.0);
        assert!((budget.remaining() - 7.0).abs() < f64::EPSILON);

        budget.commit(3.0);
        assert!((budget.consumed - 3.0).abs() < f64::EPSILON);

        budget.release(2.0);
        assert!((budget.consumed - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn focus_management() {
        let mgr = AttentionManager::new(10.0);
        let gid = GoalId::new();

        assert!(mgr.focus_on_goal(gid, "test focus".to_string(), 3.0));
        assert!(mgr.current_focus().is_some());

        mgr.clear_focus();
        assert!(mgr.current_focus().is_none());
        assert_eq!(mgr.focus_history().len(), 1);
    }

    #[test]
    fn interrupt_handling() {
        let mgr = AttentionManager::new(10.0);
        let interrupt = Interrupt::new(
            InterruptType::Critical,
            "test".to_string(),
            "urgent".to_string(),
        );

        mgr.queue_interrupt(interrupt);
        assert!(mgr.has_pending_interrupts());
        assert_eq!(mgr.pending_interrupt_count(), 1);

        let processed = mgr.process_next_interrupt().unwrap();
        assert_eq!(processed.interrupt_type, InterruptType::Critical);
        assert!(!mgr.has_pending_interrupts());
    }

    #[test]
    fn focus_budget_enforcement() {
        let mgr = AttentionManager::new(5.0);
        let gid = GoalId::new();

        assert!(mgr.focus_on_goal(gid, "expensive".to_string(), 4.0));
        let gid2 = GoalId::new();
        assert!(!mgr.focus_on_goal(gid2, "over budget".to_string(), 4.0));
    }

    #[test]
    fn context_switch_tracking() {
        let mgr = AttentionManager::new(20.0);
        let g1 = GoalId::new();
        let g2 = GoalId::new();

        mgr.focus_on_goal(g1, "first".to_string(), 2.0);
        mgr.focus_on_goal(g2, "second".to_string(), 2.0);

        assert_eq!(mgr.context_switch_count(), 1);
    }

    #[test]
    fn focus_stats() {
        let mgr = AttentionManager::new(20.0);
        let g1 = GoalId::new();
        let g2 = GoalId::new();

        mgr.focus_on_goal(g1, "work".to_string(), 1.0);
        mgr.clear_focus();
        mgr.focus_on_goal(g2, "work".to_string(), 1.0);

        let stats = mgr.focus_stats();
        assert_eq!(stats.get("work"), Some(&2));
    }
}
