use neo_agents::{AgentId, Task, TaskId, TaskPriority, TaskScheduler, TaskStatus};

#[tokio::test]
async fn submit_and_assign_task() {
    let scheduler = TaskScheduler::new(4);
    let agent = AgentId::new();

    let task = Task::new("test-task", "do something", serde_json::json!(null));
    let task_id = scheduler.submit_task(task).await.unwrap();

    let assigned = scheduler.assign_next_task(agent).await.unwrap();
    assert!(assigned.is_some());
    let assigned = assigned.unwrap();
    assert_eq!(assigned.id, task_id);
    assert_eq!(assigned.status, TaskStatus::Assigned);
    assert_eq!(scheduler.agent_task_count(&agent), 1);
}

#[tokio::test]
async fn complete_task_assignment() {
    let scheduler = TaskScheduler::new(4);
    let agent = AgentId::new();

    let task = Task::new("complete-me", "desc", serde_json::json!(null));
    let task_id = scheduler.submit_task(task).await.unwrap();
    scheduler.assign_next_task(agent).await.unwrap();

    let result = scheduler
        .complete_assignment(task_id, serde_json::json!("result"))
        .await
        .unwrap();
    assert!(result.success);
    assert_eq!(result.task_id, task_id);
    assert_eq!(scheduler.agent_task_count(&agent), 0);
}

#[tokio::test]
async fn fail_and_retry_task() {
    let scheduler = TaskScheduler::new(4);
    let agent = AgentId::new();

    let task = Task::new("retry-task", "desc", serde_json::json!(null)).with_max_retries(2);
    let task_id = scheduler.submit_task(task).await.unwrap();
    scheduler.assign_next_task(agent).await.unwrap();

    let retried = scheduler
        .fail_assignment(task_id, "oops".into())
        .await
        .unwrap();
    assert!(retried.is_some());
    let retried = retried.unwrap();
    assert_eq!(retried.retry_count, 1);

    let assigned2 = scheduler.assign_next_task(agent).await.unwrap();
    assert!(assigned2.is_some());
    assert_eq!(assigned2.unwrap().id, task_id);
}

#[tokio::test]
async fn cancel_task() {
    let scheduler = TaskScheduler::new(4);
    let agent = AgentId::new();

    let task = Task::new("cancel-me", "desc", serde_json::json!(null));
    let task_id = scheduler.submit_task(task).await.unwrap();
    scheduler.assign_next_task(agent).await.unwrap();

    scheduler.cancel_task(task_id).await.unwrap();
    assert_eq!(scheduler.agent_task_count(&agent), 0);
}

#[tokio::test]
async fn capacity_enforced() {
    let scheduler = TaskScheduler::new(1);
    let agent = AgentId::new();

    let t1 = Task::new("t1", "d", serde_json::json!(null));
    let t2 = Task::new("t2", "d", serde_json::json!(null));
    scheduler.submit_task(t1).await.unwrap();
    scheduler.submit_task(t2).await.unwrap();

    let first = scheduler.assign_next_task(agent).await.unwrap();
    assert!(first.is_some());

    let second = scheduler.assign_next_task(agent).await.unwrap();
    assert!(second.is_none());
}

#[tokio::test]
async fn priority_ordering() {
    let scheduler = TaskScheduler::new(4);

    let low = Task::new("low", "d", serde_json::json!(null)).with_priority(TaskPriority::Low);
    let high = Task::new("high", "d", serde_json::json!(null)).with_priority(TaskPriority::High);
    let crit =
        Task::new("crit", "d", serde_json::json!(null)).with_priority(TaskPriority::Critical);

    scheduler.submit_task(low).await.unwrap();
    scheduler.submit_task(high).await.unwrap();
    scheduler.submit_task(crit).await.unwrap();

    let agent = AgentId::new();
    let t1 = scheduler.assign_next_task(agent).await.unwrap().unwrap();
    assert_eq!(t1.name, "crit");

    let t2 = scheduler.assign_next_task(agent).await.unwrap().unwrap();
    assert_eq!(t2.name, "high");

    let t3 = scheduler.assign_next_task(agent).await.unwrap().unwrap();
    assert_eq!(t3.name, "low");
}

#[tokio::test]
async fn dependency_ordering() {
    let scheduler = TaskScheduler::new(4);

    let dep = Task::new("dep", "d", serde_json::json!(null));
    let dep_id = dep.id;

    let main = Task::new("main", "d", serde_json::json!(null)).with_dependency(dep_id);

    scheduler.submit_task(main).await.unwrap();
    scheduler.submit_task(dep).await.unwrap();

    let agent = AgentId::new();
    let first = scheduler.assign_next_task(agent).await.unwrap().unwrap();
    assert_eq!(first.name, "dep");

    scheduler
        .complete_assignment(first.id, serde_json::json!("done"))
        .await
        .unwrap();

    let second = scheduler.assign_next_task(agent).await.unwrap().unwrap();
    assert_eq!(second.name, "main");
}

#[tokio::test]
async fn task_queue_empty_after_all_complete() {
    let scheduler = TaskScheduler::new(4);
    let agent = AgentId::new();

    for i in 0..5 {
        let t = Task::new(format!("t{i}"), "d", serde_json::json!(null));
        let id = scheduler.submit_task(t).await.unwrap();
        scheduler.assign_next_task(agent).await.unwrap();
        scheduler
            .complete_assignment(id, serde_json::json!("done"))
            .await
            .unwrap();
    }

    assert_eq!(scheduler.pending_count().await, 0);
    assert_eq!(scheduler.agent_task_count(&agent), 0);
}
