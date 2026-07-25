# Neo AGI OS — Developer Getting Started Guide

## Prerequisites

### Hardware Requirements

- **CPU**: 8+ cores (16 recommended)
- **RAM**: 16GB minimum, 32GB recommended
- **Storage**: 100GB+ free disk space (SSD strongly recommended)
- **GPU**: NVIDIA GPU with 16GB+ VRAM (A100, RTX 4090, or better) for neural core development
- **Network**: Internet access for dependency downloads

### Software Requirements

| Tool       | Minimum Version | Purpose                        |
|-----------|----------------|--------------------------------|
| Rust      | 1.75+          | Core system, storage engine    |
| Go        | 1.22+          | API gateway, device manager    |
| Python    | 3.11+          | ML training, data pipelines    |
| Node.js   | 20+            | Web dashboard, CLI tools       |
| pnpm      | 9+             | Node.js package manager        |
| CMake     | 3.24+          | C++ build system               |
| CUDA      | 12.3+          | GPU acceleration               |
| Kotlin    | 1.9+           | Android SDK, robotics bridge   |
| Swift     | 5.9+           | iOS SDK (macOS only)           |
| Docker    | 24+            | Container builds               |

## Bootstrap

Run the bootstrap script to install all toolchains:

```bash
cd /run/media/rajesh/Rajesh/Neo_2.0
./scripts/bootstrap.sh
```

This script:

1. Installs Rust via rustup
2. Installs Go from official binaries
3. Creates a Python virtual environment and installs dependencies
4. Installs Node.js and pnpm
5. Checks for CMake, Docker, and CUDA
6. Installs Node.js dependencies via pnpm
7. Runs a basic cargo check

### Verifying Installation

```bash
./scripts/verify-toolchain.sh
```

Expected output:

```
=== Verifying Neo Toolchain ===
[OK] rustc: rustc 1.75.0 (82e1608df 2023-12-21)
[OK] cargo: cargo 1.75.0 (1d8b058dd 2023-11-20)
[OK] cmake: cmake version 3.28.1
[OK] python3: Python 3.11.7
[OK] node: v20.11.0
[OK] pnpm: 9.1.0
[OK] go: go version go1.22.0 linux/amd64
=== Done ===
```

## First Build

### Full Build

```bash
./scripts/build.sh
```

This runs all build systems in dependency order. Expect 5-15 minutes for a full build.

### Component Builds

Build individual components:

```bash
# Rust workspace only
cargo build --workspace

# C++ components only
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build --parallel $(nproc)

# TypeScript packages only
pnpm -r build

# Kotlin modules only
cd sdk/kotlin && ./gradlew build
```

## Running Tests

```bash
# All tests
./scripts/test.sh

# Rust tests only
cargo test --workspace

# Specific crate tests
cargo test -p neo-storage

# TypeScript tests
pnpm test

# Kotlin tests
cd sdk/kotlin && ./gradlew test
```

## Linting

```bash
# All linters
./scripts/lint.sh

# Rust lint
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings

# Python lint
ruff check .
ruff format --check .
```

## IDE Setup

### VS Code (Recommended)

Install the following extensions:

- `rust-analyzer` for Rust
- `golang.go` for Go
- `ms-python.python` for Python
- `dbaeumer.vscode-eslint` for TypeScript
- `mathiasfrohlich.Kotlin` for Kotlin
- `sswg.swift-lang` for Swift

### JetBrains IDEs

- **RustRover** or **IntelliJ IDEA** with Rust plugin
- **GoLand** for Go
- **PyCharm** for Python
- **WebStorm** for TypeScript
- **Android Studio** for Kotlin
- **AppCode** or **Xcode** for Swift

## Project Structure

```
Neo_2.0/
  core/                          # Rust core crates
    neural-core/                 # Neural processing
    agent-scheduler/             # Agent orchestration
    knowledge-graph/             # Knowledge management
    storage-engine/              # Persistent storage
    proto/                       # Protobuf definitions
  neural-network-framework/      # ML framework
    rust/                        # Rust runtime
    python/                      # Python training
    c++/                         # CUDA kernels
  api-gateway/                   # Go API server
  web-dashboard/                 # TypeScript frontend
  sdk/                           # Client SDKs
    kotlin/                      # Kotlin SDK
    swift/                       # Swift SDK
    typescript/                  # TypeScript SDK
    python/                      # Python SDK
  ui/                            # Native UIs
    kotlin/                      # Android
    swift/                       # iOS/macOS
  robotics/                      # Robotics control
    rust/                        # Core robotics
    c++/                         # HAL layer
    kotlin/                      # JVM bridge
  device-manager/                # Go device management
  observability/                 # Monitoring
    rust/                        # Metrics pipeline
    python/                      # Analysis
  proto/                         # Shared protobuf schemas
  scripts/                       # Build and utility scripts
  docs/                          # Documentation
  tests/                         # Integration tests
  deploy/                        # Deployment configs
```

## Common Development Tasks

### Adding a New Rust Crate

1. Create directory: `mkdir -p core/my-crate/src`
2. Create `Cargo.toml` with proper dependencies
3. Add to workspace members in root `Cargo.toml`
4. Run `cargo check -p my-crate`

### Adding a New TypeScript Package

1. Create directory: `mkdir -p web-dashboard/packages/my-package`
2. Create `package.json` with proper dependencies
3. Add to `pnpm-workspace.yaml`
4. Run `pnpm --filter my-package build`

### Adding a New Protobuf Service

1. Create `.proto` file in `proto/neo/`
2. Run `make proto` to generate code
3. Import generated code in your service

## Troubleshooting

### CUDA Not Found

If `nvcc` is not found:

```bash
# Check if CUDA is installed
ls /usr/local/cuda/bin/nvcc

# Add to PATH
export PATH=/usr/local/cuda/bin:$PATH
export LD_LIBRARY_PATH=/usr/local/cuda/lib64:$LD_LIBRARY_PATH
```

### Rust Build Fails

```bash
# Clean and rebuild
cargo clean
cargo build --workspace

# Check for missing dependencies
cargo check --workspace
```

### Node.js Version Issues

```bash
# Use nvm to manage versions
nvm install 20
nvm use 20
```

### Python Virtual Environment

```bash
# Recreate virtual environment
rm -rf .venv
python3 -m venv .venv
source .venv/bin/activate
pip install -e ".[dev]"
```

## Next Steps

1. Read [Coding Standards](coding-standards.md) for code style guidelines
2. Read [Testing Guide](testing-guide.md) for test writing instructions
3. Read [Architecture Overview](../architecture/README.md) for system design
4. Read [Bootstrap Guide](../guides/bootstrap.md) for detailed setup instructions
