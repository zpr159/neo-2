use crate::error::ConversationResult;
use crate::types::CognitiveContext;

/// Interface to the planning subsystem.
///
/// The planner provides task decomposition, dependency analysis,
/// and step-by-step plan generation for complex requests.
pub trait PlanningInterface: Send + Sync {
    /// Generate a plan for the given user query and context.
    fn plan(&self, query: &str, context: &CognitiveContext) -> ConversationResult<PlanResult>;

    /// Update a plan with new information.
    fn update_plan(
        &self,
        plan_id: &str,
        update: PlanUpdate,
    ) -> ConversationResult<PlanResult>;

    /// Get the current status of a plan.
    fn plan_status(&self, plan_id: &str) -> ConversationResult<PlanStatus>;
}

/// A generated plan.
#[derive(Debug, Clone)]
pub struct PlanResult {
    pub plan_id: String,
    pub steps: Vec<PlanStep>,
    pub summary: String,
    pub estimated_tokens: usize,
}

/// A single step in a plan.
#[derive(Debug, Clone)]
pub struct PlanStep {
    pub index: usize,
    pub description: String,
    pub dependencies: Vec<usize>,
    pub status: StepStatus,
}

/// Status of a plan step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
}

/// Update to apply to a plan.
#[derive(Debug, Clone)]
pub struct PlanUpdate {
    pub step_index: usize,
    pub status: StepStatus,
    pub result: Option<String>,
}

/// Overall status of a plan.
#[derive(Debug, Clone)]
pub struct PlanStatus {
    pub plan_id: String,
    pub total_steps: usize,
    pub completed_steps: usize,
    pub failed_steps: usize,
    pub is_complete: bool,
}

/// Default planner that returns simple step decomposition.
pub struct DefaultPlanner;

impl PlanningInterface for DefaultPlanner {
    fn plan(&self, query: &str, _context: &CognitiveContext) -> ConversationResult<PlanResult> {
        let plan_id = uuid::Uuid::new_v4().to_string();
        let steps = vec![
            PlanStep {
                index: 0,
                description: format!("Analyze the request: {query}"),
                dependencies: Vec::new(),
                status: StepStatus::Pending,
            },
            PlanStep {
                index: 1,
                description: "Gather necessary information".into(),
                dependencies: vec![0],
                status: StepStatus::Pending,
            },
            PlanStep {
                index: 2,
                description: "Execute and respond".into(),
                dependencies: vec![1],
                status: StepStatus::Pending,
            },
        ];

        Ok(PlanResult {
            plan_id,
            steps,
            summary: format!("Plan for: {query}"),
            estimated_tokens: query.len() / 4 + 100,
        })
    }

    fn update_plan(
        &self,
        _plan_id: &str,
        _update: PlanUpdate,
    ) -> ConversationResult<PlanResult> {
        Ok(PlanResult {
            plan_id: "updated".into(),
            steps: Vec::new(),
            summary: "Plan updated".into(),
            estimated_tokens: 0,
        })
    }

    fn plan_status(&self, _plan_id: &str) -> ConversationResult<PlanStatus> {
        Ok(PlanStatus {
            plan_id: "unknown".into(),
            total_steps: 0,
            completed_steps: 0,
            failed_steps: 0,
            is_complete: true,
        })
    }
}
