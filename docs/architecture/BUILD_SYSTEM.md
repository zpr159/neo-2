# Neo AGI OS — Build System Documentation

## Table of Contents

- [1. Overview](#1-overview)
- [2. Build System Matrix](#2-build-system-matrix)
- [3. Cargo (Rust)](#3-cargo-rust)
- [4. CMake (C++)](#4-cmake-c)
- [5. Gradle (Kotlin)](#5-gradle-kotlin)
- [6. pnpm (TypeScript)](#6-pnpm-typescript)
- [7. pip (Python)](#7-pip-python)
- [8. Swift Package Manager](#8-swift-package-manager)
- [9. Go Modules](#9-go-modules)
- [10. Unified Build Orchestration](#10-unified-build-orchestration)
- [11. Cross-Compilation](#11-cross-compilation)
- [12. CI/CD Pipeline](#12-cicd-pipeline)
- [13. Dependency Resolution](#13-dependency-resolution)
- [14. Build Caching](#14-build-caching)

---

## 1. Overview

Neo AGI OS uses multiple build systems because it is a polyglot project. Each language has its own canonical build tool, and a top-level orchestration script coordinates them.

The build system must handle:

- 7 languages (Rust, C++, Python, TypeScript, Go, Kotlin, Swift)
- Cross-compilation for multiple targets
- CUDA compilation (requires nvcc)
- Shared protobuf code generation
- Dependency resolution across language boundaries

### Build Order

The build order is determined by inter-language dependencies:

```
1. Proto generation (protobuf)
2. C++ (CUDA kernels, HAL)
3. Rust (core, depends on C++ libs)
4. Python (ML framework, depends on Rust via PyO3)
5. Go (API gateway, device manager)
6. TypeScript (web dashboard, CLI)
7. Kotlin (Android SDK, robotics bridge)
8. Swift (iOS SDK)
```

---

## 2. Build System Matrix

| Language    | Build Tool     | Config File                    | Package Manager        |
|------------|----------------|--------------------------------|------------------------|
| Rust       | Cargo          | Cargo.toml, Cargo.lock         | crates.io              |
| C++        | CMake + nvcc   | CMakeLists.txt                 | vcpkg, FetchContent    |
| Python     | pip + maturin  | pyproject.toml, setup.cfg      | PyPI                   |
| TypeScript | pnpm + tsc     | package.json, tsconfig.json    | npm/pnpm registry      |
| Go         | go build       | go.mod, go.sum                 | Go modules proxy       |
| Kotlin     | Gradle         | build.gradle.kts               | Maven Central          |
| Swift      | SPM            | Package.swift                  | Swift Package Registry |

---

## 3. Cargo (Rust)

### 3.1 Workspace Structure

```toml
# Root Cargo.toml
[workspace]
members = [
    "core/*",
    "neural-network-framework/rust",
    "knowledge-graph/rust",
    "storage-engine",
    "observability/rust",
    "robotics/rust",
]
resolver = "2"

[workspace.dependencies]
tokio = { version = "1.35", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
prost = "0.12"
tonic = "0.11"
```

### 3.2 Key Cargo Commands

```bash
# Build all crates
cargo build --workspace

# Build in release mode
cargo build --workspace --release

# Run all tests
cargo test --workspace

# Check without building
cargo check --workspace

# Format code
cargo fmt --all

# Lint
cargo clippy --workspace --all-targets --all-features

# Build documentation
cargo doc --workspace --no-deps
```

### 3.3 Feature Flags

```toml
[features]
default = ["cpu"]
cpu = []
gpu = ["cuda-sys", "cudarc"]
avx512 = []
simd = ["target_feature"]

[dependencies.cuda-sys]
version = "0.1"
optional = true
```

### 3.4 Build Profiles

```toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
strip = "symbols"

[profile.dev]
opt-level = 0
debug = true
incremental = true
```

---

## 4. CMake (C++)

### 4.1 CMakeLists.txt Structure

```cmake
cmake_minimum_required(VERSION 3.24)
project(neo-cpp VERSION 1.0.0 LANGUAGES CXX CUDA)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CUDA_STANDARD 20)

option(NEO_ENABLE_TESTS "Enable tests" ON)
option(NEO_ENABLE_CUDA "Enable CUDA support" ON)

# Find CUDA
if(NEO_ENABLE_CUDA)
    find_package(CUDAToolkit REQUIRED)
    enable_language(CUDA)
endif()

# Core library
add_library(neo_core STATIC
    src/neural/activation.cpp
    src/neural/layer.cpp
    src/neural/cuda/ kernels.cu
    src/hal/device.cpp
    src/hal/serial.cpp
)

target_include_directories(neo_core PUBLIC include)
target_link_libraries(neo_core PUBLIC CUDA::cudart CUDA::cublas)

# Tests
if(NEO_ENABLE_TESTS)
    enable_testing()
    find_package(GTest REQUIRED)
    add_executable(neo_tests tests/test_main.cpp)
    target_link_libraries(neo_tests PRIVATE neo_core GTest::gtest_main)
    add_test(NAME neo_tests COMMAND neo_tests)
endif()
```

### 4.2 Key CMake Commands

```bash
# Configure
cmake -B build -DCMAKE_BUILD_TYPE=Release

# Build
cmake --build build --parallel $(nproc)

# Test
ctest --test-dir build --output-on-failure

# Install
cmake --install build --prefix /usr/local
```

### 4.3 CUDA Compilation

CUDA `.cu` files are compiled with nvcc. CMake handles the toolchain integration:

```cmake
set_source_files_properties(src/neural/cuda/kernels.cu PROPERTIES
    CUDA_SEPARABLE_COMPILATION ON
    CUDA_ARCHITECTURES "80;86;89;90"
)
```

---

## 5. Gradle (Kotlin)

### 5.1 build.gradle.kts

```kotlin
plugins {
    kotlin("jvm") version "1.9.22"
    kotlin("plugin.serialization") version "1.9.22"
}

group = "com.neo"
version = "1.0.0"

repositories {
    mavenCentral()
}

dependencies {
    implementation(kotlin("stdlib"))
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.6.3")
    implementation("io.ktor:ktor-client-core:2.3.7")
    implementation("io.ktor:ktor-client-cio:2.3.7")
    testImplementation(kotlin("test"))
}

kotlin {
    jvmToolchain(21)
}
```

### 5.2 Key Gradle Commands

```bash
# Build
./gradlew build

# Test
./gradlew test

# Clean
./gradlew clean

# Run
./gradlew run

# Fat JAR
./gradlew shadowJar
```

---

## 6. pnpm (TypeScript)

### 6.1 Package Structure

```
web-dashboard/
  package.json
  pnpm-workspace.yaml
  tsconfig.json
  packages/
    ui/
      package.json
    charts/
      package.json
    api-client/
      package.json
```

### 6.2 pnpm-workspace.yaml

```yaml
packages:
  - 'packages/*'
```

### 6.3 Key pnpm Commands

```bash
# Install dependencies
pnpm install

# Build all packages
pnpm -r build

# Test all packages
pnpm -r test

# Lint all packages
pnpm -r lint

# Add dependency to specific package
pnpm --filter @neo/ui add react

# Run script in specific package
pnpm --filter @neo/api-client build
```

### 6.4 TypeScript Configuration

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true,
    "outDir": "./dist"
  }
}
```

---

## 7. pip (Python)

### 7.1 pyproject.toml

```toml
[build-system]
requires = ["setuptools>=68.0", "wheel", "maturin>=1.4"]
build-backend = "maturin"

[project]
name = "neo-neural"
version = "1.0.0"
requires-python = ">=3.11"
dependencies = [
    "torch>=2.1",
    "numpy>=1.24",
    "pydantic>=2.0",
]

[project.optional-dependencies]
dev = ["pytest", "pytest-cov", "ruff", "mypy"]
```

### 7.2 Key Python Commands

```bash
# Create virtual environment
python3 -m venv .venv
source .venv/bin/activate

# Install dependencies
pip install -e ".[dev]"

# Build Rust extension (via maturin)
maturin develop

# Run tests
pytest

# Lint
ruff check .
ruff format .

# Type check
mypy .
```

---

## 8. Swift Package Manager

### 8.1 Package.swift

```swift
// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "NeoSDK",
    platforms: [.iOS(.v17), .macOS(.v14)],
    products: [
        .library(name: "NeoSDK", targets: ["NeoSDK"]),
    ],
    dependencies: [
        .package(url: "https://github.com/grpc/grpc-swift.git", from: "1.21.0"),
    ],
    targets: [
        .target(
            name: "NeoSDK",
            dependencies: [
                .product(name: "GRPC", package: "grpc-swift"),
            ]
        ),
        .testTarget(
            name: "NeoSDKTests",
            dependencies: ["NeoSDK"]
        ),
    ]
)
```

### 8.2 Key Swift Commands

```bash
# Build
swift build

# Test
swift test

# Run
swift run

# Generate Xcode project
swift package generate-xcodeproj
```

---

## 9. Go Modules

### 9.1 go.mod

```go
module github.com/neo-agi/api-gateway

go 1.22

require (
    github.com/grpc-ecosystem/grpc-gateway/v2 v2.19.0
    github.com/rs/cors v1.10.1
    google.golang.org/grpc v1.60.0
    google.golang.org/protobuf v1.32.0
)
```

### 9.2 Key Go Commands

```bash
# Build
go build ./...

# Test
go test ./...

# Lint
golangci-lint run

# Tidy dependencies
go mod tidy

# Generate protobuf
protoc --go_out=. --go-grpc_out=. proto/**/*.proto
```

---

## 10. Unified Build Orchestration

### 10.1 Top-Level Build Script

The `scripts/build.sh` script orchestrates all build systems in dependency order:

```bash
#!/usr/bin/env bash
set -euo pipefail

# 1. Generate protobuf code
# 2. Build C++ (CUDA kernels)
# 3. Build Rust workspace
# 4. Build Python packages (maturin)
# 5. Build Go services
# 6. Build TypeScript packages
# 7. Build Kotlin modules
# 8. Build Swift packages
```

### 10.2 Make Targets

```makefile
.PHONY: build test lint clean proto

build: proto
    cargo build --workspace --release
    cmake -B build -DCMAKE_BUILD_TYPE=Release
    cmake --build build --parallel $$(nproc)
    pnpm -r build
    ./gradlew build
    swift build

test:
    cargo test --workspace
    ctest --test-dir build --output-on-failure
    pnpm -r test
    ./gradlew test
    swift test

lint:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    ruff check .
    pnpm -r lint

clean:
    cargo clean
    rm -rf build/
    pnpm -r clean
    ./gradlew clean
    swift package clean

proto:
    protoc --proto_path=proto --rust_out=core/proto/src --tonic_out=core/proto/src proto/**/*.proto
    protoc --proto_path=proto --go_out=. --go-grpc_out=. proto/**/*.proto
```

---

## 11. Cross-Compilation

### 11.1 Rust Targets

```bash
# List available targets
rustup target list

# Add targets
rustup target add aarch64-unknown-linux-gnu
rustup target add x86_64-pc-windows-gnu
rustup target add aarch64-apple-darwin

# Cross-compile
cargo build --target aarch64-unknown-linux-gnu --release
```

### 11.2 Go Targets

```bash
# Cross-compile for Linux ARM64
GOOS=linux GOARCH=arm64 go build -o neo-server-arm64 ./cmd/server

# Cross-compile for Windows
GOOS=windows GOARCH=amd64 go build -o neo-server.exe ./cmd/server
```

### 11.3 CMake Cross-Compilation

```bash
# ARM64 cross-compilation
cmake -B build-arm64 \
    -DCMAKE_TOOLCHAIN_FILE=cmake/aarch64-toolchain.cmake \
    -DCMAKE_BUILD_TYPE=Release

# RISC-V cross-compilation
cmake -B build-riscv64 \
    -DCMAKE_TOOLCHAIN_FILE=cmake/riscv64-toolchain.cmake \
    -DCMAKE_BUILD_TYPE=Release
```

### 11.4 Docker Multi-Stage Builds

```dockerfile
# Build stage
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl

# Runtime stage
FROM alpine:3.19
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/neo-server /usr/local/bin/
ENTRYPOINT ["neo-server"]
```

---

## 12. CI/CD Pipeline

### 12.1 GitHub Actions Workflow

```yaml
name: CI
on: [push, pull_request]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Rust lint
        run: |
          cargo fmt --all -- --check
          cargo clippy --workspace --all-targets -- -D warnings
      - name: Python lint
        run: |
          ruff check .
          ruff format --check .

  test:
    runs-on: ubuntu-latest
    needs: lint
    steps:
      - uses: actions/checkout@v4
      - name: Rust tests
        run: cargo test --workspace
      - name: C++ tests
        run: |
          cmake -B build -DCMAKE_BUILD_TYPE=Debug -DNEO_ENABLE_TESTS=ON
          cmake --build build
          ctest --test-dir build --output-on-failure

  build:
    runs-on: ubuntu-latest
    needs: test
    steps:
      - uses: actions/checkout@v4
      - name: Full build
        run: ./scripts/build.sh
```

---

## 13. Dependency Resolution

### 13.1 Inter-Language Dependencies

```
Rust <--FFI--> C++ (CUDA kernels)
Rust <--PyO3--> Python (ML bindings)
Go <--gRPC--> Rust (service calls)
TypeScript <--WebSocket--> Go (API calls)
Kotlin <--gRPC--> Go (SDK calls)
Swift <--gRPC--> Go (SDK calls)
```

### 13.2 Shared Protobuf Schemas

All language-specific protobuf code is generated from the same `.proto` files in the `proto/` directory. This ensures type consistency across language boundaries.

```bash
# Generate all protobuf code
make proto
```

### 13.3 Version Pinning

- **Rust**: `Cargo.lock` (committed)
- **TypeScript**: `pnpm-lock.yaml` (committed)
- **Python**: `requirements.txt` or `pyproject.toml` (committed)
- **Go**: `go.sum` (committed)
- **Kotlin**: `gradle.lockfile` (generated, committed)
- **Swift**: `Package.resolved` (generated, committed)

---

## 14. Build Caching

### 14.1 Rust

```toml
# .cargo/config.toml
[build]
# Use sccache for build caching
# RUSTC_WRAPPER=sccache
```

### 14.2 C++

```bash
# Use ccache
cmake -B build -DCMAKE_CXX_COMPILER_LAUNCHER=ccache
```

### 14.3 TypeScript

```bash
# pnpm store cache
pnpm store path
# ~/.local/share/pnpm/store/v3
```

### 14.4 Docker Layer Caching

```bash
# Build with cache
docker build --cache-from type=gha --cache-to type=gha,mode=max .
```
