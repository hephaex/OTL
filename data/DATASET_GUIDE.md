# OTL 데이터셋 가이드

## 개요

이 문서는 OTL 프로젝트의 파일럿 도메인(인사 규정 및 매뉴얼)을 위한 데이터셋 구성 전략을 설명합니다.

## 데이터셋 요구사항

프로젝트 성공을 위해 다음 데이터가 필요합니다:

| 단계 | 필요 데이터 | 규모 | 목적 |
|:---|:---|:---|:---|
| Sprint 0 | FAQ 목록 | 20개 | 도메인 분석, 온톨로지 설계 |
| Sprint 1 | 다양한 형식의 문서 | 50개 | 파서 개발 및 테스트 |
| Sprint 2 | 추출 대상 트리플 | 100개+ | NER/RE 검증, HITL 테스트 |
| Sprint 4 | QA 평가셋 | 50개 | Precision@K, 환각률 측정 |

---

## 1. 오픈 데이터셋 소스

### 1.1 SHRM (Society for Human Resource Management)

**URL**: https://www.shrm.org/topics-tools/tools

**내용**:
- 휴가 정책 템플릿 (PDF)
- 직원 핸드북 샘플 (DOCX)
- 휴가 신청서 양식 (XLSX)
- 교육 자료 (PPTX)

**라이선스**: 무료 계정 등록 후 다운로드 (비상업적 사용)

**활용 방법**:
```bash
# 다운로드 후 저장
cp ~/Downloads/shrm_*.pdf data/sample_documents/
cp ~/Downloads/shrm_*.docx data/sample_documents/
```

### 1.2 U.S. Department of Labor

**URL**: https://www.dol.gov/general/publications

**내용**:
- FMLA (가족의료휴가법) 가이드 (PDF)
- 임금/근로시간 정책 (PDF)
- 타임시트 양식 (XLSX)

**라이선스**: Public Domain (공공 문서)

**주요 문서**:
- `WHD1420_FMLAGuide.pdf` - 휴가 정책 예시
- `EmployeeRightsWHD.pdf` - 직원 권리 안내

### 1.3 GitHub 오픈소스 템플릿

**검색 URL**: https://github.com/search?q=hr+policy+templates

**추천 저장소**:
- `OpenSourceHR/awesome-hr` - HR 리소스 모음
- `basecamp/handbook` - 직원 핸드북 예시 (Markdown)

**라이선스**: MIT, CC0, Apache-2.0 (저장소별 확인)

### 1.4 Kaggle HR 데이터셋

**URL**: https://www.kaggle.com/search?q=hr+policy

**활용**:
- 직원 데이터 (XLSX) - 테스트 데이터로 활용
- 휴가 기록 - QA 셋 생성 참고

---

## 2. 합성 데이터셋 구조

오픈 소스만으로 부족하거나, ACL 테스트/한국어 지원이 필요한 경우 합성 데이터를 생성합니다.

### 2.1 파일 구조

```
data/synthetic/
├── documents/
│   ├── HR_Policy_2024.pdf          # 종합 인사 규정 (20페이지)
│   ├── HR_Policy_2024_ko.pdf       # 한국어 버전
│   ├── Vacation_Request_Form.xlsx  # 휴가 신청서 양식
│   ├── Expense_Policy.docx         # 경비 처리 규정
│   ├── Onboarding_Guide.docx       # 신입사원 가이드
│   ├── Training_HR_Basics.pptx     # HR 기초 교육
│   ├── Leave_Policy_Summary.md     # 휴가 정책 요약 (Markdown)
│   ├── Approval_Workflow.md        # 승인 프로세스
│   ├── Department_Structure.xlsx   # 조직도
│   └── Confidential_Salary.xlsx    # [RESTRICTED] 급여 정보
│
├── scanned/                        # OCR 테스트용
│   ├── Old_Policy_Scan.pdf         # 스캔된 문서 시뮬레이션
│   └── Handwritten_Form.png        # 필기 양식
│
└── metadata/
    └── documents.json              # 문서 메타데이터 및 ACL 정보
```

### 2.2 ACL 설정 예시

```json
{
  "documents": [
    {
      "id": "doc-001",
      "title": "HR Policy 2024",
      "file": "HR_Policy_2024.pdf",
      "access_level": "internal",
      "department": null,
      "required_roles": []
    },
    {
      "id": "doc-010",
      "title": "Confidential Salary Information",
      "file": "Confidential_Salary.xlsx",
      "access_level": "restricted",
      "department": "HR",
      "required_roles": ["hr_admin"],
      "allowed_users": ["ceo", "cfo", "hr_director"]
    }
  ]
}
```

---

## 3. FAQ 목록 (Sprint 0용)

도메인 전문가 인터뷰 대신 사용할 수 있는 대표 질문 20개:

### 휴가 관련 (8개)
1. 연차 휴가 신청 절차는?
2. 병가 사용 시 필요한 서류는?
3. 경조휴가는 며칠인가?
4. 입사 1년 미만일 때 연차는 몇 일?
5. 휴가 취소는 어떻게 하나?
6. 연차 이월 정책은?
7. 반차 사용이 가능한가?
8. 육아휴직 신청 조건은?

### 경비/승인 관련 (6개)
9. 경비 청구 절차는?
10. 법인카드 사용 기준은?
11. 출장비 정산 방법은?
12. 결재 라인은 어떻게 되나?
13. 긴급 결재 처리 방법은?
14. 예산 초과 시 승인 절차는?

