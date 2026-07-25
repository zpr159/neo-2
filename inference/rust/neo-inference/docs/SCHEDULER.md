# Inference Scheduler

## Overview

The `InferenceScheduler` manages the flow of inference requests into the engine. It provides priority-based queue management, dynamic batching, concurrency control, and monitoring. The scheduler sits between the API layer and the backend execution layer, ensuring fair resource allocation and preventing overload.

## Queue Management

### Priority Levels

Requests are assigned one of five priority levels, ordered from highest to lowest urgency:

| Priority | Value | Use Case |
|----------|-------|----------|
| `Critical` | 0 | Real-time interactive requests, safety-critical |
| `High` | 1 | User-facing chat, high-interactive workloads |
| `Normal` | 2 | Standard batch inference, default priority |
| `Low` | 3 | Background tasks, data processing |
| `Background` | 4 | Pre-processing, warm-up, non-urgent analytics |

Lower numeric values indicate higher priority. Within the same priority level, requests are served in FIFO order (by sequence number).

### Priority Queue Implementation

The scheduler uses a binary heap (min-heap by priority, then FIFO by sequence):

```rust
struct PriorityEntry {
    request: ScheduledRequest,
    sequence: u64,  // Monotonically increasing sequence number
}

impl Ord for PriorityEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.request.priority
            .cmp(&other.request.priority)
            .then_with(|| self.sequence.cmp(&other.sequence).reverse())
    }
}
```

### Submitting Requests

```rust
let scheduler = InferenceScheduler::new(SchedulerConfig::default());

let request = ScheduledRequest {
    request_id: "req-001".to_string(),
    model_id: "llama-3-8b".to_string(),
    priority: InferencePriority::High,
    submitted_at: Utc::now(),
    deadline_ms: Some(5000),          // 5-second deadline
    estimated_tokens: Some(256),      // Hint for batching
    device_preference: Some("cuda:0".to_string()),
    payload_bytes: 1024,
};

let accepted = scheduler.submit(request);
// Returns false if queue is full
```

### Dequeueing

```rust
// Single request
if let Some(request) = scheduler.dequeue() {
    // Process request
    scheduler.complete();
}

// Batch dequeue (for dynamic batching)
let batch = scheduler.dequeue_batch(32);
// Returns up to 32 requests, limited by available concurrency slots
```

### Cancellation

```rust
let cancelled = scheduler.cancel("req-001");
// Returns true if the request was found and removed from the queue
```

## Dynamic Batching

Dynamic batching groups multiple requests into a single batch for efficient GPU utilization.

### Configuration

```rust
let config = SchedulerConfig {
    max_queue_size: 4096,          // Maximum queued requests
    max_concurrent: 64,            // Maximum concurrent inferences
    gpu_max_concurrent: 8,         // GPU-specific concurrency limit
    batch_timeout_ms: 100,         // Max wait time before flushing batch
    max_batch_size: 32,            // Maximum requests per batch
    enable_dynamic_batching: true, // Enable batching
    enable_priority_scheduling: true,
    worker_threads: 4,
};
```

### Batch Scheduler

The `BatchScheduler` works alongside the `InferenceScheduler`:

```rust
use std::time::Duration;

let batch_scheduler = BatchScheduler::new(
    32,                          // max batch size
    Duration::from_millis(100),  // flush timeout
);

// Add items to pending queue
batch_scheduler.add(BatchItem::new(request_id, model_id, input));

// Check if batch should be flushed
if batch_scheduler.should_flush() {
    let batch = batch_scheduler.drain_batch();
    // Process batch of requests together
}

// should_flush() returns true when:
//   1. pending_count >= max_batch_size, OR
//   2. oldest item age >= timeout
```

### Batching Flow

```
Request arrives
    │
    ▼
┌──────────────────┐
│ Submit to queue   │
│ (priority-based)  │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐     ┌──────────────────┐
│ Wait for batch    │────►│ Timeout reached   │
│ conditions:       │     │ OR batch full     │
│ - Enough requests │     └────────┬─────────┘
│ - Timeout         │              │
└──────────────────┘              ▼
                          ┌──────────────────┐
                          │ Drain batch       │
                          │ (up to max_size)  │
                          └────────┬─────────┘
                                   │
                                   ▼
                          ┌──────────────────┐
                          │ Execute batch on  │
                          │ backend           │
                          └────────┬─────────┘
                                   │
                                   ▼
                          ┌──────────────────┐
                          │ Complete all      │
                          │ requests in batch │
                          └──────────────────┘
```

## GPU Scheduling

### Concurrency Control

The scheduler enforces two concurrency limits:

- **`max_concurrent`** — Total concurrent inferences across all backends
- **`gpu_max_concurrent`** — Concurrency limit for GPU backends specifically

```rust
let available = scheduler.available_slots();
// max_concurrent - active_count

if available > 0 {
    let batch = scheduler.dequeue_batch(available);
}
```

