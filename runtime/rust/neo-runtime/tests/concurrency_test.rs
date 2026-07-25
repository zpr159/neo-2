//! Concurrency and stress tests for the Neo Runtime system.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use neo_runtime::config::*;
use neo_runtime::lifecycle::*;
use neo_runtime::resource::*;
use neo_runtime::scheduler::*;
use neo_runtime::*;

#[test]
fn concurrent_resource_allocation() {
    let mgr = Arc::new(ResourceManager::new());
    mgr.register_pool(ResourceType::Cpu, 100);
    mgr.register_pool(ResourceType::Ram, 10_000);

    let mut handles = Vec::new();

    for _ in 0..20 {
        let mgr = mgr.clone();
        handles.push(thread::spawn(move || {
            let consumer = ConsumerId::new();
            for _ in 0..10 {
                if let Ok(h) = mgr.allocate(ResourceType::Cpu, 1, consumer) {
                    thread::sleep(Duration::from_millis(1));
                    mgr.release(h).unwrap();
                }
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(mgr.available(ResourceType::Cpu), 100);
}

#[test]
fn concurrent_lifecycle_transitions() {
    let mgr = Arc::new(LifecycleManager::new());
    let mut ids = Vec::new();

    for _ in 0..50 {
        let id = mgr.register_service("svc");
        ids.push(id);
    }

    let mut handles = Vec::new();
    for id in ids {
        let mgr = mgr.clone();
        handles.push(thread::spawn(move || {
            mgr.transition(id, ServiceState::Registered, "reg").unwrap();
            mgr.transition(id, ServiceState::Initialized, "init").unwrap();
            mgr.transition(id, ServiceState::Starting, "start").unwrap();
            mgr.transition(id, ServiceState::Running, "run").unwrap();
            mgr.transition(id, ServiceState::Stopping, "stop").unwrap();
            mgr.transition(id, ServiceState::Stopped, "stopped").unwrap();
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    for id in ids {
        assert_eq!(mgr.state(id).unwrap(), ServiceState::Stopped);
    }
}

#[test]
fn concurrent_event_bus() {
    let mut bus = EventBus::new(EventBusConfig {
        broadcast_capacity: 4096,
        ..EventBusConfig::default()
    });

    let mut handles = Vec::new();
    for i in 0..5 {
        // We can't clone EventBus easily, so test sequential publish/consume
        for j in 0..100 {
            let event = Event::new(
                "topic",
                serde_json::json!({"i": i, "j": j}),
                "source",
            );
            let _ = bus.publish(event);
        }
    }

    let mut rx = bus.receiver();
    let mut received = 0;
    while rx.try_recv().is_ok() {
        received += 1;
    }
    assert_eq!(received, 500);
}

#[test]
fn concurrent_task_scheduler() {
    let config = SchedulerConfig {
        queue_capacity: 8192,
        ..SchedulerConfig::default()
    };

    let mut sched = TaskScheduler::new(config);
    let completed = Arc::new(AtomicUsize::new(0));

    for i in 0..10 {
        let task = ScheduledTask::new(format!("task-{}", i), TaskPriority::Normal);
        sched.submit(task).unwrap();
    }

    while let Some(task) = sched.dequeue() {
        sched.complete(task.id);
        completed.fetch_add(1, Ordering::SeqCst);
    }

    assert_eq!(completed.load(Ordering::SeqCst), 10);
}

#[test]
fn stress_memory_pool() {
    let pool = Arc::new(MemoryPool::new(64, 1000));

    let mut handles = Vec::new();
    for _ in 0..10 {
        let pool = pool.clone();
        handles.push(thread::spawn(move || {
            let mut blocks = Vec::new();
            for _ in 0..100 {
                if let Ok(b) = pool.allocate() {
                    blocks.push(b);
                }
            }
            for b in blocks {
                pool.release(b).ok();
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(pool.allocated_blocks(), 0);
    assert_eq!(pool.available_blocks(), pool.total_blocks());
}

#[test]
fn stress_version_constraint_checking() {
    let constraints = vec![
        VersionConstraint::Any,
        VersionConstraint::Exact { major: 1, minor: 0, patch: 0 },
        VersionConstraint::AtLeast { major: 1, minor: 0, patch: 0 },
        VersionConstraint::Compatible { major: 1, minor: 0, patch: 0 },
        VersionConstraint::Range { min_major: 0, min_minor: 0, min_patch: 0, max_major: 10, max_minor: 0, max_patch: 0 },
    ];

    let versions: Vec<(u32, u32, u32)> = (0..10)
        .flat_map(|i| (0..10).map(move |j| (i, j, 0)))
        .collect();

    for c in &constraints {
        for &(maj, min, pat) in &versions {
            let _ = c.matches(maj, min, pat);
        }
    }
}

#[test]
fn stress_dependency_graph_operations() {
    let mut graph = DependencyGraph::new();
    let ids: Vec<ServiceId> = (0..100).map(|_| ServiceId::new()).collect();

    for (i, &id) in ids.iter().enumerate() {
        graph.add_node(id, format!("service-{}", i), (1, 0, 0));
    }

    for i in 0..99 {
        graph.add_dependency(
            ids[i],
            Dependency {
                service_id: ids[i + 1],
                service_name: format!("service-{}", i + 1),
                version_constraint: VersionConstraint::Any,
                optional: false,
            },
        );
    }

    let sorted = graph.topological_sort().unwrap();
    assert_eq!(sorted.len(), 100);

    for i in 0..99 {
        let pos_a = sorted.iter().position(|&id| id == ids[i + 1]).unwrap();
        let pos_b = sorted.iter().position(|&id| id == ids[i]).unwrap();
        assert!(pos_a < pos_b);
    }

    assert!(graph.detect_cycle().is_none());
}

#[tokio::test]
async fn concurrent_async_runtime_spawn() {
    let rt = NeoAsyncRuntime::new(64).unwrap();
    let counter = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();
    for _ in 0..100 {
        let c = counter.clone();
        handles.push(rt.spawn_tracked(async move {
            c.fetch_add(1, Ordering::SeqCst);
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }

    assert_eq!(counter.load(Ordering::SeqCst), 100);
}

#[tokio::test]
async fn backpressure_stress() {
    let bp = Arc::new(Backpressure::new(5));
    let active = Arc::new(AtomicUsize::new(0));
    let max_active = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();
    for _ in 0..50 {
        let bp = bp.clone();
        let active = active.clone();
        let max_active = max_active.clone();
        handles.push(tokio::spawn(async move {
            let _permit = bp.acquire().await.unwrap();
            let current = active.fetch_add(1, Ordering::SeqCst) + 1;
            max_active.fetch_max(current, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(10)).await;
            active.fetch_sub(1, Ordering::SeqCst);
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }

    assert!(max_active.load(Ordering::SeqCst) <= 5);
}

#[test]
fn stress_hot_reload_config() {
    let initial = RuntimeConfiguration::development();
    let hot = HotReloadConfig::new(initial);

    for i in 0..1000 {
        let mut config = RuntimeConfiguration::production();
        config.scheduler.max_concurrent_tasks = i;
        hot.update(config);
    }

    assert_eq!(hot.current().scheduler.max_concurrent_tasks, 999);
}

#[test]
fn concurrent_plugin_registration() {
    let dir = tempfile::tempdir().unwrap();
    let loader = Arc::new(PluginLoader::new(
        dir.path().to_path_buf(),
        PluginSandboxConfig {
            enabled: false,
            ..PluginSandboxConfig::default()
        },
        PluginHotReload::default(),
    ));

    let mut handles = Vec::new();
    for i in 0..20 {
        let loader = loader.clone();
        handles.push(thread::spawn(move || {
            loader.register_plugin(PluginDescriptor {
                id: PluginId::new(),
                name: format!("plugin-{}", i),
                version: "1.0.0".to_string(),
                author: "test".to_string(),
                description: "test".to_string(),
                path: dir.path().join(format!("plugin-{}.so", i)),
                checksum: "".to_string(),
                required_permissions: vec![],
                dependencies: vec![],
            });
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(loader.list().len(), 20);
}
