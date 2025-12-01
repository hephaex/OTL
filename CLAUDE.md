# CLAUDE.md - OTL Project Configuration

> **⚠️ CONFIDENTIAL**: 이 파일은 프로젝트 내부 설정 파일입니다. 외부에 공개하지 마세요.
> 
> **저작권**: © 2024 hephaex@gmail.com. All rights reserved.

---

## 1. 프로젝트 개요

**OTL** (Ontology-based knowledge sysTem Library)는 온톨로지 기반 지식 그래프와 RAG를 결합한 조직 지식 관리 시스템입니다.

### 1.1 핵심 원칙

1. **품질 우선**: 자동화보다 정확성 (HITL 검증 필수)
2. **보안 내재화**: ACL 기반 문서별 접근 제어
3. **점진적 확장**: 파일럿 도메인에서 시작, 검증 후 확장
4. **추적 가능성**: 모든 답변에 출처 명시 (Citation)

### 1.2 저작권 및 기여 정책

```
Author: hephaex@gmail.com
License: Apache-2.0

모든 코드, 문서, 설계는 위 저작자에게 귀속됩니다.
AI 도구 사용 여부는 공개하지 않습니다.
커밋 메시지, 문서, 주석에 AI 도구명을 언급하지 마세요.
```

---

## 2. 멀티 에이전트 개발 구성

### 2.1 에이전트 역할 정의

| 에이전트 | 역할 | 담당 영역 |
|:---------|:-----|:----------|
| **Architect** | 시스템 설계 | 아키텍처, 온톨로지, 데이터 모델 |
| **Developer** | 구현 | Rust 코드, API, CLI |
| **Reviewer** | 코드 리뷰 | 품질 검증, 보안 점검 |
| **Tester** | 테스트 | 단위/통합 테스트, QA 평가 |
| **DocWriter** | 문서화 | README, API 문서, 가이드 |

### 2.2 에이전트 협업 워크플로우

```
[Architect] → 설계 문서 작성
      ↓
[Developer] → 코드 구현
      ↓
[Reviewer] → 코드 리뷰 (PR 형식)
      ↓
[Tester] → 테스트 실행 및 검증
      ↓
[DocWriter] → 문서 업데이트
      ↓
[Architect] → 최종 승인 → Merge
```

### 2.3 에이전트별 프롬프트 템플릿

#### Architect 에이전트
```markdown
당신은 OTL 프로젝트의 시스템 아키텍트입니다.

역할:
- 시스템 아키텍처 설계 및 검토
- 온톨로지 스키마 정의
- 기술 스택 결정
- 크레이트 간 의존성 관리

규칙:
1. 설계 결정에는 항상 근거를 명시하세요
2. 대안과 트레이드오프를 함께 설명하세요
3. .history에 설계 문서를 저장하세요
4. 저작자는 hephaex@gmail.com으로 표기하세요
```

#### Developer 에이전트
```markdown
당신은 OTL 프로젝트의 Rust 개발자입니다.

역할:
- 크레이트 구현
- 단위 테스트 작성
- 에러 처리 및 로깅

규칙:
1. Rust 2021 에디션, 안정 버전 기능만 사용
2. clippy 경고 0개 유지
3. 모든 public API에 문서 주석 작성
4. 커밋 메시지에 AI 도구명 언급 금지
```

#### Reviewer 에이전트
```markdown
당신은 OTL 프로젝트의 코드 리뷰어입니다.

역할:
- 코드 품질 검토
- 보안 취약점 점검
- 성능 이슈 식별
- 베스트 프랙티스 준수 확인

규칙:
1. 리뷰 결과를 .history에 저장
2. 심각도별 분류: CRITICAL, MAJOR, MINOR, SUGGESTION
3. 수정 방안을 구체적으로 제시
```

#### Tester 에이전트
```markdown
당신은 OTL 프로젝트의 QA 엔지니어입니다.

역할:
- 단위/통합 테스트 실행
- QA 평가셋 기반 검증
- 성능 테스트
- 회귀 테스트

규칙:
1. 테스트 결과를 .history에 저장
2. 실패한 테스트는 재현 단계 명시
3. 커버리지 80% 이상 유지
```

---

## 3. 개발 프로세스 및 히스토리 관리

### 3.1 .history 폴더 구조

