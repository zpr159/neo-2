# ADR-0003: Build System

## Status

Accepted

## Context

Neo AGI OS uses 7 programming languages, each with its own canonical build tool. We need a unified build approach that leverages native tools while providing a cohesive developer experience.

Key requirements:
- Support Rust, C++, Python, TypeScript, Go, Kotlin, Swift
- Cross-compilation for multiple targets
- CUDA compilation
- Protobuf code generation
- Build caching and incremental builds
- CI/CD integration

## Decision

We use each language's native build tool, orchestrated by a top-level shell script (`scripts/build.sh`).

### Build Tool Matrix

| Language | Build Tool | Config |
|----------|-----------|--------|
| Rust | Cargo | Cargo.toml, Cargo.lock |
| C++ | CMake + nvcc | CMakeLists.txt |
| Python | pip + maturin | pyproject.toml |
| TypeScript | pnpm + tsc | package.json |
| Go | go build | go.mod |
| Kotlin | Gradle | build.gradle.kts |
| Swift | SPM | Package.swift |

### Build Order

1. Protobuf code generation
2. C++ (CUDA kernels, HAL)
3. Rust (core, depends on C++ libs)
4. Python (ML framework, depends on Rust via PyO3)
5. Go (API gateway, device manager)
6. TypeScript (web dashboard)
7. Kotlin (Android SDK)
8. Swift (iOS SDK)

## Consequences

### Positive

- Each language uses its canonical tool with full feature support
- Developers familiar with a language can use its standard build commands
- Native dependency resolution for each ecosystem
- Build caching is tool-native (sccache for Rust, ccache for C++)

### Negative

- No single build command for the entire project (mitigated by `scripts/build.sh`)
- Build ordering must be maintained manually
- Cross-language build dependencies require coordination

### Mitigations

- `scripts/build.sh` provides a single entry point
- Makefile provides convenient targets
- CI validates build order
- Documentation explains the build process

## Alternatives Considered

### Option 1: Bazel

Pros: Unified build system, hermetic builds, excellent caching, cross-language support.
Cons: Steep learning curve, complex configuration, less mature ecosystem support for some languages (Kotlin, Swift), harder to onboard developers.
Rejected because: The learning curve and configuration complexity outweigh the benefits for our team size.

### Option 2: Single Language (Rust with FFI)

Rewrite everything in Rust, using FFI for C++ (CUDA) and Python (ML).

Pros: Single build system (Cargo), consistent tooling.
Cons: Massive rewrite effort, Python ML ecosystem inaccessible, TypeScript for web is impractical in Rust, mobile SDKs need native languages.
Rejected because: The rewrite cost is prohibitive and some ecosystems have no Rust equivalent.

### Option 3: Nix

Pros: Reproducible builds, declarative configuration, cross-compilation support.
Cons: Steep learning curve, smaller community, less tooling integration.
Rejected because: Insufficient community adoption for our team.
