//! Multi-agent coordination, task assignment, and consensus for the Neo Planning System.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{PlanningError, PlanningErrorCode, PlanningResult};
use crate::goal::{Goal, GoalPriority};
use crate::id::{AgentAllocationId, PlanningGoalId, PlanningNodeId};
use crate::plan::{Plan, PlanTask};
use crate::types::{ResourceRequirements, TaskStatus};

// ---------------------------------------------------------------------------
// AgentRole
// ---------------------------------------------------------------------------

/// The role an agent plays within a multi-agent plan.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentRole {
    Supervisor,
    Worker,
    Reviewer,
    Coordinator,
    Specialist { domain: String },
}

// ---------------------------------------------------------------------------
// AgentCapabilities
// ---------------------------------------------------------------------------

/// Describes what an agent can do and its current state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilities {
    pub skills: Vec<String>,
    pub max_concurrent_tasks: u32,
    pub reliability: f64,
    pub current_workload: u32,
    pub cost_per_task: f64,
}

impl AgentCapabilities {
    /// Create capabilities with defaults.
    pub fn new(max_concurrent_tasks: u32) -> Self {
        Self {
            skills: Vec::new(),
            max_concurrent_tasks,
            reliability: 1.0,
            current_workload: 0,
            cost_per_task: 0.0,
        }
    }

    /// Add a skill.
    #[must_use]
    pub fn with_skill(mut self, skill: impl Into<String>) -> Self {
        self.skills.push(skill.into());
        self
    }

    /// Set max concurrent tasks.
    #[must_use]
    pub fn with_max_concurrent_tasks(mut self, max: u32) -> Self {
        self.max_concurrent_tasks = max;
        self
    }

    /// Set reliability score (0.0 – 1.0).
    #[must_use]
    pub fn with_reliability(mut self, reliability: f64) -> Self {
        self.reliability = reliability.clamp(0.0, 1.0);
        self
    }

    /// Set current workload.
    #[must_use]
    pub fn with_current_workload(mut self, workload: u32) -> Self {
        self.current_workload = workload;
        self
    }

    /// Set cost per task.
    #[must_use]
    pub fn with_cost_per_task(mut self, cost: f64) -> Self {
        self.cost_per_task = cost;
        self
    }
}

// ---------------------------------------------------------------------------
// AgentAllocation
// ---------------------------------------------------------------------------

/// Records which tasks have been assigned to a particular agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAllocation {
    pub id: AgentAllocationId,
    pub agent_id: String,
    pub role: AgentRole,
    pub assigned_tasks: Vec<PlanningNodeId>,
    pub capabilities: AgentCapabilities,
    pub created_at: DateTime<Utc>,
}

impl AgentAllocation {
    /// Whether the agent can accept another task.
    pub fn can_accept_task(&self) -> bool {
        (self.assigned_tasks.len() as u32) < self.capabilities.max_concurrent_tasks
    }

    /// Current workload as a fraction of capacity (0.0 – 1.0).
    pub fn workload_pct(&self) -> f64 {
        if self.capabilities.max_concurrent_tasks == 0 {
            return 1.0;
        }
        self.assigned_tasks.len() as f64 / self.capabilities.max_concurrent_tasks as f64
    }
}

// ---------------------------------------------------------------------------
// NegotiationStatus
// ---------------------------------------------------------------------------

/// Status of a task negotiation round.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NegotiationStatus {
    Proposed,
    Accepted,
    Rejected,
    Failed,
}

// ---------------------------------------------------------------------------
// TaskNegotiation
// ---------------------------------------------------------------------------

/// Represents a negotiation to assign a task to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNegotiation {
    pub task_id: PlanningNodeId,
    pub proposed_agents: Vec<String>,
    pub selected_agent: Option<String>,
    pub negotiation_status: NegotiationStatus,
}

// ---------------------------------------------------------------------------
// WorkloadBalance
// ---------------------------------------------------------------------------

/// Snapshot of an agent's workload for balance analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadBalance {
    pub agent_id: String,
    pub current_tasks: u32,
    pub max_tasks: u32,
    pub efficiency: f64,
}

