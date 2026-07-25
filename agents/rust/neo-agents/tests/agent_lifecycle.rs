use neo_agents::{
    AgentConfiguration, AgentHealth, AgentId, AgentManager, AgentRole, AgentStatus, AgentType,
};

fn config(name: &str) -> AgentConfiguration {
    AgentConfiguration::new(name)
        .with_role(AgentRole::Executor)
        .with_type(AgentType::Autonomous)
        .with_max_retries(3)
        .with_heartbeat_interval(10)
}

#[tokio::test]
async fn create_and_inspect_agent() {
    let mgr = AgentManager::new(10);
    let id = mgr.create_agent(config("alpha")).await.unwrap();
    let snap = mgr.inspect_agent(id).await.unwrap();

    assert_eq!(snap.name, "alpha");
    assert_eq!(snap.status, AgentStatus::Ready);
    assert_eq!(snap.health, AgentHealth::Healthy);
}

#[tokio::test]
async fn start_stop_cycle() {
    let mgr = AgentManager::new(10);
    let id = mgr.create_agent(config("worker")).await.unwrap();

    mgr.start_agent(id).await.unwrap();
    assert_eq!(
        mgr.inspect_agent(id).await.unwrap().status,
        AgentStatus::Running
    );

    mgr.stop_agent(id).await.unwrap();
    assert_eq!(
        mgr.inspect_agent(id).await.unwrap().status,
        AgentStatus::Stopped
    );
}

#[tokio::test]
async fn pause_resume_cycle() {
    let mgr = AgentManager::new(10);
    let id = mgr.create_agent(config("pausable")).await.unwrap();

    mgr.start_agent(id).await.unwrap();
    mgr.pause_agent(id).await.unwrap();
    assert_eq!(
        mgr.inspect_agent(id).await.unwrap().status,
        AgentStatus::Paused
    );

    mgr.resume_agent(id).await.unwrap();
    assert_eq!(
        mgr.inspect_agent(id).await.unwrap().status,
        AgentStatus::Running
    );
}

#[tokio::test]
async fn restart_agent() {
    let mgr = AgentManager::new(10);
    let id = mgr.create_agent(config("restarter")).await.unwrap();

    mgr.start_agent(id).await.unwrap();
    mgr.restart_agent(id).await.unwrap();
    assert_eq!(
        mgr.inspect_agent(id).await.unwrap().status,
        AgentStatus::Ready
    );
}

#[tokio::test]
async fn terminate_removes_agent() {
    let mgr = AgentManager::new(10);
    let id = mgr.create_agent(config("goner")).await.unwrap();
    assert_eq!(mgr.agent_count(), 1);

    mgr.terminate_agent(id).await.unwrap();
    assert_eq!(mgr.agent_count(), 0);
    assert!(mgr.inspect_agent(id).await.is_err());
}

#[tokio::test]
async fn max_agents_enforced() {
    let mgr = AgentManager::new(2);
    mgr.create_agent(config("a1")).await.unwrap();
    mgr.create_agent(config("a2")).await.unwrap();
    let err = mgr.create_agent(config("a3")).await;
    assert!(err.is_err());
}

#[tokio::test]
async fn list_agents_by_status() {
    let mgr = AgentManager::new(10);
    let id1 = mgr.create_agent(config("s1")).await.unwrap();
    let id2 = mgr.create_agent(config("s2")).await.unwrap();
    let id3 = mgr.create_agent(config("s3")).await.unwrap();

    mgr.start_agent(id1).await.unwrap();
    mgr.start_agent(id2).await.unwrap();

    let running = mgr.list_agents(Some(AgentStatus::Running));
    assert_eq!(running.len(), 2);
    assert!(running.contains(&id1));
    assert!(running.contains(&id2));

    let ready = mgr.list_agents(Some(AgentStatus::Ready));
    assert_eq!(ready.len(), 1);
    assert!(ready.contains(&id3));
}

#[tokio::test]
async fn list_agents_by_role() {
    let mgr = AgentManager::new(10);
    let id1 = mgr.create_agent(config("r1")).await.unwrap();
    mgr.create_agent(
        AgentConfiguration::new("r2")
            .with_role(AgentRole::Researcher)
            .with_heartbeat_interval(10),
    )
    .await
    .unwrap();

    let executors = mgr.list_agents_by_role(&AgentRole::Executor);
    assert_eq!(executors.len(), 1);
    assert!(executors.contains(&id1));
}

#[tokio::test]
async fn statistics_tracking() {
    let mgr = AgentManager::new(10);
    let id1 = mgr.create_agent(config("st1")).await.unwrap();
    let id2 = mgr.create_agent(config("st2")).await.unwrap();

    mgr.start_agent(id1).await.unwrap();
    mgr.start_agent(id2).await.unwrap();

    let stats = mgr.statistics();
    assert_eq!(stats.total_agents_created, 2);
    assert_eq!(stats.active_agents, 2);

    mgr.terminate_agent(id1).await.unwrap();
    mgr.terminate_agent(id2).await.unwrap();

    let stats = mgr.statistics();
    assert_eq!(stats.active_agents, 0);
}

#[tokio::test]
async fn shutdown_stops_all() {
    let mgr = AgentManager::new(10);
    mgr.create_agent(config("sh1")).await.unwrap();
    mgr.create_agent(config("sh2")).await.unwrap();
    mgr.start_agent(mgr.list_agents(None)[0]).await.unwrap();

    mgr.shutdown().await.unwrap();
    assert!(mgr.is_shutdown());
    assert!(mgr.create_agent(config("after")).await.is_err());
}

#[tokio::test]
async fn shared_properties() {
    let mgr = AgentManager::new(10);
    mgr.set_shared_property("env".into(), serde_json::json!("test"))
        .await;
    let val = mgr.get_shared_property("env").await;
    assert_eq!(val, Some(serde_json::json!("test")));
}