```
.history/
├── README.md                           # 히스토리 관리 가이드
├── sprint00/
│   ├── 2024-01-15_sprint00_phase01_dev.md      # 개발 기록
│   ├── 2024-01-15_sprint00_phase01_review.md   # 리뷰 기록
│   ├── 2024-01-16_sprint00_phase01_verify.md   # 검증 기록
│   ├── 2024-01-16_sprint00_phase01_fix.md      # 수정 기록 (오류 시)
│   └── 2024-01-17_sprint00_phase01_final.md    # 최종 완료
├── sprint01/
│   └── ...
└── templates/
    ├── dev_template.md
    ├── review_template.md
    ├── verify_template.md
    └── fix_template.md
```

### 3.2 Phase 완료 워크플로우

```
┌─────────────────────────────────────────────────────────────┐
│                    Phase 완료 프로세스                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. 개발 (Developer)                                        │
│     └─→ YYYY-MM-DD_sprintXX_phaseYY_dev.md 작성            │
│                                                             │
│  2. 리뷰 (Reviewer)                                         │
│     └─→ YYYY-MM-DD_sprintXX_phaseYY_review.md 작성         │
│     └─→ 이슈 발견 시 → 3a (수정)                            │
│     └─→ 이슈 없음 → 3b (검증)                               │
│                                                             │
│  3a. 수정 (Developer)                                       │
│      └─→ YYYY-MM-DD_sprintXX_phaseYY_fix.md 작성           │
│      └─→ 2. 리뷰로 복귀                                     │
│                                                             │
│  3b. 검증 (Tester)                                          │
│      └─→ YYYY-MM-DD_sprintXX_phaseYY_verify.md 작성        │
│      └─→ 검증 실패 → 3a (수정)                              │
│      └─→ 검증 성공 → 4 (완료)                               │
│                                                             │
│  4. 완료 (Architect)                                        │
│     └─→ YYYY-MM-DD_sprintXX_phaseYY_final.md 작성          │
│     └─→ 다음 Phase 진행                                     │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 3.3 파일 명명 규칙

```
{YYYY-MM-DD}_{sprintXX}_{phaseYY}_{type}.md

- YYYY-MM-DD: 작성 날짜
- sprintXX: 스프린트 번호 (sprint00, sprint01, ...)
- phaseYY: 페이즈 번호 (phase01, phase02, ...)
- type: 문서 유형
  - dev: 개발 기록
  - review: 리뷰 결과
  - verify: 검증 결과
  - fix: 수정 내역
  - final: 최종 완료
```

---

## 4. 온톨로지 구축 가이드

### 4.1 새 도메인 온톨로지 구축 절차

```
1. 도메인 분석 (Sprint 0)
   ├── 문서 샘플 수집 (50개+)
   ├── FAQ 목록 작성 (20개+)
   ├── 용어집 초안 작성
   └── 개체-관계 후보 식별

2. 온톨로지 설계
   ├── 클래스 계층 정의
   ├── 속성 정의 (Object/Datatype)
   ├── 제약조건 설정
   └── OWL 파일 작성

3. 온톨로지 검증
   ├── Protégé에서 일관성 검사
   ├── 샘플 인스턴스 생성
   ├── SPARQL 쿼리 테스트
   └── 전문가 리뷰

4. 온톨로지 적용
   ├── SurrealDB 스키마 변환
   ├── NER 규칙 업데이트
   ├── RE 프롬프트 조정
   └── HITL 검증 후 배포
```

### 4.2 기존 HR 온톨로지 확장 예시

```owl
<!-- 새 클래스 추가 -->
<owl:Class rdf:about="#TrainingProgram">
    <rdfs:subClassOf rdf:resource="#Document"/>
    <rdfs:label>교육 프로그램</rdfs:label>
</owl:Class>

<!-- 새 관계 추가 -->
<owl:ObjectProperty rdf:about="#completedTraining">
    <rdfs:domain rdf:resource="#Employee"/>
    <rdfs:range rdf:resource="#TrainingProgram"/>
</owl:ObjectProperty>
```

### 4.3 온톨로지 버전 관리

```
ontologies/
├── hr/
│   ├── v1.0.0/
│   │   └── hr_ontology.owl
│   ├── v1.1.0/
│   │   ├── hr_ontology.owl
│   │   └── CHANGELOG.md
│   └── current -> v1.1.0/
└── finance/  # 향후 확장
    └── ...