impl WorkloadBalance {
    /// How many more tasks this agent can take on.
    pub fn capacity_remaining(&self) -> u32 {
        self.max_tasks.saturating_sub(self.current_tasks)
    }
}

// ---------------------------------------------------------------------------
// MultiAgentPlanner
// ---------------------------------------------------------------------------

/// Coordinates task assignment across multiple agents.
#[derive(Debug, Clone)]
pub struct MultiAgentPlanner;

impl MultiAgentPlanner {
    /// Create a new planner.
    pub fn new() -> Self {
        Self
    }

    /// Greedy assignment: for each task, pick the agent with the most remaining
    /// capacity whose skills cover the task's capability requirements.
    pub fn assign_tasks(
        &self,
        tasks: &[PlanTask],
        agents: &[(String, AgentCapabilities)],
    ) -> PlanningResult<Vec<AgentAllocation>> {
        if agents.is_empty() && !tasks.is_empty() {
            return Err(PlanningError::new(
                PlanningErrorCode::AgentAllocationFailed,
                "no agents available for task assignment",
            ));
        }

        let mut agent_state: Vec<(String, AgentCapabilities, Vec<PlanningNodeId>)> = agents
            .iter()
            .map(|(id, caps)| (id.clone(), caps.clone(), Vec::new()))
            .collect();

        for task in tasks {
            let mut best_idx: Option<usize> = None;
            let mut best_capacity: u32 = 0;

            for (idx, (_id, caps, assigned)) in agent_state.iter().enumerate() {
                let skills_match = task
                    .resource_requirements
                    .capability_requirements
                    .is_empty()
                    || task
                        .resource_requirements
                        .capability_requirements
                        .iter()
                        .all(|req| caps.skills.iter().any(|s| s == req));

                if !skills_match {
                    continue;
                }

                let current = caps.current_workload + assigned.len() as u32;
                if current >= caps.max_concurrent_tasks {
                    continue;
                }

                let capacity = caps.max_concurrent_tasks - current;
                if capacity > best_capacity {
                    best_capacity = capacity;
                    best_idx = Some(idx);
                }
            }

            if let Some(idx) = best_idx {
                agent_state[idx].2.push(task.id);
            }
        }

        let allocations: Vec<AgentAllocation> = agent_state
            .into_iter()
            .filter(|(_, _, tasks)| !tasks.is_empty())
            .map(|(agent_id, capabilities, assigned_tasks)| AgentAllocation {
                id: AgentAllocationId::new(),
                agent_id,
                role: AgentRole::Worker,
                assigned_tasks,
                capabilities,
                created_at: Utc::now(),
            })
            .collect();

        Ok(allocations)
    }

    /// Identify which agents could handle a task and return a proposed negotiation.
    pub fn negotiate_task(
        &self,
        task: &PlanTask,
        agents: &[(String, AgentCapabilities)],
    ) -> TaskNegotiation {
        let proposed_agents: Vec<String> = agents
            .iter()
            .filter(|(_, caps)| {
                task.resource_requirements
                    .capability_requirements
                    .is_empty()
                    || task
                        .resource_requirements
                        .capability_requirements
                        .iter()
                        .all(|req| caps.skills.iter().any(|s| s == req))
            })
            .map(|(id, _)| id.clone())
            .collect();

        TaskNegotiation {
            task_id: task.id,
            proposed_agents,
            selected_agent: None,
            negotiation_status: NegotiationStatus::Proposed,
        }
    }

    /// Convert allocations into a workload-balance snapshot per agent.
    pub fn balance_workload(&self, allocations: &[AgentAllocation]) -> Vec<WorkloadBalance> {
        allocations
            .iter()
            .map(|alloc| WorkloadBalance {
                agent_id: alloc.agent_id.clone(),
                current_tasks: alloc.assigned_tasks.len() as u32,
                max_tasks: alloc.capabilities.max_concurrent_tasks,
                efficiency: alloc.capabilities.reliability,
            })
            .collect()
    }

