#!/usr/bin/env bash
set -euo pipefail
echo "=== Verifying Neo Toolchain ==="
for cmd in rustc cargo cmake python3 node pnpm go; do
    if command -v $cmd &> /dev/null; then
        echo "[OK] $cmd: $($cmd --version 2>&1 | head -1)"
    else
        echo "[MISSING] $cmd"
    fi
done
echo "=== Done ==="
