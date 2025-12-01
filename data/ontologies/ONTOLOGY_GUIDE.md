# 온톨로지 구축 가이드

## 개요

이 문서는 OTL 프로젝트에서 새로운 도메인 온톨로지를 구축하거나 기존 온톨로지를 확장하는 방법을 설명합니다.

> **저작자**: hephaex@gmail.com

---

## 1. 온톨로지 구축 프로세스

### 1.1 전체 워크플로우

```
┌─────────────────────────────────────────────────────────────────┐
│                    온톨로지 구축 프로세스                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Phase 1: 도메인 분석                                            │
│  ├── 문서 샘플 수집 (50+)                                        │
│  ├── FAQ 목록 작성 (20+)                                         │
│  ├── 용어집 초안                                                 │
│  └── 개체-관계 후보 식별                                          │
│           ↓                                                     │
│  Phase 2: 온톨로지 설계                                          │
│  ├── 클래스 계층 정의                                            │
│  ├── 속성 정의 (Object/Datatype)                                │
│  ├── 제약조건 설정                                               │
│  └── OWL 파일 작성                                              │
│           ↓                                                     │
│  Phase 3: 온톨로지 검증                                          │
│  ├── Protégé 일관성 검사                                         │
│  ├── 샘플 인스턴스 생성                                          │
│  ├── SPARQL 쿼리 테스트                                          │
│  └── 전문가 리뷰                                                 │
│           ↓                                                     │
│  Phase 4: 시스템 통합                                            │
│  ├── SurrealDB 스키마 변환                                       │
│  ├── NER 규칙 업데이트                                           │
│  ├── RE 프롬프트 조정                                            │
│  └── HITL 검증 후 배포                                           │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 2. Phase 1: 도메인 분석

### 2.1 문서 샘플 수집

**목표**: 최소 50개 문서 수집

#### 수집 기준

| 항목 | 기준 |
|:-----|:-----|
| 포맷 다양성 | PDF, DOCX, XLSX, PPTX, MD 포함 |
| 내용 범위 | 도메인의 주요 영역 커버 |
| 대표성 | 일상적으로 자주 참조되는 문서 |
| 복잡도 | 단순~복잡 문서 혼합 |

#### 수집 체크리스트

```markdown
## 문서 수집 체크리스트

### 필수 문서 유형
- [ ] 정책/규정 문서 (10개+)
- [ ] 절차 매뉴얼 (10개+)
- [ ] 양식 템플릿 (5개+)
- [ ] 교육 자료 (5개+)
- [ ] FAQ/가이드 (5개+)

### 포맷별 수량
- [ ] PDF: 15개+
- [ ] DOCX: 15개+
- [ ] XLSX: 10개+
- [ ] PPTX: 5개+
- [ ] MD: 5개+
```

### 2.2 FAQ 목록 작성

**목표**: 최소 20개 FAQ 수집

#### FAQ 수집 방법

1. **사용자 인터뷰**: 실제 질문 수집
2. **검색 로그 분석**: 자주 검색되는 키워드
3. **문의 기록**: 헬프데스크 문의 분석
4. **전문가 브레인스토밍**: 예상 질문 도출

#### FAQ 분류 기준

| 카테고리 | 설명 | 예시 |
|:---------|:-----|:-----|
| 절차 질문 | "어떻게 ~하나요?" | 휴가 신청 방법 |
| 사실 질문 | "~는 얼마/며칠인가요?" | 연차 일수 |
| 비교 질문 | "A와 B의 차이는?" | 병가 vs 연차 |
| 조건 질문 | "~인 경우에는?" | 1년 미만 직원 휴가 |

### 2.3 용어집 초안

#### 용어집 템플릿

```markdown
## 도메인 용어집

### [용어 1]
- **정의**: 명확한 정의
- **동의어**: 같은 의미의 다른 표현들
- **관련 용어**: 연관된 용어들
- **예시**: 사용 예시
- **출처**: 정의 출처 문서

