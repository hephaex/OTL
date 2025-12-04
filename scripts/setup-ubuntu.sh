#!/bin/bash
# OTL Development Environment Setup for Ubuntu
#
# Usage:
#   chmod +x scripts/setup-ubuntu.sh
#   ./scripts/setup-ubuntu.sh
#
# Tested on: Ubuntu 22.04 LTS, Ubuntu 24.04 LTS
#
# Author: hephaex@gmail.com

set -e

echo "========================================"
echo "OTL Development Environment Setup"
echo "Ubuntu Linux"
echo "========================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Helper functions
info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

# Check if running as root
if [ "$EUID" -eq 0 ]; then
    error "Please do not run this script as root. Use a regular user with sudo privileges."
fi

# =============================================================================
# System Update
# =============================================================================
info "Updating system packages..."
sudo apt update && sudo apt upgrade -y

# =============================================================================
# Essential Build Tools
# =============================================================================
info "Installing essential build tools..."
sudo apt install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    libpq-dev \
    curl \
    wget \
    git \
    jq \
    unzip \
    software-properties-common \
    apt-transport-https \
    ca-certificates \
    gnupg \
    lsb-release

# =============================================================================
# Rust Installation
# =============================================================================
if command -v rustc &> /dev/null; then
    RUST_VERSION=$(rustc --version)
    info "Rust already installed: $RUST_VERSION"
    info "Updating Rust..."
    rustup update stable
else
    info "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# Rust components
info "Installing Rust components..."
rustup component add clippy rustfmt
rustup target add x86_64-unknown-linux-gnu

# Verify Rust installation
rustc --version
cargo --version

# =============================================================================
# Docker Installation
# =============================================================================
if command -v docker &> /dev/null; then
    DOCKER_VERSION=$(docker --version)
    info "Docker already installed: $DOCKER_VERSION"
else
    info "Installing Docker..."

    # Remove old versions
    sudo apt remove -y docker docker-engine docker.io containerd runc 2>/dev/null || true

    # Add Docker's official GPG key
    sudo install -m 0755 -d /etc/apt/keyrings
    curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg
    sudo chmod a+r /etc/apt/keyrings/docker.gpg

    # Set up the repository
    echo \
      "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu \
      $(. /etc/os-release && echo "$VERSION_CODENAME") stable" | \
      sudo tee /etc/apt/sources.list.d/docker.list > /dev/null

    # Install Docker
    sudo apt update
    sudo apt install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin

    # Add user to docker group
    sudo usermod -aG docker $USER
    info "Added $USER to docker group. You may need to log out and back in."
fi

# =============================================================================
# OCR Dependencies (Tesseract)
# =============================================================================
info "Installing OCR dependencies..."
sudo apt install -y \
    tesseract-ocr \
    tesseract-ocr-kor \
    tesseract-ocr-eng \
    libtesseract-dev \
    libleptonica-dev

# Verify Tesseract
tesseract --version

# =============================================================================
# PDF Dependencies
# =============================================================================
info "Installing PDF processing dependencies..."
sudo apt install -y \
    poppler-utils \
    libpoppler-dev

# =============================================================================
# PostgreSQL Client (for debugging)
# =============================================================================
info "Installing PostgreSQL client..."
sudo apt install -y postgresql-client

# =============================================================================
# Optional: NVIDIA GPU Support for Ollama
# =============================================================================
if lspci | grep -i nvidia &> /dev/null; then
    warn "NVIDIA GPU detected. For GPU support with Ollama:"
    echo "  1. Install NVIDIA drivers: sudo apt install nvidia-driver-535"
    echo "  2. Install NVIDIA Container Toolkit:"
    echo "     distribution=\$(. /etc/os-release;echo \$ID\$VERSION_ID)"
    echo "     curl -s -L https://nvidia.github.io/nvidia-docker/gpgkey | sudo apt-key add -"
    echo "     curl -s -L https://nvidia.github.io/nvidia-docker/\$distribution/nvidia-docker.list | sudo tee /etc/apt/sources.list.d/nvidia-docker.list"
    echo "     sudo apt update && sudo apt install -y nvidia-container-toolkit"
    echo "     sudo systemctl restart docker"
fi

# =============================================================================
# Create Project Directory Structure
# =============================================================================
info "Setting up project directories..."
mkdir -p data/documents
mkdir -p data/embeddings
mkdir -p logs

# =============================================================================
# Environment File
# =============================================================================
if [ ! -f .env ]; then
    info "Creating .env file from template..."
    cp .env.example .env
    warn "Please edit .env and set your OPENAI_API_KEY"
else
    info ".env file already exists"
fi

# =============================================================================
# Build Project
# =============================================================================
info "Building OTL project..."
cargo build --workspace

# =============================================================================
# Run Tests
# =============================================================================
info "Running tests..."
cargo test --workspace

# =============================================================================
# Summary
# =============================================================================
echo ""
echo "========================================"
echo "Setup Complete!"
echo "========================================"
echo ""
echo "Next steps:"
echo "  1. Log out and back in (for Docker group)"
echo "  2. Edit .env and set your OPENAI_API_KEY"
echo "  3. Start services: docker compose up -d"
echo "  4. Check services: docker compose ps"
echo "  5. Run CLI: cargo run -p otl-cli -- --help"
echo ""
echo "Development commands:"
echo "  cargo build           - Build all crates"
echo "  cargo test            - Run all tests"
echo "  cargo clippy          - Run linter"
echo "  cargo fmt             - Format code"
echo ""
echo "Docker commands:"
echo "  docker compose up -d     - Start all services"
echo "  docker compose down      - Stop all services"
echo "  docker compose logs -f   - View logs"
echo ""
