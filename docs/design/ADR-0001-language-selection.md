# ADR-0001: Language Selection

## Status

Accepted

## Context

Neo AGI OS requires multiple programming languages to serve different subsystems with specific performance, safety, and ecosystem requirements. The system includes neural processing, robotic control, web interfaces, API gateways, and client SDKs.

Key requirements:
- GPU-accelerated neural processing with CUDA
- Real-time robotic control with safety guarantees
- High-concurrency API gateway
- Browser-based monitoring dashboard
- Client SDKs for Kotlin, Swift, TypeScript, Python
- Storage engine with crash safety
- Knowledge graph with graph algorithms

## Decision

We adopt a polyglot architecture with the following language assignments:

| Language | Subsystems |
|----------|-----------|
| Rust | Neural core, agent scheduler, knowledge graph, storage engine, observability |
| C++ | CUDA kernels, hardware abstraction layer |
| Python | ML training, data pipelines, knowledge graph ML rules |
| TypeScript | Web dashboard, CLI tools |
| Go | API gateway, device manager |
| Kotlin | Android SDK, robotics JVM bridge |
| Swift | iOS SDK, macOS interface |

## Consequences

### Positive

- Each subsystem uses the language best suited to its constraints
- Rust provides memory safety without garbage collection for performance-critical paths
- C++ enables direct CUDA kernel development
- Python leverages the ML ecosystem (PyTorch, Hugging Face)
- TypeScript provides browser-native development
- Go provides simple concurrency for I/O-bound services
- Kotlin and Swift provide native mobile SDKs

### Negative

- Increased build complexity (7 build systems)
- Cross-language debugging is harder
- Developer must know multiple languages
- Protobuf schemas must be maintained consistently across languages
- Integration testing across language boundaries is complex

### Mitigations

- Unified build script (`scripts/build.sh`) orchestrates all build systems
- Protobuf code generation ensures type consistency
- CI pipeline runs tests for all languages
- Developer onboarding guide covers all languages

## Alternatives Considered

### Option 1: All Rust

Pros: Single language, consistent tooling, memory safety.
Cons: No browser-native UI, CUDA kernel development requires C++, ML ecosystem is Python-centric, mobile SDKs need platform-native languages.
Rejected because: The ecosystem requirements make a single language impractical.

### Option 2: Rust + Python + TypeScript

Pros: Simpler than 7 languages, covers most use cases.
Cons: No native mobile SDKs, Go is better for API gateway concurrency, C++ is required for CUDA kernels.
Rejected because: Missing native mobile SDKs and C++ for CUDA.

### Option 3: All Go

Pros: Simple, fast compilation, great concurrency.
Cons: GC pauses unacceptable for neural core and robotics, no CUDA kernel support, no browser-native UI.
Rejected because: Performance and safety requirements exceed Go's capabilities.
