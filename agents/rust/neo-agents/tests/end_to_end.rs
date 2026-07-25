use neo_agents::memory::AgentMemoryManager;
use neo_agents::{
    AgentAnalytics, AgentConfiguration, AgentId, AgentManager, AgentMessage, AgentMetrics,
    AgentRole, AgentType, MemoryEntry, MemoryTier, MessageChannelRegistry, MessageType,
    RecoveryStrategy, SharedBlackboard, SharedWorkspace, SupervisorAgent, Task, TaskScheduler,
};

#[tokio::test]
async fn full_agent_task_workflow() {
    let manager = AgentManager::builder().with_max_agents(10).build();

    let agent_config = AgentConfiguration::new("worker-1")
        .with_role(AgentRole::Executor)
        .with_type(AgentType::Autonomous)
        .with_max_concurrent_tasks(4)
        .with_heartbeat_interval(10);

    let agent_id = manager.create_agent(agent_config).await.unwrap();
    manager.start_agent(agent_id).await.unwrap();

    let snap = manager.inspect_agent(agent_id).await.unwrap();
    assert_eq!(snap.status, AgentStatus::Running);

    let scheduler = TaskScheduler::new(4);
    let task = Task::new(
        "process-data",
        "process input data",
        serde_json::json!({"input": "test"}),
    )
    .with_priority(neo_agents::TaskPriority::High);
    let task_id = scheduler.submit_task(task).await.unwrap();

    let assigned = scheduler.assign_next_task(agent_id).await.unwrap();
    assert!(assigned.is_some());
    assert_eq!(assigned.unwrap().id, task_id);

    let result = scheduler
        .complete_assignment(task_id, serde_json::json!({"output": "processed"}))
        .await
        .unwrap();
    assert!(result.success);

    manager.stop_agent(agent_id).await.unwrap();
    let snap = manager.inspect_agent(agent_id).await.unwrap();
    assert_eq!(snap.status, AgentStatus::Stopped);
}

use neo_agents::AgentStatus;

#[tokio::test]
async fn multi_agent_collaboration() {
    let manager = AgentManager::new(10);

    let planner_id = manager
        .create_agent(
            AgentConfiguration::new("planner")
                .with_role(AgentRole::Planner)
                .with_type(AgentType::Deliberative)
                .with_heartbeat_interval(10),
        )
        .await
        .unwrap();

    let executor_id = manager
        .create_agent(
            AgentConfiguration::new("executor")
                .with_role(AgentRole::Executor)
                .with_type(AgentType::Reactive)
                .with_heartbeat_interval(10),
        )
        .await
        .unwrap();

    manager.start_agent(planner_id).await.unwrap();
    manager.start_agent(executor_id).await.unwrap();

    let (tx, _rx) = tokio::sync::mpsc::channel(64);
    manager.message_channels.register_inbox(executor_id, tx);

    let msg = AgentMessage::new(
        planner_id,
        executor_id,
        MessageType::Request,
        serde_json::json!({"action": "execute_step", "step": 1}),
    );
    manager.send_message(msg).await.unwrap();

    let bb = SharedBlackboard::new();
    bb.create_section("plan");
    bb.write("plan", "step1", serde_json::json!("data_analysis"))
        .await
        .unwrap();
    bb.write("plan", "step2", serde_json::json!("report_generation"))
        .await
        .unwrap();

    let step = bb.read("plan", "step1").await.unwrap();
    assert_eq!(step, serde_json::json!("data_analysis"));

    manager.stop_agent(planner_id).await.unwrap();
    manager.stop_agent(executor_id).await.unwrap();
}

#[tokio::test]
async fn supervisor_watches_agents() {
    let supervisor = SupervisorAgent::new();
    let manager = AgentManager::new(10);

    let mut agent_ids = Vec::new();
    for i in 0..3 {
        let id = manager
            .create_agent(
                AgentConfiguration::new(format!("agent-{i}"))
                    .with_role(AgentRole::Executor)
                    .with_heartbeat_interval(10),
            )
            .await
            .unwrap();
        supervisor.supervise(id);
        manager.start_agent(id).await.unwrap();
        agent_ids.push(id);
    }

    assert_eq!(supervisor.supervised_count(), 3);

    for id in &agent_ids {
        let snap = manager.inspect_agent(*id).await.unwrap();
        assert_eq!(snap.status, AgentStatus::Running);
    }

    for id in &agent_ids {
        manager.terminate_agent(*id).await.unwrap();
        supervisor.unsupervise(id);
    }
    assert_eq!(supervisor.supervised_count(), 0);
}

