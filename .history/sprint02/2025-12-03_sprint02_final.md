# Sprint 2 - Final Report

**Date**: 2025-12-03
**Author**: hephaex@gmail.com

---

## 1. Overview

Sprint 2 implements the complete knowledge extraction pipeline including NER (Named Entity Recognition), RE (Relation Extraction), HITL (Human-in-the-Loop) verification, graph loading, and quality metrics.

## 2. Completed Tasks

| Task | Description | Status |
|------|-------------|--------|
| S2.1 | Rule-based NER (regex + dictionary) | Completed |
| S2.2 | LLM-based NER (prompt engineering) | Completed |
| S2.3 | Hybrid NER (rule + LLM merge) | Completed |
| S2.4 | Relation Extraction (triple generation) | Completed |
| S2.5 | HITL CLI (otl verify command) | Completed |
| S2.6 | Verification Workflow (state management) | Completed |
| S2.7 | Graph Loading (store approved triples) | Completed |
| S2.8 | Quality Metrics (extraction accuracy) | Completed |

## 3. Implementation Details

### 3.1 NER Module (`ner.rs`)

**EntityType enum** - 24 entity types for HR domain:
- Person types: Employee, Manager, HrStaff
- Organization: Department, Position, Grade
- Leave types: AnnualLeave, SickLeave, ParentalLeave, CongratulatoryLeave, LeaveType
- Process: ApprovalProcess, ApprovalStep
- Documents: Regulation, Form, Document
- Time: Duration, Date, Days
- Monetary: Amount, Expense

**RuleBasedNer**: Regex patterns + dictionary with aliases
- Korean patterns: `\d+일`, `제\d+조`, position names
- Dictionary: Leave types, departments, approval terms

**LlmNer**: Prompt-based extraction with JSON output
**HybridNer**: Weighted confidence merging

### 3.2 RE Module (`relation.rs`)

**RelationType enum** - 17 relation types:
- Employment: WorksIn, HasPosition, HasGrade, ManagedBy, ReportsTo
- Leave: RequestsLeave, ApprovesLeave, RequiresDuration, RequiresDocument
- Approval: HasStep, ApprovedBy, NextStep
- Document: DefinedIn, References, Requires
- Generic: HasValue, RelatedTo

**RuleBasedRe**: Pattern-based with keyword matching
**LlmRe**: Prompt-based with entity context
**HybridRe**: Combined approach

### 3.3 HITL Module (`hitl.rs`)

**VerificationStatus**: Pending, Approved, Rejected, AutoApproved
**VerificationQueue**: In-memory queue with auto-approve threshold
**VerificationStats**: Approval rates and counts

### 3.4 CLI Commands

```bash
otl extract "text..."           # Extract entities and relations
otl verify list                 # List pending extractions
otl verify show <id>            # Show item details
otl verify approve <id>         # Approve extraction
otl verify reject <id>          # Reject extraction
otl verify stats                # Show statistics
otl verify demo                 # Load demo data
```

### 3.5 Graph Loader (`loader.rs`)

**GraphLoader**: Converts ExtractedEntity/Relation to core Entity/Triple
**LoadResult**: Summary of loading operations
**Entity deduplication**: Text-based mapping

### 3.6 Quality Metrics (`metrics.rs`)

**EntityMetrics**: Precision, Recall, F1, Accuracy
**RelationMetrics**: Same metrics for relations
**Evaluator**: Gold standard comparison (strict/relaxed)
**AggregateMetrics**: Batch evaluation with report generation

Sprint 2 criteria check:
- NER Precision >= 80%
- RE Precision >= 70%

## 4. Test Results

```
running 60 tests

otl-core:      9 passed
otl-extractor: 29 passed
otl-ocr:       4 passed
otl-parser:   14 passed
otl-rag:       4 passed

test result: ok. 60 passed; 0 failed
```

## 5. Files Created/Modified

### New Files
- `crates/otl-extractor/src/ner.rs` (659 lines)
- `crates/otl-extractor/src/relation.rs` (660 lines)
- `crates/otl-extractor/src/hitl.rs` (330 lines)
- `crates/otl-extractor/src/loader.rs` (200 lines)
- `crates/otl-extractor/src/metrics.rs` (370 lines)
- `crates/otl-extractor/src/prompts/ner_system.txt`
- `crates/otl-extractor/src/prompts/re_system.txt`

### Modified Files
- `crates/otl-extractor/src/lib.rs`
- `crates/otl-extractor/Cargo.toml`
- `crates/otl-cli/src/main.rs`
- `crates/otl-cli/Cargo.toml`

## 6. Demo Output

```
$ cargo run -p otl-cli -- extract "연차휴가는 최대 15일까지 사용할 수 있습니다."

=== Entities ===

  [0.95] AnnualLeave: "연차" @ 0..6
  [0.90] Days: "15일" @ 23..28

=== Relations ===

  [0.85] (연차) --[requiresDuration]--> (15일)
```

## 7. Architecture

```
otl-extractor/
├── lib.rs              # ExtractedEntity, ExtractedRelation, traits
├── ner.rs              # NER implementations
├── relation.rs         # RE implementations
├── hitl.rs             # HITL verification
├── loader.rs           # Graph loading
├── metrics.rs          # Quality evaluation
└── prompts/
    ├── ner_system.txt  # NER prompt
    └── re_system.txt   # RE prompt
```

## 8. Next Steps (Sprint 3)

Sprint 3 focuses on RAG pipeline:
- Hybrid search (vector + graph + keyword)
- LLM integration for answer generation
- Prompt optimization
- QA evaluation

---

*Author: hephaex@gmail.com*
