# Neo AGI OS — User Guide

## What is Neo AGI OS?

Neo AGI OS is an operating system designed to orchestrate artificial general intelligence workloads. It provides a unified platform for managing neural networks, intelligent agents, knowledge graphs, and robotic systems.

### Key Features

- **Neural Processing**: Run inference and training on state-of-the-art models with GPU acceleration
- **Agent Orchestration**: Deploy and manage intelligent agents that process tasks autonomously
- **Knowledge Management**: Store and query structured knowledge as a graph with semantic search
- **Robotics Control**: Control robotic hardware with real-time safety guarantees
- **Multi-Language SDKs**: Use Neo from Kotlin, Swift, TypeScript, or Python
- **Web Dashboard**: Monitor and manage everything through a browser-based interface

## Getting Started

### Installation

#### Using the Bootstrap Script (Recommended)

```bash
git clone <repository-url>
cd Neo_2.0
./scripts/bootstrap.sh
```

#### Using Docker

```bash
docker pull neo-agi/neo-server:latest
docker run -p 8080:8080 neo-agi/neo-server:latest
```

### Starting the Server

```bash
# Start the full Neo AGI OS server
./scripts/build.sh
cargo run --release -p neo-server
```

The server starts on `http://localhost:8080` by default.

### Accessing the Dashboard

Open your browser and navigate to:

```
http://localhost:8080/dashboard
```

## Configuration

### Configuration File

Neo AGI OS is configured via `neo.toml`:

```toml
[server]
host = "0.0.0.0"
port = 8080

[neural]
model_dir = "/var/neo/models"
gpu_device = 0
max_batch_size = 32

[storage]
data_dir = "/var/neo/data"
max_size_gb = 100

[auth]
enabled = true
jwt_secret = "your-secret-key"

[logging]
level = "info"
format = "json"
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `NEO_HOST` | Server bind address | `0.0.0.0` |
| `NEO_PORT` | Server port | `8080` |
| `NEO_DATA_DIR` | Data directory | `/var/neo/data` |
| `NEO_MODEL_DIR` | Model directory | `/var/neo/models` |
| `NEO_GPU_DEVICE` | GPU device index | `0` |
| `NEO_LOG_LEVEL` | Logging level | `info` |

## Using the SDKs

### Kotlin (Android/JVM)

```kotlin
val client = NeoClient(host = "localhost", port = 8080)
client.connect()

val agent = client.createAgent(name = "my-agent")
val task = client.submitTask(
    agentId = agent.id,
    task = mapOf("action" to "process", "data" to "hello")
)
```

### Swift (iOS/macOS)

```swift
let client = NeoClient(host: "localhost", port: 8080)
client.connect()

let agent = client.createAgent(name: "my-agent")
let task = client.submitTask(
    agentId: agent.id,
    payload: ["action": "process", "data": "hello"]
)
```

### TypeScript (Node.js/Browser)

```typescript
const client = new NeoClient({ host: "localhost", port: 8080 });
await client.connect();

const agent = await client.createAgent({ name: "my-agent" });
const task = await client.submitTask({
    agentId: agent.id,
    payload: { action: "process", data: "hello" },
});
```

### Python

```python
from neo_sdk import NeoClient

client = NeoClient(host="localhost", port=8080)
client.connect()

agent = client.create_agent(name="my-agent")
task = client.submit_task(
    agent_id=agent.id,
    task={"action": "process", "data": "hello"}
)
```

## API Reference

### Authentication

```bash
# Login
curl -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "password"}'

# Use token
curl http://localhost:8080/api/v1/agents \
  -H "Authorization: Bearer <token>"
```

### Agents

```bash
# List agents
curl http://localhost:8080/api/v1/agents

# Create agent
curl -X POST http://localhost:8080/api/v1/agents \
  -H "Content-Type: application/json" \
  -d '{"name": "my-agent", "config": {}}'

# Get agent status
curl http://localhost:8080/api/v1/agents/<id>

# Delete agent
curl -X DELETE http://localhost:8080/api/v1/agents/<id>
```

### Tasks

```bash
# Submit task
curl -X POST http://localhost:8080/api/v1/agents/<id>/tasks \
  -H "Content-Type: application/json" \
  -d '{"action": "process", "data": "hello"}'

# Get task result
curl http://localhost:8080/api/v1/tasks/<id>
```

### Models

```bash
# List models
curl http://localhost:8080/api/v1/models

# Run inference
curl -X POST http://localhost:8080/api/v1/models/<id>/infer \
  -H "Content-Type: application/json" \
  -d '{"prompt": "Hello, world!", "max_tokens": 100}'
```

## Troubleshooting

### Server Won't Start

1. Check if port 8080 is already in use: `lsof -i :8080`
2. Check CUDA availability: `nvidia-smi`
3. Check data directory permissions: `ls -la /var/neo/data`

### GPU Out of Memory

1. Reduce `max_batch_size` in configuration
2. Use a smaller model
3. Check GPU memory: `nvidia-smi`

### Slow Inference

1. Ensure GPU is being used (check `nvidia-smi` during inference)
2. Increase batch size for throughput
3. Check network latency to the server

### Connection Refused

1. Verify server is running: `curl http://localhost:8080/api/v1/system/health`
2. Check firewall rules
3. Verify host and port in client configuration
