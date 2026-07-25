#[cfg(test)]
mod tests {
    use neo_core::observability::health::*;
    use neo_core::observability::metrics::*;
    use neo_core::observability::tracing_setup::*;
    use neo_core::observability::ObservabilityManager;

    // ── MetricsCollector ──────────────────────────────────────────────

    #[test]
    fn test_metrics_collector_new_defaults() {
        let mc = MetricsCollector::new();
        assert!(mc.is_enabled());
    }

    #[test]
    fn test_metrics_collector_default_trait() {
        let mc = MetricsCollector::default();
        assert!(mc.is_enabled());
    }

    #[test]
    fn test_metrics_collector_enable_disable() {
        let mc = MetricsCollector::new();
        mc.disable();
        assert!(!mc.is_enabled());
        mc.enable();
        assert!(mc.is_enabled());
    }

    #[test]
    fn test_metrics_collector_record_conversation() {
        let mc = MetricsCollector::new();
        mc.record_conversation();
        mc.record_conversation();
        mc.record_conversation();
        let metrics = mc.collect("test".into());
        assert_eq!(metrics.conversation.total_conversations, 3);
    }

    #[test]
    fn test_metrics_collector_set_active_sessions() {
        let mc = MetricsCollector::new();
        mc.set_active_sessions(10);
        let metrics = mc.collect("test".into());
        assert_eq!(metrics.conversation.active_sessions, 10);
    }

    #[test]
    fn test_metrics_collector_record_message_latency() {
        let mc = MetricsCollector::new();
        mc.record_message_latency(100.0);
        mc.record_message_latency(200.0);
        let metrics = mc.collect("test".into());
        // avg = (100 + 200) / 2 = 150
        assert!((metrics.conversation.avg_latency_ms - 150.0).abs() < 1.0);
    }

    #[test]
    fn test_metrics_collector_set_messages_per_second() {
        let mc = MetricsCollector::new();
        mc.set_messages_per_second(42.5);
        let metrics = mc.collect("test".into());
        assert!((metrics.conversation.messages_per_second - 42.5).abs() < 0.1);
    }

    #[test]
    fn test_metrics_collector_record_tool_execution() {
        let mc = MetricsCollector::new();
        mc.record_tool_execution();
        mc.record_tool_execution();
        let metrics = mc.collect("test".into());
        assert_eq!(metrics.conversation.tool_executions, 2);
    }

    #[test]
    fn test_metrics_collector_set_cpu_usage() {
        let mc = MetricsCollector::new();
        mc.set_cpu_usage(75.5);
        let metrics = mc.collect("test".into());
        assert!((metrics.system.cpu_usage - 75.5).abs() < 0.1);
    }

    #[test]
    fn test_metrics_collector_set_memory_usage() {
        let mc = MetricsCollector::new();
        mc.set_memory_usage(1024 * 1024);
        let metrics = mc.collect("test".into());
        assert_eq!(metrics.system.memory_usage_bytes, 1024 * 1024);
    }

    #[test]
    fn test_metrics_collector_set_gpu_usage() {
        let mc = MetricsCollector::new();
        mc.set_gpu_usage(60.0);
        let metrics = mc.collect("test".into());
        assert!((metrics.system.gpu_usage - 60.0).abs() < 0.1);
    }

    #[test]
    fn test_metrics_collector_set_disk_usage() {
        let mc = MetricsCollector::new();
        mc.set_disk_usage(500_000_000);
        let metrics = mc.collect("test".into());
        assert_eq!(metrics.system.disk_usage_bytes, 500_000_000);
    }

    #[test]
    fn test_metrics_collector_language_metrics() {
        let mc = MetricsCollector::new();
        mc.record_language_request(true);
        mc.record_language_request(true);
        mc.record_language_request(false);
        mc.record_first_token_latency(50.0);
        mc.record_first_token_latency(100.0);
        mc.set_tokens_per_second(1000.0);
        let metrics = mc.collect("test".into());
        assert_eq!(metrics.language.total_requests, 3);
        assert_eq!(metrics.language.failed_requests, 1);
        assert!((metrics.language.tokens_per_second - 1000.0).abs() < 0.1);
        assert!((metrics.language.avg_first_token_ms - 75.0).abs() < 1.0);
    }

