# Neo AGI OS — Subsystem Reference

## Table of Contents

- [1. Overview](#1-overview)
- [2. Neural Core](#2-neural-core)
- [3. Agent Scheduler](#3-agent-scheduler)
- [4. Knowledge Graph](#4-knowledge-graph)
- [5. API Gateway](#5-api-gateway)
- [6. Message Bus](#6-message-bus)
- [7. Storage Engine](#7-storage-engine)
- [8. Robotics Control](#8-robotics-control)
- [9. Device Manager](#9-device-manager)
- [10. Web Dashboard](#10-web-dashboard)
- [11. SDK Libraries](#11-sdk-libraries)
- [12. Observability Pipeline](#12-observability-pipeline)
- [13. Workflow Engine](#13-workflow-engine)
- [14. Identity Service](#14-identity-service)
- [15. Configuration Service](#15-configuration-service)

---

## 1. Overview

Neo AGI OS is composed of fifteen subsystems. Each subsystem has a clearly defined purpose, public API surface, set of dependencies, and deployment characteristics. This document provides a detailed reference for every subsystem.

Subsystems communicate through well-defined interfaces. Internal communication uses gRPC with Protocol Buffers. External communication uses HTTP/2 with JSON or Protocol Buffers. Event-driven communication uses the NATS message bus.

---

## 2. Neural Core

### 2.1 Purpose

The Neural Core is the central intelligence processing unit of Neo AGI OS. It manages neural network lifecycle, inference, training, and model versioning. It interfaces directly with GPU hardware through CUDA for acceleration.

### 2.2 Responsibilities

- Neural network model loading, initialization, and teardown
- Inference request processing with batching optimization
- Training loop management with gradient accumulation
- Model versioning and rollback
- GPU memory management and allocation
- Kernel fusion and optimization
- Mixed-precision computation (FP16, BF16, INT8)
- Multi-GPU distribution (data parallelism, model parallelism)

### 2.3 Public API Surface

```protobuf
service NeuralCore {
    rpc LoadModel(LoadModelRequest) returns (LoadModelResponse);
    rpc UnloadModel(UnloadModelRequest) returns (UnloadModelResponse);
    rpc Infer(InferRequest) returns (stream InferResponse);
    rpc Train(TrainRequest) returns (stream TrainEvent);
    rpc GetModelInfo(GetModelInfoRequest) returns (ModelInfo);
    rpc ListModels(ListModelsRequest) returns (ListModelsResponse);
    rpc HealthCheck(HealthCheckRequest) returns (HealthCheckResponse);
}
```

### 2.4 Dependencies

- **CUDA Toolkit >= 12.3**: GPU computation
- **cuDNN >= 8.9**: Neural network primitives
- **Storage Engine**: Model weight persistence
- **Message Bus**: Training event broadcasting
- **Configuration Service**: Model configuration
- **Observability Pipeline**: Performance metrics

### 2.5 Deployment Characteristics

- Requires NVIDIA GPU (minimum: 1x A100 40GB for production)
- Runs as a long-lived daemon process
- Memory footprint: 2-8GB base, plus model-specific GPU memory
- CPU cores: minimum 4, recommended 16
- Network: connects to Message Bus and Storage Engine
- Scales horizontally by adding GPU nodes

### 2.6 Internal Architecture

```
+--------------------------------------------------+
| Neural Core                                       |
|                                                   |
|  +------------+    +-------------+                 |
|  | Model      |    | Inference   |                 |
|  | Registry   |    | Pipeline    |                 |
|  +-----+------+    +------+------+                 |
|        |                  |                        |
|  +-----v------+    +-----v-------+                |
|  | GPU Memory |    | Batch       |                 |
|  | Manager    |    | Scheduler   |                 |
|  +-----+------+    +------+-----+                 |
|        |                  |                        |
|  +-----v------------------v-------+                |
|  |        CUDA Kernel Layer       |                |
|  +--------------------------------+                |
+--------------------------------------------------+
```

---

## 3. Agent Scheduler

### 3.1 Purpose

The Agent Scheduler manages the lifecycle of intelligent agents, assigns tasks to agents based on capability matching, and coordinates multi-agent workflows.

### 3.2 Responsibilities

- Agent registration, deregistration, and health monitoring
- Task submission, prioritization, and assignment
- Capability matching between tasks and agents
- Resource-aware scheduling (CPU, GPU, memory)
- Agent lifecycle management (create, start, pause, resume, stop)
- Workload balancing across agent pools
- Retry logic and failure handling
- Agent state persistence and recovery

### 3.3 Public API Surface

```protobuf
service AgentScheduler {
    rpc RegisterAgent(RegisterAgentRequest) returns (AgentHandle);
    rpc DeregisterAgent(DeregisterAgentRequest) returns (DeregisterAgentResponse);
    rpc GetAgentStatus(GetAgentStatusRequest) returns (AgentStatus);
    rpc SubmitTask(SubmitTaskRequest) returns (TaskHandle);
    rpc CancelTask(CancelTaskRequest) returns (CancelTaskResponse);
    rpc ListAgents(ListAgentsRequest) returns (ListAgentsResponse);
    rpc ListTasks(ListTasksRequest) returns (ListTasksResponse);
    rpc GetTaskResult(GetTaskResultRequest) returns (TaskResult);
    rpc SubscribeEvents(SubscribeRequest) returns (stream AgentEvent);
}
```

### 3.4 Dependencies

- **Message Bus**: Event distribution and inter-component communication
- **Storage Engine**: Task and agent state persistence
- **Neural Core**: GPU resource availability queries
- **Configuration Service**: Scheduling policies and limits
- **Observability Pipeline**: Scheduling metrics

### 3.5 Deployment Characteristics

- Runs as 2-3 instances with leader election (Raft consensus)
- Memory footprint: 512MB - 2GB depending on task queue size
- CPU cores: minimum 2, recommended 8
- Network: high-throughput connection to Message Bus
- Requires low-latency connection to Storage Engine

### 3.6 Scheduling Algorithm

The scheduler uses a multi-criteria scoring algorithm:

```
Score = w1 * CapabilityMatch
      + w2 * ResourceAvailability
      + w3 * AgentLoad
      + w4 * TaskPriority
      + w5 * LocalityPreference
```

Where weights w1-w5 are configurable and default to equal distribution.

---

## 4. Knowledge Graph

### 4.1 Purpose

The Knowledge Graph stores and queries structured knowledge as a graph of entities and relationships. It supports temporal versioning, inference rules, and semantic search.

### 4.2 Responsibilities

- Entity and relationship CRUD operations
- Graph traversal with configurable depth and filters
- Temporal versioning (point-in-time queries)
- Inference rule execution
- Semantic search over entity attributes
- Graph analytics (centrality, clustering, path finding)
- Schema management and evolution
- Import/export in standard formats (RDF, JSON-LD)

### 4.3 Public API Surface

```protobuf
service KnowledgeGraph {
    rpc CreateEntity(CreateEntityRequest) returns (Entity);
    rpc UpdateEntity(UpdateEntityRequest) returns (Entity);
    rpc DeleteEntity(DeleteEntityRequest) returns (DeleteResponse);
    rpc GetEntity(GetEntityRequest) returns (Entity);
    rpc CreateRelation(CreateRelationRequest) returns (Relation);
    rpc Query(QueryRequest) returns (QueryResponse);
    rpc Traverse(TraverseRequest) returns (TraverseResponse);
    rpc Search(SearchRequest) returns (SearchResponse);
    rpc Infer(InferRequest) returns (InferResponse);
    rpc GetSubgraph(GetSubgraphRequest) returns (Subgraph);
}
```

### 4.4 Dependencies

- **Storage Engine**: Persistent graph storage
- **Neural Core**: Embedding generation for semantic search
- **Message Bus**: Mutation event broadcasting
- **Configuration Service**: Schema and inference rule configuration
- **Python Runtime**: ML-based inference rules (via PyO3)

### 4.5 Deployment Characteristics

- Runs as 2 instances with read replicas
- Memory footprint: 4-16GB (depends on graph size)
- CPU cores: minimum 4, recommended 16
- Storage: graph data on fast NVMe SSDs
- Network: requires Storage Engine and Neural Core connectivity

### 4.6 Graph Schema

```
Entity {
    id: UUID
    type: String
    attributes: Map<String, Value>
    created_at: Timestamp
    updated_at: Timestamp
    version: u64
    embeddings: Optional<Vec<f32>>
}

Relation {
    id: UUID
    source_id: UUID
    target_id: UUID
    type: String
    attributes: Map<String, Value>
    weight: f64
    created_at: Timestamp
    valid_from: Timestamp
    valid_to: Option<Timestamp>
}
```

---

## 5. API Gateway

### 5.1 Purpose

The API Gateway is the single entry point for all external traffic. It handles authentication, rate limiting, request routing, and response aggregation.

### 5.2 Responsibilities

- HTTP/2 and WebSocket request handling
- JWT authentication and token validation
- Rate limiting (per-user, per-endpoint)
- Request routing to internal services
- Response aggregation from multiple services
- CORS handling
- API versioning
- Request/response logging
- OpenAPI specification serving

### 5.3 Public API Surface

The API Gateway exposes the following endpoint groups:

```
POST   /api/v1/auth/login
POST   /api/v1/auth/refresh
GET    /api/v1/agents
POST   /api/v1/agents
GET    /api/v1/agents/:id
DELETE /api/v1/agents/:id
POST   /api/v1/agents/:id/tasks
GET    /api/v1/tasks/:id
DELETE /api/v1/tasks/:id
GET    /api/v1/models
POST   /api/v1/models/:id/infer
GET    /api/v1/knowledge/entities
POST   /api/v1/knowledge/entities
GET    /api/v1/knowledge/query
GET    /api/v1/system/health
GET    /api/v1/system/metrics
WS     /api/v1/ws/events
```

### 5.4 Dependencies

- **Agent Scheduler**: Task and agent operations
- **Neural Core**: Model operations and inference
- **Knowledge Graph**: Knowledge operations
- **Identity Service**: Authentication
- **Configuration Service**: Rate limiting and routing rules
- **Observability Pipeline**: Request metrics

### 5.5 Deployment Characteristics

- Runs as 3+ instances behind a load balancer
- Memory footprint: 128MB - 512MB per instance
- CPU cores: minimum 2, recommended 4
- Network: public-facing (requires TLS termination)
- Stateless: no local data persistence
- Auto-scales based on request rate and latency

### 5.6 Rate Limiting

Rate limits are configurable per endpoint and per user:

```yaml
rate_limits:
  default:
    requests_per_second: 100
    burst: 200
  inference:
    requests_per_second: 10
    burst: 20
  knowledge_write:
    requests_per_second: 50
    burst: 100
```

---

## 6. Message Bus

### 6.1 Purpose

The Message Bus provides asynchronous, decoupled communication between subsystems. It supports publish-subscribe, request-reply, and queue-based messaging patterns.

### 6.2 Responsibilities

- Topic-based publish-subscribe messaging
- Queue-based message delivery (competing consumers)
- Message persistence and replay
- Dead letter queue for failed messages
- Message deduplication
- Ordering guarantees within partitions
- Flow control and backpressure

### 6.3 Public API Surface

```protobuf
service MessageBus {
    rpc Publish(PublishRequest) returns (PublishResponse);
    rpc Subscribe(SubscribeRequest) returns (stream Message);
    rpc Request(RequestRequest) returns (Reply);
    rpc CreateQueue(CreateQueueRequest) returns (QueueHandle);
    rpc DeleteQueue(DeleteQueueRequest) returns (DeleteResponse);
    rpc GetQueueStats(GetQueueStatsRequest) returns (QueueStats);
}
```

### 6.4 Dependencies

- **Storage Engine**: Message persistence
- **Observability Pipeline**: Bus metrics
- **Configuration Service**: Topic and queue configuration

### 6.5 Deployment Characteristics

- NATS cluster with 3+ nodes
- Memory footprint: 2-8GB per node
- CPU cores: minimum 2, recommended 4
- Network: high-bandwidth, low-latency
- Requires fast storage for message persistence

---

## 7. Storage Engine

### 7.1 Purpose

The Storage Engine provides durable, high-performance key-value and document storage. It is built on a custom LSM-tree implementation optimized for the access patterns of Neo AGI OS.

### 7.2 Responsibilities

- Key-value storage with versioning
- Document storage with secondary indexes
- Range queries with cursor-based pagination
- Write-ahead logging (WAL) for crash recovery
- Background compaction
- Compression (LZ4, Zstandard)
- Encryption at rest (AES-256-GCM)
- Snapshot and backup
- Replication (synchronous and asynchronous)

### 7.3 Public API Surface

```protobuf
service StorageEngine {
    rpc Put(PutRequest) returns (PutResponse);
    rpc Get(GetRequest) returns (GetResponse);
    rpc Delete(DeleteRequest) returns (DeleteResponse);
    rpc Scan(ScanRequest) returns (ScanResponse);
    rpc BatchPut(BatchPutRequest) returns (BatchPutResponse);
    rpc BatchGet(BatchGetRequest) returns (BatchGetResponse);
    rpc CreateSnapshot(CreateSnapshotRequest) returns (SnapshotHandle);
    rpc RestoreSnapshot(RestoreSnapshotRequest) returns (RestoreResponse);
    rpc GetStats(GetStatsRequest) returns (StorageStats);
}
```

### 7.4 Dependencies

- **Observability Pipeline**: Storage metrics
- **Configuration Service**: Storage configuration

### 7.5 Deployment Characteristics

- Runs as 3+ nodes in a replication group
- Memory footprint: 8-32GB per node (WAL cache)
- CPU cores: minimum 4, recommended 16
- Storage: NVMe SSDs, minimum 1TB per node
- Network: requires fast inter-node replication

---

## 8. Robotics Control

### 8.1 Purpose

The Robotics Control subsystem provides real-time control of robotic hardware, including joint control, sensor fusion, motion planning, and safety monitoring.

### 8.2 Responsibilities

- Robot lifecycle management (connect, configure, start, stop)
- Joint angle and velocity control
- Sensor data acquisition and fusion
- Trajectory planning and execution
- Collision detection and avoidance
- Emergency stop handling
- Kinematic and dynamic modeling
- Hardware abstraction (supports multiple robot types)

### 8.3 Public API Surface

```protobuf
service RoboticsControl {
    rpc ConnectRobot(ConnectRequest) returns (RobotHandle);
    rpc DisconnectRobot(DisconnectRequest) returns (DisconnectResponse);
    rpc GetRobotStatus(GetStatusRequest) returns (RobotStatus);
    rpc MoveJoint(MoveJointRequest) returns (MoveResponse);
    rpc MoveToPosition(MoveToPositionRequest) returns (MoveResponse);
    rpc ExecuteTrajectory(ExecuteTrajectoryRequest) returns (stream TrajectoryEvent);
    rpc EmergencyStop(EmergencyStopRequest) returns (EmergencyStopResponse);
    rpc GetSensorData(GetSensorDataRequest) returns (SensorData);
    rpc SubscribeTelemetry(SubscribeRequest) returns (stream TelemetryEvent);
}
```

### 8.4 Dependencies

- **Device Manager**: Hardware discovery and connection
- **Neural Core**: Visual perception models
- **Message Bus**: Telemetry and command distribution
- **Configuration Service**: Robot profiles and safety limits
- **Observability Pipeline**: Telemetry metrics

### 8.5 Deployment Characteristics

- Runs on edge devices or dedicated control hardware
- Memory footprint: 256MB - 2GB
- CPU cores: minimum 2, recommended 8
- Real-time requirements: < 1ms control loop
- Requires direct hardware access (GPIO, USB, Ethernet)

### 8.6 Safety Architecture

```
+--------------------------------------------------+
| Safety Monitor (Highest Priority)                 |
|  +--------------------------------------------+  |
|  | Emergency Stop Handler                      |  |
|  | Collision Detection                         |  |
|  | Joint Limit Enforcement                     |  |
|  | Velocity Limit Enforcement                  |  |
|  | Watchdog Timer                              |  |
|  +--------------------------------------------+  |
+--------------------------------------------------+
           |
           v
+--------------------------------------------------+
| Control Loop (Real-time Thread)                   |
|  +--------------------------------------------+  |
|  | Trajectory Interpolation                    |  |
|  | PID Controller                              |  |
|  | Feedforward Compensation                    |  |
|  +--------------------------------------------+  |
+--------------------------------------------------+
           |
           v
+--------------------------------------------------+
| Hardware Abstraction Layer                        |
|  +--------------------------------------------+  |
|  | Joint Drivers                               |  |
|  | Sensor Drivers                              |  |
|  | Communication Interfaces                    |  |
|  +--------------------------------------------+  |
+--------------------------------------------------+
```

---

## 9. Device Manager

### 9.1 Purpose

The Device Manager discovers, manages, and monitors hardware devices connected to Neo AGI OS, including IoT sensors, actuators, cameras, and robotic controllers.

### 9.2 Responsibilities

- Hardware device discovery and enumeration
- Device lifecycle management
- Driver loading and management
- Device health monitoring
- Firmware update management
- Device capability reporting
- Connection management (USB, serial, Ethernet, Bluetooth)

### 9.3 Public API Surface

```protobuf
service DeviceManager {
    rpc ListDevices(ListDevicesRequest) returns (ListDevicesResponse);
    rpc GetDeviceInfo(GetDeviceInfoRequest) returns (DeviceInfo);
    rpc ConnectDevice(ConnectRequest) returns (ConnectionHandle);
    rpc DisconnectDevice(DisconnectRequest) returns (DisconnectResponse);
    rpc SubscribeEvents(SubscribeRequest) returns (stream DeviceEvent);
    rpc UpdateFirmware(UpdateFirmwareRequest) returns (UpdateResponse);
}
```

### 9.4 Dependencies

- **Message Bus**: Device event broadcasting
- **Observability Pipeline**: Device metrics
- **Configuration Service**: Device profiles

### 9.5 Deployment Characteristics

- Runs on machines with physical device connections
- Memory footprint: 64MB - 256MB
- CPU cores: minimum 1, recommended 2
- Requires root/admin access for device management
- Uses udev (Linux) for device hotplug detection

---

## 10. Web Dashboard

### 10.1 Purpose

The Web Dashboard provides a browser-based interface for monitoring and managing Neo AGI OS. It displays real-time system status, agent activity, model performance, and allows interactive knowledge graph exploration.

### 10.2 Responsibilities

- Real-time system health display
- Agent management interface
- Task monitoring and management
- Model performance visualization
- Knowledge graph explorer
- Robot status and control interface
- Alert management
- Configuration management
- User authentication and session management

### 10.3 Dependencies

- **API Gateway**: All data access
- **WebSocket Server**: Real-time updates
- **Configuration Service**: UI configuration

### 10.4 Deployment Characteristics

- Static files served by CDN or API Gateway
- Memory footprint: browser-managed
- No server-side rendering required
- Responsive design for tablet and desktop
- WebSocket connection for real-time updates

---

## 11. SDK Libraries

### 11.1 Purpose

SDK libraries provide language-native interfaces for interacting with Neo AGI OS from client applications. Libraries are available for Kotlin, Swift, TypeScript, and Python.

### 11.2 Supported Languages

| Language  | Platform            | Package Manager   |
|----------|---------------------|-------------------|
| Kotlin   | JVM, Android        | Gradle/Maven      |
| Swift    | iOS, macOS          | Swift Package Mgr  |
| TypeScript| Node.js, Browser   | npm/pnpm          |
| Python   | Any                 | pip               |

### 11.3 Common API Surface

Each SDK provides:

```typescript
// Connection
client.connect()
client.disconnect()
client.isConnected()

// Agents
client.createAgent(name, config)
client.getAgent(id)
client.deleteAgent(id)
client.listAgents()

// Tasks
client.submitTask(agentId, payload)
client.getTaskResult(taskId)
client.cancelTask(taskId)

// Health
client.health()
```

---

## 12. Observability Pipeline

### 12.1 Purpose

The Observability Pipeline collects, processes, and exports telemetry data (metrics, traces, logs) from all subsystems.

### 12.2 Responsibilities

- Metrics collection (Prometheus-compatible)
- Distributed tracing (OpenTelemetry)
- Structured log aggregation
- Alert rule evaluation
- Dashboard data provisioning
- Anomaly detection
- Performance profiling

### 12.3 Public API Surface

```protobuf
service Observability {
    rpc RecordMetric(RecordMetricRequest) returns (RecordResponse);
    rpc QueryMetrics(QueryMetricsRequest) returns (MetricsResponse);
    rpc StartSpan(StartSpanRequest) returns (SpanHandle);
    rpc EndSpan(EndSpanRequest) returns (EndResponse);
    rpc QueryTraces(QueryTracesRequest) returns (TracesResponse);
    rpc IngestLog(IngestLogRequest) returns (IngestResponse);
    rpc QueryLogs(QueryLogsRequest) returns (LogsResponse);
    rpc ListAlerts(ListAlertsRequest) returns (AlertsResponse);
}
```

---

## 13. Workflow Engine

### 13.1 Purpose

The Workflow Engine orchestrates multi-step operations across subsystems, providing durable execution, compensation logic, and visual workflow design.

### 13.2 Responsibilities

- Workflow definition (DSL and visual editor)
- Durable execution with checkpointing
- Conditional branching and looping
- Parallel execution
- Compensation (saga pattern)
- Timeout and retry handling
- Workflow versioning
- Execution history and debugging

### 13.3 Public API Surface

```protobuf
service WorkflowEngine {
    rpc CreateWorkflow(CreateWorkflowRequest) returns (WorkflowHandle);
    rpc StartWorkflow(StartWorkflowRequest) returns (ExecutionHandle);
    rpc GetExecutionStatus(GetStatusRequest) returns (ExecutionStatus);
    rpc CancelExecution(CancelRequest) returns (CancelResponse);
    rpc PauseExecution(PauseRequest) returns (PauseResponse);
    rpc ResumeExecution(ResumeRequest) returns (ResumeResponse);
    rpc ListWorkflows(ListRequest) returns (ListResponse);
}
```

---

## 14. Identity Service

### 14.1 Purpose

The Identity Service manages user authentication, authorization, and session management for Neo AGI OS.

### 14.2 Responsibilities

- User registration and authentication
- JWT token issuance and validation
- Role-based access control (RBAC)
- API key management
- Session management
- OAuth2 integration
- Audit logging

---

## 15. Configuration Service

### 15.1 Purpose

The Configuration Service provides centralized, dynamic configuration management for all subsystems.

### 15.2 Responsibilities

- Configuration storage and retrieval
- Dynamic configuration updates (no restart required)
- Configuration versioning and rollback
- Environment-specific configuration
- Secret management
- Feature flags
- Configuration validation
