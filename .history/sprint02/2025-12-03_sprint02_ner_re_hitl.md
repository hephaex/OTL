# Sprint 2 - NER/RE/HITL Implementation

**Date**: 2025-12-03
**Author**: hephaex@gmail.com

---

## 1. Overview

Sprint 2 focuses on building the knowledge extraction pipeline with NER (Named Entity Recognition), RE (Relation Extraction), and HITL (Human-in-the-Loop) verification.

## 2. Completed Tasks

### 2.1 S2.1 - Rule-based NER (Regex + Dictionary)

**File**: `crates/otl-extractor/src/ner.rs`

Implemented comprehensive NER with:
- **EntityType enum**: 24 entity types for HR domain
  - Person types: Employee, Manager, HrStaff
  - Organization: Department, Position, Grade
  - Leave types: AnnualLeave, SickLeave, ParentalLeave, etc.
  - Process: ApprovalProcess, ApprovalStep
  - Documents: Regulation, Form, Document
  - Time/Duration: Duration, Date, Days
  - Monetary: Amount, Expense

- **Regex patterns**:
  - Korean: `\d+일`, `\d+개월`, `\d+년`, `제\d+조`, etc.
  - English: `\d+\s*days?`, `\d+\s*months?`, etc.
  - Date patterns: `\d{4}[-/]\d{1,2}[-/]\d{1,2}`
  - Amount patterns: `\d{1,3}(,\d{3})*원`, `\d+만원`

- **Dictionary with aliases**:
  - Leave types: 연차/연차휴가/연가, 병가/병가휴가, etc.
  - Approval terms: 승인/결재/허가
  - Departments: 인사팀/인사부/HR팀

### 2.2 S2.2 - LLM-based NER (Prompt Engineering)

**Files**:
- `crates/otl-extractor/src/ner.rs` (LlmNer struct)
- `crates/otl-extractor/src/prompts/ner_system.txt`

Implemented LLM NER with:
- Configurable system prompt
- Entity type list specification
- JSON output format
- Response parsing with entity location finding

### 2.3 S2.3 - Hybrid NER (Rule + LLM Merge)

**File**: `crates/otl-extractor/src/ner.rs` (HybridNer struct)

Features:
- Configurable rule weight (default 0.6)
- Confidence threshold filtering
- Entity merging with confidence boosting
- Deduplication for overlapping entities

### 2.4 S2.4 - Relation Extraction (Triple Generation)

**Files**:
- `crates/otl-extractor/src/relation.rs`
- `crates/otl-extractor/src/prompts/re_system.txt`

Implemented:
- **RelationType enum**: 17 relation types
  - Employment: WorksIn, HasPosition, HasGrade, ManagedBy, ReportsTo
  - Leave: RequestsLeave, ApprovesLeave, RequiresDuration, RequiresDocument
  - Approval: HasStep, ApprovedBy, NextStep
  - Document: DefinedIn, References, Requires
  - Generic: HasValue, RelatedTo

- **Rule-based RE**:
  - Pattern matching with keywords
  - Max distance constraint
  - UTF-8 safe context extraction

- **LLM RE** and **Hybrid RE** with merging

### 2.5 S2.5 - HITL CLI (otl verify)

**Files**:
- `crates/otl-extractor/src/hitl.rs`
- `crates/otl-cli/src/main.rs`

CLI Commands:
```bash
otl verify list [-t entity|relation] [-l 10]
otl verify show <id>
otl verify approve <id> [-n "note"]
otl verify reject <id> [-r "reason"]
otl verify stats
otl verify demo
otl extract "text..."
```

### 2.6 S2.6 - Verification Workflow (State Management)

**File**: `crates/otl-extractor/src/hitl.rs`

Implemented:
- **VerificationStatus**: Pending, Approved, Rejected, AutoApproved
- **PendingEntity/PendingRelation**: With reviewer, review_note, timestamps
- **VerificationQueue**:
  - Auto-approve threshold (default 0.95)
  - Approval/rejection with reviewer tracking
  - Statistics calculation (approval rates)

## 3. Test Results

```
running 48 tests
otl-core:      9 passed
otl-extractor: 17 passed
otl-ocr:       4 passed
otl-parser:   14 passed
otl-rag:       4 passed

test result: ok. 48 passed; 0 failed
```

## 4. Files Changed

### New Files
- `crates/otl-extractor/src/ner.rs` (659 lines)
- `crates/otl-extractor/src/relation.rs` (660 lines)
- `crates/otl-extractor/src/hitl.rs` (330 lines)
- `crates/otl-extractor/src/prompts/ner_system.txt`
- `crates/otl-extractor/src/prompts/re_system.txt`

### Modified Files
- `crates/otl-extractor/src/lib.rs` - Added Serialize/Deserialize, new modules
- `crates/otl-extractor/Cargo.toml` - Added chrono dependency
- `crates/otl-cli/src/main.rs` - Full CLI implementation
- `crates/otl-cli/Cargo.toml` - Added extractor, once_cell, uuid deps

## 5. Demo Output

```
$ otl verify demo
Extracting from sample HR text...

Text: 연차휴가는 최대 15일까지 사용할 수 있습니다. 병가 신청에는 진단서가 필요합니다.
육아휴직은 최대 2년간 사용 가능합니다. 팀장 승인 후 인사팀에서 최종 결재합니다.

Added entity: [3bb047f4] AnnualLeave "연차" (conf: 0.95)
Added entity: [506470b5] Days "15일" (conf: 0.90)
Added entity: [44ce7492] SickLeave "병가" (conf: 0.95)
Added entity: [7cbc2363] Document "진단서" (conf: 0.95)
Added entity: [5ad07851] ParentalLeave "육아휴직" (conf: 0.95)
Added entity: [8ca5fd81] Duration "2년" (conf: 0.90)
Added entity: [e4c2141a] Manager "팀장" (conf: 0.95)
Added entity: [32c04e03] ApprovalProcess "승인" (conf: 0.95)
Added entity: [11ab0e11] Department "인사팀" (conf: 0.95)
Added entity: [241bcd3c] ApprovalProcess "결재" (conf: 0.90)
Added relation: [e1be21a1] (연차) --[requiresDuration]--> (15일) (conf: 0.85)
Added relation: [ffaa1b58] (병가) --[requiresDuration]--> (15일) (conf: 0.85)
Added relation: [d88cb6fa] (육아휴직) --[requiresDuration]--> (2년) (conf: 0.85)
Added relation: [513197cc] (병가) --[requiresDocument]--> (진단서) (conf: 0.80)

Demo loaded: 10 entities (3 pending), 4 relations (4 pending)
```

## 6. Remaining Tasks

- S2.7: Graph Loader (store approved triples to SurrealDB)
- S2.8: Quality Metrics (extraction accuracy measurement)

## 7. Technical Notes

### UTF-8 Handling
Korean text requires careful handling of byte vs character boundaries. Fixed by:
- Using `.is_char_boundary()` checks
- Adjusting context window to valid boundaries
- Using `.len()` on strings for byte positions

### Entity Deduplication
Overlapping entities are handled by:
1. Sorting by start position, then confidence (descending)
2. Tracking covered byte positions
3. Skipping entities that overlap with higher-confidence ones

---

*Author: hephaex@gmail.com*