    #[test]
    fn test_metrics_collector_retrieval_metrics() {
        let mc = MetricsCollector::new();
        mc.record_memory_retrieval(10.0);
        mc.record_memory_retrieval(20.0);
        mc.record_knowledge_lookup(5.0);
        mc.record_world_model_query(15.0);
        mc.record_context_assembly(30.0);
        let metrics = mc.collect("test".into());
        assert!((metrics.retrieval.memory_retrieval_ms - 15.0).abs() < 1.0);
        assert!((metrics.retrieval.knowledge_lookup_ms - 5.0).abs() < 1.0);
        assert!((metrics.retrieval.world_model_query_ms - 15.0).abs() < 1.0);
        assert!((metrics.retrieval.context_assembly_ms - 30.0).abs() < 1.0);
    }

    #[test]
    fn test_metrics_collector_reasoning_metrics() {
        let mc = MetricsCollector::new();
        mc.record_reasoning_latency(100.0);
        mc.record_reasoning_latency(200.0);
        mc.record_planning_latency(50.0);
        mc.record_contradiction();
        mc.record_contradiction();
        mc.record_inference_steps(5);
        mc.record_inference_steps(3);
        let metrics = mc.collect("test".into());
        assert!((metrics.reasoning.reasoning_latency_ms - 150.0).abs() < 1.0);
        assert!((metrics.reasoning.planning_latency_ms - 50.0).abs() < 1.0);
        assert_eq!(metrics.reasoning.contradictions_detected, 2);
        assert!((metrics.reasoning.inference_steps - 4.0).abs() < 1.0);
    }

    #[test]
    fn test_metrics_collector_workflow_metrics() {
        let mc = MetricsCollector::new();
        mc.set_active_workflows(3);
        mc.record_workflow_completion(100.0);
        mc.record_workflow_completion(200.0);
        mc.record_workflow_failure();
        let metrics = mc.collect("test".into());
        assert_eq!(metrics.workflow.active_workflows, 3);
        assert_eq!(metrics.workflow.completed_workflows, 2);
        assert_eq!(metrics.workflow.failed_workflows, 1);
        assert!((metrics.workflow.avg_execution_ms - 150.0).abs() < 1.0);
    }

    #[test]
    fn test_metrics_collector_agent_metrics() {
        let mc = MetricsCollector::new();
        mc.set_active_agents(5);
        mc.record_task_completion();
        mc.record_task_completion();
        mc.record_task_completion();
        mc.set_agent_utilization(75.0);
        let metrics = mc.collect("test".into());
        assert_eq!(metrics.agent.active_agents, 5);
        assert_eq!(metrics.agent.total_tasks_completed, 3);
        assert!((metrics.agent.agent_utilization - 75.0).abs() < 0.1);
    }

    #[test]
    fn test_metrics_collector_disabled_no_ops() {
        let mc = MetricsCollector::new();
        mc.disable();
        mc.record_conversation();
        mc.set_active_sessions(99);
        mc.set_cpu_usage(99.0);
        let metrics = mc.collect("test".into());
        assert_eq!(metrics.conversation.total_conversations, 0);
        assert_eq!(metrics.conversation.active_sessions, 0);
    }

    #[test]
    fn test_metrics_collector_collect_node_id() {
        let mc = MetricsCollector::new();
        let metrics = mc.collect("my-node".into());
        assert_eq!(metrics.node_id, "my-node");
    }

    #[test]
    fn test_metrics_collector_collect_has_timestamp() {
        let mc = MetricsCollector::new();
        let metrics = mc.collect("test".into());
        assert!(metrics.timestamp > 0);
    }

    // ── AggregatedMetrics ─────────────────────────────────────────────

