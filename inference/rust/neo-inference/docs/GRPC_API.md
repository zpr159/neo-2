# gRPC API Reference

## Overview

The `neo-inference` gRPC API provides a high-performance binary protocol for model inference, embedding, and management. gRPC is preferred for internal service-to-service communication, low-latency requirements, and streaming workloads. The server is configured via `GrpcConfig`.

### Server Configuration

```rust
pub struct GrpcConfig {
    pub bind_address: String,         // Default: "0.0.0.0"
    pub port: u16,                    // Default: 50051
    pub max_message_size: usize,      // Default: 64 MB
    pub concurrency_limit: usize,     // Default: 100
    pub keepalive_interval: Duration, // Default: 30 seconds
    pub request_timeout: Duration,    // Default: 120 seconds
}
```

## Service Definitions

The gRPC API defines four services:

```protobuf
syntax = "proto3";
package neo.inference;

service InferenceService {
  // Run a completion inference
  rpc Complete(CompleteRequest) returns (CompleteResponse);

  // Run a chat completion inference
  rpc ChatComplete(ChatCompleteRequest) returns (ChatCompleteResponse);

  // Streaming chat completion
  rpc ChatCompleteStream(ChatCompleteStreamRequest) returns (stream ChatStreamChunk);

  // Create embeddings
  rpc Embed(EmbedRequest) returns (EmbedResponse);

  // List loaded models
  rpc ListModels(ListModelsRequest) returns (ListModelsResponse);

  // Load a model
  rpc LoadModel(LoadModelRequest) returns (LoadModelResponse);

  // Unload a model
  rpc UnloadModel(UnloadModelRequest) returns (UnloadModelResponse);

  // Health check
  rpc Health(HealthRequest) returns (HealthResponse);

  // Get metrics
  rpc Metrics(MetricsRequest) returns (MetricsResponse);
}
```

## Inference RPCs

### Complete

Unary RPC for prompt-based completion.

```protobuf
message CompleteRequest {
  string model = 1;
  string prompt = 2;
  int32 max_tokens = 3;
  double temperature = 4;
  double top_p = 5;
  int32 top_k = 6;
  repeated string stop = 7;
  int64 seed = 8;
}

message CompleteResponse {
  string id = 1;
  repeated Choice choices = 2;
  Usage usage = 3;
  string model = 4;
}

message Choice {
  int32 index = 1;
  string text = 2;
  string finish_reason = 3;
}

message Usage {
  int64 prompt_tokens = 1;
  int64 completion_tokens = 2;
  int64 total_tokens = 3;
}
```

**Example (using grpcurl):**

```bash
grpcurl -plaintext -d '{
  "model": "llama-3-8b",
  "prompt": "The capital of France is",
  "max_tokens": 128,
  "temperature": 0.7
}' localhost:50051 neo.inference.InferenceService/Complete
```

### ChatComplete

Unary RPC for chat-style inference with message history.

```protobuf
message ChatCompleteRequest {
  string model = 1;
  repeated ChatMessage messages = 2;
  int32 max_tokens = 3;
  double temperature = 4;
  double top_p = 5;
  int32 top_k = 6;
  repeated string stop = 7;
  int64 seed = 8;
}

message ChatMessage {
  string role = 1;    // "system", "user", "assistant", "tool"
  string content = 2;
}

message ChatCompleteResponse {
  string id = 1;
  repeated ChatChoice choices = 2;
  Usage usage = 3;
  string model = 4;
}

message ChatChoice {
  int32 index = 1;
  ChatMessage message = 2;
  string finish_reason = 3;
}
```

**Example:**

```bash
grpcurl -plaintext -d '{
  "model": "llama-3-8b",
  "messages": [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "What is Rust?"}
  ],
  "max_tokens": 256,
  "temperature": 0.7
}' localhost:50051 neo.inference.InferenceService/ChatComplete
```

## Streaming RPCs

### ChatCompleteStream

Server-side streaming RPC for real-time token generation.

```protobuf
message ChatCompleteStreamRequest {
  string model = 1;
  repeated ChatMessage messages = 2;
  int32 max_tokens = 3;
  double temperature = 4;
  double top_p = 5;
  int32 top_k = 6;
  repeated string stop = 7;
  int64 seed = 8;
}

message ChatStreamChunk {
  string id = 1;
  repeated StreamChoice choices = 2;
  string model = 3;
}

message StreamChoice {
  int32 index = 1;
  StreamDelta delta = 2;
  string finish_reason = 3;   // null until final chunk
}

message StreamDelta {
  string role = 1;            // Set only in first chunk
  string content = 2;         // Token text
}
```

**Streaming Flow:**

```
Client                                    Server
  │                                         │
  │──── ChatCompleteStreamRequest ─────────►│
  │                                         │
  │◄─── StreamChunk { delta: {role:"assistant", content:""} } ──│
  │◄─── StreamChunk { delta: {content:"Rust"} } ───────────────│
  │◄─── StreamChunk { delta: {content:" is"} } ────────────────│
  │◄─── StreamChunk { delta: {content:" a"} } ─────────────────│
  │◄─── StreamChunk { delta: {}, finish_reason:"stop" } ───────│
  │                                         │
```

**Example (grpcurl with stream):**

```bash
grpcurl -plaintext -d '{
  "model": "llama-3-8b",
  "messages": [
    {"role": "user", "content": "Tell me a joke"}
  ],
  "max_tokens": 128
}' -stream localhost:50051 neo.inference.InferenceService/ChatCompleteStream
```

