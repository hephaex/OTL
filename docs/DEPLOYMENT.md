# OTL Deployment Guide

**Author**: hephaex@gmail.com

This guide covers deploying OTL (Ontology-based Knowledge System Library) to various environments.

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Docker Compose Deployment](#docker-compose-deployment)
3. [Kubernetes Deployment](#kubernetes-deployment)
4. [Configuration](#configuration)
5. [Health Checks](#health-checks)
6. [Monitoring](#monitoring)
7. [Troubleshooting](#troubleshooting)

---

## Prerequisites

### Required Software

- Docker 24.0+ and Docker Compose 2.20+
- kubectl 1.28+ (for Kubernetes deployment)
- Rust 1.75+ (for building from source)

### Required Services

- **SurrealDB**: Graph database for knowledge storage
- **Qdrant**: Vector database for semantic search
- **PostgreSQL**: Metadata and document storage
- **Ollama** (optional): Local LLM for inference

### API Keys

- **OpenAI API Key** (optional): Required for OpenAI LLM/embedding models
- Configure in `.env` file or Kubernetes secrets

---

## Docker Compose Deployment

### Quick Start

```bash
# Clone the repository
git clone https://github.com/hephaex/OTL.git
cd OTL

# Copy and configure environment
cp .env.example .env
# Edit .env with your settings

# Start all services
docker compose up -d

# Check status
docker compose ps

# View logs
docker compose logs -f otl-api
```

### GPU Support (Ollama)

For NVIDIA GPU acceleration with Ollama:

```bash
# Start with GPU support
docker compose -f docker-compose.yml -f docker-compose.gpu.yml up -d
```

### Service Endpoints

| Service | Port | Description |
|---------|------|-------------|
| OTL API | 8080 | REST API server |
| Swagger UI | 8080/swagger-ui | API documentation |
| SurrealDB | 8000 | Graph database |
| Qdrant | 6333/6334 | Vector database (HTTP/gRPC) |
| PostgreSQL | 5432 | Metadata storage |
| Ollama | 11434 | Local LLM |

---

## Kubernetes Deployment

### Prerequisites

```bash
# Verify kubectl is configured
kubectl cluster-info

# Create namespace
kubectl create namespace otl
```

### Deploy Using Kustomize

```bash
# Apply all manifests
kubectl apply -k k8s/

# Check deployment status
kubectl -n otl get pods
kubectl -n otl get services
```

### Manual Deployment

```bash
# Apply in order
kubectl apply -f k8s/namespace.yaml
kubectl apply -f k8s/secret.yaml
kubectl apply -f k8s/configmap.yaml
kubectl apply -f k8s/infrastructure.yaml
kubectl apply -f k8s/deployment.yaml
kubectl apply -f k8s/service.yaml
kubectl apply -f k8s/hpa.yaml

# Optional: Ingress for external access
kubectl apply -f k8s/ingress.yaml
```

### Update Secrets

```bash
# Create secrets from file
kubectl -n otl create secret generic otl-secrets \
  --from-literal=OPENAI_API_KEY=sk-your-key \
  --from-literal=SURREALDB_USER=root \
  --from-literal=SURREALDB_PASS=your-password \
  --from-literal=POSTGRES_USER=otl \
  --from-literal=POSTGRES_PASSWORD=your-password \
  --dry-run=client -o yaml | kubectl apply -f -
```

### Verify Deployment

```bash
# Check pod status
kubectl -n otl get pods -w

# Check service endpoints
kubectl -n otl get endpoints

# View logs
kubectl -n otl logs -f deployment/otl-api

# Port-forward for local testing
kubectl -n otl port-forward svc/otl-api 8080:80
```

---

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `API_HOST` | Bind address | `0.0.0.0` |
| `API_PORT` | Server port | `8080` |
| `RUST_LOG` | Log level | `otl_api=info` |
| `OPENAI_API_KEY` | OpenAI API key | - |
| `SURREALDB_URL` | SurrealDB connection | `ws://localhost:8000` |
| `QDRANT_URL` | Qdrant connection | `http://localhost:6334` |
| `POSTGRES_URL` | PostgreSQL connection | `postgres://localhost:5432/otl` |
| `LLM_PROVIDER` | LLM provider (openai/ollama) | `openai` |
| `LLM_MODEL` | LLM model name | `gpt-4o-mini` |
| `EMBEDDING_MODEL` | Embedding model | `text-embedding-3-small` |

### Example .env File

```bash
# API Server
API_HOST=0.0.0.0
API_PORT=8080
RUST_LOG=otl_api=info,tower_http=debug

# OpenAI
OPENAI_API_KEY=sk-your-api-key
LLM_PROVIDER=openai
LLM_MODEL=gpt-4o-mini
EMBEDDING_MODEL=text-embedding-3-small

# Databases
SURREALDB_URL=ws://localhost:8000
SURREALDB_USER=root
SURREALDB_PASS=root
SURREALDB_NAMESPACE=otl
SURREALDB_DATABASE=knowledge

QDRANT_URL=http://localhost:6334
QDRANT_COLLECTION=otl_chunks

POSTGRES_URL=postgres://otl:password@localhost:5432/otl

# Local LLM (optional)
OLLAMA_URL=http://localhost:11434
```

---

## Health Checks

### Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Liveness probe |
| `/ready` | GET | Readiness probe |
| `/metrics` | GET | JSON metrics |
| `/metrics/prometheus` | GET | Prometheus format |

### Example Responses

**Liveness Check:**
```bash
curl http://localhost:8080/health
```
```json
{
  "status": "ok",
  "version": "0.1.0",
  "build_info": {
    "name": "otl-api",
    "rust_version": "1.75+"
  }
}
```

**Readiness Check:**
```bash
curl http://localhost:8080/ready
```
```json
{
  "ready": true,
  "checks": {
    "database": true,
    "vector_store": true,
    "llm": true,
    "rag_initialized": false
  }
}
```

---

## Monitoring

### Prometheus Integration

Add to your Prometheus config:

```yaml
scrape_configs:
  - job_name: 'otl-api'
    static_configs:
      - targets: ['otl-api:8080']
    metrics_path: '/metrics/prometheus'
```

### Available Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `otl_uptime_seconds` | Gauge | Server uptime |
| `otl_requests_total` | Counter | Total HTTP requests |
| `otl_rag_enabled` | Gauge | RAG initialization status |
| `otl_build_info` | Gauge | Build information |

### Grafana Dashboard

Import the OTL dashboard from `docs/grafana-dashboard.json` (if available).

---

## Troubleshooting

### Common Issues

#### 1. API Server Won't Start

```bash
# Check logs
docker compose logs otl-api

# Verify database connections
curl http://localhost:8000/health  # SurrealDB
curl http://localhost:6333/healthz  # Qdrant
```

#### 2. RAG Not Initialized

The RAG system requires all databases to be available:
- Check SurrealDB is running and accessible
- Check Qdrant is running and has collections created
- Verify OpenAI API key is valid

#### 3. Slow Queries

- Increase `RAG_VECTOR_TOP_K` for better recall
- Decrease `RAG_FINAL_TOP_K` for faster response
- Check vector embedding dimension matches your model

#### 4. Memory Issues

```yaml
# Increase memory limits in docker-compose.yml or k8s deployment
resources:
  limits:
    memory: "2Gi"
```

### Debug Mode

```bash
# Enable debug logging
export RUST_LOG=otl_api=debug,tower_http=trace
cargo run -p otl-api
```

### Database Initialization

```bash
# Initialize SurrealDB schema
surreal sql --conn http://localhost:8000 --user root --pass root << EOF
USE NS otl DB knowledge;

DEFINE TABLE entity SCHEMAFULL;
DEFINE FIELD class ON entity TYPE string;
DEFINE FIELD properties ON entity TYPE object;
DEFINE FIELD source ON entity TYPE object;

DEFINE TABLE relates SCHEMAFULL;
DEFINE FIELD in ON relates TYPE record(entity);
DEFINE FIELD out ON relates TYPE record(entity);
DEFINE FIELD predicate ON relates TYPE string;
DEFINE FIELD confidence ON relates TYPE float;
EOF
```

---

## Scaling

### Horizontal Scaling

The OTL API is stateless and can be scaled horizontally:

```bash
# Docker Compose
docker compose up -d --scale otl-api=3

# Kubernetes
kubectl -n otl scale deployment/otl-api --replicas=5
```

### Auto-scaling (Kubernetes)

HPA is already configured in `k8s/hpa.yaml`:
- Min replicas: 2
- Max replicas: 10
- Scale on CPU (70%) and Memory (80%)

---

## Security

### Best Practices

1. **Never commit secrets** to version control
2. Use **TLS** for all external connections
3. Enable **CORS** only for trusted origins
4. Use **network policies** in Kubernetes
5. Rotate **API keys** regularly

### Example Network Policy

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: otl-api-policy
  namespace: otl
spec:
  podSelector:
    matchLabels:
      app.kubernetes.io/name: otl
  policyTypes:
    - Ingress
    - Egress
  ingress:
    - from:
        - namespaceSelector:
            matchLabels:
              name: ingress-nginx
      ports:
        - port: 8080
  egress:
    - to:
        - podSelector:
            matchLabels:
              app.kubernetes.io/name: surrealdb
        - podSelector:
            matchLabels:
              app.kubernetes.io/name: qdrant
        - podSelector:
            matchLabels:
              app.kubernetes.io/name: postgres
```

---

*Author: hephaex@gmail.com*
