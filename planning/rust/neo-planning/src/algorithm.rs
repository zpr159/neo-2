use std::collections::{HashMap, HashSet};
use std::time::Duration;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use tracing::info;

use crate::types::*;
use crate::goal::*;
use crate::algorithm::*;
use crate::id::*;
use crate::error::*;
use crate::strategy::*;

#[derive(Clone)]
pub struct PlanningOrchestrator {
    algorithm_registry: AlgorithmRegistry,
    strategy_generator: StrategyGenerator,
    strategy_evaluator: StrategyEvaluator,
    strategy_selector: StrategySelector,
    event_bus: EventBus,
    analytics: PlanningAnalytics,
}

impl PlanningOrchestrator {
    pub fn new(
        algorithm_registry: AlgorithmRegistry,
        strategy_generator: StrategyGenerator,
        strategy_evaluator: StrategyEvaluator,
        strategy_selector: StrategySelector,
        event_bus: EventBus,
    ) -> Self {
        Self {
            algorithm_registry,
            strategy_generator,
            strategy_evaluator,
            strategy_selector,
            event_bus,
            analytics: PlanningAnalytics::new(),
        }
    }

    pub async fn plan_from_goal(&self, goal: Goal) -> Result<Plan> {
        info!("PlanningOrchestrator: Generating plan for goal: {}", goal.metadata.name);

        self.event_bus.publish(PlanningEvent::new(
            PlanningEventType::PlanningStarted,
            "orchestrator"
        ).with_goal_id(Some(goal.id)));

        let context = PlanContext::from(&goal);
        let strategy = self.generate_strategy(&goal, &context).await?;
        info!("Strategy selected: {} (confidence: {:.2})", strategy.name, strategy.confidence_score);

        let algorithm = self.algorithm_registry.get(&strategy.algorithm).await?;
        let algorithm_config = AlgorithmConfig {
            algorithm_type: strategy.algorithm.clone(),
            max_depth: 100,
            max_iterations: 1000,
            timeout_ms: 30_000,
            heuristic_weight: 1.0,
            allow_suboptimal: true,
        };

        let planning_context = PlanningContext::from(&goal);

        let algorithm_result = algorithm.plan(&planning_context, &algorithm_config).await?;

        let mut task_graph = TaskGraph::from_algorithm_result(algorithm_result);
        task_graph = task_graph.with_strategy(strategy.clone());

        self.event_bus.publish(PlanningEvent::new(
            PlanningEventType::PlanGenerated,
            "orchestrator"
        ).with_strategy_id(Some(strategy.id)));

        Plan::from_task_graph(goal, task_graph)
    }

    async fn generate_strategy(&self, goal: &Goal, context: &PlanContext) -> Result<Strategy> {
        let candidates = self.strategy_generator.generate_candidates(goal, context);

        let selected = self.strategy_selector.select_best(
            candidates,
            StrategyPolicy::Balanced
        ).ok_or_else(|| PlanningError::validation("No strategy candidates available"))?;

        let mut evaluated_strategy = selected;
        evaluated_strategy.evaluation = self.strategy_evaluator.evaluate(&mut evaluated_strategy, context);

        self.event_bus.publish(PlanningEvent::new(
            PlanningEventType::StrategySelected,
            "orchestrator"
        ).with_strategy_id(Some(evaluated_strategy.id)));

        Ok(evaluated_strategy)
    }
}

#[async_trait]
pub trait PlanningAlgorithm: Send + Sync {
    fn name(&self) -> &str;
    fn algorithm_type(&self) -> AlgorithmType;
    async fn plan(&self, context: &PlanningContext, config: &AlgorithmConfig) -> Result<AlgorithmResult>;
    fn validate_config(&self, config: &AlgorithmConfig) -> Result<()>;
}

pub struct AlgorithmRegistry {
    algorithms: HashMap<AlgorithmType, Box<dyn PlanningAlgorithm>>,
}

impl AlgorithmRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            algorithms: HashMap::new(),
        };

        registry.register(HierarchicalTaskNetworkPlanner::new());
        registry.register(GoalOrientedActionPlanningPlanner::new());
        registry.register(AStarPlanner::new());
        registry.register(DependencyGraphPlanner::new());
        registry.register(BreadthFirstPlanner::new());
        registry.register(DepthFirstPlanner::new());
        registry.register(CostBasedPlanner::new());

        registry
    }

    pub fn register<A: PlanningAlgorithm + 'static>(&mut self, algorithm: A) {
        self.algorithms.insert(algorithm.algorithm_type(), Box::new(algorithm));
    }

    pub async fn get(&self, algorithm_type: &AlgorithmType) -> Result<&dyn PlanningAlgorithm> {
        self.algorithms.get(algorithm_type)
            .ok_or_else(|| PlanningError::validation(format!("Algorithm type not supported: {:?}", algorithm_type)))
            .map(|boxed| boxed.as_ref())
    }

    pub async fn get_candidates(&self, goal: &Goal) -> Result<Vec<Strategy>> {
        let mut strategies = Vec::new();

        for algorithm in self.algorithms.values() {
            strategies.push(Strategy {
                id: StrategyId::new(),
                plan_id: None,
                name: algorithm.name().to_string(),
                description: Some(format!("Generated by {} algorithm", algorithm.name())),
                algorithm: algorithm.algorithm_type(),
                evaluation: StrategyComparison {
                    cost: goal.budget.max_cost * (1.0 + rand::random::<f64>()),
                    duration_ms: (goal.budget.max_time_seconds * 1000) as u64,
                    probability_of_success: 0.5 + rand::random::<f64>() * 0.5,
                    resource_consumption: HashMap::new(),
                    risk_score: 1.0 - (0.5 + rand::random::<f64>() * 0.5),
                    complexity_score: rand::random::<f64>(),
                    scalability_score: rand::random::<f64>() * 0.8 + 0.2,
                    reliability_score: rand::random::<f64>() * 0.8 + 0.2,
                },
                created_at: Utc::now(),
                confidence_score: 0.7 + rand::random::<f64>() * 0.3,
            });
        }

        Ok(strategies)
    }

    pub async fn select_best(&self, candidates: Vec<Strategy>) -> Result<Strategy> {
        StrategySelector::new().select_best(candidates, StrategyPolicy::Balanced)
            .ok_or_else(|| PlanningError::validation("No suitable strategy found"))
    }
}