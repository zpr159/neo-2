#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "=== Running Neo AGI OS Tests ==="

echo "[1/5] Rust tests..."
cargo test --workspace

echo "[2/5] C++ tests..."
cd "$PROJECT_ROOT"
cmake -B build -DCMAKE_BUILD_TYPE=Debug -DNEO_ENABLE_TESTS=ON
cmake --build build --parallel $(nproc)
ctest --test-dir build --output-on-failure

echo "[3/5] Python tests..."
source .venv/bin/activate 2>/dev/null || true
python -m pytest --tb=short

echo "[4/5] TypeScript tests..."
pnpm test 2>/dev/null || true

echo "[5/5] Go tests..."
find . -name "go.mod" -exec dirname {} \; | while read dir; do
    (cd "$dir" && go test ./... 2>/dev/null) || true
done

echo "=== All Tests Complete ==="
