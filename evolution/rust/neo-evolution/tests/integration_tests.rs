use neo_evolution::prelude::*;

#[tokio::test]
async fn engine_builder_builds() {
    let engine = EvolutionEngine::builder()
        .enable_benchmarks(true)
        .enable_sandbox(true)
        .enable_policy_evolution(true)
        .build();
    assert!(engine.is_ok());
}

#[tokio::test]
async fn engine_start_and_stop() {
    let engine = EvolutionEngine::new(EvolutionConfiguration::default()).unwrap();
    engine.start().await.unwrap();
    let status = engine.get_status();
    assert_eq!(status.state.status, EvolutionStatus::Running);
    engine.stop().await.unwrap();
    let status = engine.get_status();
    assert_eq!(status.state.status, EvolutionStatus::Cancelled);
}

#[tokio::test]
async fn run_analysis_returns_results() {
    let engine = EvolutionEngine::new(EvolutionConfiguration::default()).unwrap();
    let results = engine.run_analysis(SubsystemTarget::Runtime).unwrap();
    assert!(!results.is_empty());
    assert!(results[0].score >= 0.0);
    assert!(results[0].score <= 1.0);
}

#[tokio::test]
async fn run_full_analysis_covers_all_subsystems() {
    let engine = EvolutionEngine::new(EvolutionConfiguration::default()).unwrap();
    let results = engine.run_full_analysis().unwrap();
    assert!(results.len() >= 10);
}

#[tokio::test]
async fn experiment_lifecycle() {
    let engine = EvolutionEngine::new(EvolutionConfiguration::default()).unwrap();
    let config = ExperimentConfig::new(
        "test-experiment",
        neo_evolution::experiment::experiment::ExperimentType::IsolatedExecution,
        SubsystemTarget::Runtime,
    );
    let id = engine.start_experiment(config).unwrap();
    let exp = engine.experiment_manager.get_experiment(id).unwrap();
    assert!(exp.is_running());
}

#[tokio::test]
async fn improvement_proposals_from_analysis() {
    let engine = EvolutionEngine::new(EvolutionConfiguration::default()).unwrap();
    let results = engine.run_analysis(SubsystemTarget::Runtime).unwrap();
    let ids = engine
        .propose_improvements_from_analysis(&results[0])
        .unwrap();
    assert!(!ids.is_empty());
}

#[tokio::test]
async fn rollback_works() {
    let engine = EvolutionEngine::new(EvolutionConfiguration::default()).unwrap();
    let id = uuid::Uuid::new_v4();
    engine.rollback(id, "test rollback").unwrap();
}

#[tokio::test]
async fn metrics_are_tracked() {
    let engine = EvolutionEngine::new(EvolutionConfiguration::default()).unwrap();
    let _ = engine.run_analysis(SubsystemTarget::Runtime);
    let metrics = engine.get_metrics().unwrap();
    assert!(metrics.is_object());
}

#[tokio::test]
async fn benchmark_runs() {
    let engine = EvolutionEngine::new(EvolutionConfiguration::default()).unwrap();
    let summary = engine.run_benchmark().unwrap();
    assert_eq!(summary.total_iterations, 0);
}

#[tokio::test]
async fn policy_evolution_works() {
    let config = EvolutionConfiguration::default();
    let engine = PolicyEvolutionEngine::new(config);
    let _ = engine.get_best_policy(neo_evolution::policy_evolution::policy::PolicyType::Planning);
}

#[tokio::test]
async fn heuristic_evolution_works() {
    let config = EvolutionConfiguration::default();
    let engine = HeuristicEvolution::new(config);
    let stats = engine.get_stats();
    assert_eq!(stats.total_heuristics, 0);
}

#[tokio::test]
async fn sandbox_config_default_is_valid() {
    let config = SandboxConfig::default();
    let sandbox = Sandbox::new(config, neo_evolution::sandbox::sandbox::SandboxLevel::Full);
    assert!(sandbox.validate_config().is_ok());
    assert!(sandbox.is_isolated());
}

#[tokio::test]
async fn governance_authorization() {
    use neo_evolution::governance::authorization::*;
    let auth = EvolutionAuthorization::new(
        AuthorizationLevel::Full,
        vec![],
        RiskLevel::High,
        None,
        None,
    );
    assert!(auth.authorize(SubsystemTarget::Runtime, RiskLevel::Medium));
}

#[tokio::test]
async fn approval_manager_workflow() {
    use neo_evolution::governance::approval::*;
    let mgr = ApprovalManager::new();
    let proposal_id = uuid::Uuid::new_v4();
    let approval = mgr.request_approval(proposal_id, "admin".to_string(), None);
    assert_eq!(approval.status, ApprovalStatus::Pending);

    let result = mgr.approve(&approval.id, "LGTM");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().status, ApprovalStatus::Approved);
}

#[tokio::test]
async fn audit_records_entries() {
    use neo_evolution::governance::audit::*;
    use std::collections::HashMap;
    let audit = EvolutionAudit::new();
    let entry = AuditEntry {
        id: uuid::Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        action: "test_action".into(),
        actor: "test_actor".into(),
        target: SubsystemTarget::Runtime,
        details: HashMap::new(),
        result: "success".into(),
    };
    audit.record(entry);
    assert_eq!(audit.get_count(), 1);
}