### Multi-Device Scheduling

For multi-GPU setups, the `MultiGpuManager` coordinates device assignment:

```rust
let gpu_manager = MultiGpuManager::new(devices);

// Select the best device for a model
let device = gpu_manager.select_best_device(required_memory);

// Create parallelism plans
if let Some(plan) = gpu_manager.create_tensor_parallel_plan(num_layers, layer_memory) {
    // Distribute model across GPUs
}

if let Some(plan) = gpu_manager.create_pipeline_parallel_plan(num_layers, layer_memory, batch_size) {
    // Pipeline parallelism across GPUs
}
```

### Device Assignment

```rust
pub struct DeviceAssignment {
    pub device_id: u32,
    pub device_type: DeviceType,
    pub role: DeviceRole,          // Primary, Secondary, Worker
    pub layers: Vec<u32>,          // Which layers are on this device
    pub memory_budget: u64,        // Memory available on this device
    pub compute_weight: f64,       // Relative compute capacity
}
```

## Load Balancing Across Backends

### Backend-Level Load Balancing

The engine distributes requests across backends based on:

1. **Format compatibility** — Only backends that support the model's format receive requests
2. **Priority-based selection** — Highest-priority available backend gets the request
3. **Availability** — Unavailable backends are skipped

### Distributed Load Balancing

For distributed inference across multiple nodes:

```rust
let dist_config = DistributedConfig {
    load_balance_strategy: LoadBalanceStrategy::LeastLoaded,
    max_workers: 256,
    enable_fault_tolerance: true,
    max_retries: 3,
    retry_delay_ms: 1000,
    ..Default::default()
};

let dist_manager = DistributedInferenceManager::new(dist_config);

// Select worker
if let Some(worker) = dist_manager.select_worker() {
    // Forward request to worker
}
```

### Load Balance Strategies

| Strategy | Description |
|----------|-------------|
| `RoundRobin` | Cyclically distributes requests across workers |
| `LeastLoaded` | Sends to the worker with fewest completed tasks |
| `LeastLatency` | Sends to the worker with lowest average latency |
| `WeightedRandom` | Random selection weighted by capacity |
| `ConsistentHash` | Hash-based routing for request affinity |

## Parallel Execution Patterns

### Tensor Parallelism

Splits individual layers across multiple GPUs:

```
Layer 0:  [GPU 0 | GPU 1 | GPU 2 | GPU 3]  → AllReduce
Layer 1:  [GPU 0 | GPU 1 | GPU 2 | GPU 3]  → AllReduce
...
Layer N:  [GPU 0 | GPU 1 | GPU 2 | GPU 3]  → AllReduce
```

### Pipeline Parallelism

Distributes layers across GPUs in stages:

```
GPU 0: Layers 0–7    → GPU 1: Layers 8–15  → GPU 2: Layers 16–23  → GPU 3: Layers 24–31
         ▲                    ▲                     ▲                     ▲
    micro-batch 1        micro-batch 2         micro-batch 3        micro-batch 4
```

### Expert Parallelism

For Mixture-of-Experts models, distributes expert layers across devices:

```
GPU 0: Experts 0–3    GPU 1: Experts 4–7    GPU 2: Experts 8–11   GPU 3: Experts 12–15
```

### Sequence Parallelism

Splits long sequences across devices for memory-efficient processing:

```
Sequence [0 ──────────────── 4095] split into:
  GPU 0: [0 ───────── 1023]
  GPU 1: [1024 ────── 2047]
  GPU 2: [2048 ────── 3071]
  GPU 3: [3072 ────── 4095]
```

## Scheduler Statistics and Monitoring

### Real-Time Statistics

```rust
let stats = scheduler.statistics();

println!("Queue depth:      {}", stats.queue_length);
println!("Active requests:  {}", stats.active_count);
println!("Total submitted:  {}", stats.total_submitted);
println!("Total completed:  {}", stats.total_completed);
println!("Total dropped:    {}", stats.total_dropped);
```

### SchedulerStatistics

```rust
pub struct SchedulerStatistics {
    pub queue_length: usize,      // Requests waiting in queue
    pub active_count: usize,      // Requests currently being processed
    pub total_submitted: u64,     // All-time submitted count
    pub total_completed: u64,     // All-time completed count
    pub total_dropped: u64,       // All-time dropped (queue full) count
}
```

### Engine-Level Monitoring

```rust
// From the InferenceEngine
let stats = engine.scheduler_stats();
let telemetry = engine.telemetry_snapshot();
let active = engine.active_requests();

// Telemetry includes:
// - Latency metrics (p50, p90, p95, p99, max, mean)
// - Throughput metrics (requests/sec, tokens/sec)
// - GPU metrics (utilization, memory, temperature)
// - Backend statistics (per-backend inference counts, latencies)
```
