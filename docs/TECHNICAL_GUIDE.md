# OTL Technical Guide

이 문서는 OTL(Ontology-based Triple Learning) 시스템의 기술 가이드입니다.
RAG 파이프라인 아키텍처, API 사용법, 환경 설정 및 배포 가이드를 포함합니다.

## Table of Contents

1. [시스템 개요](#시스템-개요)
2. [RAG 파이프라인 아키텍처](#rag-파이프라인-아키텍처)
3. [API Reference](#api-reference)
4. [환경 변수 설정](#환경-변수-설정)
5. [Docker Compose 설정](#docker-compose-설정)
6. [Kubernetes 배포 가이드](#kubernetes-배포-가이드)
7. [LLM 프로바이더 설정](#llm-프로바이더-설정)

---

## 시스템 개요

OTL은 조직의 문서를 분석하고 지식 그래프를 구축하여 RAG(Retrieval-Augmented Generation) 기반 질의응답을 제공하는 시스템입니다.

### 핵심 컴포넌트

| Crate | 역할 |
|-------|------|
| `otl-core` | 도메인 모델, 공통 타입, 설정 관리 |
| `otl-api` | REST API 서버 (Axum 기반) |
| `otl-rag` | Hybrid RAG 오케스트레이터 |
| `otl-vector` | 벡터 DB 추상화 (Qdrant) |
| `otl-graph` | 그래프 DB 추상화 (SurrealDB) |
| `otl-parser` | 문서 파싱 및 청킹 |
| `otl-extractor` | 지식 추출 |
| `otl-ocr` | OCR 처리 |

### 기술 스택

- **Backend**: Rust + Axum
- **Vector Database**: Qdrant
- **Graph Database**: SurrealDB
- **Metadata Database**: PostgreSQL
- **Full-text Search**: Meilisearch (Optional)
- **LLM**: OpenAI / Ollama / vLLM

---

## RAG 파이프라인 아키텍처

OTL은 **Hybrid RAG** 파이프라인을 구현하여 벡터 검색, 그래프 검색, 키워드 검색 결과를 융합합니다.

### 파이프라인 흐름

```mermaid
graph TD
    A[User Query] --> B[Query Analysis]
    B --> C{Parallel Search}
    C --> D[Vector Search<br/>Qdrant]
    C --> E[Graph Search<br/>SurrealDB]
    C --> F[Keyword Search<br/>Optional]
    D --> G[ACL Filtering]
    E --> G
    F --> G
    G --> H[RRF Merge]
    H --> I[Context Building]
    I --> J[LLM Generation]
    J --> K[Citation Extraction]
    K --> L[Response]
```

### 검색 전략

#### 1. Vector Search (Semantic)
- **Backend**: Qdrant
- **알고리즘**: Cosine Similarity
- **임베딩 모델**:
  - OpenAI: `text-embedding-3-small` (1536 dim)
  - Ollama: `nomic-embed-text` (768 dim)

```rust
// 벡터 검색 설정
pub struct VectorSearchBackend {
    store: QdrantStore,
    embedding_client: Arc<dyn EmbeddingClient>,
}
```

#### 2. Graph Search (Structural)
- **Backend**: SurrealDB
- **기능**: 온톨로지 기반 관계 탐색
- **Depth**: 기본 2-hop traversal

#### 3. Keyword Search (Lexical)
- **Backend**: Meilisearch (Optional)
- **용도**: 정확한 용어 매칭

### Reciprocal Rank Fusion (RRF)

여러 검색 결과를 융합하는 알고리즘입니다.

```rust
// RRF 스코어 계산
let rrf_score = weight / (k + rank as f32 + 1.0);

// 기본 가중치
pub struct RagConfig {
    pub rrf_k: f32,           // 60.0 (RRF 상수)
    pub vector_weight: f32,    // 1.0
    pub graph_weight: f32,     // 1.5 (그래프 결과 우선)
    pub keyword_weight: f32,   // 0.8
}
```

### ACL (Access Control List)

문서 레벨의 접근 제어를 지원합니다.

| Level | 설명 |
|-------|------|
| `public` | 모든 사용자 접근 가능 |
| `internal` | 조직 내부 사용자만 |
| `confidential` | 특정 부서/역할만 |
| `restricted` | 지정된 사용자만 |

---

## API Reference

### Base URL

```
http://localhost:8080/api/v1
```

### Health Check Endpoints

#### GET /health
서버 상태 확인 (Liveness Probe)

```bash
curl http://localhost:8080/health
```

**Response:**
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

#### GET /ready
서비스 준비 상태 확인 (Readiness Probe)

```bash
curl http://localhost:8080/ready
```

**Response:**
```json
{
  "ready": true,
  "checks": {
    "database": true,
    "vector_store": true,
    "llm": true,
    "rag_initialized": true
  }
}
```

#### GET /metrics
Prometheus 형식 메트릭

```bash
curl http://localhost:8080/metrics
```

---

### Document API

#### POST /api/v1/documents
문서 업로드 (PDF, DOCX, TXT 지원)

**Request:**
```bash
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -d '{
    "title": "인사규정_2024.pdf",
    "content": "<base64_encoded_content>",
    "file_type": "pdf",
    "access_level": "internal",
    "department": "인사팀"
  }'
```

**Request Body:**
```json
{
  "title": "string (required)",
  "content": "string (required, base64 encoded)",
  "file_type": "string (required: pdf|docx|txt)",
  "access_level": "string (optional: public|internal|confidential|restricted)",
  "department": "string (optional)"
}
```

**Response (201 Created):**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "message": "Document uploaded and processed: 45/45 chunks indexed",
  "chunk_count": 45
}
```

**파일 형식별 처리:**
- **PDF**: `%PDF-` 매직 바이트 검증, `pdf-extract` 라이브러리로 텍스트 추출
- **DOCX**: ZIP 시그니처(PK) 검증, `docx-rs` 라이브러리로 텍스트 추출
- **TXT/MD**: UTF-8 텍스트로 직접 처리

**청킹 설정:**
```rust
ChunkConfig {
    chunk_size: 1000,      // 청크 크기 (문자)
    overlap: 200,          // 오버랩
    min_chunk_size: 100,   // 최소 청크 크기
    respect_sections: true,
    respect_paragraphs: true,
}
```

#### GET /api/v1/documents
문서 목록 조회

```bash
curl "http://localhost:8080/api/v1/documents?page=1&page_size=20&file_type=pdf"
```

**Query Parameters:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| page | u32 | 1 | 페이지 번호 |
| page_size | u32 | 20 | 페이지당 항목 수 |
| file_type | string | - | 파일 형식 필터 |
| department | string | - | 부서 필터 |
| search | string | - | 제목 검색 |

**Response:**
```json
{
  "documents": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "title": "인사규정_2024.pdf",
      "file_type": "pdf",
      "access_level": "internal",
      "department": "인사팀",
      "created_at": "2024-01-15T10:00:00Z",
      "updated_at": "2024-01-15T10:00:00Z",
      "chunk_count": 45
    }
  ],
  "total": 1,
  "page": 1,
  "page_size": 20
}
```

#### GET /api/v1/documents/:id
문서 상세 조회

```bash
curl http://localhost:8080/api/v1/documents/550e8400-e29b-41d4-a716-446655440000
```

#### DELETE /api/v1/documents/:id
문서 삭제

```bash
curl -X DELETE http://localhost:8080/api/v1/documents/550e8400-e29b-41d4-a716-446655440000
```

---

### Query API

#### POST /api/v1/query
RAG 질의 (동기)

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{
    "question": "연차휴가 신청 절차가 어떻게 되나요?",
    "top_k": 5,
    "include_citations": true,
    "user_id": "user123"
  }'
```

**Request Body:**
```json
{
  "question": "string (required)",
  "top_k": 5,
  "include_citations": true,
  "user_id": "string (optional)"
}
```

**Response:**
```json
{
  "answer": "연차휴가 신청은 사내 인사시스템을 통해 진행됩니다...",
  "citations": [
    {
      "source": "인사규정_2024.pdf",
      "page": 15,
      "section": "제3장 휴가",
      "relevance": 0.92
    }
  ],
  "confidence": 0.87,
  "processing_time_ms": 1250
}
```

#### POST /api/v1/query/stream
RAG 질의 (SSE 스트리밍)

```bash
curl -X POST http://localhost:8080/api/v1/query/stream \
  -H "Content-Type: application/json" \
  -H "Accept: text/event-stream" \
  -d '{
    "question": "연차휴가 신청 절차가 어떻게 되나요?",
    "top_k": 5
  }'
```

**SSE Response:**
```
event: message
id: 0
data: 연차휴가

event: message
id: 1
data: 신청은

event: message
id: 2
data: 사내 인사시스템을

...
```

**JavaScript 클라이언트 예시:**
```javascript
const eventSource = new EventSource('/api/v1/query/stream', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ question: '연차휴가 신청 절차가 어떻게 되나요?' })
});

eventSource.onmessage = (event) => {
  console.log('Received:', event.data);
};
```

---

### Graph API

#### GET /api/v1/graph/entities
엔티티 목록 조회

```bash
curl http://localhost:8080/api/v1/graph/entities
```

#### GET /api/v1/graph/entities/:id
엔티티 상세 조회

```bash
curl http://localhost:8080/api/v1/graph/entities/550e8400-e29b-41d4-a716-446655440000
```

#### POST /api/v1/graph/search
그래프 검색

```bash
curl -X POST http://localhost:8080/api/v1/graph/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "휴가 정책",
    "limit": 10
  }'
```

---

### Ontology API

#### GET /api/v1/ontology
온톨로지 스키마 조회

```bash
curl http://localhost:8080/api/v1/ontology
```

#### PUT /api/v1/ontology
온톨로지 스키마 업데이트

```bash
curl -X PUT http://localhost:8080/api/v1/ontology \
  -H "Content-Type: application/json" \
  -d '{
    "classes": [...],
    "properties": [...]
  }'
```

---

### Verification API (HITL)

#### GET /api/v1/verify/pending
검토 대기 중인 추출 항목 조회

```bash
curl http://localhost:8080/api/v1/verify/pending
```

#### POST /api/v1/verify/:id/approve
추출 승인

```bash
curl -X POST http://localhost:8080/api/v1/verify/550e8400-e29b-41d4-a716-446655440000/approve
```

#### POST /api/v1/verify/:id/reject
추출 거부

```bash
curl -X POST http://localhost:8080/api/v1/verify/550e8400-e29b-41d4-a716-446655440000/reject
```

#### GET /api/v1/verify/stats
검증 통계 조회

```bash
curl http://localhost:8080/api/v1/verify/stats
```

---

## 환경 변수 설정

### 서버 설정

| 변수 | 기본값 | 설명 |
|------|--------|------|
| `API_HOST` | `0.0.0.0` | API 서버 바인드 주소 |
| `API_PORT` | `8080` | API 서버 포트 |
| `LOG_LEVEL` | `info` | 로그 레벨 (trace/debug/info/warn/error) |
| `RUST_LOG` | `otl_api=info` | Rust 로그 설정 |
| `CORS_ORIGINS` | (empty) | CORS 허용 오리진 (쉼표 구분) |

### 데이터베이스 설정

| 변수 | 기본값 | 설명 |
|------|--------|------|
| `DATABASE_URL` | `postgres://otl:otl_dev_password@localhost:5432/otl` | PostgreSQL 연결 URL |
| `SURREALDB_URL` | `ws://localhost:8000` | SurrealDB WebSocket URL |
| `SURREALDB_USER` | `root` | SurrealDB 사용자 |
| `SURREALDB_PASS` | `root` | SurrealDB 비밀번호 |
| `QDRANT_URL` | `http://localhost:6334` | Qdrant gRPC URL |

### LLM 설정

| 변수 | 기본값 | 설명 |
|------|--------|------|
| `LLM_PROVIDER` | `openai` | LLM 프로바이더 (openai/ollama/azure) |
| `LLM_MODEL` | `gpt-4o-mini` | LLM 모델명 |
| `EMBEDDING_MODEL` | `text-embedding-3-small` | 임베딩 모델명 |
| `OPENAI_API_KEY` | - | OpenAI API 키 |
| `OLLAMA_URL` | `http://localhost:11434` | Ollama 서버 URL |

### 설정 예시 (.env)

```bash
# Server
API_HOST=0.0.0.0
API_PORT=8080
LOG_LEVEL=info
CORS_ORIGINS=http://localhost:3000,http://localhost:5173

# Database
DATABASE_URL=postgres://otl:otl_dev_password@localhost:5433/otl
SURREALDB_URL=ws://localhost:8000
SURREALDB_USER=root
SURREALDB_PASS=root
QDRANT_URL=http://localhost:6334

# LLM - OpenAI
LLM_PROVIDER=openai
LLM_MODEL=gpt-4o-mini
EMBEDDING_MODEL=text-embedding-3-small
OPENAI_API_KEY=sk-your-api-key

# LLM - Ollama (alternative)
# LLM_PROVIDER=ollama
# LLM_MODEL=llama2
# EMBEDDING_MODEL=nomic-embed-text
# OLLAMA_URL=http://localhost:11434
```

---

## Docker Compose 설정

### 서비스 구성

```yaml
services:
  # SurrealDB - 그래프 + 문서 DB
  surrealdb:
    image: surrealdb/surrealdb:v2.4.0
    ports:
      - "8000:8000"

  # Qdrant - 벡터 DB
  qdrant:
    image: qdrant/qdrant:v1.16.0
    ports:
      - "6333:6333"  # HTTP API
      - "6334:6334"  # gRPC API

  # PostgreSQL - 메타데이터 + ACL
  postgres:
    image: postgres:16-alpine
    ports:
      - "5433:5432"

  # Meilisearch - 전문 검색 (Optional)
  meilisearch:
    image: getmeili/meilisearch:v1.10
    ports:
      - "7700:7700"

  # Ollama - 로컬 LLM
  ollama:
    image: ollama/ollama:latest
    ports:
      - "11434:11434"
```

### 실행 방법

```bash
# 기본 서비스 시작
docker compose up -d

# 로그 확인
docker compose logs -f

# 서비스 중지
docker compose down

# 볼륨 포함 삭제
docker compose down -v
```

### GPU 지원 (vLLM)

GPU가 있는 환경에서는 vLLM을 사용할 수 있습니다.

```bash
# GPU 프로필로 시작
docker compose --profile vllm up -d
```

---

## Kubernetes 배포 가이드

### 디렉토리 구조

```
deploy/kubernetes/
├── namespace.yaml
├── configmap.yaml
├── secret.yaml
├── deployment.yaml
├── service.yaml
├── ingress.yaml
├── hpa.yaml
├── serviceaccount.yaml
└── kustomization.yaml
```

### 네임스페이스 생성

```bash
kubectl apply -f deploy/kubernetes/namespace.yaml
```

### ConfigMap 설정

```yaml
# deploy/kubernetes/configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: otl-config
  namespace: otl
data:
  API_HOST: "0.0.0.0"
  API_PORT: "8080"
  LOG_LEVEL: "info"
  DATABASE_URL: "postgres://otl:otl@postgres:5432/otl"
  SURREALDB_URL: "ws://surrealdb:8000"
  QDRANT_URL: "http://qdrant:6334"
  OLLAMA_URL: "http://ollama:11434"
  LLM_PROVIDER: "ollama"
  LLM_MODEL: "llama2"
  EMBEDDING_MODEL: "nomic-embed-text"
```

### Secret 설정

```bash
# API 키 시크릿 생성
kubectl create secret generic otl-secrets \
  --namespace otl \
  --from-literal=OPENAI_API_KEY=sk-your-api-key \
  --from-literal=SURREALDB_PASS=your-password
```

### Deployment 설정

```yaml
# deploy/kubernetes/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: otl-api
  namespace: otl
spec:
  replicas: 3
  template:
    spec:
      containers:
        - name: otl-api
          image: ghcr.io/hephaex/otl-api:latest
          ports:
            - containerPort: 8080
          resources:
            requests:
              cpu: "100m"
              memory: "256Mi"
            limits:
              cpu: "1000m"
              memory: "1Gi"
          livenessProbe:
            httpGet:
              path: /health
              port: 8080
          readinessProbe:
            httpGet:
              path: /ready
              port: 8080
```

### HPA (Horizontal Pod Autoscaler)

```yaml
# deploy/kubernetes/hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: otl-api
  namespace: otl
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: otl-api
  minReplicas: 2
  maxReplicas: 10
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
```

### Ingress 설정

```yaml
# deploy/kubernetes/ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: otl-api
  namespace: otl
  annotations:
    nginx.ingress.kubernetes.io/proxy-body-size: "50m"
    nginx.ingress.kubernetes.io/proxy-read-timeout: "300"
spec:
  ingressClassName: nginx
  rules:
    - host: otl.example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: otl-api
                port:
                  name: http
```

### 배포 명령

```bash
# Kustomize로 전체 배포
kubectl apply -k deploy/kubernetes/

# 또는 개별 리소스 배포
kubectl apply -f deploy/kubernetes/namespace.yaml
kubectl apply -f deploy/kubernetes/configmap.yaml
kubectl apply -f deploy/kubernetes/secret.yaml
kubectl apply -f deploy/kubernetes/serviceaccount.yaml
kubectl apply -f deploy/kubernetes/deployment.yaml
kubectl apply -f deploy/kubernetes/service.yaml
kubectl apply -f deploy/kubernetes/ingress.yaml
kubectl apply -f deploy/kubernetes/hpa.yaml

# 배포 상태 확인
kubectl get pods -n otl
kubectl get svc -n otl
kubectl get ingress -n otl
```

---

## LLM 프로바이더 설정

### OpenAI

```bash
LLM_PROVIDER=openai
LLM_MODEL=gpt-4o-mini
EMBEDDING_MODEL=text-embedding-3-small
OPENAI_API_KEY=sk-your-api-key
```

**지원 임베딩 모델:**
| 모델 | 차원 |
|------|------|
| `text-embedding-3-small` | 1536 |
| `text-embedding-3-large` | 3072 |
| `text-embedding-ada-002` | 1536 |

### Ollama (로컬)

```bash
LLM_PROVIDER=ollama
LLM_MODEL=llama2
EMBEDDING_MODEL=nomic-embed-text
OLLAMA_URL=http://localhost:11434
```

**지원 임베딩 모델:**
| 모델 | 차원 |
|------|------|
| `nomic-embed-text` | 768 |
| `mxbai-embed-large` | 1024 |
| `all-minilm` | 384 |

**Ollama 모델 설치:**
```bash
# LLM 모델
ollama pull llama2
ollama pull qwen2.5:7b

# 임베딩 모델
ollama pull nomic-embed-text
```

### Azure OpenAI

```bash
LLM_PROVIDER=azure
LLM_MODEL=gpt-4o-mini
OPENAI_API_KEY=your-azure-api-key
OPENAI_BASE_URL=https://your-resource.openai.azure.com/openai/deployments/your-deployment
```

---

## 부록

### 프로젝트 빌드

```bash
# 개발 빌드
cargo build

# 릴리스 빌드
cargo build --release

# API 서버 실행
cargo run -p otl-api

# 테스트 실행
cargo test
```

### Docker 이미지 빌드

```bash
# 이미지 빌드
docker build -t otl-api:latest .

# 실행
docker run -p 8080:8080 \
  -e OPENAI_API_KEY=sk-your-key \
  otl-api:latest
```

### 참고 파일 경로

| 파일 | 경로 |
|------|------|
| API Routes | `/Users/mare/Simon/OTL/crates/otl-api/src/routes.rs` |
| Query Handler | `/Users/mare/Simon/OTL/crates/otl-api/src/handlers/query.rs` |
| Document Handler | `/Users/mare/Simon/OTL/crates/otl-api/src/handlers/documents.rs` |
| RAG Orchestrator | `/Users/mare/Simon/OTL/crates/otl-rag/src/lib.rs` |
| LLM Client | `/Users/mare/Simon/OTL/crates/otl-rag/src/llm.rs` |
| Core Types | `/Users/mare/Simon/OTL/crates/otl-core/src/lib.rs` |
| Configuration | `/Users/mare/Simon/OTL/crates/otl-core/src/config.rs` |
| Vector Store | `/Users/mare/Simon/OTL/crates/otl-vector/src/qdrant_store.rs` |
| Embedding | `/Users/mare/Simon/OTL/crates/otl-vector/src/embedding.rs` |
| Docker Compose | `/Users/mare/Simon/OTL/docker-compose.yml` |
| Dockerfile | `/Users/mare/Simon/OTL/Dockerfile` |
| K8s Deployment | `/Users/mare/Simon/OTL/deploy/kubernetes/deployment.yaml` |

---

*Author: hephaex@gmail.com*
*Last Updated: 2024-12*
