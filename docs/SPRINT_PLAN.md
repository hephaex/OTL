# OTL Sprint Plan

> **Author**: hephaex@gmail.com  
> **License**: Apache-2.0  
> © 2024 hephaex@gmail.com. All rights reserved.

## 스프린트 실행 계획서

**프로젝트**: OTL - Ontology-based Knowledge System  
**총 기간**: 8주 (Sprint 0 ~ Sprint 4)  
**저장소**: https://github.com/hephaex/OTL

---

## 개요

이 문서는 OTL 프로젝트의 MVP(Minimum Viable Product) 개발을 위한 상세 스프린트 계획입니다.

### 핵심 원칙

1. **Sprint 0 필수**: 도메인 분석 없는 개발은 실패의 지름길
2. **HITL (Human-in-the-loop)**: 자동 추출 결과는 반드시 사람이 검증
3. **점진적 통합**: 각 스프린트 종료 시 동작하는 시스템 유지
4. **품질 우선**: 속도보다 정확성, 테스트 커버리지 80% 이상

### 파일럿 도메인

**인사 규정 및 매뉴얼**을 파일럿 도메인으로 선정합니다.

선정 이유는 다음과 같습니다:
- 자주 묻는 질문이 명확함 (휴가, 경비, 승인 절차 등)
- 문서 형식이 다양함 (PDF 규정집, Excel 양식, PPT 교육자료)
- 보안 등급이 다양함 (전체 공개 ~ 관리자 전용)
- 개체-관계가 풍부함 (직급, 부서, 절차, 양식)

---

## Sprint 0: 기반 구축 (1주)

**목표**: 도메인 분석 완료, 온톨로지 초안 설계, 개발 환경 구축

### 태스크 목록

| ID | 태스크 | 상세 내용 | 담당 | 예상 시간 | 결과물 |
|:---|:---|:---|:---|:---|:---|
| S0.1 | 도메인 전문가 인터뷰 | 인사팀 담당자와 워크숍 진행, FAQ 20개 수집 | PM | 8h | 인터뷰 노트, FAQ 목록 |
| S0.2 | 기존 문서 샘플 분석 | 50개 문서에서 개체/관계 후보 추출 | Analyst | 12h | 용어집 초안, 개체 목록 |
| S0.3 | 온톨로지 설계 | 핵심 클래스, 속성, 관계 정의 | Architect | 16h | OWL 파일, 클래스 다이어그램 |
| S0.4 | 프로젝트 초기화 | Rust workspace, GitHub 저장소, CI 설정 | Dev | 8h | 빌드 가능한 프로젝트 |
| S0.5 | 인프라 환경 구축 | Docker Compose로 DB들 로컬 실행 | DevOps | 4h | docker-compose.yml |

### S0.3 온톨로지 설계 상세

**워크숍 진행 방법**:

1. "가장 자주 묻는 질문 20개" 브레인스토밍
2. 각 질문에서 핵심 개체 추출
3. 개체 간 관계 맵핑
4. Protégé로 OWL 파일 초안 작성

**예상 온톨로지 구조 (인사 도메인)**:

```
hr:Employee (직원)
    ├── hr:employeeId (사번)
    ├── hr:name (이름)
    ├── hr:belongsTo → hr:Department
    ├── hr:hasPosition → hr:Position
    └── hr:hasLeaveBalance → hr:LeaveBalance

hr:Department (부서)
    ├── hr:departmentCode
    ├── hr:departmentName
    └── hr:managedBy → hr:Employee

hr:LeaveType (휴가 유형)
    ├── hr:leaveName (연차, 병가, 경조)
    ├── hr:maxDays
    └── hr:requiresApproval → hr:ApprovalProcess

hr:ApprovalProcess (승인 절차)
    ├── hr:steps[]
    ├── hr:requiredApprovers[]
    └── hr:relatedForms[] → hr:Form

hr:Regulation (규정)
    ├── hr:regulationCode
    ├── hr:title
    ├── hr:effectiveDate
    └── hr:governs → [hr:LeaveType, hr:ApprovalProcess, ...]
```

### 완료 기준 (Definition of Done)

- [ ] FAQ 20개 이상 문서화
- [ ] 용어집 50개 이상 항목 정의
- [ ] 온톨로지 클래스 10개 이상 정의
- [ ] `cargo build` 성공
- [ ] GitHub Actions CI 통과

---

## Sprint 1: 파서 및 저장소 (2주)

**목표**: 문서 파싱 파이프라인 완성, 저장소 연동

### Week 1: 파서 개발

