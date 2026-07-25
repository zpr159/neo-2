# Neo — Artificial General Intelligence Operating System

> **Version:** 0.1.0 (Foundation)
> **Status:** Architecture & Foundation
> **License:** AGPL-3.0-or-later

---

## What Is Neo?

Neo is not a chatbot. Neo is not a wrapper around an LLM. Neo is a **production-grade Artificial General Intelligence Operating System** — a complete runtime environment for autonomous intelligent agents that reason, learn, plan, execute tools, coordinate across distributed systems, and improve themselves over time.

Neo is designed to scale from a single laptop to cloud clusters of thousands of nodes, providing a unified substrate for AGI-class workloads.

### Core Capabilities

| Domain | Description |
|---|---|
| **Reasoning** | Multi-paradigm reasoning engine (logical, causal, analogical, probabilistic) |
| **Learning** | Continuous online and offline learning with experience replay |
| **Planning** | Hierarchical task decomposition, scheduling, and plan execution |
| **Tool Use** | Dynamic tool discovery, invocation, and composition |
| **Autonomous Execution** | Self-directed goal pursuit with safety constraints |
| **Distributed Computing** | Horizontally scalable agent execution across clusters |
| **Neural Computation** | GPU-accelerated neural network inference and training |
| **Multimodal Intelligence** | Unified processing of text, image, audio, video, and structured data |
| **Memory** | Multi-tier memory system (working, episodic, semantic, procedural) |
| **Knowledge Graph** | Dynamic knowledge representation and relational reasoning |
| **Robotics** | Real-time control loops and sensor fusion for physical agents |
| **Business Automation** | Enterprise workflow orchestration and decision support |
| **Self-Improvement** | Meta-cognitive monitoring and autonomous capability expansion |

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                          UI / SDK Layer                         │
│         (TypeScript, Kotlin, Swift, Python, Go)                 │
├─────────────────────────────────────────────────────────────────┤
│                        Plugin System                            │
├──────────┬──────────┬──────────┬──────────┬────────────────────┤
│          │          │          │          │                      │
│  Agents  │Workflows │  Tools   │Business  │   Robotics          │
│  (Rust)  │(Rust/TS) │(Rust/Go) │(Rust/TS) │ (Rust/C++/Kotlin)  │
│          │          │          │          │                      │
├──────────┴──────────┴──────────┴──────────┴────────────────────┤
│                     Executive Layer (Rust/Go)                   │
├─────────────────────────────────────────────────────────────────┤
│               Reasoning  │  Memory  │  Knowledge Graph          │
│               (Rust)      │  (Rust)  │  (Rust)                  │
├─────────────────────────────────────────────────────────────────┤
│              Neural Engine  │  Inference  │  NN Framework       │
│              (Rust/CUDA/C++)│  (Rust/CUDA)│ (Rust/Python/CUDA)  │
├─────────────────────────────────────────────────────────────────┤
│            Distributed Computing (Rust/Go/C++)                  │
├─────────────────────────────────────────────────────────────────┤
│                    Security Layer (Rust/Go)                      │
├─────────────────────────────────────────────────────────────────┤
│                 Runtime (Rust/C++/WASM)                          │
├─────────────────────────────────────────────────────────────────┤
│                      Core (Rust/C++)                             │
└─────────────────────────────────────────────────────────────────┘
```

---

## Language Selection Rationale

| Language | Role | Justification |
|---|---|---|
| **Rust** | Primary system language | Memory safety without GC, zero-cost abstractions, fearless concurrency, native performance for core engine, neural operations, and all performance-critical paths |
| **C++** | Performance-critical subsystems | GPU driver interfaces, CUDA interop, robotics real-time control, legacy ML library integration (cuDNN, TensorRT) |
| **CUDA** | GPU kernel development | Direct GPU programming for neural network operations, custom kernels, and hardware-accelerated inference |
| **Go** | Distributed systems, services | Excellent concurrency primitives (goroutines), fast compilation, strong standard library for networking, consensus protocols, and cluster management |
| **Python** | ML/AI ecosystem integration | Unmatched ML library ecosystem (PyTorch, JAX, NumPy), rapid prototyping, SDK layer, plugin development |
| **TypeScript** | UI, workflows, SDK | Type safety for web UIs, Node.js runtime for workflow orchestration, npm ecosystem |
| **Kotlin** | Android UI, SDK | Native Android development, JetBrains ecosystem, coroutine support |
| **Swift** | iOS/macOS UI, SDK | Apple platform native, performance, safety |
| **CMake** | C/C++ build system | Industry standard, cross-platform, CUDA integration |
| **Cargo** | Rust build system | Dependency management, workspace support, testing, benchmarking |

---

## Repository Structure

```
neo/
├── core/                    # Core runtime primitives, type system, error handling
├── runtime/                 # Execution runtime (native, WASM)
├── neural-engine/           # GPU-accelerated neural computation engine
├── neural-network-framework/# Neural network definition and training framework
├── inference/               # Model inference engine
├── memory/                  # Multi-tier memory subsystem
├── knowledge-graph/         # Knowledge representation and graph database
├── reasoning/               # Multi-paradigm reasoning engine
├── executive/               # Goal management, planning, task execution
├── capabilities/            # Capability registry and lifecycle management
├── agents/                  # Autonomous agent framework
├── workflows/               # Workflow orchestration engine
├── tools/                   # Tool registry and execution framework
├── distributed/             # Distributed computing, cluster management
├── robotics/                # Robot control, sensor fusion, planning
├── business/                # Business automation, workflow, analytics
├── security/                # Auth, encryption, audit, sandboxing
├── ui/                      # User interfaces (Web, Android, iOS)
├── sdk/                     # Client SDKs (Python, Rust, TS, Go, Kotlin, Swift)
├── plugins/                 # Plugin system and built-in plugins
├── testing/                 # Shared test utilities, mocks, benchmarks
├── docs/                    # Documentation
├── deployment/              # Docker, Kubernetes, Terraform
├── benchmarks/              # Performance benchmarks
├── config/                  # Configuration files and profiles
└── scripts/                 # Build, CI, and utility scripts
```

---

## Quick Start

### Prerequisites

- **Rust** >= 1.75 (via rustup)
- **Go** >= 1.22
- **Python** >= 3.11
- **Node.js** >= 20 LTS
- **CMake** >= 3.28
- **CUDA Toolkit** >= 12.3 (for GPU support)
- **Docker** >= 24.0 (for containerized builds)

### Bootstrap

```bash
# Clone the repository
git clone https://github.com/neo-agi/neo.git
cd neo

