use neo_inference::telemetry::{InferenceTelemetry, TelemetryConfig};

#[test]
fn test_record_request_start_increments_count() {
    let telemetry = InferenceTelemetry::new(TelemetryConfig::default());
    assert_eq!(telemetry.active_requests(), 0);

    telemetry.record_request_start();
    assert_eq!(telemetry.active_requests(), 1);

    telemetry.record_request_start();
    assert_eq!(telemetry.active_requests(), 2);
}

#[test]
fn test_record_request_complete_decrements_active() {
    let telemetry = InferenceTelemetry::new(TelemetryConfig::default());
    telemetry.record_request_start();
    telemetry.record_request_start();
    assert_eq!(telemetry.active_requests(), 2);

    telemetry.record_request_complete(10.0, true, 100, 50, 50);
    assert_eq!(telemetry.active_requests(), 1);

    telemetry.record_request_complete(20.0, true, 200, 100, 100);
    assert_eq!(telemetry.active_requests(), 0);
}

#[test]
fn test_snapshot_contains_valid_data() {
    let telemetry = InferenceTelemetry::new(TelemetryConfig::default());
    telemetry.record_request_start();
    telemetry.record_request_complete(10.0, true, 100, 50, 50);
    telemetry.record_request_start();
    telemetry.record_request_complete(20.0, false, 0, 0, 0);

    let snapshot = telemetry.snapshot();
    assert_eq!(snapshot.total_requests, 2);
    assert_eq!(snapshot.active_requests, 0);
}

#[test]
fn test_latency_percentiles() {
    let telemetry = InferenceTelemetry::new(TelemetryConfig::default());
    for i in 0..100 {
        telemetry.record_request_start();
        telemetry.record_request_complete(i as f64, true, 1, 1, 1);
    }

    let snapshot = telemetry.snapshot();
    assert!(snapshot.latency.p50_ms >= 0.0);
    assert!(snapshot.latency.p90_ms >= snapshot.latency.p50_ms);
    assert!(snapshot.latency.p99_ms >= snapshot.latency.p90_ms);
    assert!(snapshot.latency.max_ms > 0.0);
    assert!(snapshot.latency.mean_ms >= 0.0);
}

#[test]
fn test_throughput_metrics() {
    let telemetry = InferenceTelemetry::new(TelemetryConfig::default());
    for _ in 0..10 {
        telemetry.record_request_start();
        telemetry.record_request_complete(5.0, true, 50, 25, 25);
    }

    let snapshot = telemetry.snapshot();
    assert!(snapshot.throughput.requests_per_second >= 0.0);
    assert!(snapshot.throughput.tokens_per_second >= 0.0);
}

#[test]
fn test_failed_requests_tracking() {
    let telemetry = InferenceTelemetry::new(TelemetryConfig::default());
    telemetry.record_request_start();
    telemetry.record_request_complete(10.0, false, 0, 0, 0);
    telemetry.record_request_start();
    telemetry.record_request_complete(10.0, true, 10, 5, 5);

    let snapshot = telemetry.snapshot();
    assert_eq!(snapshot.total_requests, 2);
}

#[test]
fn test_uptime() {
    let telemetry = InferenceTelemetry::new(TelemetryConfig::default());
    let uptime = telemetry.uptime_seconds();
    assert!(uptime < 2);
}

#[test]
fn test_concurrent_request_tracking() {
    let telemetry = InferenceTelemetry::new(TelemetryConfig::default());
    for _ in 0..5 {
        telemetry.record_request_start();
    }
    assert_eq!(telemetry.active_requests(), 5);

    for _ in 0..3 {
        telemetry.record_request_complete(5.0, true, 10, 5, 5);
    }
    assert_eq!(telemetry.active_requests(), 2);
}

#[test]
fn test_snapshot_initial_state() {
    let telemetry = InferenceTelemetry::new(TelemetryConfig::default());
    let snapshot = telemetry.snapshot();
    assert_eq!(snapshot.total_requests, 0);
    assert_eq!(snapshot.active_requests, 0);
    assert_eq!(snapshot.latency.p50_ms, 0.0);
}

#[test]
fn test_max_latency_tracked() {
    let telemetry = InferenceTelemetry::new(TelemetryConfig::default());
    telemetry.record_request_start();
    telemetry.record_request_complete(100.0, true, 10, 5, 5);

    let snapshot = telemetry.snapshot();
    assert!(snapshot.latency.max_ms >= 100.0);
}

#[test]
fn test_token_tracking() {
    let telemetry = InferenceTelemetry::new(TelemetryConfig::default());
    telemetry.record_request_start();
    telemetry.record_request_complete(5.0, true, 100, 40, 60);
    telemetry.record_request_start();
    telemetry.record_request_complete(5.0, true, 200, 80, 120);

    let snapshot = telemetry.snapshot();
    assert!(snapshot.throughput.tokens_per_second >= 0.0);
}