| ID | 태스크 | 상세 내용 | 예상 시간 | 결과물 |
|:---|:---|:---|:---|:---|
| S1.1 | otl-core 기본 구조 | 도메인 모델, 에러 타입, ACL 구조체 | 8h | otl-core 크레이트 |
| S1.2 | PDF 파서 구현 | pdf-extract 활용, 텍스트/테이블 추출 | 12h | otl-parser (PDF) |
| S1.3 | DOCX 파서 구현 | docx-rs 활용 | 8h | otl-parser (DOCX) |
| S1.4 | Excel 파서 구현 | calamine 활용 | 8h | otl-parser (XLSX) |
| S1.5 | OCR 통합 | Tesseract CLI 래퍼 | 8h | otl-ocr |

### Week 2: 저장소 연동

| ID | 태스크 | 상세 내용 | 예상 시간 | 결과물 |
|:---|:---|:---|:---|:---|
| S1.6 | SurrealDB 연동 | 온톨로지 스키마 정의, CRUD 구현 | 12h | otl-graph |
| S1.7 | Qdrant 연동 | 임베딩 저장/검색 구현 | 8h | otl-vector |
| S1.8 | PostgreSQL 연동 | 메타데이터, ACL 테이블 | 8h | SQLx 마이그레이션 |
| S1.9 | 청킹 로직 구현 | 의미 단위 청킹, 오버랩 | 8h | chunker.rs |
| S1.10 | 통합 테스트 | 파일 → DB 저장 E2E | 4h | 통합 테스트 코드 |

### ACL 테이블 스키마

```sql
-- PostgreSQL 마이그레이션

CREATE TYPE access_level AS ENUM (
    'public',
    'internal',
    'confidential',
    'restricted'
);

CREATE TABLE documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title VARCHAR(500) NOT NULL,
    file_path VARCHAR(1000) NOT NULL,
    file_type VARCHAR(50) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    -- ACL 필드
    access_level access_level NOT NULL DEFAULT 'internal',
    owner_id VARCHAR(100),
    department VARCHAR(100),
    required_roles TEXT[] DEFAULT '{}',
    allowed_users TEXT[] DEFAULT '{}'
);

CREATE TABLE document_chunks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID REFERENCES documents(id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,
    content TEXT NOT NULL,
    page_number INTEGER,
    section_name VARCHAR(200),
    vector_id VARCHAR(100),  -- Qdrant의 point ID
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_documents_access ON documents(access_level);
CREATE INDEX idx_documents_department ON documents(department);
CREATE INDEX idx_chunks_document ON document_chunks(document_id);
```

### 완료 기준

- [ ] PDF, DOCX, XLSX 파싱 단위 테스트 통과
- [ ] 50개 파일럿 문서 DB 저장 완료
- [ ] ACL 기반 필터링 쿼리 동작 확인
- [ ] 테스트 커버리지 80% 이상

---

## Sprint 2: 지식 추출 (2주)

**목표**: NER/RE 파이프라인 구축, HITL 검증 프로세스 확립

### Week 1: 추출 엔진

| ID | 태스크 | 상세 내용 | 예상 시간 | 결과물 |
|:---|:---|:---|:---|:---|
| S2.1 | 규칙 기반 NER | 정규식 + 용어 사전 매칭 | 12h | ner.rs (규칙 엔진) |
| S2.2 | LLM 기반 NER | 프롬프트 엔지니어링, 배치 처리 | 12h | ner.rs (LLM 연동) |
| S2.3 | 하이브리드 NER | 규칙 + LLM 결과 병합, 신뢰도 점수 | 8h | NER 오케스트레이터 |
| S2.4 | 관계 추출 (RE) | LLM 기반 트리플 생성 | 12h | relation.rs |

### Week 2: 검증 및 로딩

| ID | 태스크 | 상세 내용 | 예상 시간 | 결과물 |
|:---|:---|:---|:---|:---|
| S2.5 | **HITL 검증 UI** | CLI 기반 검증 도구 (승인/수정/거부) | 12h | otl-cli verify 명령 |
| S2.6 | 검증 워크플로우 | 상태 관리 (pending → reviewed → approved) | 8h | 검증 상태 테이블 |
| S2.7 | 그래프 로딩 | 승인된 트리플만 SurrealDB 저장 | 8h | 그래프 로더 |
| S2.8 | 품질 메트릭 | 추출 정확도, 검증 통과율 측정 | 8h | 품질 대시보드 |

### HITL 검증 상세 설계

**검증 워크플로우**:

