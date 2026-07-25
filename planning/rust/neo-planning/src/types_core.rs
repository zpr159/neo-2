#![forbid(unsafe_code)]
#![deny(
    missing_docs,
    warnings,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_extern_crates
)]

//! Foundation types for the Neo Planning System.
//!
//! This module provides the core data types and traits that form the foundation
//! of the planning system, including goal management, plan structures, and
//! execution tracking.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use derive_more::Display;
use async_trait::async_trait;
use neo_core::error::Result;

/// Re-export common ID types
pub use crate::id::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DependencyType {
    FinishToStart,
    StartToStart,
    FinishToFinish,
    StartToFinish,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlgorithmType {
    HierarchicalTaskNetwork,
    GoalOrientedActionPlanning,
    AStar,
    BreadthFirstSearch,
    DepthFirstSearch,
    DependencyGraphPlanning,
    ConstraintSatisfactionPlanning,
    ResourceConstrainedPlanning,
    CostBasedOptimization,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlanningNodeType {
    Start,
    End,
    Task,
    Decision,
    Parallel,
    Milestone,
    Composite,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Ready,
    Running,
    Completed,
    Failed,
    Cancelled,
    Skipped,
    Retrying,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub agents: u32,
    pub cpu_units: u32,
    pub memory_mb: u32,
    pub network_bandwidth: u32,
    pub storage_gb: u32,
    pub custom: HashMap<String, f64>,
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            agents: 0,
            cpu_units: 0,
            memory_mb: 0,
            network_bandwidth: 0,
            storage_gb: 0,
            custom: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionBudget {
    pub max_cost: f64,
    pub max_time_seconds: u64,
    pub max_resources: ResourceRequirements,
    pub required_capabilities: Vec<String>,
}

impl Default for ExecutionBudget {
    fn default() -> Self {
        Self {
            max_cost: 0.0,
            max_time_seconds: 0,
            max_resources: ResourceRequirements::default(),
            required_capabilities: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanTask {
    pub id: PlanningNodeId,
    pub name: String,
    pub description: String,
    pub task_type: PlanningNodeType,
    pub status: TaskStatus,
    pub dependencies: Vec<PlanningNodeId>,
    pub cost_estimate: f64,
    pub duration_estimate_secs: u64,
    pub resource_requirements: ResourceRequirements,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningNode {
    pub id: PlanningNodeId,
    pub label: String,
    pub node_type: PlanningNodeType,
    pub goal_id: Option<PlanningGoalId>,
    pub cost: f64,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningEdge {
    pub from: PlanningNodeId,
    pub to: PlanningNodeId,
    pub weight: f64,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecution {
    pub id: PlanningNodeId,
    pub plan_id: PlanId,
    pub task_definition: PlanTask,
    pub status: TaskStatus,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_ms: u64,
    pub retries: u32,
    pub error_message: Option<String>,
    pub output: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    pub id: ResourceAllocationId,
    pub plan_id: PlanId,
    pub allocated_resources: ResourceRequirements,
    pub allocated_agents: Vec<String>,
    pub allocated_capabilities: Vec<String>,
    pub allocation_strategy: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    pub id: CostEstimateId,
    pub plan_id: PlanId,
    pub total_cost: f64,
    pub cpu_cost: f64,
    pub memory_cost: f64,
    pub network_cost: f64,
    pub tool_invocation_cost: f64,
    pub currency: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub id: RiskAssessmentId,
    pub plan_id: PlanId,
    pub dependency_risk: f32,
    pub execution_risk: f32,
    pub tool_failure_risk: f32,
    pub resource_exhaustion_risk: f32,
    pub scheduling_conflict_risk: f32,
    pub uncertainty_score: f32,
    pub overall_risk_score: f32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningSession {
    pub id: PlanningSessionId,
    pub plan_id: Option<PlanId>,
    pub state: PlanState,
    pub configuration: PlanningConfiguration,
    pub started_at: DateTime<Utc>,
    pub timeout_secs: u64,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningConfiguration {
    pub max_planning_time_ms: u64,
    pub allow_parallelism: bool,
    pub optimization_level: u8,
    pub default_algorithm: String,
    pub budget: ExecutionBudget,
    pub constraints: Vec<GoalConstraint>,
    pub resource_requirements: ResourceRequirements,
    pub environment: HashMap<String, serde_json::Value>,
}

impl Default for PlanningConfiguration {
    fn default() -> Self {
        Self {
            max_planning_time_ms: 30_000,
            allow_parallelism: true,
            optimization_level: 2,
            default_algorithm: "HTN".to_string(),
            budget: ExecutionBudget::default(),
            constraints: Vec::new(),
            resource_requirements: ResourceRequirements::default(),
            environment: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRule {
    pub id: OptimizationRuleId,
    pub name: String,
    pub description: Option<String>,
    pub priority: u8,
    pub rule_type: OptimizationRuleType,
    pub enabled: bool,
    pub parameters: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationRuleType {
    Parallelism,
    ResourceOptimization,
    CostOptimization,
    Reliability,
    Scalability,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationPass {
    pub id: OptimizationPassId,
    pub rule: OptimizationRule,
    pub applied: bool,
    pub impact_score: f32,
    pub created_at: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationPipeline {
    pub passes: Vec<OptimizationPass>,
    pub order: OptimizationOrder,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationOrder {
    pub sequential: bool,
    pub priority_ordering: bool,
    pub dependency_tracking: bool,
}

impl Default for OptimizationOrder {
    fn default() -> Self {
        Self {
            sequential: false,
            priority_ordering: true,
            dependency_tracking: true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PlanningAnalytics {
    pub planning_latency_ms: u64,
    pub optimization_gains: f32,
    pub success_rate: f32,
    pub replanning_frequency: f32,
    pub execution_efficiency: f32,
    pub resource_utilization: f32,
    pub generated_at: DateTime<Utc>,
}

impl PlanningAnalytics {
    pub fn new() -> Self {
        Self {
            planning_latency_ms: 0,
            optimization_gains: 0.0,
            success_rate: 0.0,
            replanning_frequency: 0.0,
            execution_efficiency: 0.0,
            resource_utilization: 0.0,
            generated_at: Utc::now(),
        }
    }

    pub fn update_planning_latency(&mut self, latency: u64) {
        self.planning_latency_ms = latency;
        self.generated_at = Utc::now();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningEvent {
    pub event_type: PlanningEventType,
    pub plan_id: Option<PlanId>,
    pub goal_id: Option<PlanningGoalId>,
    pub strategy_id: Option<StrategyId>,
    pub timestamp: DateTime<Utc>,
    pub payload: serde_json::Value,
    pub source: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlanningEventType {
    PlanningStarted,
    GoalCreated,
    GoalDecomposed,
    PlanGenerated,
    StrategySelected,
    PlanOptimized,
    PlanValidated,
    ExecutionStarted,
    ExecutionCompleted,
    ExecutionFailed,
    PlanCancelled,
    PlanArchived,
    ReplanningTriggered,
}

impl std::fmt::Display for PlanningEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PlanningStarted => write!(f, "PlanningStarted"),
            Self::GoalCreated => write!(f, "GoalCreated"),
            Self::GoalDecomposed => write!(f, "GoalDecomposed"),
            Self::PlanGenerated => write!(f, "PlanGenerated"),
            Self::StrategySelected => write!(f, "StrategySelected"),
            Self::PlanOptimized => write!(f, "PlanOptimized"),
            Self::PlanValidated => write!(f, "PlanValidated"),
            Self::ExecutionStarted => write!(f, "ExecutionStarted"),
            Self::ExecutionCompleted => write!(f, "ExecutionCompleted"),
            Self::ExecutionFailed => write!(f, "ExecutionFailed"),
            Self::PlanCancelled => write!(f, "PlanCancelled"),
            Self::PlanArchived => write!(f, "PlanArchived"),
            Self::ReplanningTriggered => write!(f, "ReplanningTriggered"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskValidator {
    pub strict_mode: bool,
}

impl TaskValidator {
    pub fn new() -> Self {
        Self { strict_mode: true }
    }

    pub fn validate_tasks(&self, tasks: Vec<PlanTask>) -> Result<bool> {
        for task in &tasks {
            for dep_id in &task.dependencies {
                if !tasks.iter().any(|t| t.id == *dep_id) {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanOptimizer {
    pub rules: Vec<OptimizationRule>,
}

impl PlanOptimizer {
    pub fn new() -> Self {
        Self {
            rules: vec![
                OptimizationRule {
                    id: OptimizationRuleId::new(),
                    name: "Parallelism".to_string(),
                    description: Some("Enables parallel task execution".to_string()),
                    priority: 1,
                    rule_type: OptimizationRuleType::Parallelism,
                    enabled: true,
                    parameters: HashMap::new(),
                },
                OptimizationRule {
                    id: OptimizationRuleId::new(),
                    name: "Cost Optimization".to_string(),
                    description: Some("Reduces execution cost".to_string()),
                    priority: 2,
                    rule_type: OptimizationRuleType::CostOptimization,
                    enabled: true,
                    parameters: HashMap::new(),
                },
                OptimizationRule {
                    id: OptimizationRuleId::new(),
                    name: "Resource Optimization".to_string(),
                    description: Some("Optimizes resource allocation".to_string()),
                    priority: 3,
                    rule_type: OptimizationRuleType::ResourceOptimization,
                    enabled: true,
                    parameters: HashMap::new(),
                },
            ],
        }
    }

    pub async fn optimize_tasks(&self, tasks: Vec<PlanTask>, rules: Vec<OptimizationRule>) -> Result<Vec<PlanTask>> {
        let mut optimized_tasks = tasks;

        for rule in rules {
            if !rule.enabled {
                continue;
            }

            optimized_tasks = self.apply_rule(rule, optimized_tasks).await?;
        }

        Ok(optimized_tasks)
    }

    async fn apply_rule(&self, rule: OptimizationRule, mut tasks: Vec<PlanTask>) -> Result<Vec<PlanTask>> {
        match rule.rule_type {
            OptimizationRuleType::Parallelism => {
                for task in &mut tasks {
                    if task.dependencies.len() > 1 {
                        task.task_type = PlanningNodeType::Parallel;
                    }
                }
            }
            OptimizationRuleType::CostOptimization => {
                for task in &mut tasks {
                    task.cost_estimate *= 0.9;
                    task.duration_estimate_secs = (task.duration_estimate_secs as f64 * 0.9) as u64;
                }
            }
            _ => {}
        }

        Ok(tasks)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimator {
    pub base_cost_per_unit: f64,
}

impl CostEstimator {
    pub fn new() -> Self {
        Self {
            base_cost_per_unit: 0.1,
        }
    }

    pub async fn estimate(&self, tasks: Vec<PlanTask>) -> Result<CostEstimate> {
        let mut total_cost = 0.0;
        let mut cpu_cost = 0.0;
        let mut memory_cost = 0.0;
        let mut network_cost = 0.0;
        let mut tool_invocation_cost = 0.0;

        for task in tasks {
            total_cost += task.cost_estimate;
            cpu_cost += task.resource_requirements.cpu_units as f64 * self.base_cost_per_unit;
            memory_cost += task.resource_requirements.memory_mb as f64 * self.base_cost_per_unit * 0.5;
            network_cost += task.resource_requirements.network_bandwidth as f64 * self.base_cost_per_unit * 0.3;
            tool_invocation_cost += 1.0;
        }

        Ok(CostEstimate {
            id: CostEstimateId::new(),
            plan_id: PlanId::new(),
            total_cost,
            cpu_cost,
            memory_cost,
            network_cost,
            tool_invocation_cost,
            currency: "USD".to_string(),
            created_at: Utc::now(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocator {
    pub default_allocations: ResourceRequirements,
}

impl ResourceAllocator {
    pub fn new() -> Self {
        Self {
            default_allocations: ResourceRequirements::default(),
        }
    }

    pub async fn allocate(&self, tasks: Vec<PlanTask>, cost_estimate: CostEstimate) -> Result<ResourceAllocation> {
        let mut total_agents = 0u32;
        let mut total_cpu = 0u32;
        let mut total_memory = 0u32;
        let mut total_network = 0u32;

        for task in tasks {
            total_agents += task.resource_requirements.agents;
            total_cpu += task.resource_requirements.cpu_units;
            total_memory += task.resource_requirements.memory_mb;
            total_network += task.resource_requirements.network_bandwidth;
        }

        let allocated = ResourceAllocation {
            id: ResourceAllocationId::new(),
            plan_id: cost_estimate.plan_id,
            allocated_resources: ResourceRequirements {
                agents: total_agents,
                cpu_units: total_cpu,
                memory_mb: total_memory,
                network_bandwidth: total_network,
                storage_gb: 0,
                custom: HashMap::new(),
            },
            allocated_agents: vec![],
            allocated_capabilities: vec![],
            allocation_strategy: "default".to_string(),
            created_at: Utc::now(),
        };

        Ok(allocated)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAnalyzer {
    pub base_risk_per_unit: f32,
}

impl RiskAnalyzer {
    pub fn new() -> Self {
        Self {
            base_risk_per_unit: 0.1,
        }
    }

    pub async fn analyze(&self, tasks: Vec<PlanTask>) -> Result<RiskAssessment> {
        let mut dependency_risk = 0.0f32;
        let mut execution_risk = 0.0f32;
        let mut tool_failure_risk = 0.0f32;
        let mut resource_exhaustion_risk = 0.0f32;
        let mut scheduling_conflict_risk = 0.0f32;

        let task_count = tasks.len() as f32;

        dependency_risk = (0.1 + rand::random::<f32>() * 0.3) * task_count.min(5.0) / 5.0;
        execution_risk = (0.1 + rand::random::<f32>() * 0.4) * task_count.min(5.0) / 5.0;
        tool_failure_risk = (0.05 + rand::random::<f32>() * 0.2) * task_count.min(3.0) / 3.0;
        resource_exhaustion_risk = (0.05 + rand::random::<f32>() * 0.2);
        scheduling_conflict_risk = (0.0 + rand::random::<f32>() * 0.1);

        let mut uncertainty_score = (dependency_risk + execution_risk + tool_failure_risk + resource_exhaustion_risk + scheduling_conflict_risk) / 5.0;
        let overall_risk_score = (dependency_risk * 0.3 + execution_risk * 0.4 + tool_failure_risk * 0.2 + resource_exhaustion_risk * 0.05 + scheduling_conflict_risk * 0.05)
            + uncertainty_score * 0.2;

        Ok(RiskAssessment {
            id: RiskAssessmentId::new(),
            plan_id: PlanId::new(),
            dependency_risk,
            execution_risk,
            tool_failure_risk,
            resource_exhaustion_risk,
            scheduling_conflict_risk,
            uncertainty_score,
            overall_risk_score,
            created_at: Utc::now(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskMitigation {
    pub id: RiskMitigationId,
    pub risk_id: RiskAssessmentId,
    pub strategy: RiskMitigationStrategy,
    pub effectiveness_score: f32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskMitigationStrategy {
    Redundancy,
    Fallback,
    Monitoring,
    AllocationAdjustment,
    ManualIntervention,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanResult {
    pub plan_id: PlanId,
    pub success: bool,
    pub plan: Option<Plan>,
    pub execution: Option<PlanExecution>,
    pub error: Option<String>,
    pub metrics: Option<PlanMetrics>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl PlanResult {
    pub fn new(plan_id: PlanId, metrics: PlanMetrics) -> Self {
        Self {
            plan_id,
            success: true,
            plan: None,
            execution: None,
            error: None,
            metrics: Some(metrics),
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
        }
    }

    pub fn success(&self) -> bool {
        self.success
    }
}

#[async_trait]
pub trait PlanningAlgorithm: Send + Sync {
    fn name(&self) -> &str;
    fn algorithm_type(&self) -> AlgorithmType;
    async fn plan(&self, context: &PlanningContext, config: &AlgorithmConfig) -> Result<AlgorithmResult>;
    fn validate_config(&self, config: &AlgorithmConfig) -> Result<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningContext {
    pub goal: Goal,
    pub budget: ExecutionBudget,
    pub available_resources: ResourceRequirements,
    pub constraints: Vec<GoalConstraint>,
    pub environment: HashMap<String, serde_json::Value>,
}

impl PlanningContext {
    pub fn new(goal: Goal) -> Self {
        Self {
            goal,
            budget: ExecutionBudget::default(),
            available_resources: ResourceRequirements::default(),
            constraints: Vec::new(),
            environment: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmResult {
    pub success: bool,
    pub tasks: Vec<PlanTask>,
    pub graph: Option<TaskGraph>,
    pub cost_estimate: f64,
    pub duration_estimate_secs: u64,
    pub iterations_used: usize,
    pub nodes_explored: usize,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningError {
    pub code: PlanningErrorCode,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlanningErrorCode {
    PlanNotFound,
    GoalNotFound,
    PlanGraphCycleDetected,
    PlanInvalidState,
    PlanValidationFailed,
    AlgorithmNotSupported,
    ResourceExhausted,
    ConfigurationError,
    InternalError,
}

impl PlanningError {
    pub fn new(code: PlanningErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}