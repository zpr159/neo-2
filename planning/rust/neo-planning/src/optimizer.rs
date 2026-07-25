use crate::types::*;

pub struct PlanOptimizer;

impl PlanOptimizer {
    pub fn optimize(&self, plan: Plan, rules: Vec<OptimizationRule>) -> Plan {
        let mut optimized_plan = plan;
        for rule in rules {
            optimized_plan = rule.apply(optimized_plan);
        }
        optimized_plan
    }
}

pub trait OptimizationRule: Send + Sync {
    fn apply(&self, plan: Plan) -> Plan;
}

pub struct ParallelismRule;

impl OptimizationRule for ParallelismRule {
    fn apply(&self, mut plan: Plan) -> Plan {
        // Logic to increase parallelism
        plan
    }
}