```
[자동 추출]
     │
     ▼
┌─────────────────────────────────────────┐
│ extraction_queue 테이블                  │
│                                         │
│ • id: UUID                              │
│ • document_id: UUID                     │
│ • extracted_triples: JSONB              │
│ • confidence_score: FLOAT               │
│ • status: pending | reviewed | approved │
│ • reviewer_id: VARCHAR                  │
│ • review_notes: TEXT                    │
│ • created_at, reviewed_at               │
└──────────────────┬──────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────┐
│ HITL 검증 CLI                           │
│                                         │
│ $ otl verify list                       │
│ $ otl verify show <id>                  │
│ $ otl verify approve <id>               │
│ $ otl verify reject <id> --reason "..." │
│ $ otl verify edit <id>                  │
└──────────────────┬──────────────────────┘
                   │
                   ▼ (approved만)
┌─────────────────────────────────────────┐
│ SurrealDB 지식 그래프                   │
└─────────────────────────────────────────┘
```

**검증 우선순위**:
1. 신뢰도 점수 < 0.7인 추출 결과 먼저 검증
2. 새로운 개체 타입 발견 시 즉시 리뷰
3. 기존 개체와 충돌하는 관계는 반드시 검토

**검증 UI 예시**:

```
┌─────────────────────────────────────────────────────────────┐
│ OTL Verification CLI                                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│ Document: 인사규정_2024.pdf (Page 15)                       │
│ Confidence: 0.65 (Low - requires review)                   │
│                                                             │
│ ─────────────────────────────────────────────────────────── │
│ Extracted Triples:                                          │
│                                                             │
│ [1] (연차휴가) --requires--> (팀장승인)      ✓ Correct     │
│ [2] (연차휴가) --maxDays--> (15일)           ? Uncertain   │
│ [3] (병가) --requires--> (진단서)            ✓ Correct     │
│ [4] (육아휴직) --duration--> (1년)           ✗ Incorrect   │
│                                                             │
│ ─────────────────────────────────────────────────────────── │
│ Original Text:                                              │
│ "연차휴가는 팀장의 사전 승인을 받아야 하며, 연간 15일이    │
│  기본 부여된다. 병가 신청 시 진단서를 첨부해야 한다.        │
│  육아휴직은 최대 2년까지 사용 가능하다."                    │
│                                                             │
│ ─────────────────────────────────────────────────────────── │
│ Actions:                                                    │
│ [a] Approve all    [e] Edit triple    [r] Reject           │
│ [n] Next           [q] Quit                                │
│                                                             │
│ > e 4                                                       │
│ Edit triple [4]:                                            │
│ Subject [육아휴직]: <enter>                                 │
│ Predicate [duration]: maxDuration                          │
│ Object [1년]: 2년                                           │
│ Saved. [a/e/r/n/q] > a                                     │
│                                                             │
│ ✓ 4 triples approved and queued for graph loading          │
└─────────────────────────────────────────────────────────────┘
```

### 완료 기준

- [ ] NER 정확도 Precision ≥ 80%
- [ ] RE 정확도 Precision ≥ 70%
- [ ] HITL CLI 기본 기능 동작
- [ ] 승인된 트리플 그래프 로딩 확인
- [ ] 품질 메트릭 대시보드 (CLI 출력)

---

## Sprint 3: RAG 오케스트레이션 (2주)

**목표**: 하이브리드 RAG 파이프라인 완성, 출처 추적 시스템 구축

### Week 1: 검색 엔진

| ID | 태스크 | 상세 내용 | 예상 시간 | 결과물 |
|:---|:---|:---|:---|:---|
| S3.1 | 벡터 검색 구현 | Qdrant 유사도 검색, Top-K | 8h | vector_search() |
| S3.2 | 그래프 검색 구현 | SurrealDB 서브그래프 추출 | 12h | graph_search() |
| S3.3 | ACL 필터링 | 검색 결과에서 권한 없는 문서 제외 | 8h | acl_filter() |
| S3.4 | 결과 병합 | RRF(Reciprocal Rank Fusion) 알고리즘 | 8h | merge_results() |

### Week 2: 응답 생성

| ID | 태스크 | 상세 내용 | 예상 시간 | 결과물 |
|:---|:---|:---|:---|:---|
| S3.5 | 프롬프트 엔지니어링 | 시스템 프롬프트, 컨텍스트 포맷 | 8h | prompt.rs |
| S3.6 | LLM 연동 | OpenAI API / Ollama 추상화 | 8h | llm_client.rs |
| S3.7 | 스트리밍 응답 | SSE 기반 실시간 출력 | 8h | streaming.rs |
| S3.8 | 출처 추적 | Citation 메타데이터 추출 및 포맷 | 8h | citation.rs |
| S3.9 | E2E 통합 테스트 | 질문 → 답변 전체 흐름 | 4h | 통합 테스트 |

