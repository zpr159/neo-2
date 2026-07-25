use criterion::{criterion_group, criterion_main, Criterion};
use neo_reasoning::strategy::{DeductiveStrategy, InductiveStrategy, AbductiveStrategy, ReasoningStrategyExecutor};

fn bench_strategy_execution(c: &mut Criterion) {
    let registry = neo_reasoning::StrategyRegistry::new();
    let ctx = neo_reasoning::StrategyContext::new("test query".to_string());

    c.bench_function("strategy_registry_select_best", |b| {
        b.iter(|| {
            registry.select_best("if X then Y");
        });
    });

    c.bench_function("deductive_strategy", |b| {
        let strategy = DeductiveStrategy;
        b.iter(|| {
            strategy.execute(&ctx).unwrap();
        });
    });

    c.bench_function("inductive_strategy", |b| {
        let strategy = InductiveStrategy;
        b.iter(|| {
            strategy.execute(&ctx).unwrap();
        });
    });

    c.bench_function("abductive_strategy", |b| {
        let strategy = AbductiveStrategy;
        b.iter(|| {
            strategy.execute(&ctx).unwrap();
        });
    });
}

fn bench_planning(c: &mut Criterion) {
    let engine = neo_reasoning::PlanningEngine::new();

    c.bench_function("plan_creation", |b| {
        let goal = neo_reasoning::Goal::new("decompose this problem".to_string());
        b.iter(|| {
            engine.create_plan(goal.clone(), neo_reasoning::ReasoningStrategy::ChainOfThought).unwrap();
        });
    });

    c.bench_function("plan_validation", |b| {
        let goal = neo_reasoning::Goal::new("validate this".to_string());
        let plan = engine.create_plan(goal, neo_reasoning::ReasoningStrategy::ChainOfThought).unwrap();
        b.iter(|| {
            engine.validate_plan(&plan).unwrap();
        });
    });
}

fn bench_decision(c: &mut Criterion) {
    let engine = neo_reasoning::DecisionEngine::new();

    c.bench_function("decision_selection", |b| {
        let options = vec![
            neo_reasoning::DecisionOption::new("option A".to_string()).with_utility(0.7).with_risk(0.3),
            neo_reasoning::DecisionOption::new("option B".to_string()).with_utility(0.5).with_risk(0.2),
            neo_reasoning::DecisionOption::new("option C".to_string()).with_utility(0.8).with_risk(0.6),
        ];
        b.iter(|| {
            engine.select_best(&options, None).unwrap();
        });
    });
}

fn bench_reflection(c: &mut Criterion) {
    let engine = neo_reasoning::ReflectionEngine::new();

    c.bench_function("reflection", |b| {
        let mut state = neo_reasoning::InternalReasoningState::new();
        let chain_id = state.start_chain(neo_reasoning::ReasoningStrategy::ChainOfThought);
        let step = neo_reasoning::InternalStep::new(
            neo_reasoning::StepType::Premise,
            "test premise".to_string(),
            0.8,
        );
        if let Some(chain) = state.active_chain_mut() {
            chain.add_step(step);
            let conclusion = neo_reasoning::InternalStep::new(
                neo_reasoning::StepType::Conclusion,
                "test conclusion".to_string(),
                0.7,
            );
            chain.add_step(conclusion);
        }
        state.finalize_active_chain();

        let context = std::collections::HashMap::new();
        b.iter(|| {
            engine.reflect(&state, &context).unwrap();
        });
    });
}

fn bench_hypothesis(c: &mut Criterion) {
    let engine = neo_reasoning::HypothesisEngine::new();

    c.bench_function("hypothesis_generation", |b| {
        let context = std::collections::HashMap::new();
        b.iter(|| {
            engine.generate_hypotheses("test query", &context, 5);
        });
    });
}

fn bench_cache(c: &mut Criterion) {
    let cache = neo_reasoning::ReasoningCache::new(1000, 3600);

    c.bench_function("cache_store_and_get", |b| {
        let result = neo_reasoning::CachedReasoningResult {
            conclusion: "test".to_string(),
            confidence: 0.8,
            explanation: "test explanation".to_string(),
            strategy_used: "deductive".to_string(),
            step_count: 5,
            metadata: std::collections::HashMap::new(),
        };
        cache.store("test query", result);
        b.iter(|| {
            cache.get("test query");
        });
    });
}

criterion_group!(
    benches,
    bench_strategy_execution,
    bench_planning,
    bench_decision,
    bench_reflection,
    bench_hypothesis,
    bench_cache,
);
criterion_main!(benches);
