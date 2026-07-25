#!/usr/bin/env bash
# Neo AGI OS — Bootstrap Script
# Installs all toolchains and dependencies for development.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "=== Neo AGI OS — Development Bootstrap ==="
echo "Project root: $PROJECT_ROOT"
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { echo -e "${GREEN}[INFO]${NC}  $1"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; }

check_command() {
    command -v "$1" &> /dev/null
}

install_rust() {
    if check_command rustc; then
        info "Rust already installed: $(rustc --version)"
    else
        info "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
        source "$HOME/.cargo/env"
    fi
    
    rustup component add rustfmt clippy rust-src 2>/dev/null || true
    info "Rust components installed"
}

install_go() {
    if check_command go; then
        info "Go already installed: $(go version)"
    else
        info "Installing Go..."
        GO_VERSION="1.22.0"
        ARCH=$(uname -m)
        case $ARCH in
            x86_64) ARCH="amd64" ;;
            aarch64) ARCH="arm64" ;;
        esac
        curl -sL "https://go.dev/dl/go${GO_VERSION}.linux-${ARCH}.tar.gz" | sudo tar -C /usr/local -xzf -
        echo 'export PATH=$PATH:/usr/local/go/bin' >> "$HOME/.bashrc"
        export PATH=$PATH:/usr/local/go/bin
    fi
    info "Go: $(go version)"
}

install_python() {
    if check_command python3; then
        PYTHON_VER=$(python3 --version 2>&1 | awk '{print $2}')
        info "Python already installed: $PYTHON_VER"
    else
        info "Installing Python..."
        sudo apt-get update && sudo apt-get install -y python3 python3-pip python3-venv
    fi
    
    info "Setting up Python virtual environment..."
    python3 -m venv "$PROJECT_ROOT/.venv"
    source "$PROJECT_ROOT/.venv/bin/activate"
    pip install --upgrade pip
    pip install ruff mypy pytest pytest-cov maturin
    info "Python environment ready"
}

install_node() {
    if check_command node; then
        info "Node.js already installed: $(node --version)"
    else
        info "Installing Node.js..."
        curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
        sudo apt-get install -y nodejs
    fi
    
    if check_command pnpm; then
        info "pnpm already installed: $(pnpm --version)"
    else
        info "Installing pnpm..."
        npm install -g pnpm@9
    fi
}

install_cmake() {
    if check_command cmake; then
        info "CMake already installed: $(cmake --version | head -1)"
    else
        info "Installing CMake..."
        sudo apt-get update && sudo apt-get install -y cmake
    fi
}

install_docker() {
    if check_command docker; then
        info "Docker already installed: $(docker --version)"
    else
        warn "Docker not installed. Please install Docker manually."
        warn "https://docs.docker.com/engine/install/"
    fi
}

install_cuda() {
    if check_command nvcc; then
        info "CUDA already installed: $(nvcc --version | tail -1)"
    else
        warn "CUDA not found. GPU acceleration requires CUDA Toolkit >= 12.3"
        warn "https://developer.nvidia.com/cuda-downloads"
    fi
}

# Main
info "Checking system..."
echo ""

install_rust
install_go
install_python
install_node
install_cmake
install_docker
install_cuda

echo ""
info "Installing Node.js dependencies..."
cd "$PROJECT_ROOT"
pnpm install --frozen-lockfile 2>/dev/null || pnpm install

echo ""
info "Running initial build check..."
cd "$PROJECT_ROOT"
cargo check --workspace 2>/dev/null || warn "Cargo check had issues (CUDA may be required)"

echo ""
echo "=== Bootstrap Complete ==="
echo ""
info "To activate the Python environment: source .venv/bin/activate"
info "To build: ./scripts/build.sh"
info "To test: ./scripts/test.sh"
