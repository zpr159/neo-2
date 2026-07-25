//! Benchmark tests for the Neo Runtime system.

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use neo_runtime::config::*;
use neo_runtime::dependency::*;
use neo_runtime::lifecycle::*;
use neo_runtime::resource::*;
use neo_runtime::scheduler::*;
use neo_runtime::*;

fn benchmark_lifecycle_transitions(c: &mut Criterion) {
    let mut group = c.benchmark_group("lifecycle_transitions");

    for count in [10, 100, 1000] {
        group.bench_with_input(
            BenchmarkId::new("register_and_transition", count),
            &count,
            |b, &count| {
                b.iter(|| {
                    let mgr = LifecycleManager::new();
                    let ids: Vec<ServiceId> = (0..count)
                        .map(|_| mgr.register_service("svc"))
                        .collect();
                    for id in &ids {
                        mgr.transition(*id, ServiceState::Registered, "reg")
                            .unwrap();
                        mgr.transition(*id, ServiceState::Initialized, "init")
                            .unwrap();
                        mgr.transition(*id, ServiceState::Starting, "start")
                            .unwrap();
                        mgr.transition(*id, ServiceState::Running, "run")
                            .unwrap();
                    }
                });
            },
        );
    }
    group.finish();
}

fn benchmark_dependency_resolution(c: &mut Criterion) {
    let mut group = c.benchmark_group("dependency_resolution");

    for count in [10, 50, 200] {
        group.bench_with_input(
            BenchmarkId::new("topological_sort", count),
            &count,
            |b, &count| {
                b.iter_batched(
                    || {
                        let mut graph = DependencyGraph::new();
                        let ids: Vec<ServiceId> = (0..count).map(|_| ServiceId::new()).collect();
                        for (i, &id) in ids.iter().enumerate() {
                            graph.add_node(id, format!("svc-{}", i), (1, 0, 0));
                        }
                        for i in 0..count.saturating_sub(1) {
                            graph.add_dependency(
                                ids[i],
                                Dependency {
                                    service_id: ids[i + 1],
                                    service_name: format!("svc-{}", i + 1),
                                    version_constraint: VersionConstraint::Any,
                                    optional: false,
                                },
                            );
                        }
                        graph
                    },
                    |mut graph| {
                        let _ = graph.topological_sort();
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish();
}

fn benchmark_resource_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("resource_allocation");

    group.bench_function("allocate_release_1000", |b| {
        b.iter(|| {
            let mgr = ResourceManager::new();
            mgr.register_pool(ResourceType::Cpu, 1000);
            let consumer = ConsumerId::new();
            let mut handles = Vec::new();
            for _ in 0..500 {
                if let Ok(h) = mgr.allocate(ResourceType::Cpu, 1, consumer) {
                    handles.push(h);
                }
            }
            for h in handles {
                mgr.release(h).unwrap();
            }
        });
    });
    group.finish();
}

fn benchmark_scheduler_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("scheduler_throughput");

    group.bench_function("submit_dequeue_1000", |b| {
        b.iter(|| {
            let mut sched = TaskScheduler::new(SchedulerConfig::default());
            for i in 0..1000 {
                sched
                    .submit(ScheduledTask::new(
                        format!("task-{}", i),
                        TaskPriority::Normal,
                    ))
                    .unwrap();
            }
            while sched.dequeue().is_some() {}
        });
    });
    group.finish();
}

fn benchmark_version_constraint(c: &mut Criterion) {
    let mut group = c.benchmark_group("version_constraint");

    group.bench_function("check_10000", |b| {
        let constraint = VersionConstraint::Compatible {
            major: 1,
            minor: 0,
            patch: 0,
        };
        b.iter(|| {
            for i in 0..100 {
                for j in 0..100 {
                    let _ = constraint.matches(i % 5, j % 10, 0);
                }
            }
        });
    });
    group.finish();
}

fn benchmark_event_bus_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("event_bus_throughput");

    group.bench_function("publish_1000", |b| {
        b.iter(|| {
            let mut bus = EventBus::new(EventBusConfig {
                broadcast_capacity: 4096,
                ..EventBusConfig::default()
            });
            for _ in 0..1000 {
                let event = Event::new("topic", serde_json::json!(42), "src");
                let _ = bus.publish(event);
            }
        });
    });
    group.finish();
}

fn benchmark_memory_pool(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_pool");

    group.bench_function("alloc_release_1000", |b| {
        b.iter(|| {
            let pool = MemoryPool::new(64, 1000);
            let mut blocks = Vec::new();
            for _ in 0..500 {
                blocks.push(pool.allocate().unwrap());
            }
            for b in blocks {
                pool.release(b).unwrap();
            }
        });
    });
    group.finish();
}

fn benchmark_latency_histogram(c: &mut Criterion) {
    let mut group = c.benchmark_group("latency_histogram");

    group.bench_function("record_10000", |b| {
        b.iter(|| {
            let mut hist = performance::LatencyHistogram::new(64);
            for i in 0..10000 {
                hist.record(i * 10);
            }
            let _ = hist.p50();
            let _ = hist.p95();
            let _ = hist.p99();
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    benchmark_lifecycle_transitions,
    benchmark_dependency_resolution,
    benchmark_resource_allocation,
    benchmark_scheduler_throughput,
    benchmark_version_constraint,
    benchmark_event_bus_throughput,
    benchmark_memory_pool,
    benchmark_latency_histogram,
);

criterion_main!(benches);
