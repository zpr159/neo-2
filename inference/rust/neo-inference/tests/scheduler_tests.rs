use neo_inference::scheduler::{
    InferencePriority, InferenceScheduler, ScheduledRequest, SchedulerConfig,
};
use chrono::Utc;

fn make_request(id: &str, priority: InferencePriority) -> ScheduledRequest {
    ScheduledRequest {
        request_id: id.to_string(),
        model_id: "model-1".to_string(),
        priority,
        submitted_at: Utc::now(),
        deadline_ms: None,
        estimated_tokens: Some(128),
        device_preference: None,
        payload_bytes: 1024,
    }
}

#[test]
fn test_submit_and_dequeue() {
    let scheduler = InferenceScheduler::new(SchedulerConfig::default());
    let req = make_request("req-1", InferencePriority::Normal);
    assert!(scheduler.submit(req));
    assert_eq!(scheduler.queue_len(), 1);

    let dequeued = scheduler.dequeue().unwrap();
    assert_eq!(dequeued.request_id, "req-1");
    assert_eq!(scheduler.queue_len(), 0);
    assert_eq!(scheduler.active_count(), 1);
}

#[test]
fn test_priority_ordering() {
    let scheduler = InferenceScheduler::new(SchedulerConfig::default());
    scheduler.submit(make_request("background", InferencePriority::Background));
    scheduler.submit(make_request("normal", InferencePriority::Normal));
    scheduler.submit(make_request("critical", InferencePriority::Critical));

    let first = scheduler.dequeue().unwrap();
    assert_eq!(first.request_id, "critical");

    let second = scheduler.dequeue().unwrap();
    assert_eq!(second.request_id, "normal");

    let third = scheduler.dequeue().unwrap();
    assert_eq!(third.request_id, "background");
}

#[test]
fn test_queue_full() {
    let config = SchedulerConfig {
        max_queue_size: 2,
        ..Default::default()
    };
    let scheduler = InferenceScheduler::new(config);
    assert!(scheduler.submit(make_request("a", InferencePriority::Normal)));
    assert!(scheduler.submit(make_request("b", InferencePriority::Normal)));
    assert!(!scheduler.submit(make_request("c", InferencePriority::Normal)));

    let stats = scheduler.statistics();
    assert_eq!(stats.total_dropped, 1);
}

#[test]
fn test_dequeue_batch() {
    let config = SchedulerConfig {
        max_concurrent: 4,
        ..Default::default()
    };
    let scheduler = InferenceScheduler::new(config);
    scheduler.submit(make_request("a", InferencePriority::Normal));
    scheduler.submit(make_request("b", InferencePriority::Normal));
    scheduler.submit(make_request("c", InferencePriority::Normal));

    let batch = scheduler.dequeue_batch(2);
    assert_eq!(batch.len(), 2);
    assert_eq!(scheduler.active_count(), 2);
    assert_eq!(scheduler.queue_len(), 1);
}

#[test]
fn test_complete_tracking() {
    let scheduler = InferenceScheduler::new(SchedulerConfig::default());
    scheduler.submit(make_request("r1", InferencePriority::Normal));
    scheduler.dequeue();
    scheduler.complete();

    let stats = scheduler.statistics();
    assert_eq!(stats.total_completed, 1);
    assert_eq!(stats.active_count, 0);
}

#[test]
fn test_cancel() {
    let scheduler = InferenceScheduler::new(SchedulerConfig::default());
    scheduler.submit(make_request("cancel-me", InferencePriority::Normal));
    scheduler.submit(make_request("keep-me", InferencePriority::Normal));
    assert_eq!(scheduler.queue_len(), 2);

    let cancelled = scheduler.cancel("cancel-me");
    assert!(cancelled);
    assert_eq!(scheduler.queue_len(), 1);

    let remaining = scheduler.dequeue().unwrap();
    assert_eq!(remaining.request_id, "keep-me");
}

#[test]
fn test_cancel_nonexistent() {
    let scheduler = InferenceScheduler::new(SchedulerConfig::default());
    scheduler.submit(make_request("a", InferencePriority::Normal));
    assert!(!scheduler.cancel("does-not-exist"));
}

#[test]
fn test_statistics() {
    let scheduler = InferenceScheduler::new(SchedulerConfig::default());
    let stats = scheduler.statistics();
    assert_eq!(stats.queue_length, 0);
    assert_eq!(stats.active_count, 0);
    assert_eq!(stats.total_submitted, 0);
    assert_eq!(stats.total_completed, 0);
    assert_eq!(stats.total_dropped, 0);

    scheduler.submit(make_request("r1", InferencePriority::Normal));
    scheduler.submit(make_request("r2", InferencePriority::Critical));
    let stats = scheduler.statistics();
    assert_eq!(stats.total_submitted, 2);
    assert_eq!(stats.queue_length, 2);
}

#[test]
fn test_dequeue_empty_returns_none() {
    let scheduler = InferenceScheduler::new(SchedulerConfig::default());
    assert!(scheduler.dequeue().is_none());
}

#[test]
fn test_max_concurrent_limit() {
    let config = SchedulerConfig {
        max_concurrent: 1,
        ..Default::default()
    };
    let scheduler = InferenceScheduler::new(config);
    scheduler.submit(make_request("a", InferencePriority::Normal));
    scheduler.submit(make_request("b", InferencePriority::Normal));

    scheduler.dequeue();
    assert_eq!(scheduler.active_count(), 1);
    assert!(scheduler.dequeue().is_none());
    assert_eq!(scheduler.queue_len(), 1);
}

#[test]
fn test_available_slots() {
    let config = SchedulerConfig {
        max_concurrent: 3,
        ..Default::default()
    };
    let scheduler = InferenceScheduler::new(config);
    assert_eq!(scheduler.available_slots(), 3);

    scheduler.submit(make_request("a", InferencePriority::Normal));
    scheduler.dequeue();
    assert_eq!(scheduler.available_slots(), 2);
}

#[test]
fn test_same_priority_fifo_order() {
    let scheduler = InferenceScheduler::new(SchedulerConfig::default());
    scheduler.submit(make_request("first", InferencePriority::Normal));
    scheduler.submit(make_request("second", InferencePriority::Normal));
    scheduler.submit(make_request("third", InferencePriority::Normal));

    let a = scheduler.dequeue().unwrap();
    let b = scheduler.dequeue().unwrap();
    let c = scheduler.dequeue().unwrap();
    assert_eq!(a.request_id, "first");
    assert_eq!(b.request_id, "second");
    assert_eq!(c.request_id, "third");
}

#[test]
fn test_all_priorities() {
    let scheduler = InferenceScheduler::new(SchedulerConfig::default());
    scheduler.submit(make_request("high", InferencePriority::High));
    scheduler.submit(make_request("low", InferencePriority::Low));
    scheduler.submit(make_request("critical", InferencePriority::Critical));
    scheduler.submit(make_request("background", InferencePriority::Background));
    scheduler.submit(make_request("normal", InferencePriority::Normal));

    let order: Vec<_> = (0..5)
        .map(|_| scheduler.dequeue().unwrap().request_id)
        .collect();
    assert_eq!(
        order,
        vec!["critical", "high", "normal", "low", "background"]
    );
}

#[test]
fn test_multiple_cancel() {
    let scheduler = InferenceScheduler::new(SchedulerConfig::default());
    for i in 0..5 {
        scheduler.submit(make_request(&format!("req-{}", i), InferencePriority::Normal));
    }
    scheduler.cancel("req-0");
    scheduler.cancel("req-2");
    scheduler.cancel("req-4");
    assert_eq!(scheduler.queue_len(), 2);
}