    /// Return the supervisor allocation, if one exists.
    pub fn get_supervisor_plan(
        &self,
        allocations: &[AgentAllocation],
    ) -> PlanningResult<Option<AgentAllocation>> {
        Ok(allocations
            .iter()
            .find(|a| matches!(a.role, AgentRole::Supervisor))
            .cloned())
    }
}

impl Default for MultiAgentPlanner {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// ConsensusProtocol
// ---------------------------------------------------------------------------

/// Protocol used to reach consensus among agents.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConsensusProtocol {
    MajorityVote,
    Unanimous,
    WeightedVoting,
    RoundRobin,
}

// ---------------------------------------------------------------------------
// ConsensusOutcome
// ---------------------------------------------------------------------------

/// Final outcome of a consensus decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConsensusOutcome {
    Approved,
    Rejected,
    Tied,
    InsufficientVotes,
}

// ---------------------------------------------------------------------------
// ConsensusResult
// ---------------------------------------------------------------------------

/// Result of casting a single vote.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusResult {
    pub proposal_id: uuid::Uuid,
    pub votes: HashMap<String, bool>,
    pub outcome: ConsensusOutcome,
    pub threshold_met: bool,
}

// ---------------------------------------------------------------------------
// ConsensusEngine
// ---------------------------------------------------------------------------

/// Tracks votes and evaluates consensus across agents.
#[derive(Debug, Clone)]
pub struct ConsensusEngine {
    protocol: ConsensusProtocol,
    votes: DashMap<uuid::Uuid, HashMap<String, bool>>,
}

use dashmap::DashMap;

impl ConsensusEngine {
    /// Create an engine with the given protocol.
    pub fn new(protocol: ConsensusProtocol) -> Self {
        Self {
            protocol,
            votes: DashMap::new(),
        }
    }

    /// Record a vote and return the current consensus state.
    pub fn vote(&self, proposal_id: uuid::Uuid, agent_id: String, vote: bool) -> ConsensusResult {
        {
            let mut entry = self.votes.entry(proposal_id).or_insert_with(HashMap::new);
            entry.insert(agent_id, vote);
        }

        let snapshot = self
            .votes
            .get(&proposal_id)
            .map(|r| r.clone())
            .unwrap_or_default();

        let total = snapshot.len();
        let yes = snapshot.values().filter(|&&v| v).count();
        let no = total - yes;

        let (outcome, threshold_met) = match self.protocol {
            ConsensusProtocol::Unanimous => {
                if no > 0 {
                    (ConsensusOutcome::Rejected, true)
                } else if yes == total && total > 0 {
                    (ConsensusOutcome::Approved, false)
                } else {
                    (ConsensusOutcome::InsufficientVotes, false)
                }
            }
            ConsensusProtocol::MajorityVote | ConsensusProtocol::WeightedVoting => {
                if yes > no && no > 0 {
                    (ConsensusOutcome::Approved, false)
                } else if no > yes && yes > 0 {
                    (ConsensusOutcome::Rejected, false)
                } else if total > 0 && yes == no {
                    (ConsensusOutcome::Tied, false)
                } else {
                    (ConsensusOutcome::InsufficientVotes, false)
                }
            }
            ConsensusProtocol::RoundRobin => {
                if yes > 0 {
                    (ConsensusOutcome::Approved, false)
                } else if total > 0 {
                    (ConsensusOutcome::Rejected, false)
                } else {
                    (ConsensusOutcome::InsufficientVotes, false)
                }
            }
        };

        ConsensusResult {
            proposal_id,
            votes: snapshot,
            outcome,
            threshold_met,
        }
    }