### RAG 오케스트레이터 흐름

```rust
// 의사 코드
pub async fn query(&self, question: &str, user: &User) -> Result<RagResponse> {
    // 1. 질문 분석
    let analysis = self.analyze_question(question).await?;
    
    // 2. 병렬 검색
    let (vector_results, graph_results) = tokio::join!(
        self.vector_search(&analysis, 20),
        self.graph_search(&analysis, 3)  // depth=3
    );
    
    // 3. ACL 필터링
    let filtered_vector = self.acl_filter(vector_results?, user)?;
    let filtered_graph = self.acl_filter(graph_results?, user)?;
    
    // 4. RRF 병합
    let merged = self.rrf_merge(filtered_vector, filtered_graph, 10);
    
    // 5. 프롬프트 구성
    let prompt = self.build_prompt(question, &merged, &analysis);
    
    // 6. LLM 응답 생성
    let response = self.llm.generate_stream(&prompt).await?;
    
    // 7. Citation 추출
    let citations = self.extract_citations(&response, &merged);
    
    Ok(RagResponse {
        answer: response,
        citations,
        sources: merged.into_iter().map(|r| r.source).collect(),
    })
}
```

### 프롬프트 템플릿

```
<system>
당신은 조직의 지식 전문가입니다.
제공된 컨텍스트 정보만을 사용하여 질문에 답변하세요.
답변에 사용한 정보의 출처를 반드시 명시하세요.
컨텍스트에 없는 정보는 "해당 정보를 찾을 수 없습니다"라고 답변하세요.

온톨로지 스키마:
{ontology_schema}
</system>

<context>
[1] 출처: {source_1}
{content_1}

[2] 출처: {source_2}
{content_2}

...
</context>

<question>
{user_question}
</question>

<instructions>
1. 컨텍스트를 주의 깊게 읽으세요.
2. 질문에 직접 관련된 정보만 사용하세요.
3. 답변 작성 시 [출처: N] 형식으로 인용하세요.
4. 확실하지 않은 정보는 언급하지 마세요.
</instructions>
```

### 완료 기준

- [ ] 하이브리드 검색 동작 확인
- [ ] ACL 필터링 테스트 통과
- [ ] 스트리밍 응답 구현
- [ ] Citation이 포함된 답변 생성
- [ ] 평균 응답 시간 < 5초 (최적화 전)

---

## Sprint 4: API 및 배포 (1주)

**목표**: REST API 완성, Kubernetes 배포, 문서화

### 태스크 목록

| ID | 태스크 | 상세 내용 | 예상 시간 | 결과물 |
|:---|:---|:---|:---|:---|
| S4.1 | REST API 구현 | Axum 기반 엔드포인트 | 12h | otl-api |
| S4.2 | OpenAPI 문서 | utoipa 활용 자동 생성 | 4h | openapi.json |
| S4.3 | 인증 미들웨어 | JWT 검증, 역할 추출 | 8h | auth.rs |
| S4.4 | Dockerfile | 멀티스테이지 빌드 | 4h | Dockerfile |
| S4.5 | K8s 매니페스트 | Deployment, Service, ConfigMap | 8h | deploy/kubernetes/ |
| S4.6 | ArgoCD 설정 | Application 정의, 동기화 | 4h | deploy/argocd/ |
| S4.7 | README 작성 | 설치, 사용법, 아키텍처 | 4h | README.md |
| S4.8 | 최종 테스트 | QA 셋 기반 평가 | 8h | 평가 보고서 |

### API 엔드포인트

```
POST   /api/v1/query              # RAG 질의
GET    /api/v1/documents          # 문서 목록 (ACL 적용)
POST   /api/v1/documents          # 문서 업로드
GET    /api/v1/documents/:id      # 문서 상세
DELETE /api/v1/documents/:id      # 문서 삭제

GET    /api/v1/graph/entities     # 개체 목록
GET    /api/v1/graph/entity/:id   # 개체 상세 (관계 포함)
GET    /api/v1/graph/search       # 그래프 검색

GET    /api/v1/ontology           # 온톨로지 스키마
PUT    /api/v1/ontology           # 온톨로지 업데이트 (admin)

GET    /api/v1/verify/pending     # 검증 대기 목록
POST   /api/v1/verify/:id/approve # 검증 승인
POST   /api/v1/verify/:id/reject  # 검증 거부

GET    /health                    # 헬스체크
GET    /metrics                   # Prometheus 메트릭
```

### Kubernetes 아키텍처

