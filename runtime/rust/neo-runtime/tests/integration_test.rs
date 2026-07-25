//! Integration tests for the Neo Runtime system.
//!
//! Tests the full runtime lifecycle: initialization, service registration,
//! dependency resolution, startup, and shutdown.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use neo_runtime::config::*;
use neo_runtime::dependency::*;
use neo_runtime::lifecycle::*;
use neo_runtime::manager::*;
use neo_runtime::resource::*;
use neo_runtime::scheduler::*;
use neo_runtime::*;

#[test]
fn full_runtime_lifecycle() {
    let rt = RuntimeManager::testing();

    // Register services with dependencies
    let db_id = rt.register_service(ServiceRegistration {
        name: "database".to_string(),
        version: (1, 0, 0),
        dependencies: vec![],
        optional_dependencies: vec![],
        priority: 0,
    });

    let cache_id = rt.register_service(ServiceRegistration {
        name: "cache".to_string(),
        version: (1, 0, 0),
        dependencies: vec![ServiceDependency {
            name: "database".to_string(),
            version_constraint: VersionConstraint::AtLeast {
                major: 1,
                minor: 0,
                patch: 0,
            },
            optional: false,
        }],
        optional_dependencies: vec![],
        priority: 1,
    });

    let api_id = rt.register_service(ServiceRegistration {
        name: "api".to_string(),
        version: (1, 0, 0),
        dependencies: vec![
            ServiceDependency {
                name: "database".to_string(),
                version_constraint: VersionConstraint::Any,
                optional: false,
            },
            ServiceDependency {
                name: "cache".to_string(),
                version_constraint: VersionConstraint::Any,
                optional: false,
            },
        ],
        optional_dependencies: vec![],
        priority: 2,
    });

    // Validate dependencies (no cycles, all resolved)
    assert!(rt.validate_dependencies().is_ok());

    // Start the runtime
    rt.start().unwrap();
    assert_eq!(rt.state(), RuntimeState::Running);
    assert!(rt.is_running());

    // Verify services are running
    assert_eq!(rt.lifecycle().state(db_id).unwrap(), ServiceState::Running);
    assert_eq!(rt.lifecycle().state(cache_id).unwrap(), ServiceState::Running);
    assert_eq!(rt.lifecycle().state(api_id).unwrap(), ServiceState::Running);

    // Verify dependency order
    let order = rt.dependency_order();
    let db_pos = order.iter().position(|&id| id == db_id).unwrap();
    let cache_pos = order.iter().position(|&id| id == cache_id).unwrap();
    let api_pos = order.iter().position(|&id| id == api_id).unwrap();
    assert!(db_pos < cache_pos);
    assert!(cache_pos < api_pos);

    // Shutdown
    rt.shutdown().unwrap();
    assert_eq!(rt.state(), RuntimeState::Stopped);
    assert!(!rt.is_running());
}

#[test]
fn circular_dependency_detection() {
    let mut graph = DependencyGraph::new();
    let a = ServiceId::new();
    let b = ServiceId::new();

    graph.add_node(a, "a", (1, 0, 0));
    graph.add_node(b, "b", (1, 0, 0));

    graph.add_dependency(
        a,
        Dependency {
            service_id: b,
            service_name: "b".to_string(),
            version_constraint: VersionConstraint::Any,
            optional: false,
        },
    );
    graph.add_dependency(
        b,
        Dependency {
            service_id: a,
            service_name: "a".to_string(),
            version_constraint: VersionConstraint::Any,
            optional: false,
        },
    );

    assert!(graph.detect_cycle().is_some());
    assert!(graph.topological_sort().is_err());
}

#[test]
fn lifecycle_full_state_machine() {
    let mgr = LifecycleManager::new();
    let id = mgr.register_service("full-lifecycle");

    // Full lifecycle: Created -> Registered -> Initialized -> Starting -> Running
    mgr.transition(id, ServiceState::Registered, "reg").unwrap();
    mgr.transition(id, ServiceState::Initialized, "init").unwrap();
    mgr.transition(id, ServiceState::Starting, "start").unwrap();
    mgr.transition(id, ServiceState::Running, "run").unwrap();
    assert_eq!(mgr.state(id).unwrap(), ServiceState::Running);

    // Pause
    mgr.transition(id, ServiceState::Paused, "pause").unwrap();
    assert_eq!(mgr.state(id).unwrap(), ServiceState::Paused);

    // Resume
    mgr.transition(id, ServiceState::Running, "resume").unwrap();
    assert_eq!(mgr.state(id).unwrap(), ServiceState::Running);

    // Stop
    mgr.transition(id, ServiceState::Stopping, "stop").unwrap();
    mgr.transition(id, ServiceState::Stopped, "stopped").unwrap();
    assert_eq!(mgr.state(id).unwrap(), ServiceState::Stopped);

    // Destroy
    mgr.transition(id, ServiceState::Destroyed, "destroy").unwrap();
    assert_eq!(mgr.state(id).unwrap(), ServiceState::Destroyed);

    // Verify full history
    let history = mgr.history(id);
    assert_eq!(history.len(), 8);
    assert_eq!(history[0].to, ServiceState::Registered);
    assert_eq!(history[7].to, ServiceState::Destroyed);
}

