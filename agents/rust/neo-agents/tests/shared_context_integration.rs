use neo_agents::{
    AgentId, ContextVersion, SharedBlackboard, SharedContext, SharedWorkspace, WorkingMemory,
};

#[tokio::test]
async fn shared_context_read_write() {
    let ctx = SharedContext::new("test");
    let agent = AgentId::new();

    ctx.set("key1".into(), serde_json::json!("value1"), agent)
        .await
        .unwrap();
    let val = ctx.get("key1").await;
    assert_eq!(val, Some(serde_json::json!("value1")));

    ctx.set("key2".into(), serde_json::json!(42), agent)
        .await
        .unwrap();
    assert_eq!(ctx.get("key2").await, Some(serde_json::json!(42)));
}

#[tokio::test]
async fn shared_context_optimistic_locking() {
    let ctx = SharedContext::new("test");
    let agent = AgentId::new();

    let v1 = ctx
        .set("k".into(), serde_json::json!("v1"), agent)
        .await
        .unwrap();

    let v2 = ctx
        .update_with_version("k", serde_json::json!("v2"), agent, v1)
        .await
        .unwrap();

    let result = ctx
        .update_with_version("k", serde_json::json!("v3"), agent, v1)
        .await;
    assert!(result.is_err());

    assert_eq!(ctx.get("k").await, Some(serde_json::json!("v2")));
    assert_eq!(v2, ContextVersion(3));
}

#[tokio::test]
async fn shared_context_snapshot() {
    let ctx = SharedContext::new("test");
    let agent = AgentId::new();

    ctx.set("a".into(), serde_json::json!(1), agent)
        .await
        .unwrap();
    ctx.set("b".into(), serde_json::json!(2), agent)
        .await
        .unwrap();

    let snap = ctx.snapshot().await;
    assert_eq!(snap.entries.len(), 2);
}

#[tokio::test]
async fn blackboard_sections() {
    let bb = SharedBlackboard::new();

    bb.create_section("plans");
    bb.create_section("results");

    bb.write("plans", "step1", serde_json::json!("research"))
        .await
        .unwrap();
    bb.write("plans", "step2", serde_json::json!("implement"))
        .await
        .unwrap();
    bb.write("results", "r1", serde_json::json!("found"))
        .await
        .unwrap();

    let val = bb.read("plans", "step1").await.unwrap();
    assert_eq!(val, serde_json::json!("research"));

    let keys = bb.list_section("plans").await.unwrap();
    assert_eq!(keys.len(), 2);
}

#[tokio::test]
async fn shared_workspace() {
    let ws = SharedWorkspace::new("ws-test");
    let agent = AgentId::new();

    ws.context()
        .set("shared".into(), serde_json::json!("data"), agent)
        .await
        .unwrap();

    ws.blackboard().create_section("notes");
    ws.blackboard()
        .write("notes", "n1", serde_json::json!("note1"))
        .await
        .unwrap();

    ws.register_agent(agent, 10);
    ws.update_working_memory(&agent, "task".into(), serde_json::json!("coding"))
        .unwrap();

    let wm = ws.get_working_memory(&agent).unwrap();
    assert_eq!(wm.get("task").cloned(), Some(serde_json::json!("coding")));
}

#[tokio::test]
async fn working_memory_operations() {
    let mut wm = WorkingMemory::new(5);
    wm.store("a".into(), serde_json::json!("1")).unwrap();
    wm.store("b".into(), serde_json::json!("2")).unwrap();
    wm.store("c".into(), serde_json::json!("3")).unwrap();

    assert_eq!(wm.get("a"), Some(&serde_json::json!("1")));
    assert_eq!(wm.data.len(), 3);

    wm.remove("b");
    assert_eq!(wm.get("b"), None);
    assert_eq!(wm.data.len(), 2);
}

#[tokio::test]
async fn working_memory_capacity_rejection() {
    let mut wm = WorkingMemory::new(3);
    wm.store("a".into(), serde_json::json!(1)).unwrap();
    wm.store("b".into(), serde_json::json!(2)).unwrap();
    wm.store("c".into(), serde_json::json!(3)).unwrap();
    let err = wm.store("d".into(), serde_json::json!(4));
    assert!(err.is_err());
    assert_eq!(wm.data.len(), 3);
}

#[tokio::test]
async fn working_memory_scratch_pad() {
    let mut wm = WorkingMemory::new(10);
    wm.push_scratch(serde_json::json!("temp1"));
    wm.push_scratch(serde_json::json!("temp2"));

    let val = wm.pop_scratch().unwrap();
    assert_eq!(val, serde_json::json!("temp2"));
    let val = wm.pop_scratch().unwrap();
    assert_eq!(val, serde_json::json!("temp1"));
    assert!(wm.pop_scratch().is_none());
}
