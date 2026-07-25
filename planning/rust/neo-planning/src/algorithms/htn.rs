use crate::engine::Planner;
use crate::types::*;
use crate::goal::*;
use async_trait::async_trait;
use neo_core::error::Result;

pub struct HtnPlanner;

impl HtnPlanner {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Planner for HtnPlanner {
    fn name(&self) -> &str {
        "HTN"
    }

    async fn plan(&self, _context: &PlanContext, goal: &Goal) -> Result<Plan> {
        // HTN Decomposition logic would go here
        // For now, return a basic plan based on the goal
        Plan::builder().goal(goal.id).build()
    }
}