#[test]
fn resource_allocation_and_release() {
    let mgr = ResourceManager::new();
    mgr.register_pool(ResourceType::Cpu, 8);
    mgr.register_pool(ResourceType::Ram, 1024);
    mgr.register_pool(ResourceType::Gpu, 2);

    let consumer1 = ConsumerId::new();
    let consumer2 = ConsumerId::new();

    // Allocate from consumer 1
    let h1 = mgr.allocate(ResourceType::Cpu, 3, consumer1).unwrap();
    let h2 = mgr.allocate(ResourceType::Ram, 256, consumer1).unwrap();
    assert_eq!(mgr.available(ResourceType::Cpu), 5);
    assert_eq!(mgr.available(ResourceType::Ram), 768);

    // Allocate from consumer 2
    let h3 = mgr.allocate(ResourceType::Cpu, 4, consumer2).unwrap();
    assert_eq!(mgr.available(ResourceType::Cpu), 1);

    // Exceed available
    let result = mgr.allocate(ResourceType::Cpu, 2, consumer1);
    assert!(result.is_err());

    // Release
    mgr.release(h1).unwrap();
    mgr.release(h3).unwrap();
    assert_eq!(mgr.available(ResourceType::Cpu), 8);

    // Stats
    let stats = mgr.all_stats();
    assert_eq!(stats.len(), 3);
}

#[test]
fn scheduler_priority_execution_order() {
    let mut sched = TaskScheduler::new(SchedulerConfig::default());

    // Submit tasks in reverse priority order
    sched
        .submit(ScheduledTask::new("background", TaskPriority::Background))
        .unwrap();
    sched
        .submit(ScheduledTask::new("critical", TaskPriority::Critical))
        .unwrap();
    sched
        .submit(ScheduledTask::new("low", TaskPriority::Low))
        .unwrap();
    sched
        .submit(ScheduledTask::new("high", TaskPriority::High))
        .unwrap();
    sched
        .submit(ScheduledTask::new("normal", TaskPriority::Normal))
        .unwrap();

    // Should dequeue in priority order
    assert_eq!(sched.dequeue().unwrap().name, "critical");
    assert_eq!(sched.dequeue().unwrap().name, "high");
    assert_eq!(sched.dequeue().unwrap().name, "normal");
    assert_eq!(sched.dequeue().unwrap().name, "low");
    assert_eq!(sched.dequeue().unwrap().name, "background");
}

#[test]
fn event_bus_publish_subscribe() {
    let mut bus = EventBus::new(EventBusConfig::default());
    let mut rx = bus.receiver();

    // Publish events
    let event1 = Event::new("test.topic", serde_json::json!({"value": 1}), "source")
        .with_priority(EventPriority::Normal);
    let event2 = Event::new("test.topic", serde_json::json!({"value": 2}), "source")
        .with_priority(EventPriority::High);

    bus.publish(event1).unwrap();
    bus.publish(event2).unwrap();

    // Receive events
    let received1 = rx.try_recv().unwrap();
    assert_eq!(received1.topic, "test.topic");
    let received2 = rx.try_recv().unwrap();
    assert_eq!(received2.topic, "test.topic");

    // Statistics
    let stats = bus.statistics();
    assert_eq!(stats.events_published, 2);
}

#[test]
fn message_bus_topics_and_routing() {
    let bus = MessageBus::new(MessageBusConfig::default());

    // Register topics
    let mut rx_a = bus.register_topic("topic.a");
    let mut rx_b = bus.register_topic("topic.b");

    // Send messages
    let msg1 = Message::new("topic.a", vec![1, 2, 3]);
    let msg2 = Message::new("topic.b", vec![4, 5, 6]);
    let msg3 = Message::new("topic.a", vec![7, 8, 9]);

    bus.send(msg1).unwrap();
    bus.send(msg2).unwrap();
    bus.send(msg3).unwrap();

    // Verify topic isolation
    assert_eq!(rx_a.try_recv().unwrap().payload, vec![1, 2, 3]);
    assert_eq!(rx_a.try_recv().unwrap().payload, vec![7, 8, 9]);
    assert_eq!(rx_b.try_recv().unwrap().payload, vec![4, 5, 6]);
}

