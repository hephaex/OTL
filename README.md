# OTL - Ontology-based Knowledge System

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)

**OTL** (Ontology-based knowledge sysTem Library)는 온톨로지 기반 지식 그래프와 RAG(Retrieval-Augmented Generation)를 결합한 조직 지식 관리 시스템입니다.

## 주요 기능

- **문서 파싱**: PDF, DOCX, XLSX 등 다양한 형식 지원
- **지식 추출**: NER(Named Entity Recognition) 및 관계 추출(RE)
- **지식 그래프**: SurrealDB 기반 온톨로지 그래프 저장
- **벡터 검색**: Qdrant 기반 의미 검색
- **하이브리드 RAG**: 벡터 + 그래프 + 키워드 검색 결합
- **HITL 검증**: Human-in-the-loop 기반 품질 관리
- **ACL 보안**: 문서별 접근 제어
- **출처 추적**: 모든 답변에 Citation 제공

## 아키텍처

```
┌─────────────────────────────────────────────────────────────────┐
│                         OTL System                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐      │
│  │   otl-cli    │    │   otl-api    │    │  Swagger UI  │      │
│  │  (CLI Tool)  │    │ (REST API)   │    │   (Docs)     │      │
│  └──────┬───────┘    └──────┬───────┘    └──────────────┘      │
│         │                   │                                    │
│         └─────────┬─────────┘                                    │
│                   ▼                                              │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                      otl-rag                                │ │
│  │              (RAG Orchestration Layer)                      │ │
│  │  ┌─────────────┬─────────────┬─────────────┐               │ │
│  │  │Vector Search│Graph Search │  Keyword    │               │ │
│  │  │  (Qdrant)   │ (SurrealDB) │   Search    │               │ │
│  │  └─────────────┴─────────────┴─────────────┘               │ │
│  │                       │                                     │ │
│  │              ┌────────┴────────┐                           │ │
│  │              │  LLM Integration │                          │ │
│  │              │ (OpenAI/Ollama)  │                          │ │
│  │              └─────────────────┘                           │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐      │
│  │ otl-extractor│    │  otl-parser  │    │   otl-ocr    │      │
│  │  (NER/RE)    │    │ (PDF/DOCX)   │    │ (Tesseract)  │      │
│  └──────────────┘    └──────────────┘    └──────────────┘      │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## 빠른 시작

### 요구 사항

- Rust 1.75+
- Docker & Docker Compose
- Tesseract OCR (한국어 지원)

### 설치

```bash
# 저장소 클론
git clone https://github.com/hephaex/OTL.git
cd OTL

# Ubuntu 설정 스크립트 실행 (선택사항)
./scripts/setup-ubuntu.sh

# 환경 설정
cp .env.example .env
# .env 파일을 편집하여 API 키 설정

# 인프라 시작
docker compose up -d

# 빌드
cargo build --release
```

### CLI 사용

```bash
# 도움말
cargo run -p otl-cli -- --help

# 텍스트에서 개체/관계 추출
cargo run -p otl-cli -- extract "연차휴가는 최대 15일까지 사용할 수 있습니다."

# RAG 질의 (OpenAI)
cargo run -p otl-cli -- query "연차휴가 신청 절차가 어떻게 되나요?"

# RAG 질의 (Ollama, 로컬)
cargo run -p otl-cli -- query "병가 신청에 필요한 서류는?" --ollama

# HITL 검증
cargo run -p otl-cli -- verify demo
cargo run -p otl-cli -- verify stats
```

### API 서버 실행

```bash
# API 서버 시작
cargo run -p otl-api

# 또는 Docker로 실행
docker build -t otl-api .
docker run -p 8080:8080 otl-api
```

API 문서: http://localhost:8080/swagger-ui/

## API 엔드포인트

| Method | Endpoint | 설명 |
|--------|----------|------|
| POST | `/api/v1/query` | RAG 질의 |
| POST | `/api/v1/query/stream` | 스트리밍 RAG 질의 |
| GET | `/api/v1/documents` | 문서 목록 |
| POST | `/api/v1/documents` | 문서 업로드 |
| GET | `/api/v1/documents/:id` | 문서 상세 |
| DELETE | `/api/v1/documents/:id` | 문서 삭제 |
| GET | `/api/v1/graph/entities` | 개체 목록 |
| GET | `/api/v1/graph/entities/:id` | 개체 상세 |
| POST | `/api/v1/graph/search` | 그래프 검색 |
| GET | `/api/v1/verify/pending` | 검증 대기 목록 |
| POST | `/api/v1/verify/:id/approve` | 검증 승인 |
| POST | `/api/v1/verify/:id/reject` | 검증 거부 |
| GET | `/health` | 헬스체크 |
| GET | `/ready` | 준비 상태 |

## 프로젝트 구조

```
OTL/
├── crates/
│   ├── otl-core/       # 핵심 도메인 모델, 설정
│   ├── otl-parser/     # 문서 파서 (PDF, DOCX, XLSX)
│   ├── otl-ocr/        # OCR 통합 (Tesseract)
│   ├── otl-graph/      # 지식 그래프 (SurrealDB)
│   ├── otl-vector/     # 벡터 저장소 (Qdrant)
│   ├── otl-extractor/  # NER/RE 추출기
│   ├── otl-rag/        # RAG 오케스트레이터
│   ├── otl-api/        # REST API 서버
│   └── otl-cli/        # CLI 도구
├── deploy/
│   ├── kubernetes/     # K8s 매니페스트
│   └── argocd/         # ArgoCD 설정
├── scripts/            # 유틸리티 스크립트
├── docker-compose.yml  # 로컬 개발 환경
└── Dockerfile          # 프로덕션 이미지
```

## 개발

### Makefile 명령어

```bash
make help          # 사용 가능한 명령어 보기
make dev           # 개발 환경 시작
make build         # 빌드
make test          # 테스트
make lint          # Clippy 실행
make fmt           # 코드 포맷팅
make docker-up     # Docker 서비스 시작
make docker-down   # Docker 서비스 중지
```

### 테스트

```bash
# 전체 테스트
cargo test --workspace

# 특정 크레이트 테스트
cargo test -p otl-rag

# 상세 출력
cargo test --workspace -- --nocapture
```

## Kubernetes 배포

```bash
# Kustomize로 배포
kubectl apply -k deploy/kubernetes/

# 또는 ArgoCD 사용
kubectl apply -f deploy/argocd/application.yaml
```

자세한 내용은 [deploy/argocd/README.md](deploy/argocd/README.md) 참조.

## 환경 변수

| 변수 | 설명 | 기본값 |
|------|------|--------|
| `API_HOST` | API 서버 호스트 | `0.0.0.0` |
| `API_PORT` | API 서버 포트 | `8080` |
| `DATABASE_URL` | PostgreSQL URL | - |
| `SURREALDB_URL` | SurrealDB URL | `ws://localhost:8000` |
| `QDRANT_URL` | Qdrant URL | `http://localhost:6334` |
| `OPENAI_API_KEY` | OpenAI API 키 | - |
| `OLLAMA_URL` | Ollama URL | `http://localhost:11434` |
| `LLM_PROVIDER` | LLM 제공자 (`openai`/`ollama`) | `openai` |
| `JWT_SECRET` | JWT 서명 키 | - |

## 기여

기여를 환영합니다! 이슈를 생성하거나 PR을 제출해 주세요.

## 라이선스

Apache License 2.0

## 저작권

© 2024 hephaex@gmail.com. All rights reserved.
