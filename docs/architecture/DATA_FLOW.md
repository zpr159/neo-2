# Neo AGI OS — Data Flow

## Table of Contents

- [1. Overview](#1-overview)
- [2. Inference Data Flow](#2-inference-data-flow)
- [3. Training Data Flow](#3-training-data-flow)
- [4. Agent Task Data Flow](#4-agent-task-data-flow)
- [5. Knowledge Graph Data Flow](#5-knowledge-graph-data-flow)
- [6. Robotics Control Data Flow](#6-robotics-control-data-flow)
- [7. Observability Data Flow](#7-observability-data-flow)
- [8. Authentication Data Flow](#8-authentication-data-flow)
- [9. Error Handling Data Flow](#9-error-handling-data-flow)
- [10. Data Serialization Formats](#10-data-serialization-formats)

---

## 1. Overview

Data flows through Neo AGI OS via multiple paths depending on the operation type. This document traces data from input to output for each major operation, including all intermediate transformations, validation steps, and persistence points.

All data flow diagrams use ASCII sequence diagrams for clarity.

---

## 2. Inference Data Flow

This is the most performance-critical data path in the system. A user submits a prompt and receives a model-generated response.

```
User/Client         API Gateway         Neural Core         Storage Engine
    |                    |                    |                    |
    |-- POST /infer ---->|                    |                    |
    |                    |-- Auth check ----->|                    |
    |                    |<-- Token valid ----|                    |
    |                    |                    |                    |
    |                    |-- Validate req --->|                    |
    |                    |<-- Request valid --|                    |
    |                    |                    |                    |
    |                    |-- Check cache --->|                    |
    |                    |                    |-- Get cache key -->|
    |                    |                    |<-- Cache hit? ----|
    |                    |                    |                    |
    |                    |              [Cache Miss]               |
    |                    |                    |                    |
    |                    |-- Queue task ----->|                    |
    |                    |<-- Task ID --------|                    |
    |                    |                    |                    |
    |                    |                    |-- Load model ---->|
    |                    |                    |<-- Model weights --|
    |                    |                    |                    |
    |                    |                    |-- Allocate GPU -->|
    |                    |                    |-- Tokenize input  |
    |                    |                    |-- Run inference   |
    |                    |                    |-- Detokenize      |
    |                    |                    |-- Free GPU mem    |
    |                    |                    |                    |
    |                    |                    |-- Cache result -->|
    |                    |                    |<-- Cached --------|
    |                    |                    |                    |
    |                    |<-- Result ---------|                    |
    |<-- Response -------|                    |                    |
```

### Step-by-Step

1. **Client sends POST /api/v1/models/:id/infer** with JSON body containing the prompt and parameters.
2. **API Gateway** extracts the JWT token and validates it against the Identity Service.
3. **API Gateway** validates the request schema (prompt length, parameter ranges).
4. **API Gateway** checks the result cache (keyed by model_id + prompt_hash + parameters).
5. **Cache hit**: Return cached result immediately.
6. **Cache miss**: Submit an inference task to the Neural Core via gRPC.
7. **Neural Core** checks if the model is loaded; if not, loads it from the Storage Engine.
8. **Neural Core** allocates GPU memory, tokenizes input, runs the forward pass, and detokenizes output.
9. **Neural Core** caches the result and returns it to the API Gateway.
10. **API Gateway** returns the response to the client.

### Data Transformations

```
Raw Prompt (String)
    --> Tokenized (Vec<u32>)        [Tokenization]
    --> Embedded (Vec<Vec<f32>>)     [Embedding Layer]
    --> Hidden States (Tensor)       [Transformer Layers]
    --> Logits (Tensor)              [Output Layer]
    --> Token Probabilities (Vec<f32>) [Softmax]
    --> Generated Tokens (Vec<u32>)  [Sampling/Decoding]
    --> Response Text (String)       [Detokenization]
```

---

## 3. Training Data Flow

Training is a longer-running process that iterates over datasets.

```
Data Source         Preprocessing       Neural Core         Checkpoint Store
    |                    |                    |                    |
    |-- Read batch ----->|                    |                    |
    |<-- Batch data -----|                    |                    |
    |                    |                    |                    |
    |                    |-- Validate ------->|                    |
    |                    |-- Normalize ------>|                    |
    |                    |-- Augment -------->|                    |
    |                    |                    |                    |
    |                    |-- Forward pass --->|                    |
    |                    |<-- Loss -----------|                    |
    |                    |                    |                    |
    |                    |-- Backward pass -->|                    |
    |                    |<-- Gradients ------|                    |
    |                    |                    |                    |
    |                    |-- Optimizer step ->|                    |
    |                    |<-- Updated weights |                    |
    |                    |                    |                    |
    |                    |              [Every N steps]            |
    |                    |                    |-- Save checkpoint->|
    |                    |                    |<-- Checkpoint ID --|
    |                    |                    |                    |
    |                    |-- Emit metrics --->|                    |
```

### Step-by-Step

1. **Data loader** reads a batch from the dataset (disk, S3, or database).
2. **Preprocessor** validates data integrity, normalizes values, and applies augmentation.
3. **Neural Core** performs the forward pass to compute predictions.
4. **Loss function** computes the training loss.
5. **Neural Core** performs the backward pass to compute gradients.
6. **Optimizer** updates model weights.
7. **Every N steps**, a checkpoint is saved to the Storage Engine.
8. **Metrics** (loss, learning rate, gradient norms) are emitted to the Observability Pipeline.

---

## 4. Agent Task Data Flow

Agents process tasks submitted by users or other agents.

```
User/Client       API Gateway       Agent Scheduler       Agent (Worker)
    |                |                    |                    |
    |-- Submit task->|                    |                    |
    |                |-- Validate ------->|                    |
    |                |                    |                    |
    |                |-- Submit task ---->|                    |
    |                |<-- Task ID --------|                    |
    |                |                    |                    |
    |                |                    |-- Match capabilities|
    |                |                    |-- Assign to agent ->|
    |                |                    |                    |
    |                |                    |              [Agent picks up task]
    |                |                    |                    |
    |                |                    |                    |-- Execute task
    |                |                    |                    |-- Report progress
    |                |                    |<-- Progress --------|
    |                |                    |                    |
    |                |                    |              [Task complete]
    |                |                    |<-- Result ----------|
    |                |                    |-- Store result ---->|
    |                |                    |-- Notify ---------->|
    |                |<-- Task complete --|                    |
    |<-- Response ---|                    |                    |
```

### Task Lifecycle

```
[Created] --> [Queued] --> [Assigned] --> [Running] --> [Completed]
                          |                |
                          v                v
                      [Unassigned]     [Failed]
                      [Reassigned]     [Retrying]
```

---

## 5. Knowledge Graph Data Flow

### 5.1 Entity Creation

```
Client          API Gateway       Knowledge Graph      Storage Engine
  |                |                    |                    |
  |-- Create ----->|                    |                    |
  |                |-- Validate ------->|                    |
  |                |                    |                    |
  |                |-- Create entity -->|                    |
  |                |                    |-- Validate schema  |
  |                |                    |-- Generate ID      |
  |                |                    |-- Compute embedding|
  |                |                    |-- Persist -------->|
  |                |                    |<-- Stored ---------|
  |                |                    |-- Emit event ----->|
  |                |<-- Entity ---------|                    |
  |<-- Response ---|                    |                    |
```

### 5.2 Graph Query

```
Client          API Gateway       Knowledge Graph      Storage Engine
  |                |                    |                    |
  |-- Query ------>|                    |                    |
  |                |-- Parse query ---->|                    |
  |                |                    |-- Plan traversal   |
  |                |                    |-- Execute -------->|
  |                |                    |<-- Subgraph -------|
  |                |                    |-- Merge results    |
  |                |                    |-- Rank by relevance|
  |                |<-- Results --------|                    |
  |<-- Response ---|                    |                    |
```

### Query Execution Pipeline

```
[Parse] --> [Validate] --> [Plan] --> [Optimize] --> [Execute] --> [Merge] --> [Rank] --> [Return]
   |           |            |           |              |             |           |
   v           v            v           v              v             v           v
 [Syntax    [Permission  [Cost-based  [Index        [Traverse    [Dedup     [Score
  check]     check]       optimizer]   selection]    nodes]       results]   results]
```

---

## 6. Robotics Control Data Flow

### 6.1 Joint Control

```
Client/Planner     API Gateway       Robotics Control     Robot Hardware
    |                |                    |                    |
    |-- Move joint ->|                    |                    |
    |                |-- Validate ------->|                    |
    |                |                    |                    |
    |                |-- Send command --->|                    |
    |                |                    |-- Safety check     |
    |                |                    |-- Interpolate ---->|
    |                |                    |-- PID control ---->|
    |                |                    |                    |-- Actuate motor
    |                |                    |<-- Encoder feedback|
    |                |                    |-- Update state     |
    |                |<-- Done -----------|                    |
    |<-- Response ---|                    |                    |
```

### 6.2 Safety Check Pipeline

```
[Command Received]
    |
    v
[Joint Limit Check] --> [Velocity Limit Check] --> [Torque Limit Check]
    |                         |                          |
    v                         v                          v
 [Clamp]                  [Clamp]                     [Clamp]
    |                         |                          |
    v                         v                          v
[Collision Check] --> [Workspace Check] --> [Execute]
    |
    v
 [Emergency Stop if collision detected]
```

---

## 7. Observability Data Flow

### 7.1 Metrics Pipeline

```
All Services        Metric Collector      Aggregator        Prometheus/Grafana
    |                    |                    |                    |
    |-- Emit metric ---->|                    |                    |
    |                    |-- Buffer ----------|                    |
    |                    |-- Batch flush ---->|                    |
    |                    |                    |-- Aggregate ------>|
    |                    |                    |<-- Query ----------|
    |                    |                    |                    |-- Dashboard
    |                    |                    |                    |-- Alerting
```

### 7.2 Trace Pipeline

```
Service A           Trace Collector       Trace Storage       Jaeger/Zipkin
    |                    |                    |                    |
    |-- Start span ---->|                    |                    |
    |                    |-- Create trace     |                    |
    |                    |-- Propagate ctx -->|                    |
    |                    |                    |                    |
    |              [Service B processes]      |                    |
    |                    |-- End span -------->|                    |
    |                    |                    |-- Store ---------->|
    |                    |                    |<-- Query ----------|
    |                    |                    |                    |-- UI
```

---

## 8. Authentication Data Flow

```
Client          API Gateway       Identity Service       Storage Engine
  |                |                    |                    |
  |-- Login ------>|                    |                    |
  |                |-- Validate creds ->|                    |
  |                |                    |-- Hash password -->|
  |                |                    |<-- Verify match ---|
  |                |                    |-- Generate JWT     |
  |                |<-- JWT token ------|                    |
  |<-- Token ------|                    |                    |
  |                |                    |                    |
  |-- API call --->|                    |                    |
  |                |-- Extract token    |                    |
  |                |-- Validate JWT --->|                    |
  |                |<-- Token valid ----|                    |
  |                |-- Check RBAC ----->|                    |
  |                |<-- Permission -----|                    |
  |                |-- Route request    |                    |
```

---

## 9. Error Handling Data Flow

```
Service           Error Handler         Message Bus         Alert Service
  |                    |                    |                    |
  |-- Error --------->|                    |                    |
  |                    |-- Classify error   |                    |
  |                    |   (retryable?)     |                    |
  |                    |                    |                    |
  |              [Retryable]                |                    |
  |                    |-- Emit retry event>|                    |
  |                    |-- Schedule retry   |                    |
  |                    |                    |                    |
  |              [Non-retryable]            |                    |
  |                    |-- Emit error event>|                    |
  |                    |-- Update circuit -->|                    |
  |                    |                    |-- Alert --------->|
  |                    |                    |                    |-- Notify
```

### Error Classification

| Error Type        | Retryable | Action                              |
|------------------|-----------|-------------------------------------|
| Timeout          | Yes       | Exponential backoff retry           |
| Connection reset | Yes       | Reconnect and retry                 |
| Invalid request  | No        | Return 400 to client                |
| Unauthorized     | No        | Return 401 to client                |
| Internal error   | Partial   | Retry once, then alert              |
| Resource exhausted | Yes     | Queue and retry when resources free |

---

## 10. Data Serialization Formats

### 10.1 Format Matrix

| Boundary                    | Format      | Justification                        |
|----------------------------|-------------|--------------------------------------|
| Client <-> API Gateway     | JSON        | Human-readable, browser-native       |
| Service <-> Service (sync) | Protobuf    | Compact, fast, schema-enforced       |
| Service <-> Service (async)| MessagePack | Compact, no schema needed            |
| Neural Core <-> GPU        | Raw bytes   | Zero-copy, maximum throughput        |
| Storage Engine (on disk)   | Custom bin  | Optimized for LSM-tree               |
| Observability (wire)       | OpenTelemetry protobuf | Standard format              |

### 10.2 Message Size Limits

| Message Type     | Max Size | Rationale                           |
|-----------------|----------|--------------------------------------|
| API request     | 10 MB    | Reasonable for most use cases        |
| API response    | 50 MB    | Large model outputs                  |
| gRPC message    | 100 MB   | Internal, larger payloads allowed    |
| NATS message    | 1 MB     | Event-driven, small payloads         |
| Neural tensor   | 2 GB     | Large model weights, shared memory   |
