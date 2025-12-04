# OTL Development Guide

이 문서는 OTL 프로젝트의 개발 환경 설정 및 개발 가이드를 제공합니다.

## 시스템 요구사항

### 최소 요구사항
- **OS**: Ubuntu 22.04+ / macOS 13+ / Windows 11 (WSL2)
- **RAM**: 8GB (16GB 권장)
- **Disk**: 20GB 이상
- **CPU**: 4 cores 이상

### GPU 지원 (선택)
- NVIDIA GPU with CUDA 11.8+
- nvidia-container-toolkit

## 빠른 시작 (Ubuntu)

```bash
# 1. 저장소 클론
git clone https://github.com/hephaex/OTL.git
cd OTL

# 2. 설정 스크립트 실행
chmod +x scripts/setup-ubuntu.sh
./scripts/setup-ubuntu.sh

# 3. 환경 변수 설정
cp .env.example .env
# .env 파일을 편집하여 OPENAI_API_KEY 설정

# 4. Docker 서비스 시작
make docker-up

# 5. 빌드 및 테스트
make build
make test
```

## 수동 설치

### 1. 필수 패키지 설치 (Ubuntu)

```bash
# 시스템 패키지
sudo apt update && sudo apt install -y \
    build-essential pkg-config libssl-dev libpq-dev \
    curl wget git jq unzip

# OCR 지원
sudo apt install -y tesseract-ocr tesseract-ocr-kor tesseract-ocr-eng \
    libtesseract-dev libleptonica-dev

# PDF 지원
sudo apt install -y poppler-utils libpoppler-dev
```

### 2. Rust 설치

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 추가 컴포넌트
rustup component add clippy rustfmt
```

### 3. Docker 설치

```bash
# Docker 설치 (Ubuntu)
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh
sudo usermod -aG docker $USER

# 로그아웃 후 다시 로그인
```

### 4. GPU 지원 (선택)

```bash
# NVIDIA Container Toolkit 설치
distribution=$(. /etc/os-release;echo $ID$VERSION_ID)
curl -s -L https://nvidia.github.io/nvidia-docker/gpgkey | sudo apt-key add -
curl -s -L https://nvidia.github.io/nvidia-docker/$distribution/nvidia-docker.list | \
    sudo tee /etc/apt/sources.list.d/nvidia-docker.list

sudo apt update && sudo apt install -y nvidia-container-toolkit
sudo systemctl restart docker

# GPU 지원으로 서비스 시작
make docker-gpu
```

## 개발 워크플로우

### Makefile 명령어

```bash
# 개발 환경 시작
make dev           # Docker 시작 + 빌드

# 빌드
make build         # 디버그 빌드
make build-release # 릴리스 빌드

# 테스트
make test          # 모든 테스트 실행
make test-verbose  # 상세 출력

# 코드 품질
make lint          # Clippy 실행
make fmt           # 코드 포맷
make check         # 포맷 + 린트 체크

# Docker
make docker-up     # 서비스 시작
make docker-down   # 서비스 중지
make docker-logs   # 로그 보기
make docker-gpu    # GPU 지원으로 시작

# 데이터베이스
make db-status     # DB 상태 확인
make db-reset      # DB 초기화 (주의: 데이터 삭제)

# CLI 데모
make cli-extract   # 엔티티/관계 추출 데모
make cli-query     # RAG 쿼리 데모

# Ollama
make ollama-pull   # 모델 다운로드
make ollama-list   # 설치된 모델 목록
```

### CLI 사용법

```bash
# 엔티티/관계 추출
cargo run -p otl-cli -- extract "연차휴가는 최대 15일까지 사용할 수 있습니다."

# HITL 검증
cargo run -p otl-cli -- verify demo
cargo run -p otl-cli -- verify list
cargo run -p otl-cli -- verify approve <id>

