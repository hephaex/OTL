# .history - 개발 히스토리 관리

## 개요

이 폴더는 OTL 프로젝트의 모든 개발 과정을 기록합니다.
각 스프린트의 Phase별로 개발, 리뷰, 검증, 수정 기록을 체계적으로 관리합니다.

---

## 폴더 구조

```
.history/
├── README.md                           # 이 파일
├── templates/                          # 문서 템플릿
│   ├── dev_template.md
│   ├── review_template.md
│   ├── verify_template.md
│   └── fix_template.md
├── sprint00/
│   ├── 2024-XX-XX_sprint00_phase01_dev.md
│   ├── 2024-XX-XX_sprint00_phase01_review.md
│   └── ...
├── sprint01/
├── sprint02/
├── sprint03/
└── sprint04/
```

---

## 파일 명명 규칙

```
{YYYY-MM-DD}_{sprintXX}_{phaseYY}_{type}.md
```

### 구성 요소

| 요소 | 설명 | 예시 |
|:-----|:-----|:-----|
| YYYY-MM-DD | 작성 날짜 | 2024-01-15 |
| sprintXX | 스프린트 번호 | sprint00, sprint01 |
| phaseYY | 페이즈 번호 | phase01, phase02 |
| type | 문서 유형 | dev, review, verify, fix, final |

### 문서 유형

| Type | 설명 | 작성자 |
|:-----|:-----|:-------|
| `dev` | 개발 완료 보고서 | Developer |
| `review` | 코드 리뷰 결과 | Reviewer |
| `verify` | 검증/테스트 결과 | Tester |
| `fix` | 오류 수정 내역 | Developer |
| `final` | Phase 완료 승인 | Architect |

---

## 워크플로우

### Phase 완료 프로세스

```
Developer → dev.md 작성
    ↓
Reviewer → review.md 작성
    ↓
    ├─ [이슈 있음] → Developer → fix.md 작성 → Reviewer (반복)
    │
    └─ [이슈 없음] → Tester → verify.md 작성
                        ↓
                        ├─ [검증 실패] → Developer → fix.md 작성 (반복)
                        │
                        └─ [검증 성공] → Architect → final.md 작성
                                            ↓
                                        다음 Phase 진행
```

### 예시: Sprint 1, Phase 01

```
2024-01-20_sprint01_phase01_dev.md      # Developer: otl-parser 구현 완료
2024-01-21_sprint01_phase01_review.md   # Reviewer: 코드 리뷰 (MINOR 이슈 3개)
2024-01-21_sprint01_phase01_fix.md      # Developer: MINOR 이슈 수정
2024-01-22_sprint01_phase01_review.md   # Reviewer: 재리뷰 통과
2024-01-22_sprint01_phase01_verify.md   # Tester: 테스트 통과 (coverage 85%)
2024-01-23_sprint01_phase01_final.md    # Architect: Phase 완료 승인
```

---

## 상태 추적

### Sprint/Phase 상태

| 상태 | 의미 |
|:-----|:-----|
| 🔴 Not Started | 시작 전 |
| 🟡 In Progress | 진행 중 |
| 🔵 In Review | 리뷰 중 |
| 🟣 In Verification | 검증 중 |
| 🟢 Completed | 완료 |
| ⚫ Blocked | 차단됨 |

### 현재 진행 상황

> 이 섹션은 스프린트 진행 시 업데이트됩니다.

| Sprint | Phase | Status | Last Updated |
|:-------|:------|:-------|:-------------|
| Sprint 0 | P01 | 🔴 Not Started | - |
| Sprint 0 | P02 | 🔴 Not Started | - |
| Sprint 0 | P03 | 🔴 Not Started | - |
| Sprint 0 | P04 | 🔴 Not Started | - |

---

## 주의사항

### ⚠️ 필수 규칙

1. **모든 Phase는 dev.md로 시작해야 합니다**
2. **review.md 없이 verify.md 작성 불가**
3. **fix.md 작성 후 반드시 재리뷰 진행**
4. **final.md는 Architect만 작성 가능**

### 🚫 금지 사항

```
❌ AI 도구명 언급 금지 (Claude, GPT, Copilot 등)
❌ 자동 생성 표시 금지
❌ 외부 공개 금지

✅ 저작자는 hep@gmail.com으로 표기
✅ "Designed by author", "Implemented by author" 사용
```

---

## 문서 작성 가이드

### 1. dev.md (개발 보고서)

```markdown
# [Sprint X] Phase Y - 개발 보고서

## 메타데이터
- 작성일: YYYY-MM-DD
- 작성자: hep@gmail.com
- 담당 에이전트: Developer

## 구현 내용
- 구현한 기능 목록
- 변경된 파일 목록
- 의존성 추가 내역

## 테스트 결과
- 단위 테스트 실행 결과
- 커버리지

## 알려진 이슈
- 해결되지 않은 문제점

## 다음 단계
- 리뷰 요청 사항
```

### 2. review.md (리뷰 결과)

```markdown
# [Sprint X] Phase Y - 코드 리뷰

## 메타데이터
- 리뷰일: YYYY-MM-DD
- 리뷰어: hep@gmail.com
- 담당 에이전트: Reviewer

## 리뷰 요약
- 전체 평가: APPROVED / CHANGES_REQUESTED
- 발견된 이슈 수: N개

## 이슈 목록

### CRITICAL (즉시 수정 필요)
- [ ] 이슈 설명

### MAJOR (수정 권장)
- [ ] 이슈 설명

### MINOR (선택적 수정)
- [ ] 이슈 설명

### SUGGESTION (제안)
- [ ] 이슈 설명

## 결론
- 다음 단계 권고
```

### 3. verify.md (검증 결과)

```markdown
# [Sprint X] Phase Y - 검증 보고서

## 메타데이터
- 검증일: YYYY-MM-DD
- 검증자: hep@gmail.com
- 담당 에이전트: Tester

## 테스트 환경
- OS, Rust 버전, 의존성

## 테스트 결과

### 단위 테스트
- 총 테스트: N개
- 성공: N개
- 실패: N개
- 커버리지: XX%

### 통합 테스트
- 테스트 시나리오 결과

### 성능 테스트 (해당 시)
- 응답 시간, 처리량

## 발견된 문제
- 재현 단계 포함

## 결론
- PASS / FAIL
```

### 4. fix.md (수정 내역)

```markdown
# [Sprint X] Phase Y - 수정 보고서

## 메타데이터
- 수정일: YYYY-MM-DD
- 수정자: hep@gmail.com
- 담당 에이전트: Developer

## 수정 사항

### 수정된 이슈
- [ ] 이슈 ID: 수정 내용

### 변경된 파일
- 파일 목록

## 테스트 결과
- 수정 후 테스트 결과

## 재리뷰 요청
- 확인 필요 사항
```

### 5. final.md (완료 승인)

```markdown
# [Sprint X] Phase Y - 완료 승인

## 메타데이터
- 승인일: YYYY-MM-DD
- 승인자: hep@gmail.com
- 담당 에이전트: Architect

## Phase 요약
- 완료된 기능
- 주요 변경 사항

## 검토 결과
- 품질 평가
- 보안 검토

## 산출물
- 생성된 파일 목록

## 다음 Phase
- 다음 Phase 설명
- 시작 조건

## 승인
✅ Phase Y 완료 승인
```

---

*Author: hep@gmail.com*