### [용어 2]
...
```

#### 용어 식별 기준

- 문서에서 반복적으로 등장하는 용어
- 도메인 특화 전문 용어
- 여러 표현으로 사용되는 개념 (동의어)
- 계층 관계가 있는 용어들

### 2.4 개체-관계 후보 식별

#### 개체(Entity) 후보

| 유형 | 식별 기준 | 예시 |
|:-----|:----------|:-----|
| 행위자 | 주어로 자주 등장 | 직원, 팀장, 인사팀 |
| 대상 | 목적어로 자주 등장 | 휴가, 경비, 문서 |
| 개념 | 정의가 필요한 용어 | 승인절차, 정책 |
| 속성 | 개체의 특성 | 일수, 금액, 날짜 |

#### 관계(Relation) 후보

| 패턴 | 관계 유형 | 예시 |
|:-----|:----------|:-----|
| A는 B에 속한다 | 소속 | 직원 belongsTo 부서 |
| A는 B를 요구한다 | 요구 | 휴가 requiresApproval 승인 |
| A는 B를 관리한다 | 관리 | 팀장 manages 팀원 |
| A는 B로 정의된다 | 정의 | 정책 defines 절차 |

---

## 3. Phase 2: 온톨로지 설계

### 3.1 클래스 계층 정의

#### 클래스 설계 원칙

1. **단일 상속 우선**: 복잡한 다중 상속 피하기
2. **적절한 추상화**: 너무 일반적이거나 구체적이지 않게
3. **확장 가능성**: 향후 확장을 고려한 구조
4. **명확한 구분**: 클래스 간 경계 명확하게

#### 클래스 템플릿

```owl
<owl:Class rdf:about="#ClassName">
    <rdfs:subClassOf rdf:resource="#ParentClass"/>
    <rdfs:label>클래스 라벨</rdfs:label>
    <rdfs:comment>클래스 설명</rdfs:comment>
</owl:Class>
```

### 3.2 속성 정의

#### Object Property (관계)

```owl
<owl:ObjectProperty rdf:about="#propertyName">
    <rdfs:domain rdf:resource="#DomainClass"/>
    <rdfs:range rdf:resource="#RangeClass"/>
    <rdfs:label>속성 라벨</rdfs:label>
    <rdfs:comment>속성 설명</rdfs:comment>
</owl:ObjectProperty>
```

#### Datatype Property (데이터 속성)

```owl
<owl:DatatypeProperty rdf:about="#propertyName">
    <rdfs:domain rdf:resource="#DomainClass"/>
    <rdfs:range rdf:resource="xsd:string"/>  <!-- string, integer, date, boolean, etc. -->
    <rdfs:label>속성 라벨</rdfs:label>
</owl:DatatypeProperty>
```

### 3.3 제약조건 설정

#### Cardinality 제약

```owl
<!-- 정확히 1개 -->
<owl:Restriction>
    <owl:onProperty rdf:resource="#propertyName"/>
    <owl:cardinality rdf:datatype="xsd:nonNegativeInteger">1</owl:cardinality>
</owl:Restriction>

<!-- 최소 1개 -->
<owl:Restriction>
    <owl:onProperty rdf:resource="#propertyName"/>
    <owl:minCardinality rdf:datatype="xsd:nonNegativeInteger">1</owl:minCardinality>
</owl:Restriction>
```

### 3.4 OWL 파일 구조

```owl
<?xml version="1.0"?>
<rdf:RDF xmlns="http://your-domain/ontology/domain#"
     xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
     xmlns:owl="http://www.w3.org/2002/07/owl#"
     xmlns:rdfs="http://www.w3.org/2000/01/rdf-schema#"
     xmlns:xsd="http://www.w3.org/2001/XMLSchema#">
    
    <!-- Ontology Declaration -->
    <owl:Ontology rdf:about="http://your-domain/ontology/domain">
        <rdfs:label>Domain Ontology</rdfs:label>
        <owl:versionInfo>1.0.0</owl:versionInfo>
    </owl:Ontology>

    <!-- Classes -->
    <!-- ... -->

    <!-- Object Properties -->
    <!-- ... -->

    <!-- Datatype Properties -->
    <!-- ... -->

</rdf:RDF>
```

---

## 4. Phase 3: 온톨로지 검증

### 4.1 Protégé 검증

#### 설치 및 실행

```bash
# Protégé 다운로드
# https://protege.stanford.edu/products.php

# OWL 파일 열기
# File > Open > your_ontology.owl
```

#### 검증 체크리스트

- [ ] Reasoner 실행 (HermiT/Pellet)
- [ ] 일관성 검사 통과
- [ ] 불만족 클래스 없음
- [ ] 순환 종속성 없음

### 4.2 샘플 인스턴스

#### 인스턴스 생성 예시

```owl
<owl:NamedIndividual rdf:about="#Employee001">
    <rdf:type rdf:resource="#Employee"/>
    <employeeId>EMP001</employeeId>
    <name>홍길동</name>
    <belongsTo rdf:resource="#EngineeringDept"/>
</owl:NamedIndividual>
```

### 4.3 SPARQL 테스트 쿼리

```sparql
# 모든 직원과 소속 부서 조회
PREFIX hr: <http://your-domain/ontology/hr#>

SELECT ?employee ?department
WHERE {
    ?employee a hr:Employee .
    ?employee hr:belongsTo ?department .
}