# Run the bootstrap script (installs all toolchains and dependencies)
./scripts/bootstrap.sh

# Build the entire project
./scripts/build.sh

# Run the full test suite
./scripts/test.sh
```

### Building Individual Subsystems

```bash
# Build only Rust crates
cargo build --workspace

# Build C++ components
cmake --build build/

# Build TypeScript packages
pnpm -r build

# Build Python packages
pip install -e ./neural-network-framework/python
```

---

## Configuration

Neo uses a layered configuration system:

1. **Built-in defaults** — compiled into the binary
2. **Environment profiles** — `config/environments/{development,staging,production}.toml`
3. **Environment variables** — `NEO_*` prefixed
4. **Local overrides** — `.neo/config.toml` (git-ignored)
5. **Secrets** — `config/secrets/` (encrypted at rest, never committed)

```bash
# Set environment
export NEO_ENV=development

# Or use the CLI
neo config set environment production
```

---

## Documentation

- [Architecture Documentation](docs/architecture/)
- [API Reference](docs/api/)
- [Developer Guide](docs/developer/)
- [User Guide](docs/user/)
- [Design Documents](docs/design/)
- [Contributing Guide](CONTRIBUTING.md)

---

## Security

See [SECURITY.md](SECURITY.md) for vulnerability reporting. See [security/](security/) for the security subsystem implementation.

---

## License

Copyright (c) 2024 Rajesh Pawar. All rights reserved.

This software and associated documentation files (the "Software") are
confidential and proprietary to Rajesh Pawar.

No part of this Software may be reproduced, distributed, modified,
transmitted, reused, republished, downloaded, or otherwise used in any
form or by any means without prior written permission from Rajesh Pawar.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL
THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR
OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,
ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
OTHER DEALINGS IN THE SOFTWARE.


---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, coding standards, and contribution workflow.

---

<p align="center">
  <strong>Neo</strong> — An operating system for artificial general intelligence.
</p>
