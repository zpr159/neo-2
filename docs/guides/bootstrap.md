# Neo AGI OS — Bootstrap Guide

## Prerequisites

### System Requirements

- **OS**: Ubuntu 22.04+ (recommended), macOS 14+, or WSL2 on Windows
- **CPU**: 8+ cores (16 recommended)
- **RAM**: 16GB minimum, 32GB recommended
- **Storage**: 100GB+ free (SSD required for Storage Engine)
- **GPU**: NVIDIA GPU with 16GB+ VRAM (for neural core development)

### Required Software

| Tool | Version | Purpose |
|------|---------|---------|
| Git | 2.40+ | Version control |
| curl | any | Downloading dependencies |
| sudo | any | System package installation |

## Step-by-Step Bootstrap

### Step 1: Clone the Repository

```bash
git clone <repository-url>
cd /run/media/rajesh/Rajesh/Neo_2.0
```

### Step 2: Run the Bootstrap Script

```bash
chmod +x scripts/bootstrap.sh
./scripts/bootstrap.sh
```

The script performs the following actions:

1. **Rust Installation** (if not present)
   - Installs rustup and stable Rust toolchain
   - Adds rustfmt, clippy, and rust-src components
   - Configures PATH

2. **Go Installation** (if not present)
   - Downloads Go 1.22 for your architecture
   - Installs to /usr/local/go
   - Configures PATH

3. **Python Setup**
   - Creates virtual environment in `.venv/`
   - Activates the environment
   - Installs ruff, mypy, pytest, pytest-cov, maturin

4. **Node.js and pnpm** (if not present)
   - Installs Node.js 20 LTS
   - Installs pnpm 9 globally

5. **CMake Check**
   - Verifies CMake >= 3.24 is installed
   - Installs via apt if missing

6. **Docker Check**
   - Warns if Docker is not installed

7. **CUDA Check**
   - Warns if CUDA toolkit is not installed

8. **Dependencies Installation**
   - Runs `pnpm install` for Node.js packages
   - Runs `cargo check` for Rust workspace

### Step 3: Verify Installation

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

### Step 4: First Build

```bash
./scripts/build.sh
```

This runs all build systems in dependency order. First build takes 5-15 minutes.

### Step 5: Run Tests

```bash
./scripts/test.sh
```

## Manual Installation

If you prefer to install tools manually:

### Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustup component add rustfmt clippy rust-src
```

### Go

```bash
GO_VERSION="1.22.0"
ARCH=$(uname -m)
case $ARCH in
    x86_64) ARCH="amd64" ;;
    aarch64) ARCH="arm64" ;;
esac
curl -sL "https://go.dev/dl/go${GO_VERSION}.linux-${ARCH}.tar.gz" | sudo tar -C /usr/local -xzf -
echo 'export PATH=$PATH:/usr/local/go/bin' >> "$HOME/.bashrc"
export PATH=$PATH:/usr/local/go/bin
```

### Python

```bash
sudo apt-get update && sudo apt-get install -y python3 python3-pip python3-venv
python3 -m venv .venv
source .venv/bin/activate
pip install --upgrade pip
pip install ruff mypy pytest pytest-cov maturin
```

### Node.js and pnpm

```bash
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt-get install -y nodejs
npm install -g pnpm@9
```

### CMake

```bash
sudo apt-get update && sudo apt-get install -y cmake
```

### CUDA Toolkit

Download from NVIDIA: https://developer.nvidia.com/cuda-downloads

## Troubleshooting

### "rustc: command not found"

```bash
source "$HOME/.cargo/env"
# Or add to shell profile:
echo 'source "$HOME/.cargo/env"' >> "$HOME/.bashrc"
```

### "go: command not found"

```bash
export PATH=$PATH:/usr/local/go/bin
# Or add to shell profile:
echo 'export PATH=$PATH:/usr/local/go/bin' >> "$HOME/.bashrc"
```

### "python3: No such file or directory"

```bash
sudo apt-get install -y python3 python3-pip python3-venv
```

### "pnpm: command not found"

```bash
npm install -g pnpm@9
```

### CUDA Not Detected

1. Verify CUDA installation: `ls /usr/local/cuda/bin/nvcc`
2. Add to PATH: `export PATH=/usr/local/cuda/bin:$PATH`
3. Verify: `nvcc --version`
4. Check GPU: `nvidia-smi`

### Cargo Build Fails with CUDA Errors

```bash
# Ensure CUDA is in PATH
export PATH=/usr/local/cuda/bin:$PATH
export LD_LIBRARY_PATH=/usr/local/cuda/lib64:$LD_LIBRARY_PATH

# Clean and rebuild
cargo clean
cargo build --workspace
```

### pnpm Install Fails

```bash
# Clear cache
pnpm store prune
rm -rf node_modules
pnpm install
```

### Python Virtual Environment Issues

```bash
# Recreate
rm -rf .venv
python3 -m venv .venv
source .venv/bin/activate
pip install --upgrade pip
pip install -e ".[dev]"
```

### Gradle Build Fails

```bash
cd sdk/kotlin
./gradlew clean
./gradlew build
```

### Disk Space Issues

```bash
# Check disk usage
df -h

# Clean build artifacts
cargo clean
rm -rf build/
pnpm -r clean
./gradlew clean
```

## Next Steps

After successful bootstrap:

1. Read [Getting Started](../developer/getting-started.md) for development workflow
2. Read [Coding Standards](../developer/coding-standards.md) for code style
3. Read [Architecture Overview](../architecture/README.md) for system design
4. Start the server: `cargo run --release -p neo-server`
5. Access the dashboard: `http://localhost:8080/dashboard`