# RAG 쿼리
cargo run -p otl-cli -- query "연차휴가 사용 일수는?" --ollama
cargo run -p otl-cli -- query "병가 신청 절차는?" --stream
```

## 프로젝트 구조

```
OTL/
├── crates/
│   ├── otl-core/       # 핵심 타입, 트레이트, 설정
│   ├── otl-parser/     # 문서 파서 (PDF, DOCX, XLSX)
│   ├── otl-ocr/        # OCR 처리 (Tesseract)
│   ├── otl-graph/      # 그래프 DB (SurrealDB)
│   ├── otl-vector/     # 벡터 DB (Qdrant)
│   ├── otl-extractor/  # NER, RE, HITL
│   ├── otl-rag/        # RAG 오케스트레이터
│   ├── otl-api/        # REST API (Axum)
│   └── otl-cli/        # CLI 도구
├── scripts/
│   ├── setup-ubuntu.sh # Ubuntu 설정 스크립트
│   └── init-db.sql     # DB 초기화 SQL
├── ontologies/         # OWL 온톨로지 파일
├── docker-compose.yml  # Docker 서비스 정의
├── docker-compose.gpu.yml  # GPU 오버라이드
├── Makefile           # 개발 명령어
└── .env.example       # 환경 변수 템플릿
```

## 환경 변수

### 필수

| 변수 | 설명 | 기본값 |
|------|------|--------|
| `OPENAI_API_KEY` | OpenAI API 키 | - |

### 선택

| 변수 | 설명 | 기본값 |
|------|------|--------|
| `LLM_PROVIDER` | LLM 제공자 (openai/ollama) | openai |
| `LLM_MODEL` | 모델 이름 | gpt-4o-mini |
| `EMBEDDING_MODEL` | 임베딩 모델 | text-embedding-3-small |
| `OLLAMA_URL` | Ollama 서버 URL | http://localhost:11434 |
| `DATABASE_URL` | PostgreSQL URL | postgres://otl:otl_dev_password@localhost:5432/otl |
| `SURREALDB_URL` | SurrealDB URL | ws://localhost:8000 |
| `QDRANT_URL` | Qdrant URL | http://localhost:6334 |

## Docker 서비스

| 서비스 | 포트 | 설명 |
|--------|------|------|
| PostgreSQL | 5432 | 메타데이터, ACL |
| SurrealDB | 8000 | 지식 그래프 |
| Qdrant | 6333, 6334 | 벡터 검색 |
| Meilisearch | 7700 | 키워드 검색 |
| Ollama | 11434 | 로컬 LLM |

### 서비스 상태 확인

```bash
# 전체 상태
docker compose ps

# 개별 서비스
curl http://localhost:6333/collections  # Qdrant
curl http://localhost:7700/health       # Meilisearch
curl http://localhost:11434/api/tags    # Ollama
```

## 트러블슈팅

### Docker 권한 오류

```bash
# docker: permission denied
sudo usermod -aG docker $USER
# 로그아웃 후 다시 로그인
```

### Rust 빌드 오류

```bash
# OpenSSL 관련 오류
sudo apt install libssl-dev pkg-config

# PostgreSQL 관련 오류
sudo apt install libpq-dev
```

### 포트 충돌

```bash
# 포트 사용 확인
sudo lsof -i :5432  # PostgreSQL
sudo lsof -i :8000  # SurrealDB
sudo lsof -i :6334  # Qdrant

# 기존 컨테이너 정리
docker compose down
docker system prune -f
```

### Ollama GPU 인식 안됨

```bash
# NVIDIA 드라이버 확인
nvidia-smi

# Docker GPU 테스트
docker run --rm --gpus all nvidia/cuda:11.8-base-ubuntu22.04 nvidia-smi
```

## 기여 가이드

### 브랜치 전략

- `main`: 안정 버전
- `develop`: 개발 브랜치
- `feature/*`: 기능 개발
- `fix/*`: 버그 수정

### 커밋 메시지

```
<type>(<scope>): <subject>

type: feat, fix, docs, style, refactor, test, chore
scope: core, parser, rag, cli, etc.
```

### PR 체크리스트

- [ ] `make check` 통과
- [ ] `make test` 통과
- [ ] 문서 업데이트 (필요시)

---

*Author: hephaex@gmail.com*
