#![allow(missing_docs, unused_extern_crates)]

use std::sync::Arc;

use neo_tools::{
    Composition, CompositionStep, CompositionStrategy, DynamicTool, ExecutionRecord,
    FilesystemSandbox, HealthMonitor, HealthStatus, NetworkSandbox, PermissionManager,
    PermissionPolicy, PermissionScope, PersistenceConfig, Sandbox, SandboxConfig, SandboxManager,
    ToolAnalytics, ToolBuilder, ToolCategory, ToolComposer, ToolContext, ToolEvent, ToolEventLog,
    ToolId, ToolLifecycleState, ToolManager, ToolPermission, ToolPersistence, ToolRegistry,
    ToolRequest, ToolSdk, ToolType, ToolVersion,
};

fn make_echo_tool(name: &str) -> DynamicTool {
    ToolBuilder::new(
        name,
        ToolVersion::new(1, 0, 0),
        format!("Echo tool: {name}"),
        ToolType::Custom("echo".into()),
        ToolCategory::Execute,
    )
    .on_execute(|params, _ctx| Box::pin(async move { Ok(params) }))
    .build()
    .unwrap()
}

fn make_failing_tool(name: &str) -> DynamicTool {
    ToolBuilder::new(
        name,
        ToolVersion::new(1, 0, 0),
        format!("Failing tool: {name}"),
        ToolType::Custom("failing".into()),
        ToolCategory::Execute,
    )
    .on_execute(|_params, _ctx| {
        Box::pin(async move {
            Err(neo_tools::ToolError::execution_failed(
                "intentional failure",
            ))
        })
    })
    .build()
    .unwrap()
}

#[tokio::test]
async fn test_full_tool_lifecycle() {
    let registry = Arc::new(ToolRegistry::new());
    let manager = ToolManager::new(Arc::clone(&registry));

    let tool = make_echo_tool("lifecycle_test");
    manager.activate_tool(tool).await.unwrap();

    let state = registry.state("lifecycle_test").await.unwrap();
    assert_eq!(state, ToolLifecycleState::Ready);

    manager.deactivate_tool("lifecycle_test").await.unwrap();
    assert!(!registry.contains("lifecycle_test").await);
}

#[tokio::test]
async fn test_registry_operations() {
    let registry = Arc::new(ToolRegistry::new());

    registry.register(make_echo_tool("tool_a")).await.unwrap();
    registry.register(make_echo_tool("tool_b")).await.unwrap();
    registry.register(make_echo_tool("tool_c")).await.unwrap();

    assert_eq!(registry.count().await, 3);

    let names = registry.list_names().await;
    assert!(names.contains(&"tool_a".to_string()));

    let search = registry.search("tool_a").await;
    assert_eq!(search.len(), 1);

    registry.disable("tool_a").await.unwrap();
    let tool = registry.get("tool_a").await.unwrap();
    assert!(!tool.read().await.manifest.config.enabled);

    registry.unregister("tool_a").await.unwrap();
    assert_eq!(registry.count().await, 2);
}

#[tokio::test]
async fn test_executor() {
    let registry = Arc::new(ToolRegistry::new());
    registry.register(make_echo_tool("echo")).await.unwrap();

    let executor = neo_tools::ToolExecutorBuilder::new()
        .registry(Arc::clone(&registry))
        .max_concurrent(5)
        .build()
        .unwrap();

    let ctx = ToolContext::new("test", neo_tools::CallerType::Internal);
    let request = ToolRequest::named(
        ToolId::new(),
        "echo",
        "echo",
        serde_json::json!({"key": "value"}),
        ctx,
    );

    let response = executor.execute(request).await.unwrap();
    assert!(response.success);
    assert_eq!(response.output["key"], "value");
    assert_eq!(executor.completed_count(), 1);
}

