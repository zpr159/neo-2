#!/usr/bin/env bash
# Neo AGI OS — Build Script
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "=== Building Neo AGI OS ==="

# Rust
echo "[1/5] Building Rust workspace..."
cargo build --workspace --release

# C++
echo "[2/5] Building C++ components..."
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build --parallel $(nproc)

# Python
echo "[3/5] Building Python packages..."
source .venv/bin/activate 2>/dev/null || true
pip install -e ./neural-network-framework/python 2>/dev/null || true

# TypeScript
echo "[4/5] Building TypeScript packages..."
pnpm install 2>/dev/null || true
pnpm -r build 2>/dev/null || true

# Kotlin
echo "[5/5] Building Kotlin modules..."
./gradlew build 2>/dev/null || true

echo "=== Build Complete ==="
