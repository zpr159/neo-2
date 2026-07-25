# REST API Reference

## Overview

The `neo-inference` REST API provides an OpenAI-compatible HTTP interface for model inference, embedding, and management. The API server is configured via `RestConfig` and supports CORS, authentication, rate limiting, and request timeouts.

### Server Configuration

```rust
pub struct RestConfig {
    pub bind_address: String,       // Default: "0.0.0.0"
    pub port: u16,                  // Default: 8080
    pub max_request_size: usize,    // Default: 10 MB
    pub enable_cors: bool,          // Default: true
    pub request_timeout: Duration,  // Default: 120 seconds
    pub enable_auth: bool,          // Default: false
    pub api_key_header: String,     // Default: "X-API-Key"
    pub rate_limit_per_second: u64, // Default: 100
}
```

## POST /v1/completions

Create a completion for a prompt.

### Request

```json
{
  "model": "llama-3-8b",
  "prompt": "The capital of France is",
  "max_tokens": 128,
  "temperature": 0.7,
  "top_p": 0.9,
  "top_k": 50,
  "stream": false,
  "stop": ["\n\n"],
  "seed": 42
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `model` | string | Yes | Model name or ID |
| `prompt` | string | Yes | The text prompt |
| `max_tokens` | integer | No | Maximum tokens to generate (default: 512) |
| `temperature` | float | No | Sampling temperature (default: 1.0) |
| `top_p` | float | No | Nucleus sampling threshold |
| `top_k` | integer | No | Top-K sampling parameter |
| `stream` | boolean | No | Enable streaming response |
| `stop` | string[] | No | Stop sequences |
| `seed` | integer | No | Random seed for reproducibility |

### Response

```json
{
  "id": "cmpl-550e8400-e9fc-4e4a-9a39-e0abc5f12345",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Paris is the capital and largest city of France."
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 7,
    "completion_tokens": 14,
    "total_tokens": 21
  },
  "model": "llama-3-8b"
}
```

## POST /v1/chat/completions

Create a chat completion with a conversation history.

### Request

```json
{
  "model": "llama-3-8b",
  "messages": [
    { "role": "system", "content": "You are a helpful assistant." },
    { "role": "user", "content": "What is the meaning of life?" }
  ],
  "max_tokens": 256,
  "temperature": 0.7,
  "stream": false
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `model` | string | Yes | Model name or ID |
| `messages` | ChatMessage[] | Yes | Conversation history |
| `max_tokens` | integer | No | Maximum tokens to generate |
| `temperature` | float | No | Sampling temperature |
| `top_p` | float | No | Nucleus sampling threshold |
| `top_k` | integer | No | Top-K sampling parameter |
| `stream` | boolean | No | Enable streaming response |
| `stop` | string[] | No | Stop sequences |
| `seed` | integer | No | Random seed |

### ChatMessage

```json
{
  "role": "user | assistant | system | tool",
  "content": "Message text"
}
```

### Response

```json
{
  "id": "chatcmpl-550e8400-e9fc-4e4a-9a39-e0abc5f12345",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "The meaning of life is a philosophical question..."
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 25,
    "completion_tokens": 150,
    "total_tokens": 175
  },
  "model": "llama-3-8b"
}
```

### Streaming Response (SSE)

When `stream: true`, the response is a series of Server-Sent Events:

```
data: {"id":"chatcmpl-...","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}],"model":"llama-3-8b"}

data: {"id":"chatcmpl-...","choices":[{"index":0,"delta":{"content":"The"},"finish_reason":null}],"model":"llama-3-8b"}

data: {"id":"chatcmpl-...","choices":[{"index":0,"delta":{"content":" meaning"},"finish_reason":null}],"model":"llama-3-8b"}

...

data: {"id":"chatcmpl-...","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"model":"llama-3-8b"}

data: [DONE]
```

### StreamChunk Schema

```json
{
  "id": "chatcmpl-...",
  "choices": [
    {
      "index": 0,
      "delta": {
        "role": "assistant",
        "content": "token text"
      },
      "finish_reason": null | "stop" | "max_tokens"
    }
  ],
  "model": "llama-3-8b"
}
```

## POST /v1/embeddings

Create embeddings for input text.

### Request

```json
{
  "model": "text-embedding-ada-002",
  "input": [
    "The quick brown fox jumps over the lazy dog",
    "A second text to embed"
  ],
  "dimensions": 1536,
  "normalize": true
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `model` | string | Yes | Embedding model name |
| `input` | string[] | Yes | Texts to embed |
| `dimensions` | integer | No | Output dimensions |
| `normalize` | boolean | No | L2-normalize vectors |

### Response

```json
{
  "data": [
    {
      "index": 0,
      "embedding": [0.0023, -0.0091, 0.0156, "..."]
    },
    {
      "index": 1,
      "embedding": [0.0045, 0.0012, -0.0078, "..."]
    }
  ],
  "model": "text-embedding-ada-002",
  "usage": {
    "prompt_tokens": 16,
    "completion_tokens": 0,
    "total_tokens": 16
  }
}
```

## GET /v1/models

List all registered models.

### Response

```json
{
  "data": [
    {
      "id": "550e8400-e9fc-4e4a-9a39-e0abc5f12345",
      "name": "llama-3-8b",
      "version": "1.0.0",
      "architecture": "llama",
      "format": "safetensors",
      "quantization": "fp16",
      "parameter_count": 8000000000,
      "context_length": 8192,
      "loaded": true,
      "memory_bytes": 16000000000
    }
  ]
}
```

## POST /v1/models/load

Load a model into memory.

### Request

```json
{
  "id": "550e8400-e9fc-4e4a-9a39-e0abc5f12345",
  "name": "llama-3-8b",
  "version": "1.0.0",
  "architecture": "llama",
  "format": "safetensors",
  "quantization": "fp16",
  "path": "./models/llama-3-8b",
  "file_size": 16000000000,
  "parameter_count": 8000000000,
  "num_layers": 32,
  "hidden_size": 4096,
  "num_attention_heads": 32,
  "vocab_size": 32000,
  "max_position_embeddings": 8192,
  "context_length": 8192
}
```

### Response

```json
{
  "id": "550e8400-e9fc-4e4a-9a39-e0abc5f12345",
  "status": "loaded"
}
```

## POST /v1/models/unload

Unload a model from memory.

### Request

```json
{
  "model_id": "550e8400-e9fc-4e4a-9a39-e0abc5f12345"
}
```

### Response

```json
{
  "status": "unloaded"
}
```

## GET /health

Health check endpoint.

### Response

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime_seconds": 3600,
  "models_loaded": 3,
  "active_requests": 2,
  "gpu_available": true
}
```

## GET /metrics

Telemetry and metrics endpoint.

### Response

```json
{
  "timestamp": "2026-07-22T10:30:00Z",
  "uptime_seconds": 3600,
  "latency": {
    "p50_ms": 45.2,
    "p90_ms": 120.5,
    "p95_ms": 180.3,
    "p99_ms": 350.1,
    "max_ms": 1200.0,
    "mean_ms": 67.8
  },
  "throughput": {
    "requests_per_second": 12.5,
    "tokens_per_second": 2500.0,
    "input_tokens_per_second": 1500.0,
    "output_tokens_per_second": 1000.0,
    "batches_per_second": 1.2
  },
  "gpu_metrics": [
    {
      "device_id": 0,
      "utilization": 0.85,
      "memory_used_bytes": 8000000000,
      "memory_total_bytes": 16000000000,
      "memory_utilization": 0.5,
      "temperature_celsius": 72.0,
      "power_watts": 250.0
    }
  ],
  "total_requests": 15000,
  "active_requests": 2,
  "models_loaded": 3,
  "memory_used_bytes": 24000000000,
  "memory_total_bytes": 64000000000,
  "queue_depth": 5
}
```

## Error Responses

All error responses follow a consistent schema:

```json
{
  "error": {
    "message": "Model not found: llama-3-8b",
    "code": "not_found",
    "status": 404,
    "details": null
  }
}
```

### Error Codes

| Status | Code | Description |
|--------|------|-------------|
| 400 | `bad_request` | Invalid request parameters |
| 404 | `not_found` | Model or resource not found |
| 429 | `rate_limited` | Rate limit exceeded |
| 500 | `internal_error` | Server-side error |

### Predefined Error Responses

```rust
ErrorResponse::not_found("Model not found: llama-3-8b");
ErrorResponse::bad_request("Invalid temperature value");
ErrorResponse::internal("Backend initialization failed");
ErrorResponse::rate_limited();
```

## Authentication

When `enable_auth` is true, requests must include an API key in the specified header:

```
X-API-Key: your-api-key-here
```

The header name is configurable via `api_key_header` in `RestConfig`.

## Rate Limiting

Rate limiting is enforced per-IP (or per-API key when auth is enabled):

- **Default**: 100 requests per second
- **Configurable**: via `rate_limit_per_second` in `RestConfig`
- **Response**: `429 Too Many Requests` with `rate_limited` error code

## CORS

When `enable_cors` is true, the server includes standard CORS headers:

```
Access-Control-Allow-Origin: *
Access-Control-Allow-Methods: GET, POST, OPTIONS
Access-Control-Allow-Headers: Content-Type, X-API-Key
```

## Request Timeout

Requests that exceed `request_timeout` (default: 120 seconds) are terminated and return a timeout error.