#[tokio::test]
async fn async_runtime_spawn_and_complete() {
    let rt = NeoAsyncRuntime::new(16).unwrap();
    let counter = Arc::new(AtomicUsize::new(0));

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let c = counter.clone();
            rt.spawn_tracked(async move {
                c.fetch_add(1, Ordering::SeqCst);
            })
        })
        .collect();

    for handle in handles {
        handle.await.unwrap();
    }

    assert_eq!(counter.load(Ordering::SeqCst), 10);
}

#[test]
fn health_monitor_full_cycle() {
    let monitor = HealthMonitor::new(HealthConfig::default());

    let id1 = monitor.register("database");
    let id2 = monitor.register("cache");
    let id3 = monitor.register("api");

    // All start as Unknown
    assert_eq!(monitor.status(id1).unwrap(), HealthStatus::Unknown);
    assert_eq!(monitor.status(id2).unwrap(), HealthStatus::Unknown);
    assert_eq!(monitor.status(id3).unwrap(), HealthStatus::Unknown);

    // Heartbeats
    monitor.heartbeat(id1, HealthStatus::Healthy, None);
    monitor.heartbeat(id2, HealthStatus::Healthy, None);
    monitor.heartbeat(id3, HealthStatus::Degraded, Some("high latency".to_string()));

    // Summary
    let summary = monitor.summary();
    assert_eq!(summary.get(&HealthStatus::Healthy), Some(&2));
    assert_eq!(summary.get(&HealthStatus::Degraded), Some(&1));
}

#[test]
fn plugin_loader_lifecycle() {
    let dir = tempfile::tempdir().unwrap();
    let loader = PluginLoader::new(
        dir.path().to_path_buf(),
        PluginSandboxConfig {
            enabled: false,
            ..PluginSandboxConfig::default()
        },
        PluginHotReload::default(),
    );

    // Register plugins
    for i in 0..5 {
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
    }

    // Load and start all plugins
    let plugins = loader.list();
    assert_eq!(plugins.len(), 5);

    for (id, _, _) in &plugins {
        loader.load(*id).unwrap();
        loader.initialize(*id).unwrap();
        loader.start(*id).unwrap();
    }

    // All should be running
    let running: Vec<_> = loader
        .list()
        .into_iter()
        .filter(|(_, _, s)| *s == PluginState::Running)
        .collect();
    assert_eq!(running.len(), 5);

    // Shutdown
    loader.shutdown();
    let stopped: Vec<_> = loader
        .list()
        .into_iter()
        .filter(|(_, _, s)| *s == PluginState::Stopped)
        .collect();
    assert_eq!(stopped.len(), 5);
}

#[test]
fn performance_monitor_comprehensive() {
    let monitor = PerformanceMonitor::new(PerformanceConfig::default());

    // Record various metrics
    for i in 0..100 {
        monitor.record_latency(i * 100);
        monitor.record_task(i as f64, i % 10 != 0);
    }

    monitor.update_cpu(45.0, 12.5);
    monitor.update_memory(16 * 1024 * 1024 * 1024, 8 * 1024 * 1024 * 1024, 512 * 1024 * 1024);
    monitor.update_gpu(75.0, 4_000_000_000, 8_000_000_000, 65.0);
    monitor.update_threads(16, 8);
    monitor.record_event(50);

    let snap = monitor.snapshot();
    assert_eq!(snap.latency.count, 100);
    assert!(snap.latency.avg_us > 0);
    assert!(snap.latency.p50_us > 0);
    assert!((snap.cpu.usage_percent - 45.0).abs() < f64::EPSILON);
    assert_eq!(snap.memory.total_bytes, 16 * 1024 * 1024 * 1024);
    assert!((snap.gpu.utilization_percent - 75.0).abs() < f64::EPSILON);
    assert_eq!(snap.threads.total_threads, 16);
    assert_eq!(snap.tasks.total_completed + snap.tasks.total_failed, 100);
    assert_eq!(snap.events.total_published, 50);
}

#[test]
fn version_constraint_comprehensive() {
    // Exact
    let c = VersionConstraint::Exact { major: 1, minor: 2, patch: 3 };
    assert!(c.matches(1, 2, 3));
    assert!(!c.matches(1, 2, 4));

    // AtLeast
    let c = VersionConstraint::AtLeast { major: 1, minor: 2, patch: 3 };
    assert!(c.matches(1, 2, 3));
    assert!(c.matches(2, 0, 0));
    assert!(!c.matches(1, 1, 0));

    // Compatible (^)
    let c = VersionConstraint::Compatible { major: 1, minor: 2, patch: 3 };
    assert!(c.matches(1, 2, 3));
    assert!(c.matches(1, 2, 5));
    assert!(c.matches(1, 3, 0));
    assert!(!c.matches(1, 1, 0));
    assert!(!c.matches(2, 0, 0));

    // Range
    let c = VersionConstraint::Range {
        min_major: 1, min_minor: 0, min_patch: 0,
        max_major: 2, max_minor: 0, max_patch: 0,
    };
    assert!(c.matches(1, 5, 3));
    assert!(c.matches(2, 0, 0));
    assert!(!c.matches(2, 0, 1));

    // Any
    assert!(VersionConstraint::Any.matches(99, 99, 99));
}