```

---

## 5. 코드 스타일 및 규칙

### 5.1 Rust 코딩 규칙

```rust
//! 모듈 문서는 //!로 시작
//! 
//! Author: hephaex@gmail.com

/// 함수/구조체 문서는 ///로 시작
/// 
/// # Arguments
/// 
/// * `param` - 파라미터 설명
/// 
/// # Returns
/// 
/// 반환값 설명
/// 
/// # Errors
/// 
/// 발생 가능한 에러
pub fn example_function(param: &str) -> Result<(), Error> {
    // 구현
}
```

### 5.2 커밋 메시지 규칙

```
<type>(<scope>): <subject>

<body>

<footer>

# type: feat, fix, docs, style, refactor, test, chore
# scope: otl-core, otl-parser, otl-rag, etc.
# 
# ⚠️ AI 도구명 언급 금지
# ✅ Author: hephaex@gmail.com
```

### 5.3 금지 사항

```
❌ 금지:
- 커밋 메시지에 "Claude", "GPT", "AI", "LLM generated" 등 언급
- 주석에 AI 도구 참조
- README, CONTRIBUTING에 AI 사용 언급
- PR 설명에 AI 어시스턴트 언급

✅ 권장:
- 저작자: hephaex@gmail.com
- "Designed by author", "Implemented by author"
```

---

## 6. 스프린트별 Phase 정의

### Sprint 0: 기반 구축

| Phase | 담당 | 산출물 |
|:------|:-----|:-------|
| P01 | Architect | 도메인 분석 보고서 |
| P02 | Architect | 온톨로지 설계 (OWL) |
| P03 | Developer | 프로젝트 초기화 |
| P04 | DocWriter | 문서화 |

### Sprint 1: 파서 및 저장소

| Phase | 담당 | 산출물 |
|:------|:-----|:-------|
| P01 | Developer | otl-parser 구현 |
| P02 | Developer | otl-graph 구현 |
| P03 | Developer | otl-vector 구현 |
| P04 | Tester | 통합 테스트 |

### Sprint 2: 지식 추출

| Phase | 담당 | 산출물 |
|:------|:-----|:-------|
| P01 | Developer | NER 구현 |
| P02 | Developer | RE 구현 |
| P03 | Developer | HITL CLI 구현 |
| P04 | Tester | 추출 품질 평가 |

### Sprint 3: RAG 파이프라인

| Phase | 담당 | 산출물 |
|:------|:-----|:-------|
| P01 | Developer | 하이브리드 검색 |
| P02 | Developer | LLM 연동 |
| P03 | Developer | 프롬프트 최적화 |
| P04 | Tester | QA 평가 |

### Sprint 4: API 및 배포

| Phase | 담당 | 산출물 |
|:------|:-----|:-------|
| P01 | Developer | REST API |
| P02 | Developer | Docker/K8s |
| P03 | Tester | 성능 테스트 |
| P04 | DocWriter | 최종 문서화 |

---

## 7. 환경 설정

### 7.1 필수 도구

```bash
# Rust
rustup default stable
rustup component add clippy rustfmt

# Docker
docker --version  # 24.0+
docker compose version  # 2.20+

# 기타
protege  # 온톨로지 편집
jq       # JSON 처리
```

### 7.2 개발 환경 시작

```bash
# 인프라 시작
docker compose up -d

# 빌드
cargo build

# 테스트
cargo test

# 린트
cargo clippy --all-targets
cargo fmt --all -- --check
```

---

## 8. 보안 규칙

### 8.1 민감 정보 관리

```
❌ 절대 커밋하지 말 것:
- .env (API 키, 비밀번호)
- config.toml (실제 설정)
- *.pem, *.key (인증서)

✅ 템플릿만 커밋:
- .env.example
- config.example.toml
```

### 8.2 ACL 규칙

```
- public: 인증 없이 접근 가능
- internal: 조직 구성원만
- confidential: 특정 역할/부서만
- restricted: 지정된 개인만
```

---

*이 문서는 프로젝트 내부용입니다. 외부 공개 금지.*

*Author: hephaex@gmail.com*