#[tokio::test]
async fn task_failure_and_recovery_workflow() {
    let manager = AgentManager::new(10);
    let supervisor = SupervisorAgent::new();

    let agent_id = manager
        .create_agent(
            AgentConfiguration::new("fragile")
                .with_role(AgentRole::Executor)
                .with_heartbeat_interval(10),
        )
        .await
        .unwrap();
    supervisor.supervise(agent_id);
    manager.start_agent(agent_id).await.unwrap();

    let scheduler = TaskScheduler::new(4);
    let task = Task::new("flaky-task", "might fail", serde_json::json!(null)).with_max_retries(2);
    let task_id = scheduler.submit_task(task).await.unwrap();

    let assigned = scheduler.assign_next_task(agent_id).await.unwrap().unwrap();
    assert_eq!(assigned.id, task_id);

    let retried = scheduler
        .fail_assignment(task_id, "transient error".into())
        .await
        .unwrap();
    assert!(retried.is_some());

    let strategy = supervisor.handle_failure(agent_id, "transient error".into());
    match strategy {
        RecoveryStrategy::Restart | RecoveryStrategy::FreshRestart => {}
        _ => {}
    }

    let reassigned = scheduler.assign_next_task(agent_id).await.unwrap().unwrap();
    assert_eq!(reassigned.id, task_id);
    assert_eq!(reassigned.retry_count, 1);

    scheduler
        .complete_assignment(task_id, serde_json::json!("ok"))
        .await
        .unwrap();

    supervisor.record_recovery(agent_id, RecoveryStrategy::Restart, true);
    manager.stop_agent(agent_id).await.unwrap();
}

#[tokio::test]
async fn memory_and_context_integration() {
    let manager = AgentManager::new(10);
    let memory_manager = AgentMemoryManager::new();
    let workspace = SharedWorkspace::new("project-1");

    let agent_id = manager
        .create_agent(
            AgentConfiguration::new("learner")
                .with_role(AgentRole::Researcher)
                .with_type(AgentType::Learning)
                .with_heartbeat_interval(10),
        )
        .await
        .unwrap();

    memory_manager.register_agent(agent_id);
    workspace.register_agent(agent_id, 50);

    let mut entry = MemoryEntry::new(
        serde_json::json!({"fact": "Rust is safe"}),
        MemoryTier::Working,
        agent_id,
    );
    entry.importance = 0.9;
    entry.tags.push("rust".to_string());
    memory_manager
        .get_memory_mut(&agent_id)
        .unwrap()
        .store(entry)
        .unwrap();

    workspace
        .context()
        .set(
            "current_topic".into(),
            serde_json::json!("memory systems"),
            agent_id,
        )
        .await
        .unwrap();

    workspace
        .update_working_memory(
            &agent_id,
            "focus".into(),
            serde_json::json!("memory integration"),
        )
        .unwrap();

    let mem = memory_manager.get_memory(&agent_id).unwrap();
    assert!(!mem.retrieve("rust", 10).is_empty());

    let topic = workspace.context().get("current_topic").await;
    assert_eq!(topic, Some(serde_json::json!("memory systems")));

    let wm = workspace.get_working_memory(&agent_id).unwrap();
    assert_eq!(
        wm.get("focus").cloned(),
        Some(serde_json::json!("memory integration"))
    );
}

#[tokio::test]
async fn analytics_from_metrics() {
    let metrics = AgentMetrics {
        tasks_completed: 10,
        tasks_failed: 1,
        tasks_active: 2,
        messages_sent: 50,
        messages_received: 45,
        error_count: 3,
        recovery_count: 1,
        memory_used_bytes: 1024 * 1024,
        cpu_time_ms: 5000,
        uptime_secs: 3600,
        ..Default::default()
    };

    let analytics =
        AgentAnalytics::from_metrics(AgentId::new(), "test-agent".to_string(), &metrics);

    assert!(analytics.task_completion_rate > 0.9);
    assert!(analytics.message_throughput > 0.0);
    assert!(analytics.error_rate >= 0.0);
}
