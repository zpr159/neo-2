use neo_inference::distributed::{
    ClusterNode, DistributedConfig, DistributedInferenceManager, LoadBalanceStrategy,
    NodeRole, RemoteWorker, WorkerState,
};
use chrono::Utc;

fn make_worker(id: uuid::Uuid, state: WorkerState, tasks: u64) -> RemoteWorker {
    RemoteWorker {
        id,
        addr: "127.0.0.1".to_string(),
        port: 8080,
        state,
        capabilities: vec!["inference".to_string()],
        gpu_count: 1,
        memory_bytes: 8_000_000_000,
        cpu_cores: 8,
        connected_at: Utc::now(),
        last_heartbeat: Utc::now(),
        tasks_completed: tasks,
        tasks_failed: 0,
        average_latency_ms: 10.0,
    }
}

#[test]
fn test_register_and_unregister_worker() {
    let mgr = DistributedInferenceManager::new(DistributedConfig::default());
    let id = uuid::Uuid::new_v4();
    let worker = make_worker(id, WorkerState::Connected, 0);
    mgr.register_worker(worker);
    assert_eq!(mgr.worker_count(), 1);

    mgr.unregister_worker(id);
    assert_eq!(mgr.worker_count(), 0);
}

#[test]
fn test_select_worker_round_robin() {
    let config = DistributedConfig {
        load_balance_strategy: LoadBalanceStrategy::RoundRobin,
        ..Default::default()
    };
    let mgr = DistributedInferenceManager::new(config);

    let id1 = uuid::Uuid::new_v4();
    let id2 = uuid::Uuid::new_v4();
    mgr.register_worker(make_worker(id1, WorkerState::Connected, 0));
    mgr.register_worker(make_worker(id2, WorkerState::Connected, 0));

    let w1 = mgr.select_worker().unwrap();
    let w2 = mgr.select_worker().unwrap();
    assert_ne!(w1.id, w2.id);
}

#[test]
fn test_select_worker_least_loaded() {
    let config = DistributedConfig {
        load_balance_strategy: LoadBalanceStrategy::LeastLoaded,
        ..Default::default()
    };
    let mgr = DistributedInferenceManager::new(config);

    let id1 = uuid::Uuid::new_v4();
    let id2 = uuid::Uuid::new_v4();
    mgr.register_worker(make_worker(id1, WorkerState::Connected, 100));
    mgr.register_worker(make_worker(id2, WorkerState::Connected, 10));

    let selected = mgr.select_worker().unwrap();
    assert_eq!(selected.id, id2);
}

#[test]
fn test_select_worker_only_connected() {
    let config = DistributedConfig {
        load_balance_strategy: LoadBalanceStrategy::RoundRobin,
        ..Default::default()
    };
    let mgr = DistributedInferenceManager::new(config);

    let id1 = uuid::Uuid::new_v4();
    let id2 = uuid::Uuid::new_v4();
    mgr.register_worker(make_worker(id1, WorkerState::Disconnected, 0));
    mgr.register_worker(make_worker(id2, WorkerState::Connected, 0));

    let selected = mgr.select_worker().unwrap();
    assert_eq!(selected.id, id2);
}

#[test]
fn test_select_worker_none_available() {
    let mgr = DistributedInferenceManager::new(DistributedConfig::default());
    assert!(mgr.select_worker().is_none());
}

#[test]
fn test_retry_with_fallback_success() {
    let config = DistributedConfig {
        enable_fault_tolerance: true,
        max_retries: 2,
        retry_delay_ms: 1,
        ..Default::default()
    };
    let mgr = DistributedInferenceManager::new(config);

    let result = mgr.retry_with_fallback(|| Some(42));
    assert_eq!(result, Some(42));
}

#[test]
fn test_retry_with_fallback_failure() {
    let config = DistributedConfig {
        enable_fault_tolerance: true,
        max_retries: 1,
        retry_delay_ms: 1,
        ..Default::default()
    };
    let mgr = DistributedInferenceManager::new(config);

    let mut attempts = 0;
    let result = mgr.retry_with_fallback(|| {
        attempts += 1;
        None::<i32>
    });
    assert!(result.is_none());
    assert_eq!(attempts, 2);
}

#[test]
fn test_cluster_node_management() {
    let mgr = DistributedInferenceManager::new(DistributedConfig::default());
    let node_id = uuid::Uuid::new_v4();
    let node = ClusterNode {
        id: node_id,
        addr: "127.0.0.1".to_string(),
        port: 9000,
        role: NodeRole::Follower,
        state: WorkerState::Connected,
        metadata: Default::default(),
        joined_at: Utc::now(),
    };
    mgr.add_cluster_node(node);
    mgr.remove_cluster_node(node_id);
}

#[test]
fn test_leader_follower() {
    let mgr = DistributedInferenceManager::new(DistributedConfig::default());
    assert!(!mgr.is_leader());

    mgr.set_leader(true);
    assert!(mgr.is_leader());

    mgr.set_leader(false);
    assert!(!mgr.is_leader());
}

#[test]
fn test_local_node_id() {
    let mgr = DistributedInferenceManager::new(DistributedConfig::default());
    let id = mgr.local_node_id();
    assert_eq!(id, mgr.local_node_id());
}

#[test]
fn test_active_worker_count() {
    let mgr = DistributedInferenceManager::new(DistributedConfig::default());
    let id1 = uuid::Uuid::new_v4();
    let id2 = uuid::Uuid::new_v4();
    let id3 = uuid::Uuid::new_v4();

    mgr.register_worker(make_worker(id1, WorkerState::Connected, 0));
    mgr.register_worker(make_worker(id2, WorkerState::Busy, 0));
    mgr.register_worker(make_worker(id3, WorkerState::Disconnected, 0));

    assert_eq!(mgr.active_worker_count(), 2);
}

#[test]
fn test_get_worker() {
    let mgr = DistributedInferenceManager::new(DistributedConfig::default());
    let id = uuid::Uuid::new_v4();
    mgr.register_worker(make_worker(id, WorkerState::Connected, 5));

    let w = mgr.get_worker(id);
    assert!(w.is_some());
    assert_eq!(w.unwrap().id, id);

    assert!(mgr.get_worker(uuid::Uuid::new_v4()).is_none());
}

#[test]
fn test_select_worker_least_latency() {
    let config = DistributedConfig {
        load_balance_strategy: LoadBalanceStrategy::LeastLatency,
        ..Default::default()
    };
    let mgr = DistributedInferenceManager::new(config);

    let id1 = uuid::Uuid::new_v4();
    let id2 = uuid::Uuid::new_v4();
    let mut w1 = make_worker(id1, WorkerState::Connected, 0);
    w1.average_latency_ms = 50.0;
    let mut w2 = make_worker(id2, WorkerState::Connected, 0);
    w2.average_latency_ms = 5.0;

    mgr.register_worker(w1);
    mgr.register_worker(w2);

    let selected = mgr.select_worker().unwrap();
    assert_eq!(selected.id, id2);
}
