# Neo AGI OS — Architecture Overview

## Table of Contents

- [1. Introduction](#1-introduction)
- [2. Design Principles](#2-design-principles)
- [3. System Layers](#3-system-layers)
- [4. Component Interaction Model](#4-component-interaction-model)
- [5. Data Flow](#5-data-flow)
- [6. Communication Protocols](#6-communication-protocols)
- [7. Deployment Topology](#7-deployment-topology)
- [8. Security Architecture](#8-security-architecture)
- [9. Scalability Model](#9-scalability-model)
- [10. Fault Tolerance](#10-fault-tolerance)

---

## 1. Introduction

Neo AGI OS is a polyglot, modular operating system designed to orchestrate artificial general intelligence workloads across heterogeneous compute resources. It provides a unified abstraction layer over neural processing, robotic control, knowledge management, and multi-agent coordination.

The system is built as a monorepo containing components in Rust, C++, Python, TypeScript, Go, Kotlin, and Swift. Each language is chosen for its specific strengths in the subsystem it serves.

### High-Level System Map

```
+=====================================================================+
|                        NEO AGI OS                                    |
+=====================================================================+
|                                                                     |
|  +-------------------+  +-------------------+  +-----------------+  |
|  |   Neural Core     |  |  Agent Scheduler  |  | Knowledge Graph |  |
|  |   (Rust/C++)      |  |  (Rust)           |  | (Rust/Python)   |  |
|  +--------+----------+  +--------+----------+  +--------+--------+  |
|           |                      |                      |            |
|  +--------v----------------------v----------------------v--------+  |
|  |                     Message Bus (gRPC/NATS)                   |  |
|  +--------+----------------------^----------------------^--------+  |
|           |                      |                      |            |
|  +--------v----------+  +--------v----------+  +--------v--------+  |
|  |  Robotics Control  |  |   API Gateway     |  |  Web Dashboard  |  |
|  |  (Rust/C++/Kotlin) |  |  (Go/TypeScript)  |  | (TypeScript)    |  |
|  +-------------------+  +-------------------+  +-----------------+  |
|                                                                     |
|  +-------------------+  +-------------------+  +-----------------+  |
|  |  Device Manager   |  |  Storage Engine   |  |  Observability  |  |
|  |  (Go)             |  |  (Rust)           |  |  (Rust/Python)  |  |
|  +-------------------+  +-------------------+  +-----------------+  |
+=====================================================================+
```

---

## 2. Design Principles

### 2.1 Zero-Copy Where Possible

The neural core and storage engine use zero-copy deserialization to minimize latency in hot paths. Messages passed through the bus are backed by memory-mapped regions when the payload exceeds 4KB.

### 2.2 Language Per Subsystem

Each subsystem is implemented in the language best suited to its constraints:

| Subsystem       | Language(s)     | Rationale                                      |
|----------------|-----------------|------------------------------------------------|
| Neural Core    | Rust, C++       | Zero-cost abstractions, CUDA interop            |
| Agent Scheduler| Rust            | Deterministic scheduling, memory safety         |
| Knowledge Graph| Rust, Python    | Graph algorithms in Rust, ML bindings in Python |
| API Gateway    | Go, TypeScript  | High concurrency in Go, ecosystem in TS         |
| Robotics       | Rust, C++, Kotlin | Real-time in Rust/C++, JVM integration       |
| Web Dashboard  | TypeScript      | Browser-native, React ecosystem                 |
| Storage Engine | Rust            | Custom LSM-tree, crash safety                   |
| Observability  | Rust, Python    | Metrics pipeline in Rust, analysis in Python    |

### 2.3 Fail-Fast, Recover-Gracefully

Components fail fast on invariant violations (debug asserts, precondition checks). Recovery is handled at the orchestration layer through circuit breakers and supervisor trees.

### 2.4 Typed Interfaces Across Boundaries

All inter-language communication uses Protocol Buffers or Cap'n Proto schemas. No raw JSON crosses language boundaries in performance-critical paths.

---

## 3. System Layers

The architecture is organized into five distinct layers, each with clear responsibilities and well-defined interfaces.

```
Layer 5: Presentation Layer
  +-- Web Dashboard (TypeScript/React)
  +-- Mobile Clients (Swift/Kotlin)
  +-- CLI Tools (Rust/Go)
  +-- SDK Libraries (Kotlin, Swift, TypeScript, Python)

Layer 4: API Gateway Layer
  +-- REST API (Go)
  +-- WebSocket Server (Go)
  +-- gRPC Services (Rust)
  +-- Authentication & Authorization

Layer 3: Orchestration Layer
  +-- Agent Scheduler (Rust)
  +-- Task Queue (Rust)
  +-- Workflow Engine (Rust)
  +-- Message Bus (gRPC/NATS)

Layer 2: Core Intelligence Layer
  +-- Neural Core (Rust/C++)
  +-- Knowledge Graph (Rust/Python)
  +-- Reasoning Engine (Rust)
  +-- Memory Manager (Rust)

Layer 1: Infrastructure Layer
  +-- Storage Engine (Rust)
  +-- Device Manager (Go)
  +-- Robotics Control (Rust/C++/Kotlin)
  +-- Observability Pipeline (Rust/Python)
```

### Layer Descriptions

**Presentation Layer** provides all user-facing interfaces. The web dashboard is built with TypeScript and React, communicating with the API Gateway over WebSocket and REST. Mobile clients in Swift (iOS) and Kotlin (Android) use the SDK libraries. CLI tools are written in Rust for performance and in Go for scripting convenience.

**API Gateway Layer** is the single entry point for all external traffic. It handles authentication via JWT tokens, rate limiting, request routing, and response aggregation. The Go server handles HTTP/1.1 and HTTP/2 traffic. gRPC services handle machine-to-machine communication.

**Orchestration Layer** manages the lifecycle of agents and tasks. The agent scheduler assigns tasks to available agents based on capability matching, resource availability, and priority. The task queue provides durable, ordered delivery with at-least-once semantics. The workflow engine chains multi-step operations with compensation logic.

**Core Intelligence Layer** contains the neural processing pipeline, knowledge graph, reasoning engine, and memory manager. This is where the AGI capabilities live. The neural core interfaces with CUDA for GPU acceleration. The knowledge graph stores entities and relationships with temporal versioning.

**Infrastructure Layer** provides foundational services: persistent storage via a custom LSM-tree engine, device management for IoT and robotic hardware, real-time robotic control with safety guarantees, and observability through distributed tracing, metrics collection, and log aggregation.

---

## 4. Component Interaction Model

### 4.1 Synchronous Interactions

Synchronous calls follow a request-response pattern over gRPC. They are used for health checks, configuration queries, and low-latency operations where the caller blocks until a response is received.

```
Client                API Gateway           Agent Scheduler
  |                        |                        |
  |--- CreateAgent() ----->|                        |
  |                        |--- CreateAgent() ----->|
  |                        |<-- AgentHandle --------|
  |<-- AgentHandle --------|                        |
```

### 4.2 Asynchronous Interactions

Asynchronous interactions use the message bus (NATS or gRPC streaming). They are used for task submission, event notification, and inter-component communication where immediate response is not required.

```
API Gateway          Message Bus          Neural Core
  |                     |                     |
  |--- SubmitTask ----->|                     |
  |<-- TaskPending -----|                     |
  |                     |--- ProcessTask --->|
  |                     |<-- TaskComplete ---|
  |                     |--- Notify -------->|
  |<-- TaskComplete ----|                     |
```

### 4.3 Event-Driven Interactions

The system emits events for state changes, errors, and milestones. Observability components subscribe to these events for tracing and alerting.

```
Agent Scheduler        Event Bus         Observability
  |                      |                    |
  |--- AgentStarted ---->|                    |
  |                      |--- SpanStart ---->|
  |                      |                    |
  |--- TaskCompleted --->|                    |
  |                      |--- MetricUpdate ->|
  |                      |                    |
  |--- AgentFailed ----->|                    |
  |                      |--- AlertFire ---->|
```

---

## 5. Data Flow

### 5.1 Neural Processing Pipeline

Input data flows through the following stages:

```
[Input Source] --> [Preprocessing] --> [Neural Core] --> [Postprocessing] --> [Output Sink]
                      |                    |                  |
                      v                    v                  v
                 [Validation]        [GPU Kernel]       [Result Cache]
                 [Normalization]     [Batch Processing] [Persistence]
                 [Augmentation]      [Gradient Check]   [Event Emit]
```

1. **Input Source**: Data arrives from API Gateway, device sensors, or other agents.
2. **Preprocessing**: Input is validated, normalized, and optionally augmented.
3. **Neural Core**: The core dispatches work to GPU kernels, manages batching, and performs gradient checks during training.
4. **Postprocessing**: Results are validated, cached, and persisted.
5. **Output Sink**: Results are sent back through the message bus to the requesting component.

### 5.2 Agent Lifecycle

```
[Created] --> [Initializing] --> [Ready] --> [Processing] --> [Completed]
    |              |                |            |                |
    v              v                v            v                v
 [Failed]     [Failed]          [Idle]      [Failed]         [Archived]
                                 [Paused]
```

### 5.3 Knowledge Graph Query Flow

```
[Query] --> [Parse] --> [Plan] --> [Execute] --> [Merge] --> [Response]
              |           |           |             |
              v           v           v             v
          [Validate]  [Optimize]  [Traverse]   [Deduplicate]
          [Rewrite]   [Index]     [Filter]     [Rank]
```

---

## 6. Communication Protocols

### 6.1 Protocol Matrix

| Protocol     | Use Case                          | Port  | Serialization  |
|-------------|-----------------------------------|-------|----------------|
| gRPC        | Internal service-to-service       | varies| Protobuf       |
| HTTP/2      | External API access               | 443   | JSON/Protobuf  |
| WebSocket   | Real-time dashboard updates       | 443   | JSON           |
| NATS        | Event bus, pub/sub messaging      | 4222  | MessagePack    |
| Custom UDP  | Low-latency robotic control       | 5000  | FlatBuffers    |
| Shared Mem  | Inter-process neural data transfer| N/A  | Raw bytes      |

### 6.2 gRPC Service Definitions

All gRPC services are defined in `proto/` directory:

```
proto/
  +-- neo/
      +-- agent/
      |   +-- scheduler.proto
      |   +-- lifecycle.proto
      +-- neural/
      |   +-- inference.proto
      |   +-- training.proto
      +-- knowledge/
      |   +-- graph.proto
      |   +-- query.proto
      +-- robotics/
          +-- control.proto
          +-- sensor.proto
```

### 6.3 Message Bus Topics

```
neo.agents.{agent_id}.events     -- Agent lifecycle events
neo.tasks.{task_id}.status       -- Task status updates
neo.neural.inference.{model_id}  -- Inference request/response
neo.neural.training.{model_id}   -- Training progress events
neo.knowledge.graph.mutations    -- Knowledge graph change events
neo.robotics.{device_id}.telemetry -- Device telemetry data
neo.system.health                -- System-wide health events
neo.system.alerts                -- System-wide alerts
```

---

## 7. Deployment Topology

### 7.1 Single-Node Development

```
+--------------------------------------------------+
| Development Machine                              |
|                                                  |
|  +----------+  +----------+  +----------+        |
|  | Neo Core |  | Agent    |  | Web UI   |        |
|  | (Rust)   |  | Scheduler|  | (TS)     |        |
|  +----+-----+  +----+-----+  +----+-----+        |
|       |              |              |             |
|  +----v--------------v--------------v-----+       |
|  |           Local Message Bus            |       |
|  +----------------------------------------+       |
|                                                  |
|  +----------+  +----------+  +----------+        |
|  | SQLite   |  | Device   |  | Obs      |        |
|  | Storage  |  | Manager  |  | Stack    |        |
|  +----------+  +----------+  +----------+        |
+--------------------------------------------------+
```

### 7.2 Production Cluster

```
                    +------------------+
                    |   Load Balancer  |
                    +--------+---------+
                             |
              +--------------+--------------+
              |                             |
     +--------v--------+          +--------v--------+
     |  API Gateway    |          |  API Gateway    |
     |  (Go) x3        |          |  (Go) x3        |
     +--------+--------+          +--------+--------+
              |                             |
              +--------------+--------------+
                             |
                    +--------v---------+
                    |   Message Bus    |
                    |   (NATS Cluster) |
                    +--------+---------+
                             |
           +-----------------+-----------------+
           |                 |                 |
  +--------v------+  +------v--------+  +-----v---------+
  | Neural Core   |  | Agent         |  | Knowledge     |
  | (Rust) x4 GPU |  | Scheduler    |  | Graph (Rust)  |
  +---------------+  | (Rust) x2    |  | x2            |
                     +---------------+  +---------------+
           |                 |                 |
  +--------v------+  +------v--------+  +-----v---------+
  | Storage Engine|  | Device Mgr   |  | Observability |
  | (Rust) x3     |  | (Go) x2      |  | (Rust/Py)     |
  +---------------+  +---------------+  +---------------+
```

### 7.3 Edge Deployment

For edge deployments (robotics, IoT), a lightweight subset of the system runs on constrained hardware:

```
+--------------------------------------------------+
| Edge Device (ARM64 / x86)                        |
|                                                  |
|  +----------+  +----------+  +----------+        |
|  | Neural   |  | Robotics |  | Device   |        |
|  | Runtime  |  | Control  |  | Manager  |        |
|  | (Rust)   |  | (Rust)   |  | (Go)     |        |
|  +----+-----+  +----+-----+  +----+-----+        |
|       |              |              |             |
|  +----v--------------v--------------v-----+       |
|  |           Local Bus (Unix Sockets)    |       |
|  +----------------------------------------+       |
|                                                  |
|  +----------+                                    |
|  | Embedded |                                    |
|  | Storage  |                                    |
|  +----------+                                    |
+--------------------------------------------------+
          |  (Syncs with cloud when connected)
          v
+--------------------------------------------------+
| Cloud Backend                                     |
+--------------------------------------------------+
```

---

## 8. Security Architecture

### 8.1 Authentication

All external API access requires JWT authentication. Tokens are issued by the identity service and validated at the API Gateway. Internal service communication uses mTLS.

### 8.2 Authorization

Role-Based Access Control (RBAC) with the following roles:

- **Admin**: Full system access
- **Operator**: Agent management, task submission
- **Viewer**: Read-only access to dashboards and logs
- **Agent**: Limited to assigned tasks and resources

### 8.3 Data Protection

- All data at rest is encrypted using AES-256-GCM
- All data in transit uses TLS 1.3
- Neural model weights are signed with Ed25519 keys
- Audit logs are immutable (append-only with hash chaining)

---

## 9. Scalability Model

### 9.1 Horizontal Scaling

Components scale independently:

- **API Gateway**: Scale by adding instances behind load balancer
- **Neural Core**: Scale by adding GPU nodes
- **Agent Scheduler**: Scale to 2-3 instances with leader election
- **Storage**: Scale with sharding across nodes
- **Message Bus**: NATS cluster handles partitioning automatically

### 9.2 Vertical Scaling

- Neural Core benefits from multi-GPU nodes (NVLink/NVSwitch)
- Storage benefits from large NVMe pools
- Agent Scheduler benefits from high-memory nodes for large task graphs

### 9.3 Auto-Scaling Triggers

| Metric                        | Threshold | Action                    |
|------------------------------|-----------|---------------------------|
| GPU utilization               | > 85%     | Add Neural Core instance  |
| Task queue depth              | > 1000    | Add Agent Scheduler       |
| API latency p99               | > 200ms   | Add API Gateway           |
| Storage disk usage            | > 80%     | Add Storage node          |
| Memory usage                  | > 90%     | Alert, consider scaling   |

---

## 10. Fault Tolerance

### 10.1 Circuit Breaker Pattern

Each external dependency call is wrapped in a circuit breaker:

```
CLOSED (normal) --[failure threshold]--> OPEN (rejecting) --[timeout]--> HALF-OPEN (testing)
                                                                   |
                                                              [success] --> CLOSED
                                                              [failure] --> OPEN
```

### 10.2 Supervisor Trees

Agent processes run under supervisors that restart failed agents with exponential backoff:

```
Supervisor
  +-- Agent 1 (restart: 1s, 2s, 4s, max 30s)
  +-- Agent 2 (restart: 1s, 2s, 4s, max 30s)
  +-- Agent 3 (restart: 1s, 2s, 4s, max 30s)
```

### 10.3 Data Durability

- Write-ahead log (WAL) for all state mutations
- Periodic snapshots (configurable interval)
- Cross-region replication for production deployments
- Point-in-time recovery from WAL + snapshots

### 10.4 Graceful Degradation

When components fail, the system degrades gracefully:

| Component Failure      | Degradation                                  |
|-----------------------|----------------------------------------------|
| Neural Core           | Queue inference requests, serve from cache   |
| Agent Scheduler       | Tasks run on local agent queues              |
| Knowledge Graph       | Fall back to cached queries                  |
| Storage Engine        | Serve from in-memory cache, queue writes     |
| Observability         | System continues, lose monitoring            |
| Message Bus           | Direct RPC fallback between services         |

---

## Appendix A: Technology Stack Summary

| Layer            | Languages       | Key Technologies                    |
|-----------------|-----------------|-------------------------------------|
| Presentation     | TypeScript, Swift, Kotlin | React, SwiftUI, Jetpack Compose  |
| API Gateway      | Go, TypeScript  | Chi router, WebSocket, gRPC-Gateway |
| Orchestration    | Rust            | Tokio, Tower, prost                 |
| Neural Core      | Rust, C++       | CUDA, cuDNN, burn                   |
| Knowledge Graph  | Rust, Python    | Custom engine, PyO3 bindings        |
| Storage          | Rust            | Custom LSM-tree, io_uring           |
| Device Manager   | Go              | gRPC, udev, serial                  |
| Robotics         | Rust, C++, Kotlin | HAL, ROS2 bridges                 |
| Observability    | Rust, Python    | OpenTelemetry, Prometheus           |
| Message Bus      | Infrastructure  | NATS, gRPC streaming                |

## Appendix B: Directory Structure

```
Neo_2.0/
  +-- core/                    # Rust core components
  +-- neural-network-framework/ # Neural processing (Rust/C++/Python)
  +-- knowledge-graph/         # Knowledge graph (Rust/Python)
  +-- api-gateway/             # API gateway (Go/TypeScript)
  +-- web-dashboard/           # Web UI (TypeScript)
  +-- sdk/                     # Client SDKs (Kotlin, Swift, TypeScript, Python)
  +-- ui/                      # Native UIs (Kotlin, Swift)
  +-- robotics/                # Robotics control (Rust/C++/Kotlin)
  +-- device-manager/          # Device management (Go)
  +-- observability/           # Monitoring (Rust/Python)
  +-- proto/                   # Protobuf definitions
  +-- scripts/                 # Build and utility scripts
  +-- docs/                    # Documentation
  +-- tests/                   # Integration tests
  +-- deploy/                  # Deployment configurations
```
