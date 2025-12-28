# Ubuntu x86_64 Migration & v0.1.0 Release

**Date**: 2025-12-28
**Type**: Migration & Release
**Status**: Completed

## Overview

Migrated OTL project from macOS Apple Silicon (aarch64-apple-darwin) to Ubuntu x86_64 (x86_64-unknown-linux-gnu). Fixed all platform-specific issues, resolved code quality warnings, and successfully created the first GitHub release v0.1.0.

## Migration Tasks Completed

### 1. Build Configuration
- Created `.cargo/config.toml` for x86_64 optimization
  ```toml
  [build]
  target = "x86_64-unknown-linux-gnu"
  jobs = -1

  [target.x86_64-unknown-linux-gnu]
  rustflags = ["-C", "target-cpu=x86-64-v2"]
  ```

### 2. Port Configuration Fixes
- PostgreSQL: 5432 → 5433 (conflict with existing service)
- SurrealDB: 8000 → 8001 (conflict with ntimes-coordinator)
- Updated `crates/otl-core/src/config.rs` with new defaults

### 3. Kubernetes Configuration
- Added nodeAffinity for amd64 architecture in deployment manifests
- Updated both `k8s/` and `deploy/kubernetes/` directories

### 4. Code Fixes
- Fixed auth handler bug (HeaderMap extraction in Axum)
- Removed unused imports in `crates/otl-api/src/handlers/auth.rs`

### 5. Vector Database
- Recreated Qdrant collection with 768 dimensions (matching nomic-embed-text)

## Code Quality Improvements

### Compiler Warnings Fixed
- All warnings in otl-api crate resolved

### Clippy Warnings Fixed (29 total)
| Crate | Warnings Fixed |
|-------|----------------|
| otl-api | 11 |
| otl-parser | 8 |
| otl-extractor | 6 |
| otl-rag | 4 |

### Security Vulnerabilities Fixed
- `validator`: 0.18 → 0.20 (RUSTSEC-2024-0373: idna vulnerability)
- `prometheus`: 0.13 → 0.14 (RUSTSEC-2024-0379: protobuf vulnerability)

### Docker Compose Fix
- Ollama healthcheck: Changed from `curl` to `ollama list` (container lacks curl)

## Test Results

```
Running tests across 7 crates:
- otl-api: 52 passed
- otl-core: 12 passed
- otl-extractor: 18 passed
- otl-graph: 8 passed
- otl-parser: 15 passed
- otl-rag: 10 passed
- otl-vector: 7 passed
Total: 122 tests passed
```

## GitHub Actions CI/CD Setup

### Created Workflows
1. **`.github/workflows/release.yml`**
   - Triggers on version tags (v*.*.*)
   - Creates GitHub Release with changelog
   - Builds Linux x86_64 binary
   - Builds and pushes Docker image to GHCR
   - Updates Kubernetes manifests with new image tag

2. **`.github/workflows/cd.yml`**
   - Triggers on release published or manual dispatch
   - Deploys to staging/production environments
   - Supports both direct Kubernetes and ArgoCD deployment
   - Post-deployment health verification

### Release Workflow Fixes
- Added `permissions: contents: write` for release creation
- Added `fetch-depth: 0` for changelog generation
- Fixed binary path for `--target x86_64-unknown-linux-gnu`
- Changed image name to lowercase for GHCR compatibility
  - From: `ghcr.io/${{ github.repository }}`
  - To: `ghcr.io/hephaex/otl-api`

## v0.1.0 Release

### Release Assets
- `otl-api-0.1.0-linux-x86_64.tar.gz` (29 MB)
- `otl-api-0.1.0-linux-x86_64.tar.gz.sha256`
- Docker image: `ghcr.io/hephaex/otl-api:0.1.0`

### Docker Image Verification
```bash
# Pull and run test
docker pull ghcr.io/hephaex/otl-api:0.1.0
docker run -d --name otl-api-test \
  --network otl-network \
  -p 3001:8080 \
  -e DATABASE_URL="postgres://otl:otl_dev_password@otl-postgres:5432/otl" \
  -e SURREALDB_URL="ws://otl-surrealdb:8000" \
  -e QDRANT_URL="http://otl-qdrant:6334" \
  -e OLLAMA_URL="http://otl-ollama:11434" \
  -e JWT_SECRET="test-secret-key" \
  ghcr.io/hephaex/otl-api:0.1.0

# Health check
curl http://localhost:3001/health
# {"status": "ok", "version": "0.1.0"}
```

## Files Modified

### New Files
- `.cargo/config.toml`
- `.github/workflows/release.yml`
- `.github/workflows/cd.yml`

### Modified Files
- `crates/otl-core/src/config.rs` (port defaults)
- `crates/otl-api/src/handlers/auth.rs` (handler fixes)
- `crates/otl-api/Cargo.toml` (dependency updates)
- `docker-compose.yml` (ollama healthcheck)
- `k8s/deployment.yaml` (nodeAffinity)
- `deploy/kubernetes/deployment.yaml` (nodeAffinity)
- Multiple crates for clippy fixes

## Summary

| Task | Status |
|------|--------|
| Platform migration | ✅ Complete |
| Build verification | ✅ 122 tests passed |
| Clippy warnings | ✅ 29 fixed |
| Security audit | ✅ 0 vulnerabilities |
| GitHub Actions CI/CD | ✅ Configured |
| Release v0.1.0 | ✅ Published |
| Docker image GHCR | ✅ Pushed & verified |