### 조직/인사 관련 (6개)
15. 부서 이동 신청 방법은?
16. 승진 평가 기준은?
17. 신입사원 온보딩 일정은?
18. 재택근무 신청 절차는?
19. 퇴직 절차는?
20. 인사 담당자 연락처는?

---

## 4. QA 평가셋 (50개)

### 4.1 카테고리별 분배

| 카테고리 | 설명 | 개수 |
|:---|:---|:---|
| 절차 질문 | "어떻게 ~하나요?" | 15 |
| 사실 질문 | "~는 며칠/몇 개?" | 15 |
| 비교 질문 | "A와 B의 차이는?" | 10 |
| 조건 질문 | "~인 경우 ~는?" | 10 |

### 4.2 평가셋 파일 형식

`data/evaluation/qa_set.json`:

```json
{
  "version": "1.0",
  "domain": "hr_policy",
  "questions": [
    {
      "id": "qa-001",
      "category": "procedural",
      "question": "연차 휴가 신청 절차는?",
      "expected_answer": "1. 휴가신청서 작성 2. 팀장 승인 요청 3. 인사팀 검토 4. 최종 승인",
      "expected_sources": [
        {"document": "HR_Policy_2024.pdf", "page": 15, "section": "3.2 연차휴가"}
      ],
      "difficulty": "easy"
    },
    {
      "id": "qa-002",
      "category": "factual",
      "question": "경조휴가는 며칠인가?",
      "expected_answer": "본인 결혼 5일, 자녀 결혼 2일, 부모 사망 5일, 배우자 부모 사망 3일",
      "expected_sources": [
        {"document": "HR_Policy_2024.pdf", "page": 18, "section": "3.4 경조휴가"}
      ],
      "difficulty": "easy"
    },
    {
      "id": "qa-003",
      "category": "comparative",
      "question": "연차와 병가의 차이는?",
      "expected_answer": "연차는 사전 승인 필요, 병가는 사후 진단서 제출. 연차는 연간 15일, 병가는 유급 10일까지.",
      "expected_sources": [
        {"document": "HR_Policy_2024.pdf", "page": 15, "section": "3.2"},
        {"document": "HR_Policy_2024.pdf", "page": 16, "section": "3.3"}
      ],
      "difficulty": "medium"
    },
    {
      "id": "qa-004",
      "category": "conditional",
      "question": "입사 1년 미만이면 연차가 몇 일인가?",
      "expected_answer": "입사 후 매월 1일씩 발생하여 최대 11일까지 사용 가능",
      "expected_sources": [
        {"document": "HR_Policy_2024.pdf", "page": 15, "section": "3.2.1 연차 발생"}
      ],
      "difficulty": "medium"
    }
  ]
}
```

---

## 5. 온톨로지 스키마 (OWL)

`data/ontologies/hr_ontology.owl` 참조.

### 주요 클래스

```
hr:Employee          # 직원
hr:Department        # 부서
hr:Position          # 직급
hr:LeaveType         # 휴가 유형
hr:ApprovalProcess   # 승인 프로세스
hr:Regulation        # 규정
hr:Form              # 양식
```

### 주요 관계

```
hr:belongsTo         # Employee → Department
hr:hasPosition       # Employee → Position
hr:requiresApproval  # LeaveType → ApprovalProcess
hr:governs           # Regulation → [LeaveType, ApprovalProcess, ...]
hr:usesForm          # ApprovalProcess → Form
hr:managedBy         # Department → Employee
```

---

## 6. 데이터 수집 스크립트

### 6.1 오픈 데이터 다운로드 가이드

```bash
#!/bin/bash
# scripts/download_datasets.sh

mkdir -p data/sample_documents

echo "=== 데이터셋 수집 가이드 ==="
echo ""
echo "1. SHRM 템플릿:"
echo "   - https://www.shrm.org/topics-tools/tools 방문"
echo "   - 'Sample Policies' 검색"
echo "   - 다운로드 후 data/sample_documents/ 에 저장"
echo ""
echo "2. DOL 문서:"
echo "   - https://www.dol.gov/agencies/whd/fmla 방문"
echo "   - FMLA Guide 다운로드"
echo "   - data/sample_documents/ 에 저장"
echo ""
echo "3. GitHub 템플릿:"
echo "   - git clone https://github.com/basecamp/handbook.git /tmp/handbook"
echo "   - cp /tmp/handbook/*.md data/sample_documents/"
echo ""
echo "=== 완료 후 다음 명령 실행 ==="
echo "cargo run -p otl-cli -- ingest data/sample_documents/"
```

### 6.2 합성 데이터 검증

```bash
# 데이터셋 검증
cargo run -p otl-cli -- validate data/synthetic/

# QA 셋으로 평가
cargo run -p otl-cli -- evaluate --qa-set data/evaluation/qa_set.json
```

---

## 7. 다음 단계

1. **Sprint 0**: FAQ 20개 기반으로 도메인 분석 및 온톨로지 설계
2. **Sprint 1**: 오픈 데이터셋으로 파서 테스트, 합성 데이터로 ACL 검증
3. **Sprint 2**: 추출된 트리플을 HITL CLI로 검증
4. **Sprint 4**: QA 평가셋으로 Precision@5 ≥ 85% 달성 확인

---

## 참고 자료

- [SHRM Resource Hub](https://www.shrm.org/topics-tools/tools)
- [U.S. DOL Publications](https://www.dol.gov/general/publications)
- [GitHub HR Templates](https://github.com/search?q=hr+policy+templates)
- [Kaggle HR Datasets](https://www.kaggle.com/search?q=hr+policy)