```yaml
# 배포 구성요소
┌─────────────────────────────────────────────────────────────────┐
│                        Kubernetes Cluster                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │ otl-api         │  │ otl-api         │  │ otl-api         │ │
│  │ (Deployment)    │  │ (replica 2)     │  │ (replica 3)     │ │
│  │ replicas: 3     │  │                 │  │                 │ │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘ │
│           │                    │                    │          │
│           └────────────────────┼────────────────────┘          │
│                                │                               │
│                    ┌───────────▼───────────┐                   │
│                    │ otl-api-service       │                   │
│                    │ (LoadBalancer)        │                   │
│                    └───────────────────────┘                   │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    StatefulSets                          │   │
│  │                                                          │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │   │
│  │  │ SurrealDB   │  │ Qdrant      │  │ PostgreSQL  │      │   │
│  │  │ (1 replica) │  │ (1 replica) │  │ (1 replica) │      │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘      │   │
│  │                                                          │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 완료 기준

- [ ] API 전체 엔드포인트 동작
- [ ] OpenAPI 문서 자동 생성
- [ ] Docker 이미지 빌드 및 푸시
- [ ] K8s 클러스터 배포 완료
- [ ] ArgoCD 동기화 확인
- [ ] README 및 문서화 완료
- [ ] QA 평가: Precision@5 ≥ 85%

---

## 평가 및 메트릭

### QA 평가 셋 구성

파일럿 도메인에서 50개의 질문-답변 쌍을 구성합니다:

| 카테고리 | 예시 질문 | 개수 |
|:---|:---|:---|
| 절차 질문 | "연차 신청 절차는?" | 15 |
| 사실 질문 | "경조휴가는 며칠인가?" | 15 |
| 비교 질문 | "병가와 연차의 차이는?" | 10 |
| 조건 질문 | "입사 1년 미만이면 연차가 몇 일?" | 10 |

### 평가 지표

```
Precision@K = (관련 문서 수 in Top-K) / K

환각률 = (출처 없는 문장 수) / (전체 문장 수)

응답 시간 = LLM 첫 토큰까지 시간 (p50, p95, p99)
```

### 평가 자동화

```bash
# QA 평가 실행
$ otl evaluate --qa-set qa_50.json --output report.json

# 결과 예시
{
  "precision_at_5": 0.87,
  "hallucination_rate": 0.03,
  "response_time_p50_ms": 1200,
  "response_time_p95_ms": 2800,
  "total_questions": 50,
  "passed": 46,
  "failed": 4
}
```

---

## 리스크 관리

| 리스크 | 확률 | 영향 | 완화 전략 | 담당 |
|:---|:---|:---|:---|:---|
| 온톨로지 설계 지연 | 중 | 높 | Sprint 0 기간 연장 버퍼, 반복적 개선 | PM |
| LLM API 비용 초과 | 중 | 중 | 캐싱, 로컬 모델(Ollama) 대안 | Dev |
| NER/RE 정확도 미달 | 높 | 높 | HITL 강화, 규칙 엔진 보강 | Analyst |
| 성능 목표 미달 | 중 | 중 | 인덱스 최적화, 캐싱 레이어 | Dev |
| 보안 취약점 | 낮 | 높 | 보안 감사, 의존성 스캔 | DevOps |

---

## 일정 요약

```
Week 1:  Sprint 0 - 기반 구축
         ├── 도메인 분석
         ├── 온톨로지 설계
         └── 프로젝트 초기화

Week 2-3: Sprint 1 - 파서 및 저장소
         ├── 문서 파서 (PDF, DOCX, XLSX)
         ├── OCR 통합
         └── DB 연동 (SurrealDB, Qdrant, PostgreSQL)

Week 4-5: Sprint 2 - 지식 추출
         ├── NER/RE 파이프라인
         ├── HITL 검증 도구
         └── 그래프 로딩

Week 6-7: Sprint 3 - RAG 오케스트레이션
         ├── 하이브리드 검색
         ├── LLM 연동
         └── 출처 추적

Week 8:  Sprint 4 - API 및 배포
         ├── REST API
         ├── Kubernetes 배포
         └── 문서화 및 평가
```

---

## 부록: 체크리스트

### Sprint 시작 전 체크리스트

- [ ] 이전 Sprint 회고 완료
- [ ] 태스크 우선순위 확정
- [ ] 담당자 배정
- [ ] 의존성 확인

### Sprint 종료 시 체크리스트

- [ ] 모든 태스크 완료 또는 이월 결정
- [ ] 테스트 커버리지 확인 (≥80%)
- [ ] 문서 업데이트
- [ ] 데모 준비
- [ ] 회고 미팅 진행
