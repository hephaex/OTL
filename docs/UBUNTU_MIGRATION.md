# Ubuntu 22.04 Migration Guide

macOS 개발환경에서 Ubuntu 22.04로 이전하기 위한 가이드

## Pre-requisites

### Ubuntu 22.04 Requirements
- Ubuntu 22.04 LTS (Server or Desktop)
- 8GB RAM (16GB recommended for LLM)
- 50GB disk space
- sudo privileges

### Optional: GPU Support
- NVIDIA GPU (RTX 3060 이상 권장)
- CUDA 12.x compatible driver

## Quick Start

```bash
# 1. Clone repository
git clone git@github.com:hephaex/OTL.git
cd OTL

# 2. Run setup script
chmod +x scripts/setup-ubuntu.sh
./scripts/setup-ubuntu.sh

# 3. Logout and login (for docker group)

# 4. Configure environment
cp .env.example .env
# Edit .env with your settings

# 5. Start services
docker compose up -d

# 6. Verify
cargo build --workspace
cargo test --workspace
```

## Dependencies Checklist

### System Packages
| Package | Purpose | Install |
|---------|---------|---------|
| build-essential | Compiler toolchain | `sudo apt install build-essential` |
| pkg-config | Build configuration | `sudo apt install pkg-config` |
| libssl-dev | TLS/SSL support | `sudo apt install libssl-dev` |
| libpq-dev | PostgreSQL client | `sudo apt install libpq-dev` |
| tesseract-ocr | OCR processing | `sudo apt install tesseract-ocr` |
| poppler-utils | PDF processing | `sudo apt install poppler-utils` |

### Rust Toolchain
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add components
rustup component add clippy rustfmt
rustup target add x86_64-unknown-linux-gnu
```

### Docker
```bash
# Install Docker
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER
```

## Services

### Docker Compose Services
| Service | Port | Description |
|---------|------|-------------|
| SurrealDB | 8000 | Graph database |
| Qdrant | 6333, 6334 | Vector database |
| PostgreSQL | 5433 | Metadata storage |
| Meilisearch | 7700 | Full-text search |
| Ollama | 11434 | Local LLM |

### Start Services
```bash
# Start all services
docker compose up -d

# Check status
docker compose ps

# View logs
docker compose logs -f

# Stop services
docker compose down
```

## Environment Configuration

### .env File
```bash
# Database
DATABASE_URL=postgres://otl:otl_dev_password@localhost:5433/otl
SURREALDB_URL=ws://localhost:8000
QDRANT_URL=http://localhost:6334

# LLM (choose one)
LLM_PROVIDER=ollama          # For local LLM
OLLAMA_URL=http://localhost:11434
# OR
LLM_PROVIDER=openai          # For OpenAI API
OPENAI_API_KEY=sk-xxx

# Logging
RUST_LOG=otl=debug,tower_http=debug
```

## GPU Setup (Optional)

### NVIDIA Driver
```bash
# Check GPU
lspci | grep -i nvidia

# Install driver
sudo apt install nvidia-driver-535
sudo reboot
```

### NVIDIA Container Toolkit
```bash
distribution=$(. /etc/os-release;echo $ID$VERSION_ID)
curl -fsSL https://nvidia.github.io/libnvidia-container/gpgkey | sudo gpg --dearmor -o /usr/share/keyrings/nvidia-container-toolkit-keyring.gpg
curl -s -L https://nvidia.github.io/libnvidia-container/$distribution/libnvidia-container.list | \
  sed 's#deb https://#deb [signed-by=/usr/share/keyrings/nvidia-container-toolkit-keyring.gpg] https://#g' | \
  sudo tee /etc/apt/sources.list.d/nvidia-container-toolkit.list
sudo apt update
sudo apt install -y nvidia-container-toolkit
sudo nvidia-ctk runtime configure --runtime=docker
sudo systemctl restart docker
```

### Use GPU Docker Compose
```bash
docker compose -f docker-compose.gpu.yml up -d
```

## Build & Test

```bash
# Build
cargo build --workspace --release

# Test
cargo test --workspace

# Lint
cargo clippy --workspace

# Format
cargo fmt --check
```

## Troubleshooting

### Docker Permission Denied
```bash
sudo usermod -aG docker $USER
# Logout and login again
```

### Tesseract Language Data Missing
```bash
sudo apt install tesseract-ocr-kor tesseract-ocr-eng
```

### PostgreSQL Connection Refused
```bash
# Check if container is running
docker compose ps postgres
# Check logs
docker compose logs postgres
```

### Ollama Model Not Found
```bash
# Pull model after container starts
docker exec -it otl-ollama ollama pull llama3.2
```

## Data Migration

### From macOS to Ubuntu
```bash
# On macOS: Export data
docker compose exec postgres pg_dump -U otl otl > backup.sql

# On Ubuntu: Import data
docker compose exec -T postgres psql -U otl otl < backup.sql
```

### Volume Backup
```bash
# Backup volumes
docker run --rm -v otl_postgres_data:/data -v $(pwd):/backup alpine tar czf /backup/postgres_backup.tar.gz /data

# Restore volumes
docker run --rm -v otl_postgres_data:/data -v $(pwd):/backup alpine tar xzf /backup/postgres_backup.tar.gz -C /
```

## Verification Checklist

- [ ] System packages installed
- [ ] Rust toolchain working (`rustc --version`)
- [ ] Docker running (`docker ps`)
- [ ] All containers healthy (`docker compose ps`)
- [ ] Project builds (`cargo build`)
- [ ] Tests pass (`cargo test`)
- [ ] API responds (`curl http://localhost:8080/health`)