    #[test]
    fn test_aggregated_metrics_default() {
        let m = AggregatedMetrics::default();
        assert_eq!(m.node_id, String::new());
        assert!(m.timestamp > 0);
        assert_eq!(m.system.cpu_usage, 0.0);
        assert_eq!(m.conversation.active_sessions, 0);
    }

    #[test]
    fn test_aggregated_metrics_serde_roundtrip() {
        let m = AggregatedMetrics::default();
        let json = serde_json::to_string(&m).unwrap();
        let deserialized: AggregatedMetrics = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.node_id, m.node_id);
        assert_eq!(deserialized.timestamp, m.timestamp);
    }

    // ── All default metric types ──────────────────────────────────────

    #[test]
    fn test_system_metrics_default() {
        let m = SystemMetrics::default();
        assert_eq!(m.cpu_usage, 0.0);
        assert_eq!(m.memory_usage_bytes, 0);
        assert_eq!(m.gpu_usage, 0.0);
        assert_eq!(m.disk_usage_bytes, 0);
    }

    #[test]
    fn test_conversation_metrics_default() {
        let m = ConversationMetrics::default();
        assert_eq!(m.active_sessions, 0);
        assert_eq!(m.total_conversations, 0);
        assert_eq!(m.messages_per_second, 0.0);
        assert_eq!(m.avg_latency_ms, 0.0);
        assert_eq!(m.tool_executions, 0);
    }

    #[test]
    fn test_language_metrics_default() {
        let m = LanguageMetrics::default();
        assert!(m.provider_health.is_empty());
        assert_eq!(m.tokens_per_second, 0.0);
        assert_eq!(m.total_requests, 0);
        assert_eq!(m.failed_requests, 0);
        assert_eq!(m.avg_first_token_ms, 0.0);
    }

    #[test]
    fn test_retrieval_metrics_default() {
        let m = RetrievalMetrics::default();
        assert_eq!(m.memory_retrieval_ms, 0.0);
        assert_eq!(m.knowledge_lookup_ms, 0.0);
        assert_eq!(m.world_model_query_ms, 0.0);
        assert_eq!(m.context_assembly_ms, 0.0);
    }

    #[test]
    fn test_reasoning_metrics_default() {
        let m = ReasoningMetrics::default();
        assert_eq!(m.reasoning_latency_ms, 0.0);
        assert_eq!(m.planning_latency_ms, 0.0);
        assert_eq!(m.contradictions_detected, 0);
        assert_eq!(m.inference_steps, 0.0);
    }

    #[test]
    fn test_workflow_metrics_default() {
        let m = WorkflowMetrics::default();
        assert_eq!(m.active_workflows, 0);
        assert_eq!(m.completed_workflows, 0);
        assert_eq!(m.failed_workflows, 0);
        assert_eq!(m.avg_execution_ms, 0.0);
    }

    #[test]
    fn test_agent_metrics_default() {
        let m = AgentMetrics::default();
        assert_eq!(m.active_agents, 0);
        assert_eq!(m.total_tasks_completed, 0);
        assert_eq!(m.agent_utilization, 0.0);
    }

    // ── HealthChecker ─────────────────────────────────────────────────

    #[tokio::test]
    async fn test_health_checker_new() {
        let hc = HealthChecker::new();
        assert_eq!(hc.subsystem_count().await, 0);
    }

    #[tokio::test]
    async fn test_health_checker_check_all_empty() {
        let hc = HealthChecker::new();
        let results = hc.check_all().await;
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_health_checker_overall_status_empty() {
        let hc = HealthChecker::new();
        let status = hc.get_overall_status().await;
        assert_eq!(status, HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn test_health_checker_register_and_check() {
        let hc = HealthChecker::new();
        hc.register_static("test_sub", HealthStatus::Healthy, String::from("all good")).await;
        let results = hc.check_all().await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "test_sub");
        assert_eq!(results[0].status, HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn test_health_checker_check_subsystem() {
        let hc = HealthChecker::new();
        hc.register_static("db", HealthStatus::Healthy, String::from("ok")).await;
        let result = hc.check_subsystem("db").await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "db");
    }

    #[tokio::test]
    async fn test_health_checker_check_subsystem_not_found() {
        let hc = HealthChecker::new();
        let result = hc.check_subsystem("missing").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_health_checker_overall_unhealthy() {
        let hc = HealthChecker::new();
        hc.register_static("good", HealthStatus::Healthy, String::from("ok")).await;
        hc.register_static("bad", HealthStatus::Unhealthy, String::from("down")).await;
        let status = hc.get_overall_status().await;
        assert_eq!(status, HealthStatus::Unhealthy);
    }

    #[tokio::test]
    async fn test_health_checker_overall_degraded() {
        let hc = HealthChecker::new();
        hc.register_static("good", HealthStatus::Healthy, String::from("ok")).await;
        hc.register_static("slow", HealthStatus::Degraded, String::from("slow")).await;
        let status = hc.get_overall_status().await;
        assert_eq!(status, HealthStatus::Degraded);
    }

    #[tokio::test]
    async fn test_health_checker_subsystem_count() {
        let hc = HealthChecker::new();
        hc.register_static("a", HealthStatus::Healthy, String::from("")).await;
        hc.register_static("b", HealthStatus::Healthy, String::from("")).await;
        assert_eq!(hc.subsystem_count().await, 2);
    }

    #[tokio::test]
    async fn test_health_checker_registered_subsystems() {
        let hc = HealthChecker::new();
        hc.register_static("alpha", HealthStatus::Healthy, String::from("")).await;
        hc.register_static("beta", HealthStatus::Degraded, String::from("")).await;
        let names = hc.registered_subsystems().await;
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"alpha".to_string()));
        assert!(names.contains(&"beta".to_string()));
    }

    #[tokio::test]
    async fn test_health_checker_custom_check_fn() {
        let hc = HealthChecker::new();
        hc.register_subsystem("custom", || SubsystemHealthCheck {
            name: "custom".into(),
            status: HealthStatus::Degraded,
            latency_ms: 5.0,
            message: "custom check".into(),
            last_checked: 0,
        })
        .await;
        let result = hc.check_subsystem("custom").await.unwrap();
        assert_eq!(result.status, HealthStatus::Degraded);
        assert_eq!(result.latency_ms, 5.0);
    }

    // ── HealthConfig ──────────────────────────────────────────────────

    #[test]
    fn test_health_config_default() {
        let hc = HealthConfig::default();
        assert!(hc.unhealthy_threshold);
        assert!(hc.degraded_threshold);
    }

    #[tokio::test]
    async fn test_health_checker_with_custom_config() {
        let config = HealthConfig {
            unhealthy_threshold: false,
            degraded_threshold: false,
        };
        let hc = HealthChecker::with_config(config);
        hc.register_static("sub", HealthStatus::Unhealthy, String::from("down")).await;
        let status = hc.get_overall_status().await;
        assert_eq!(status, HealthStatus::Healthy);
    }

    // ── HealthStatus display ──────────────────────────────────────────

    #[test]
    fn test_health_status_display() {
        assert_eq!(format!("{}", HealthStatus::Healthy), "Healthy");
        assert_eq!(format!("{}", HealthStatus::Degraded), "Degraded");
        assert_eq!(format!("{}", HealthStatus::Unhealthy), "Unhealthy");
    }

    // ── SubsystemHealthCheck ──────────────────────────────────────────

    #[test]
    fn test_subsystem_health_check_serde_roundtrip() {
        let check = SubsystemHealthCheck {
            name: "db".into(),
            status: HealthStatus::Healthy,
            latency_ms: 1.23,
            message: "all good".into(),
            last_checked: 1700000000,
        };
        let json = serde_json::to_string(&check).unwrap();
        let deserialized: SubsystemHealthCheck = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "db");
        assert_eq!(deserialized.status, HealthStatus::Healthy);
    }

    // ── TracingSetup ──────────────────────────────────────────────────

    #[test]
    fn test_tracing_setup_new() {
        let setup = TracingSetup::new();
        assert!(!setup.is_initialized());
    }

    #[test]
    fn test_tracing_setup_default() {
        let setup = TracingSetup::default();
        assert!(!setup.is_initialized());
    }

    // ── TracingConfig ─────────────────────────────────────────────────

    #[test]
    fn test_tracing_config_default() {
        let cfg = TracingConfig::default();
        assert_eq!(cfg.log_level, "info");
        assert_eq!(cfg.format, LogFormat::Compact);
        assert!(cfg.log_file.is_none());
        assert!(!cfg.enable_json);
        assert!(cfg.enable_console);
        assert_eq!(cfg.sample_rate, 1.0);
    }

    #[test]
    fn test_tracing_config_serde_roundtrip() {
        let cfg = TracingConfig {
            log_level: "debug".into(),
            format: LogFormat::Json,
            log_file: Some("/tmp/log".into()),
            enable_json: true,
            enable_console: false,
            sample_rate: 0.5,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let deserialized: TracingConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.log_level, "debug");
        assert_eq!(deserialized.format, LogFormat::Json);
        assert_eq!(deserialized.sample_rate, 0.5);
    }

    // ── LogFormat ─────────────────────────────────────────────────────

    #[test]
    fn test_log_format_default() {
        assert_eq!(LogFormat::default(), LogFormat::Compact);
    }

    #[test]
    fn test_log_format_all_variants() {
        let formats = [LogFormat::Pretty, LogFormat::Compact, LogFormat::Json, LogFormat::Full];
        for f in &formats {
            let json = serde_json::to_string(f).unwrap();
            let deserialized: LogFormat = serde_json::from_str(&json).unwrap();
            assert_eq!(&deserialized, f);
        }
    }

    // ── TracingSetup create_env_filter ────────────────────────────────

    #[test]
    fn test_tracing_setup_create_env_filter() {
        let _filter = TracingSetup::create_env_filter("info");
        let _filter2 = TracingSetup::create_env_filter("debug");
        let _filter3 = TracingSetup::create_env_filter("neo_core=trace,info");
    }

    // ── ObservabilityManager ──────────────────────────────────────────

    #[test]
    fn test_observability_manager_new() {
        let mgr = ObservabilityManager::new("test-node");
        assert_eq!(mgr.node_id, "test-node");
    }

    #[test]
    fn test_observability_manager_collect_metrics() {
        let mgr = ObservabilityManager::new("node-1");
        mgr.metrics().set_cpu_usage(50.0);
        mgr.metrics().record_conversation();
        let metrics = mgr.collect_metrics();
        assert_eq!(metrics.node_id, "node-1");
        assert_eq!(metrics.conversation.total_conversations, 1);
        assert!((metrics.system.cpu_usage - 50.0).abs() < 0.1);
    }

    #[tokio::test]
    async fn test_observability_manager_get_health() {
        let mgr = ObservabilityManager::new("node-1");
        let health = mgr.get_health().await;
        assert!(health.is_empty());
    }

    #[test]
    fn test_observability_manager_metrics_ref() {
        let mgr = ObservabilityManager::new("node-1");
        mgr.metrics().set_active_sessions(42);
        let metrics = mgr.collect_metrics();
        assert_eq!(metrics.conversation.active_sessions, 42);
    }

    #[test]
    fn test_observability_manager_health_ref() {
        let _mgr = ObservabilityManager::new("node-1");
        // Just verify the health() accessor compiles and returns a reference
        let _hc = _mgr.health();
    }

    #[tokio::test]
    async fn test_observability_manager_register_subsystem() {
        let mgr = ObservabilityManager::new("node-1");
        mgr.health()
            .register_static("custom", HealthStatus::Healthy, String::from("ok"))
            .await;
        let health = mgr.get_health().await;
        assert_eq!(health.len(), 1);
        assert_eq!(health[0].name, "custom");
    }
}