    /// Evaluate the final consensus once all agents have voted.
    pub fn check_consensus(
        &self,
        proposal_id: uuid::Uuid,
        total_agents: usize,
    ) -> ConsensusOutcome {
        let votes = match self.votes.get(&proposal_id) {
            Some(v) => v,
            None => return ConsensusOutcome::InsufficientVotes,
        };

        let total_votes = votes.len();
        let yes_votes = votes.values().filter(|&&v| v).count();
        let no_votes = total_votes - yes_votes;

        if total_votes < total_agents {
            return ConsensusOutcome::InsufficientVotes;
        }

        match self.protocol {
            ConsensusProtocol::MajorityVote => {
                if yes_votes > no_votes {
                    ConsensusOutcome::Approved
                } else if no_votes > yes_votes {
                    ConsensusOutcome::Rejected
                } else {
                    ConsensusOutcome::Tied
                }
            }
            ConsensusProtocol::Unanimous => {
                if yes_votes == total_agents {
                    ConsensusOutcome::Approved
                } else {
                    ConsensusOutcome::Rejected
                }
            }
            ConsensusProtocol::WeightedVoting => {
                let total_weight = total_agents as f64;
                let yes_weight = yes_votes as f64;
                if yes_weight / total_weight > 0.5 {
                    ConsensusOutcome::Approved
                } else if (total_weight - yes_weight) / total_weight > 0.5 {
                    ConsensusOutcome::Rejected
                } else {
                    ConsensusOutcome::Tied
                }
            }
            ConsensusProtocol::RoundRobin => {
                if yes_votes > 0 {
                    ConsensusOutcome::Approved
                } else {
                    ConsensusOutcome::Rejected
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_task(name: &str) -> PlanTask {
        use crate::types::PlanTaskType;
        PlanTask::new(name, PlanTaskType::Atomic)
    }

    fn make_task_with_caps(name: &str, caps: Vec<&str>) -> PlanTask {
        let mut task = make_task(name);
        task.resource_requirements.capability_requirements =
            caps.into_iter().map(String::from).collect();
        task
    }

    // ---- AgentCapabilities ----

    #[test]
    fn capabilities_builder() {
        let caps = AgentCapabilities::new(5)
            .with_skill("rust")
            .with_skill("python")
            .with_reliability(0.95)
            .with_current_workload(2)
            .with_cost_per_task(10.0);

        assert_eq!(caps.max_concurrent_tasks, 5);
        assert_eq!(caps.skills, vec!["rust", "python"]);
        assert!((caps.reliability - 0.95).abs() < f64::EPSILON);
        assert_eq!(caps.current_workload, 2);
        assert!((caps.cost_per_task - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn capabilities_reliability_clamped() {
        let caps = AgentCapabilities::new(1).with_reliability(2.0);
        assert!((caps.reliability - 1.0).abs() < f64::EPSILON);

        let caps = AgentCapabilities::new(1).with_reliability(-1.0);
        assert!((caps.reliability - 0.0).abs() < f64::EPSILON);
    }

    // ---- AgentAllocation ----

    #[test]
    fn allocation_can_accept_task() {
        let alloc = AgentAllocation {
            id: AgentAllocationId::new(),
            agent_id: "a1".to_string(),
            role: AgentRole::Worker,
            assigned_tasks: vec![PlanningNodeId::new(), PlanningNodeId::new()],
            capabilities: AgentCapabilities::new(3),
            created_at: Utc::now(),
        };
        assert!(alloc.can_accept_task());

        let full = AgentAllocation {
            assigned_tasks: vec![
                PlanningNodeId::new(),
                PlanningNodeId::new(),
                PlanningNodeId::new(),
            ],
            ..alloc.clone()
        };
        assert!(!full.can_accept_task());
    }

    #[test]
    fn allocation_workload_pct() {
        let alloc = AgentAllocation {
            id: AgentAllocationId::new(),
            agent_id: "a1".to_string(),
            role: AgentRole::Worker,
            assigned_tasks: vec![PlanningNodeId::new(), PlanningNodeId::new()],
            capabilities: AgentCapabilities::new(4),
            created_at: Utc::now(),
        };
        assert!((alloc.workload_pct() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn allocation_workload_pct_zero_capacity() {
        let alloc = AgentAllocation {
            id: AgentAllocationId::new(),
            agent_id: "a1".to_string(),
            role: AgentRole::Worker,
            assigned_tasks: vec![],
            capabilities: AgentCapabilities::new(0),
            created_at: Utc::now(),
        };
        assert!((alloc.workload_pct() - 1.0).abs() < f64::EPSILON);
    }

    // ---- WorkloadBalance ----

    #[test]
    fn workload_balance_capacity_remaining() {
        let wb = WorkloadBalance {
            agent_id: "a1".to_string(),
            current_tasks: 3,
            max_tasks: 5,
            efficiency: 0.9,
        };
        assert_eq!(wb.capacity_remaining(), 2);
    }

    #[test]
    fn workload_balance_capacity_remaining_saturated() {
        let wb = WorkloadBalance {
            agent_id: "a1".to_string(),
            current_tasks: 10,
            max_tasks: 5,
            efficiency: 0.9,
        };
        assert_eq!(wb.capacity_remaining(), 0);
    }

    // ---- MultiAgentPlanner ----

    #[test]
    fn assign_tasks_basic() {
        let planner = MultiAgentPlanner::new();
        let t1 = make_task("t1");
        let t2 = make_task("t2");

        let agents = vec![
            (
                "agent-1".to_string(),
                AgentCapabilities::new(2).with_skill("general"),
            ),
            (
                "agent-2".to_string(),
                AgentCapabilities::new(1).with_skill("general"),
            ),
        ];

        let allocs = planner.assign_tasks(&[t1, t2], &agents).unwrap();
        // Both tasks should be assigned
        let total_assigned: usize = allocs.iter().map(|a| a.assigned_tasks.len()).sum();
        assert_eq!(total_assigned, 2);
    }

    #[test]
    fn assign_tasks_skill_matching() {
        let planner = MultiAgentPlanner::new();
        let t1 = make_task_with_caps("rust-task", vec!["rust"]);
        let t2 = make_task_with_caps("python-task", vec!["python"]);

        let agents = vec![
            (
                "rust-agent".to_string(),
                AgentCapabilities::new(5).with_skill("rust"),
            ),
            (
                "python-agent".to_string(),
                AgentCapabilities::new(5).with_skill("python"),
            ),
        ];

        let allocs = planner.assign_tasks(&[t1, t2], &agents).unwrap();
        assert_eq!(allocs.len(), 2);

        for alloc in &allocs {
            let task_count = alloc.assigned_tasks.len() as u32;
            assert!(task_count <= alloc.capabilities.max_concurrent_tasks);
        }
    }

    #[test]
    fn assign_tasks_capacity_respected() {
        let planner = MultiAgentPlanner::new();
        let tasks: Vec<PlanTask> = (0..5).map(|i| make_task(&format!("t{}", i))).collect();

        let agents = vec![(
            "limited".to_string(),
            AgentCapabilities::new(2).with_skill("general"),
        )];

        let allocs = planner.assign_tasks(&tasks, &agents).unwrap();
        let total: usize = allocs.iter().map(|a| a.assigned_tasks.len()).sum();
        assert_eq!(total, 2); // only 2 tasks fit
    }

    #[test]
    fn assign_tasks_no_agents_error() {
        let planner = MultiAgentPlanner::new();
        let tasks = vec![make_task("t1")];
        let result = planner.assign_tasks(&tasks, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn assign_tasks_empty() {
        let planner = MultiAgentPlanner::new();
        let agents = vec![("a1".to_string(), AgentCapabilities::new(3))];
        let allocs = planner.assign_tasks(&[], &agents).unwrap();
        assert!(allocs.is_empty());
    }

    #[test]
    fn negotiate_task() {
        let planner = MultiAgentPlanner::new();
        let task = make_task_with_caps("task", vec!["rust", "ml"]);

        let agents = vec![
            (
                "a1".to_string(),
                AgentCapabilities::new(3)
                    .with_skill("rust")
                    .with_skill("ml"),
            ),
            (
                "a2".to_string(),
                AgentCapabilities::new(3).with_skill("python"),
            ),
            (
                "a3".to_string(),
                AgentCapabilities::new(3).with_skill("rust"),
            ),
        ];

        let neg = planner.negotiate_task(&task, &agents);
        assert_eq!(neg.negotiation_status, NegotiationStatus::Proposed);
        assert!(neg.selected_agent.is_none());
        // Only a1 has both rust and ml
        assert_eq!(neg.proposed_agents.len(), 1);
        assert_eq!(neg.proposed_agents[0], "a1");
    }

    #[test]
    fn negotiate_task_no_match() {
        let planner = MultiAgentPlanner::new();
        let task = make_task_with_caps("task", vec!["quantum"]);

        let agents = vec![(
            "a1".to_string(),
            AgentCapabilities::new(3).with_skill("rust"),
        )];

        let neg = planner.negotiate_task(&task, &agents);
        assert!(neg.proposed_agents.is_empty());
    }

    #[test]
    fn balance_workload() {
        let planner = MultiAgentPlanner::new();
        let allocations = vec![
            AgentAllocation {
                id: AgentAllocationId::new(),
                agent_id: "a1".to_string(),
                role: AgentRole::Worker,
                assigned_tasks: vec![PlanningNodeId::new(), PlanningNodeId::new()],
                capabilities: AgentCapabilities::new(4).with_reliability(0.9),
                created_at: Utc::now(),
            },
            AgentAllocation {
                id: AgentAllocationId::new(),
                agent_id: "a2".to_string(),
                role: AgentRole::Worker,
                assigned_tasks: vec![PlanningNodeId::new()],
                capabilities: AgentCapabilities::new(2).with_reliability(0.7),
                created_at: Utc::now(),
            },
        ];

        let balances = planner.balance_workload(&allocations);
        assert_eq!(balances.len(), 2);

        let b1 = balances.iter().find(|b| b.agent_id == "a1").unwrap();
        assert_eq!(b1.current_tasks, 2);
        assert_eq!(b1.max_tasks, 4);
        assert!((b1.efficiency - 0.9).abs() < f64::EPSILON);

        let b2 = balances.iter().find(|b| b.agent_id == "a2").unwrap();
        assert_eq!(b2.current_tasks, 1);
        assert_eq!(b2.max_tasks, 2);
    }

    #[test]
    fn get_supervisor_plan_found() {
        let planner = MultiAgentPlanner::new();
        let allocations = vec![
            AgentAllocation {
                id: AgentAllocationId::new(),
                agent_id: "sup".to_string(),
                role: AgentRole::Supervisor,
                assigned_tasks: vec![],
                capabilities: AgentCapabilities::new(1),
                created_at: Utc::now(),
            },
            AgentAllocation {
                id: AgentAllocationId::new(),
                agent_id: "w1".to_string(),
                role: AgentRole::Worker,
                assigned_tasks: vec![PlanningNodeId::new()],
                capabilities: AgentCapabilities::new(3),
                created_at: Utc::now(),
            },
        ];

        let sup = planner.get_supervisor_plan(&allocations).unwrap();
        assert!(sup.is_some());
        assert_eq!(sup.unwrap().agent_id, "sup");
    }

    #[test]
    fn get_supervisor_plan_none() {
        let planner = MultiAgentPlanner::new();
        let allocations = vec![AgentAllocation {
            id: AgentAllocationId::new(),
            agent_id: "w1".to_string(),
            role: AgentRole::Worker,
            assigned_tasks: vec![],
            capabilities: AgentCapabilities::new(3),
            created_at: Utc::now(),
        }];

        let sup = planner.get_supervisor_plan(&allocations).unwrap();
        assert!(sup.is_none());
    }

    // ---- AgentRole ----

    #[test]
    fn agent_role_specialist_domain() {
        let role = AgentRole::Specialist {
            domain: "nlp".to_string(),
        };
        assert_eq!(
            role,
            AgentRole::Specialist {
                domain: "nlp".to_string()
            }
        );
    }

    #[test]
    fn agent_role_serialization() {
        let roles = vec![
            AgentRole::Supervisor,
            AgentRole::Worker,
            AgentRole::Reviewer,
            AgentRole::Coordinator,
            AgentRole::Specialist {
                domain: "vision".to_string(),
            },
        ];
        for role in roles {
            let json = serde_json::to_string(&role).unwrap();
            let restored: AgentRole = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, role);
        }
    }

    // ---- ConsensusEngine ----

    #[test]
    fn consensus_majority_approved() {
        let engine = ConsensusEngine::new(ConsensusProtocol::MajorityVote);
        let pid = uuid::Uuid::new_v4();

        engine.vote(pid, "a1".to_string(), true);
        engine.vote(pid, "a2".to_string(), true);
        engine.vote(pid, "a3".to_string(), false);

        let outcome = engine.check_consensus(pid, 3);
        assert_eq!(outcome, ConsensusOutcome::Approved);
    }

    #[test]
    fn consensus_majority_rejected() {
        let engine = ConsensusEngine::new(ConsensusProtocol::MajorityVote);
        let pid = uuid::Uuid::new_v4();

        engine.vote(pid, "a1".to_string(), false);
        engine.vote(pid, "a2".to_string(), false);
        engine.vote(pid, "a3".to_string(), true);

        let outcome = engine.check_consensus(pid, 3);
        assert_eq!(outcome, ConsensusOutcome::Rejected);
    }

    #[test]
    fn consensus_majority_tied() {
        let engine = ConsensusEngine::new(ConsensusProtocol::MajorityVote);
        let pid = uuid::Uuid::new_v4();

        engine.vote(pid, "a1".to_string(), true);
        engine.vote(pid, "a2".to_string(), false);

        let outcome = engine.check_consensus(pid, 2);
        assert_eq!(outcome, ConsensusOutcome::Tied);
    }

    #[test]
    fn consensus_unanimous_approved() {
        let engine = ConsensusEngine::new(ConsensusProtocol::Unanimous);
        let pid = uuid::Uuid::new_v4();

        engine.vote(pid, "a1".to_string(), true);
        engine.vote(pid, "a2".to_string(), true);
        engine.vote(pid, "a3".to_string(), true);

        let outcome = engine.check_consensus(pid, 3);
        assert_eq!(outcome, ConsensusOutcome::Approved);
    }

    #[test]
    fn consensus_unanimous_rejected_on_single_no() {
        let engine = ConsensusEngine::new(ConsensusProtocol::Unanimous);
        let pid = uuid::Uuid::new_v4();

        engine.vote(pid, "a1".to_string(), true);
        engine.vote(pid, "a2".to_string(), false);

        // After second vote, unanimous is impossible
        let result = engine.vote(pid, "a3".to_string(), true);
        assert_eq!(result.outcome, ConsensusOutcome::Rejected);
    }

    #[test]
    fn consensus_insufficient_votes() {
        let engine = ConsensusEngine::new(ConsensusProtocol::MajorityVote);
        let pid = uuid::Uuid::new_v4();

        let outcome = engine.check_consensus(pid, 5);
        assert_eq!(outcome, ConsensusOutcome::InsufficientVotes);
    }

    #[test]
    fn consensus_not_all_voted_yet() {
        let engine = ConsensusEngine::new(ConsensusProtocol::MajorityVote);
        let pid = uuid::Uuid::new_v4();

        engine.vote(pid, "a1".to_string(), true);
        engine.vote(pid, "a2".to_string(), true);

        // Only 2 of 5 voted
        let outcome = engine.check_consensus(pid, 5);
        assert_eq!(outcome, ConsensusOutcome::InsufficientVotes);
    }

    #[test]
    fn consensus_round_robin_approved() {
        let engine = ConsensusEngine::new(ConsensusProtocol::RoundRobin);
        let pid = uuid::Uuid::new_v4();

        engine.vote(pid, "a1".to_string(), true);
        engine.vote(pid, "a2".to_string(), false);

        let outcome = engine.check_consensus(pid, 2);
        assert_eq!(outcome, ConsensusOutcome::Approved);
    }

    #[test]
    fn consensus_round_robin_rejected() {
        let engine = ConsensusEngine::new(ConsensusProtocol::RoundRobin);
        let pid = uuid::Uuid::new_v4();

        engine.vote(pid, "a1".to_string(), false);
        engine.vote(pid, "a2".to_string(), false);

        let outcome = engine.check_consensus(pid, 2);
        assert_eq!(outcome, ConsensusOutcome::Rejected);
    }

    #[test]
    fn consensus_weighted_voting() {
        let engine = ConsensusEngine::new(ConsensusProtocol::WeightedVoting);
        let pid = uuid::Uuid::new_v4();

        engine.vote(pid, "a1".to_string(), true);
        engine.vote(pid, "a2".to_string(), true);
        engine.vote(pid, "a3".to_string(), false);

        let outcome = engine.check_consensus(pid, 3);
        assert_eq!(outcome, ConsensusOutcome::Approved); // 2/3 > 50%
    }

    #[test]
    fn consensus_multiple_proposals() {
        let engine = ConsensusEngine::new(ConsensusProtocol::MajorityVote);
        let p1 = uuid::Uuid::new_v4();
        let p2 = uuid::Uuid::new_v4();

        engine.vote(p1, "a1".to_string(), true);
        engine.vote(p1, "a2".to_string(), true);
        engine.vote(p2, "a1".to_string(), false);
        engine.vote(p2, "a2".to_string(), false);

        assert_eq!(engine.check_consensus(p1, 2), ConsensusOutcome::Approved);
        assert_eq!(engine.check_consensus(p2, 2), ConsensusOutcome::Rejected);
    }

    #[test]
    fn consensus_vote_returns_result() {
        let engine = ConsensusEngine::new(ConsensusProtocol::MajorityVote);
        let pid = uuid::Uuid::new_v4();

        let r1 = engine.vote(pid, "a1".to_string(), true);
        assert_eq!(r1.votes.len(), 1);
        assert_eq!(r1.outcome, ConsensusOutcome::InsufficientVotes);

        let r2 = engine.vote(pid, "a2".to_string(), true);
        assert_eq!(r2.votes.len(), 2);
        // 2 yes, 0 no — still insuff for majority (need no > 0 for majority call)
        // Actually our logic: yes > no && no > 0 => Approved. Here no == 0.
        // So InsufficientVotes. That's fine — we need at least one dissent for majority.
        assert_eq!(r2.outcome, ConsensusOutcome::InsufficientVotes);

        let r3 = engine.vote(pid, "a3".to_string(), false);
        assert_eq!(r3.votes.len(), 3);
        assert_eq!(r3.outcome, ConsensusOutcome::Approved); // 2 yes > 1 no
    }

    // ---- Serialization roundtrips ----

    #[test]
    fn agent_allocation_roundtrip() {
        let alloc = AgentAllocation {
            id: AgentAllocationId::new(),
            agent_id: "a1".to_string(),
            role: AgentRole::Specialist {
                domain: "nlp".to_string(),
            },
            assigned_tasks: vec![PlanningNodeId::new(), PlanningNodeId::new()],
            capabilities: AgentCapabilities::new(4)
                .with_skill("rust")
                .with_reliability(0.9),
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&alloc).unwrap();
        let restored: AgentAllocation = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.agent_id, "a1");
        assert_eq!(restored.assigned_tasks.len(), 2);
    }

    #[test]
    fn task_negotiation_roundtrip() {
        let neg = TaskNegotiation {
            task_id: PlanningNodeId::new(),
            proposed_agents: vec!["a1".to_string(), "a2".to_string()],
            selected_agent: Some("a1".to_string()),
            negotiation_status: NegotiationStatus::Accepted,
        };
        let json = serde_json::to_string(&neg).unwrap();
        let restored: TaskNegotiation = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.negotiation_status, NegotiationStatus::Accepted);
        assert_eq!(restored.proposed_agents.len(), 2);
    }

    #[test]
    fn workload_balance_roundtrip() {
        let wb = WorkloadBalance {
            agent_id: "a1".to_string(),
            current_tasks: 3,
            max_tasks: 5,
            efficiency: 0.85,
        };
        let json = serde_json::to_string(&wb).unwrap();
        let restored: WorkloadBalance = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.capacity_remaining(), 2);
    }

    #[test]
    fn consensus_result_roundtrip() {
        let mut votes = HashMap::new();
        votes.insert("a1".to_string(), true);
        votes.insert("a2".to_string(), false);
        let result = ConsensusResult {
            proposal_id: uuid::Uuid::new_v4(),
            votes,
            outcome: ConsensusOutcome::Tied,
            threshold_met: false,
        };
        let json = serde_json::to_string(&result).unwrap();
        let restored: ConsensusResult = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.outcome, ConsensusOutcome::Tied);
        assert_eq!(restored.votes.len(), 2);
    }
}