#[tokio::test]
async fn workflow_evolution_analysis() {
    let config = EvolutionConfiguration::default();
    let engine = WorkflowEvolution::new(config);
    let steps: Vec<String> = vec![
        "step_a".into(),
        "step_b".into(),
        "step_a".into(),
        "sync_wait".into(),
    ];
    let result = engine.analyze_workflow("wf1", &steps);
    assert!(!result.redundant_steps.is_empty());
    assert!(!result.unnecessary_synchronization.is_empty());
    assert!(result.efficiency_score < 1.0);
}

#[tokio::test]
async fn capability_evolution_versioning() {
    let config = EvolutionConfiguration::default();
    let engine = CapabilityEvolution::new(config);
    let v1 = engine.create_version("cap1", "1.0.0", vec!["initial".into()]);
    assert_eq!(v1.version, "1.0.0");
    let _v2 = engine.create_version("cap1", "1.1.0", vec!["improved".into()]);
    let versions = engine.get_versions("cap1");
    assert_eq!(versions.len(), 2);
}

#[tokio::test]
async fn agent_evolution_tracking() {
    let config = EvolutionConfiguration::default();
    let engine = AgentEvolution::new(config);
    let evo = engine.evolve_role("agent1", "researcher", "analyst", "better fit");
    assert_eq!(evo.old_role, "researcher");
    assert_eq!(evo.new_role, "analyst");
    let history = engine.get_role_history("agent1");
    assert_eq!(history.len(), 1);
}

#[tokio::test]
async fn distributed_evolution_cluster() {
    let config = EvolutionConfiguration::default();
    let engine = DistributedEvolution::new(config);
    let exp_id = uuid::Uuid::new_v4();
    let exp = engine.create_cluster_experiment(exp_id, vec!["node1".into(), "node2".into()]);
    assert!(exp.synchronized);
    assert_eq!(exp.participating_nodes.len(), 2);
    let active = engine.get_active_experiments();
    assert_eq!(active.len(), 1);
}

#[tokio::test]
async fn regression_detector_works() {
    use neo_evolution::benchmark::regression::RegressionDetector;
    let detector = RegressionDetector::new(10.0);
    detector.set_baseline("latency", vec![100.0, 105.0, 95.0, 102.0]);
    let results =
        detector.detect_regressions(&[("latency".to_string(), vec![101.0, 103.0, 98.0, 100.0])]);
    assert!(!results.is_empty());
    assert!(!results[0].is_regression);
}

#[tokio::test]
async fn performance_optimizer_measurement() {
    use neo_evolution::performance::optimizer::PerformanceOptimizer;
    let optimizer = PerformanceOptimizer::new();
    let metrics = optimizer.measure();
    assert!(metrics.cpu_usage >= 0.0);
    assert!(metrics.memory_usage_mb > 0.0);
}

#[tokio::test]
async fn strategy_selector() {
    use neo_evolution::strategies::StrategySelector;
    let selector = StrategySelector::new();
    let strategy =
        selector.select_strategy(SubsystemTarget::Runtime, ImprovementCategory::Performance);
    let _ = strategy;
}

#[test]
fn context_records_metrics() {
    let ctx = EvolutionContext::new(EvolutionConfiguration::default());
    ctx.record_analysis(SubsystemTarget::Runtime);
    ctx.record_experiment(SubsystemTarget::Runtime);
    let metrics = ctx.get_metrics();
    assert_eq!(metrics.len(), 1);
}

#[test]
fn evolution_config_defaults() {
    let config = EvolutionConfiguration::default();
    assert_eq!(config.max_concurrent_cycles, 4);
    assert!(config.sandbox_mode);
    assert_eq!(config.analysis_history_limit, 1000);
}

#[test]
fn risk_level_ordering() {
    assert!(RiskLevel::None < RiskLevel::Low);
    assert!(RiskLevel::Low < RiskLevel::Medium);
    assert!(RiskLevel::Medium < RiskLevel::High);
    assert!(RiskLevel::High < RiskLevel::Critical);
}

#[test]
fn heuristic_evolution_cycle() {
    let config = EvolutionConfiguration::default();
    let engine = HeuristicEvolution::new(config);
    let candidates = engine.generate_candidates(SubsystemTarget::Runtime);
    assert!(candidates.is_empty());

    use neo_evolution::heuristic_evolution::heuristic::Heuristic;
    let mut h = Heuristic::new("test", "test heuristic", SubsystemTarget::Runtime);
    h.update_score(0.8);
    engine.repository().save(h);

    let candidates = engine.generate_candidates(SubsystemTarget::Runtime);
    assert_eq!(candidates.len(), 1);
}

#[test]
fn benchmark_scenario_builder() {
    use neo_evolution::benchmark::scenario::ScenarioBuilder;
    let scenario = ScenarioBuilder::new("test")
        .with_description("a test scenario")
        .with_target(SubsystemTarget::Runtime)
        .with_parameter("iterations", 100.0)
        .with_iterations(10)
        .build();
    assert_eq!(scenario.name, "test");
    assert_eq!(scenario.iterations, 10);
}