## Embedding RPC

### Embed

Create embeddings for one or more text inputs.

```protobuf
message EmbedRequest {
  string model = 1;
  repeated string input = 2;
  int32 dimensions = 3;
  bool normalize = 4;
}

message EmbedResponse {
  repeated EmbedData data = 1;
  string model = 2;
  Usage usage = 3;
}

message EmbedData {
  int32 index = 1;
  repeated float embedding = 2;
}
```

**Example:**

```bash
grpcurl -plaintext -d '{
  "model": "text-embedding-ada-002",
  "input": ["Hello world", "Testing embeddings"],
  "normalize": true
}' localhost:50051 neo.inference.InferenceService/Embed
```

## Model Management RPCs

### ListModels

```protobuf
message ListModelsRequest {}

message ListModelsResponse {
  repeated ModelInfo models = 1;
}

message ModelInfo {
  string id = 1;
  string name = 2;
  string version = 3;
  string architecture = 4;
  string format = 5;
  string quantization = 6;
  int64 parameter_count = 7;
  int32 context_length = 8;
  bool loaded = 9;
  int64 memory_bytes = 10;
}
```

### LoadModel

```protobuf
message LoadModelRequest {
  string id = 1;
  string name = 2;
  string version = 3;
  string architecture = 4;
  string format = 5;
  string quantization = 6;
  string path = 7;
  int64 file_size = 8;
  int64 parameter_count = 9;
  int32 num_layers = 10;
  int32 hidden_size = 11;
  int32 num_attention_heads = 12;
  int32 vocab_size = 13;
  int32 max_position_embeddings = 14;
  int32 context_length = 15;
}

message LoadModelResponse {
  string id = 1;
  string status = 2;     // "loaded", "already_loaded"
}
```

### UnloadModel

```protobuf
message UnloadModelRequest {
  string model_id = 1;
}

message UnloadModelResponse {
  string status = 1;     // "unloaded", "not_found"
}
```

## Health and Metrics RPCs

### Health

```protobuf
message HealthRequest {}

message HealthResponse {
  string status = 1;         // "healthy", "degraded", "unhealthy"
  string version = 2;
  int64 uptime_seconds = 3;
  int32 models_loaded = 4;
  int32 active_requests = 5;
  bool gpu_available = 6;
}
```

### Metrics

```protobuf
message MetricsRequest {}

message MetricsResponse {
  string timestamp = 1;
  int64 uptime_seconds = 2;
  LatencyMetrics latency = 3;
  ThroughputMetrics throughput = 4;
  repeated GpuMetrics gpu_metrics = 5;
  int64 total_requests = 6;
  int32 active_requests = 7;
  int32 models_loaded = 8;
  int64 memory_used_bytes = 9;
  int64 memory_total_bytes = 10;
  int32 queue_depth = 11;
}

message LatencyMetrics {
  double p50_ms = 1;
  double p90_ms = 2;
  double p95_ms = 3;
  double p99_ms = 4;
  double max_ms = 5;
  double mean_ms = 6;
}

message ThroughputMetrics {
  double requests_per_second = 1;
  double tokens_per_second = 2;
  double input_tokens_per_second = 3;
  double output_tokens_per_second = 4;
}

message GpuMetrics {
  uint32 device_id = 1;
  double utilization = 2;
  int64 memory_used_bytes = 3;
  int64 memory_total_bytes = 4;
  double memory_utilization = 5;
  double temperature_celsius = 6;
  double power_watts = 7;
}
```

## Error Handling

gRPC uses standard status codes for error handling:

| Status Code | Description | When Used |
|-------------|-------------|-----------|
| `OK` (0) | Success | Request completed successfully |
| `INVALID_ARGUMENT` (3) | Bad request | Invalid parameters, missing fields |
| `NOT_FOUND` (5) | Resource not found | Model ID does not exist |
| `ALREADY_EXISTS` (6) | Resource exists | Model already loaded |
| `RESOURCE_EXHAUSTED` (8) | Out of resources | Queue full, memory exhausted |
| `FAILED_PRECONDITION` (9) | Precondition failed | Backend not available |
| `ABORTED` (10) | Operation aborted | Request cancelled |
| `UNAVAILABLE` (14) | Service unavailable | Server shutting down |
| `INTERNAL` (13) | Internal error | Unexpected server error |

### Error Details

Error messages include descriptive text and structured details:

```protobuf
message ErrorDetail {
  string message = 1;
  string code = 2;
  map<string, google.protobuf.Value> details = 3;
}
```

## Keepalive and Connection Management

The server uses HTTP/2 keepalive to detect stale connections:

- **Keepalive interval**: 30 seconds (configurable)
- **Max message size**: 64 MB (configurable)
- **Concurrency limit**: 100 simultaneous streams (configurable)

### Client Configuration

```rust
// Recommended client settings
let channel = tonic::transport::Channel::from_static("http://localhost:50051")
    .keep_alive(true, Duration::from_secs(30))
    .timeout(Duration::from_secs(120))
    .max_message_length(64 * 1024 * 1024)
    .connect()
    .await?;
```

## Authentication

When authentication is enabled, gRPC metadata must include the API key:

```
metadata: {
  "x-api-key": "your-api-key-here"
}
```

### Client-Side Auth

```rust
use tonic::metadata::MetadataValue;

let api_key: MetadataValue<_> = "your-api-key".parse().unwrap();
let mut request = tonic::Request::new(CompleteRequest { ... });
request.metadata_mut().insert("x-api-key", api_key);
```