#[test]
fn memory_pool_lifecycle() {
    let pool = MemoryPool::new(1024, 8);
    assert_eq!(pool.total_blocks(), 8);
    assert_eq!(pool.available_blocks(), 8);
    assert_eq!(pool.allocated_blocks(), 0);
    assert!((pool.utilization() - 0.0).abs() < f64::EPSILON);

    // Allocate all blocks
    let mut blocks = Vec::new();
    for _ in 0..8 {
        blocks.push(pool.allocate().unwrap());
    }
    assert_eq!(pool.available_blocks(), 0);
    assert_eq!(pool.allocated_blocks(), 8);
    assert!((pool.utilization() - 1.0).abs() < f64::EPSILON);

    // Fail to allocate more
    assert!(pool.allocate().is_err());

    // Release half
    for block in blocks.drain(4..) {
        pool.release(block).unwrap();
    }
    assert_eq!(pool.available_blocks(), 4);
    assert_eq!(pool.allocated_blocks(), 4);
}

#[test]
fn runtime_pause_resume() {
    let rt = RuntimeManager::testing();

    let id1 = rt.register_service(ServiceRegistration {
        name: "svc1".to_string(),
        version: (1, 0, 0),
        dependencies: vec![],
        optional_dependencies: vec![],
        priority: 0,
    });

    let id2 = rt.register_service(ServiceRegistration {
        name: "svc2".to_string(),
        version: (1, 0, 0),
        dependencies: vec![],
        optional_dependencies: vec![],
        priority: 0,
    });

    rt.start().unwrap();
    assert_eq!(rt.state(), RuntimeState::Running);

    // Pause
    rt.pause().unwrap();
    assert_eq!(rt.state(), RuntimeState::Paused);
    assert_eq!(rt.lifecycle().state(id1).unwrap(), ServiceState::Paused);
    assert_eq!(rt.lifecycle().state(id2).unwrap(), ServiceState::Paused);

    // Resume
    rt.resume().unwrap();
    assert_eq!(rt.state(), RuntimeState::Running);
    assert_eq!(rt.lifecycle().state(id1).unwrap(), ServiceState::Running);
    assert_eq!(rt.lifecycle().state(id2).unwrap(), ServiceState::Running);

    rt.shutdown().unwrap();
}

#[test]
fn configuration_profiles() {
    let dev = RuntimeConfiguration::development();
    assert_eq!(dev.profile, RuntimeProfile::Development);

    let test = RuntimeConfiguration::testing();
    assert_eq!(test.profile, RuntimeProfile::Testing);
    assert!(!test.thread_pool.auto_scale);

    let prod = RuntimeConfiguration::production();
    assert_eq!(prod.profile, RuntimeProfile::Production);
    assert!(prod.thread_pool.auto_scale);
    assert!(prod.scheduler.max_concurrent_tasks >= 1024);
}

#[tokio::test]
async fn cancellation_token_propagation() {
    let parent = CancellationToken::new();
    let child1 = parent.child_token();
    let child2 = parent.child_token();

    assert!(!child1.is_cancelled());
    assert!(!child2.is_cancelled());

    parent.cancel();
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert!(child1.is_cancelled());
    assert!(child2.is_cancelled());
}

#[test]
fn scheduler_retry_mechanism() {
    let mut sched = TaskScheduler::new(SchedulerConfig {
        retry_base_delay_ms: 10,
        retry_max_delay_ms: 100,
        ..SchedulerConfig::default()
    });

    let task = ScheduledTask::new("retry-test", TaskPriority::Normal).with_max_retries(3);
    let id = sched.submit(task).unwrap();

    // First failure -> retry
    assert!(sched.fail(id, "error 1"));
    assert_eq!(sched.get_task(id).unwrap().retry_count, 1);
    assert_eq!(sched.get_task(id).unwrap().status, TaskStatus::Retrying);

    // Second failure -> retry
    assert!(sched.fail(id, "error 2"));
    assert_eq!(sched.get_task(id).unwrap().retry_count, 2);

    // Third failure -> retry
    assert!(sched.fail(id, "error 3"));
    assert_eq!(sched.get_task(id).unwrap().retry_count, 3);

    // Fourth failure -> no more retries
    assert!(!sched.fail(id, "error 4"));
    assert_eq!(sched.get_task(id).unwrap().status, TaskStatus::Failed);
}