# 특정 부서의 모든 휴가 유형 조회
SELECT ?leaveType ?maxDays
WHERE {
    ?leaveType a hr:LeaveType .
    ?leaveType hr:maxDays ?maxDays .
}
```

---

## 5. Phase 4: 시스템 통합

### 5.1 SurrealDB 스키마 변환

#### OWL → SurrealDB 매핑

| OWL 요소 | SurrealDB 요소 |
|:---------|:---------------|
| Class | Table |
| ObjectProperty | RELATE (graph edge) |
| DatatypeProperty | Field |
| Individual | Record |

#### 스키마 정의 예시

```surql
-- 테이블 정의
DEFINE TABLE employee SCHEMAFULL;
DEFINE FIELD employee_id ON employee TYPE string;
DEFINE FIELD name ON employee TYPE string;
DEFINE FIELD department ON employee TYPE record(department);

-- 관계 정의
DEFINE TABLE belongs_to SCHEMAFULL;
DEFINE FIELD in ON belongs_to TYPE record(employee);
DEFINE FIELD out ON belongs_to TYPE record(department);
```

### 5.2 NER 규칙 업데이트

#### 규칙 기반 NER

```rust
// 새 개체 유형 추가
pub enum EntityType {
    Employee,
    Department,
    LeaveType,
    // 새 도메인 개체 추가
    NewEntityType,
}

// 패턴 추가
lazy_static! {
    static ref PATTERNS: HashMap<EntityType, Vec<Regex>> = {
        let mut m = HashMap::new();
        m.insert(EntityType::NewEntityType, vec![
            Regex::new(r"패턴1").unwrap(),
            Regex::new(r"패턴2").unwrap(),
        ]);
        m
    };
}
```

### 5.3 RE 프롬프트 조정

#### LLM 프롬프트 템플릿

```
당신은 [도메인] 전문가입니다.

다음 텍스트에서 개체 간 관계를 추출하세요.

개체 유형:
- Entity1: 설명
- Entity2: 설명

관계 유형:
- relation1: Entity1 → Entity2 (설명)
- relation2: Entity2 → Entity1 (설명)

텍스트:
{input_text}

JSON 형식으로 출력:
{
  "relations": [
    {"subject": "...", "predicate": "...", "object": "...", "confidence": 0.9}
  ]
}
```

---

## 6. 기존 온톨로지 확장

### 6.1 확장 절차

1. **요구사항 분석**: 새로 필요한 클래스/관계 식별
2. **영향 분석**: 기존 구조에 미치는 영향 평가
3. **버전 관리**: 새 버전 브랜치 생성
4. **확장 구현**: OWL 파일 수정
5. **검증**: Protégé에서 일관성 검사
6. **마이그레이션**: 기존 데이터 마이그레이션 계획

### 6.2 확장 예시

```owl
<!-- 기존 HR 온톨로지에 교육 관련 클래스 추가 -->

<owl:Class rdf:about="#TrainingProgram">
    <rdfs:subClassOf rdf:resource="#Document"/>
    <rdfs:label>교육 프로그램</rdfs:label>
</owl:Class>

<owl:ObjectProperty rdf:about="#completedTraining">
    <rdfs:domain rdf:resource="#Employee"/>
    <rdfs:range rdf:resource="#TrainingProgram"/>
    <rdfs:label>completed training</rdfs:label>
</owl:ObjectProperty>

<owl:DatatypeProperty rdf:about="#completionDate">
    <rdfs:domain rdf:resource="#TrainingProgram"/>
    <rdfs:range rdf:resource="xsd:date"/>
</owl:DatatypeProperty>
```

### 6.3 버전 관리

```
ontologies/
├── hr/
│   ├── v1.0.0/
│   │   └── hr_ontology.owl
│   ├── v1.1.0/
│   │   ├── hr_ontology.owl
│   │   └── CHANGELOG.md
│   └── current -> v1.1.0/
```

#### CHANGELOG.md 예시

```markdown
# HR Ontology Changelog

## v1.1.0 (2024-XX-XX)

### Added
- TrainingProgram class
- completedTraining property
- completionDate property

### Changed
- Employee class: added training relations

### Fixed
- None

## v1.0.0 (2024-01-01)

- Initial release
```

---

## 7. 품질 체크리스트

### 7.1 설계 품질

- [ ] 모든 클래스에 rdfs:label과 rdfs:comment 있음
- [ ] 클래스 계층이 논리적임
- [ ] 속성 domain/range가 적절함
- [ ] 순환 참조 없음

### 7.2 완성도

- [ ] 도메인의 주요 개념 모두 포함
- [ ] FAQ 질문에 답변 가능한 구조
- [ ] 확장 가능한 설계

### 7.3 기술적 검증

- [ ] OWL 문법 오류 없음
- [ ] Reasoner 일관성 검사 통과
- [ ] 샘플 인스턴스 생성 가능
- [ ] SPARQL 쿼리 정상 동작

---

*Author: hephaex@gmail.com*
