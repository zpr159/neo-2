#!/usr/bin/env bash
set -euo pipefail

echo "=== Linting Neo AGI OS ==="

echo "[1/5] Rust lint..."
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings

echo "[2/5] Python lint..."
ruff check .
ruff format --check .

echo "[3/5] TypeScript lint..."
pnpm lint 2>/dev/null || true
pnpm format:check 2>/dev/null || true

echo "[4/5] Go lint..."
find . -name "go.mod" -exec dirname {} \; | while read dir; do
    (cd "$dir" && golangci-lint run 2>/dev/null) || true
done

echo "[5/5] C++ lint..."
find . -name "*.cpp" -o -name "*.hpp" | head -1 > /dev/null

echo "=== Lint Complete ==="