#[tokio::test]
async fn test_executor_retries() {
    let registry = Arc::new(ToolRegistry::new());
    registry
        .register(make_failing_tool("fail_tool"))
        .await
        .unwrap();

    let executor = neo_tools::ToolExecutor::new(Arc::clone(&registry), 5);

    let ctx = ToolContext::new("test", neo_tools::CallerType::Internal);
    let request = ToolRequest::named(
        ToolId::new(),
        "fail_tool",
        "fail",
        serde_json::json!({}),
        ctx,
    );

    let result = executor.execute_with_retries(request, 2).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_permissions() {
    let pm = PermissionManager::new();

    let perm = ToolPermission::new(
        "tool_a",
        "agent_1",
        PermissionScope::Filesystem,
        PermissionPolicy::AllowList(vec!["read".into(), "write".into()]),
    )
    .with_rate_limit(10);

    pm.grant(perm);

    assert!(pm.check("tool_a", "agent_1", "read").is_ok());
    assert!(pm.check("tool_a", "agent_1", "write").is_ok());
    assert!(pm.check("tool_a", "agent_1", "delete").is_err());
    assert!(pm.check("tool_a", "agent_2", "read").is_err());
}

#[tokio::test]
async fn test_sandbox() {
    let sandbox = Sandbox::new("test")
        .with_filesystem(
            FilesystemSandbox::new()
                .allow_path("/tmp")
                .deny_path("/etc"),
        )
        .with_network(NetworkSandbox::new().allow_host("example.com"));

    assert!(sandbox
        .check_filesystem(std::path::Path::new("/tmp/file.txt"), false)
        .is_ok());
    assert!(sandbox
        .check_filesystem(std::path::Path::new("/etc/passwd"), false)
        .is_err());
    assert!(sandbox.check_network("example.com", 443).is_ok());
    assert!(sandbox.check_network("evil.com", 443).is_err());
}

#[tokio::test]
async fn test_sandbox_manager() {
    let manager = SandboxManager::new();
    let config = SandboxConfig {
        cpu_limit_pct: Some(50.0),
        memory_limit_bytes: Some(128 * 1024 * 1024),
        disk_limit_bytes: None,
        network_allowed: false,
        allowed_paths: vec!["/tmp".into()],
        denied_paths: vec!["/etc".into()],
        temp_dir: Some("/tmp/sandbox".into()),
    };

    let _sb = manager.create_sandbox("exec_1", Some(&config));
    assert!(manager.get("exec_1").is_some());
    assert_eq!(manager.active_count(), 1);

    manager.remove("exec_1");
    assert_eq!(manager.active_count(), 0);
}

#[tokio::test]
async fn test_composition() {
    let registry = Arc::new(ToolRegistry::new());
    registry.register(make_echo_tool("step1")).await.unwrap();
    registry.register(make_echo_tool("step2")).await.unwrap();

    let executor = Arc::new(neo_tools::ToolExecutor::new(Arc::clone(&registry), 5));
    let composer = ToolComposer::new(executor);

    let composition =
        Composition::new("pipeline", "Test pipeline", CompositionStrategy::Sequential)
            .add_step(CompositionStep::new("step1", "step1", "execute"))
            .add_step(CompositionStep::new("step2", "step2", "execute"));

    let ctx = ToolContext::new("test", neo_tools::CallerType::Internal);
    let result = composer
        .execute(&composition, serde_json::json!({"input": "data"}), &ctx)
        .await
        .unwrap();

    assert!(result.success);
    assert_eq!(result.step_results.len(), 2);
}

#[test]
fn test_analytics() {
    let analytics = ToolAnalytics::new();
    analytics.record(ExecutionRecord {
        execution_id: "exec1".into(),
        tool_name: "tool_a".into(),
        operation: "test".into(),
        success: true,
        duration_ms: 100,
        started_at: chrono::Utc::now(),
        finished_at: chrono::Utc::now(),
        error: None,
        retry_count: 0,
        caller_id: "test".into(),
        input_size_bytes: None,
        output_size_bytes: None,
    });

    assert_eq!(analytics.total_executions(), 1);
    assert_eq!(analytics.success_rate("tool_a"), 1.0);
    assert!(analytics.avg_latency_ms("tool_a") > 0.0);

    let agg = analytics.aggregate();
    assert_eq!(agg.total_executions, 1);
    assert_eq!(agg.total_successes, 1);
}

#[test]
fn test_health_monitor() {
    let monitor = HealthMonitor::new(60);
    monitor.record(neo_tools::health::HealthCheckRecord {
        tool_name: "tool_a".into(),
        status: HealthStatus::Healthy,
        message: "OK".into(),
        latency_ms: 5.0,
        checked_at: chrono::Utc::now(),
    });

    let summary = monitor.summary();
    assert_eq!(summary.total, 1);
    assert_eq!(summary.healthy, 1);
    assert_eq!(summary.health_pct(), 100.0);
}

#[test]
fn test_event_log() {
    let log = ToolEventLog::new(100);
    log.push(ToolEvent::ToolRegistered {
        tool_name: "test".into(),
        tool_type: "custom".into(),
        version: ToolVersion::new(1, 0, 0),
        timestamp: chrono::Utc::now(),
    });

    assert_eq!(log.len(), 1);
    assert!(!log.is_empty());

    let recent = log.recent(10);
    assert_eq!(recent.len(), 1);
}

#[test]
fn test_lifecycle_state_machine() {
    let mut tracker = neo_tools::LifecycleTracker::new(ToolLifecycleState::Registered);
    assert_eq!(tracker.current(), ToolLifecycleState::Registered);

    tracker.transition(ToolLifecycleState::Loading).unwrap();
    tracker.transition(ToolLifecycleState::Loaded).unwrap();
    tracker
        .transition(ToolLifecycleState::Initializing)
        .unwrap();
    tracker.transition(ToolLifecycleState::Ready).unwrap();
    assert!(tracker.current().can_execute());

    tracker.transition(ToolLifecycleState::Running).unwrap();
    assert!(tracker.current().is_active());

    tracker.transition(ToolLifecycleState::Ready).unwrap();
    tracker.transition(ToolLifecycleState::Stopping).unwrap();
    tracker.transition(ToolLifecycleState::Stopped).unwrap();
    assert!(tracker.current().is_terminal());

    assert_eq!(tracker.history().len(), 9);
}

#[test]
fn test_tool_version() {
    let v1 = ToolVersion::new(1, 0, 0);
    let v2 = ToolVersion::new(1, 1, 0);
    let v3 = ToolVersion::new(2, 0, 0);

    assert!(v1 < v2);
    assert!(v2 < v3);
    assert!(v1.is_compatible(&v1));
    assert!(v2.is_compatible(&v1));
    assert!(!v1.is_compatible(&v2));
}

#[test]
fn test_tool_builder() {
    let tool = ToolBuilder::new(
        "builder_test",
        ToolVersion::new(1, 0, 0),
        "Builder test tool",
        ToolType::Custom("test".into()),
        ToolCategory::Execute,
    )
    .author("test")
    .license("MIT")
    .tag("test")
    .timeout_ms(5000)
    .max_retries(3)
    .requiring_permission()
    .on_execute(|params, _ctx| Box::pin(async move { Ok(params) }))
    .build();

    assert!(tool.is_ok());
    let tool = tool.unwrap();
    assert_eq!(tool.name(), "builder_test");
    assert!(!tool.is_executable());
}

#[test]
fn test_persistence() {
    let dir = tempfile::tempdir().unwrap();
    let persistence = ToolPersistence::new(PersistenceConfig::new(dir.path()));
    persistence.config.ensure_dirs().unwrap();

    let manifest = neo_tools::ToolManifest::new(
        neo_tools::ToolMetadata::new(
            "persist_test",
            "test",
            ToolType::Shell,
            ToolCategory::Execute,
            ToolVersion::new(1, 0, 0),
        ),
        neo_tools::ToolConfiguration::enabled(),
    );

    persistence
        .save_manifest("persist_test", &manifest)
        .unwrap();
    let loaded = persistence.load_manifest("persist_test").unwrap();
    assert_eq!(loaded.metadata.name, "persist_test");

    let config = neo_tools::ToolConfiguration::enabled();
    persistence.save_config("persist_test", &config).unwrap();
    let loaded_config = persistence.load_config("persist_test").unwrap();
    assert!(loaded_config.enabled);
}

#[test]
fn test_api_types() {
    use neo_tools::api::*;

    let resp = ApiResponse::ok(ToolListResponse {
        tools: vec![],
        total: 0,
    });
    assert!(resp.success);

    let err: ApiResponse<String> = ApiResponse::err("bad request");
    assert!(!err.success);
}

#[tokio::test]
async fn test_sdk() {
    let registry = Arc::new(ToolRegistry::new());
    let executor = ToolSdk::executor(Arc::clone(&registry), 10);
    assert_eq!(executor.max_concurrent(), 10);

    let tool = ToolSdk::tool(
        "sdk_test",
        ToolVersion::new(1, 0, 0),
        "SDK test",
        ToolType::Custom("test".into()),
        ToolCategory::Execute,
    )
    .on_execute(|params, _ctx| Box::pin(async move { Ok(params) }))
    .build()
    .unwrap();

    registry.register(tool).await.unwrap();
    assert!(registry.contains("sdk_test").await);
}

#[tokio::test]
async fn test_direct_tool_execute() {
    let tool = make_echo_tool("direct_echo");
    let ctx = ToolContext::new("test", neo_tools::CallerType::Internal);
    let req = ToolRequest::new(
        ToolId::new(),
        "echo",
        serde_json::json!({"message": "hello"}),
        ctx,
    );
    let result = tool.execute(&req).await.unwrap();
    assert_eq!(result["message"], "hello");
}
