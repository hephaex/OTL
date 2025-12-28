#!/bin/bash
# Create v0.2.0 roadmap issues for OTL
# Run: ./scripts/create-v0.2.0-issues.sh
# Requires: gh auth login

set -e

REPO="hephaex/OTL"

echo "Creating v0.2.0 roadmap issues..."

# Issue 1: Rate Limiting
gh issue create --repo "$REPO" \
  --title "feat: Complete rate limiting implementation" \
  --label "enhancement,security,priority:high" \
  --milestone "v0.2.0" \
  --body "$(cat <<'EOF'
## Summary
Re-enable and complete the rate limiting middleware that was disabled in routes.rs.

## Current State
- Rate limiting middleware exists in `crates/otl-api/src/middleware/rate_limit.rs`
- Currently disabled due to tower_governor compatibility issues
- TODOs exist for auth (5 req/min), streaming (10 req/min), and API endpoints (100 req/min)

## Tasks
- [ ] Update tower_governor to 0.8+ when compatible
- [ ] Enable rate limiting for auth endpoints (5 req/min)
- [ ] Enable rate limiting for streaming endpoints (10 req/min)
- [ ] Enable rate limiting for general API endpoints (100 req/min)
- [ ] Add rate limit headers to responses (X-RateLimit-*)
- [ ] Add configuration options for rate limits

## Files
- `crates/otl-api/src/routes.rs`
- `crates/otl-api/src/middleware/rate_limit.rs`

## Priority
High - Critical for production security
EOF
)"
echo "✅ Created: Rate limiting issue"

# Issue 2: Query Analysis
gh issue create --repo "$REPO" \
  --title "feat: Advanced query analysis & multi-intent support" \
  --label "enhancement,rag,priority:high" \
  --milestone "v0.2.0" \
  --body "$(cat <<'EOF'
## Summary
Enhance the query analysis system in otl-rag to support multi-intent queries and improve RAG results.

## Current State
- Simple rule-based keyword detection in `crates/otl-rag/src/lib.rs`
- Basic QueryAnalysis struct with intent detection

## Tasks
- [ ] Add LLM-based semantic query analysis
- [ ] Implement query decomposition for multi-intent queries
- [ ] Add intent confidence scoring
- [ ] Support compound queries (e.g., "Compare X AND show Y")
- [ ] Add query rewriting for better retrieval
- [ ] Implement query expansion with synonyms

## Example
```
Input: "Compare vacation policies AND show approval steps"
Output: [
  { intent: "compare", entities: ["vacation policies"], confidence: 0.95 },
  { intent: "explain", entities: ["approval steps"], confidence: 0.92 }
]
```

## Files
- `crates/otl-rag/src/lib.rs` (QueryAnalysis module)

## Priority
High - Improves RAG quality significantly
EOF
)"
echo "✅ Created: Query analysis issue"

# Issue 3: Full-Text Search
gh issue create --repo "$REPO" \
  --title "feat: Full-text search / keyword search backend" \
  --label "feature,rag,priority:medium" \
  --milestone "v0.2.0" \
  --body "$(cat <<'EOF'
## Summary
Implement actual keyword search backend for hybrid search (vector + keyword).

## Current State
- `keyword_store: Option` stub in RAG pipeline
- No actual keyword search implementation

## Options
1. PostgreSQL full-text search (pg_trgm, tsvector)
2. Meilisearch integration
3. Elasticsearch/OpenSearch integration

## Tasks
- [ ] Evaluate and choose keyword search backend
- [ ] Implement keyword indexing pipeline
- [ ] Add BM25 or TF-IDF scoring
- [ ] Implement hybrid ranking (RRF or weighted)
- [ ] Add exact phrase matching
- [ ] Support boolean operators (AND, OR, NOT)

## Benefits
- Better results for exact phrase matching
- Reduces LLM dependency for simple queries
- Improves retrieval for technical terms

## Files
- `crates/otl-rag/src/lib.rs`
- New: `crates/otl-rag/src/keyword.rs` or separate crate

## Priority
Medium - Significant RAG quality improvement
EOF
)"
echo "✅ Created: Full-text search issue"

# Issue 4: HITL Enhancement
gh issue create --repo "$REPO" \
  --title "feat: HITL verification workflow enhancement" \
  --label "feature,extraction,priority:medium" \
  --milestone "v0.2.0" \
  --body "$(cat <<'EOF'
## Summary
Complete the Human-in-the-Loop verification pipeline with batch processing and quality metrics.

## Current State
- Basic HITL structure in `crates/otl-extractor/src/hitl.rs`
- Simple verification handlers in API

## Tasks
- [ ] Implement batch approval workflow
- [ ] Add extraction confidence thresholds
- [ ] Auto-route high-confidence items vs uncertain ones
- [ ] Add quality metrics dashboard
- [ ] Implement feedback loops for model improvement
- [ ] Add ML retraining signals based on corrections
- [ ] Support bulk accept/reject operations

## Workflow
```
Extract → Confidence Score → Route
  ├─ High confidence (>0.9) → Auto-approve
  ├─ Medium (0.7-0.9) → Quick review queue
  └─ Low (<0.7) → Detailed review queue
```

## Files
- `crates/otl-extractor/src/hitl.rs`
- `crates/otl-api/src/handlers/verify.rs`

## Priority
Medium - Improves data quality and throughput
EOF
)"
echo "✅ Created: HITL enhancement issue"

# Issue 5: CLI Ingestion
gh issue create --repo "$REPO" \
  --title "feat: CLI ingestion pipeline completion" \
  --label "feature,cli,priority:medium" \
  --milestone "v0.2.0" \
  --body "$(cat <<'EOF'
## Summary
Complete the CLI document ingestion flow for standalone batch processing.

## Current State
- TODO at line 132 in `crates/otl-cli/src/main.rs`
- Basic CLI structure exists

## Tasks
- [ ] Implement directory watch mode
- [ ] Add document ingestion pipeline (parse → chunk → extract → embed → index)
- [ ] Add progress reporting with progress bars
- [ ] Implement error recovery and retry logic
- [ ] Add batch processing with configurable parallelism
- [ ] Support resume from checkpoint
- [ ] Add dry-run mode for validation

## CLI Interface
```bash
otl ingest ./documents --watch --parallel 4 --progress
otl ingest ./docs --batch-size 100 --resume
otl ingest ./file.pdf --dry-run
```

## Files
- `crates/otl-cli/src/main.rs`
- New: `crates/otl-cli/src/ingest.rs`

## Priority
Medium - Enables automation workflows
EOF
)"
echo "✅ Created: CLI ingestion issue"

# Issue 6: Ontology Management
gh issue create --repo "$REPO" \
  --title "feat: Ontology management & schema evolution" \
  --label "feature,graph,priority:medium" \
  --milestone "v0.2.0" \
  --body "$(cat <<'EOF'
## Summary
Implement full ontology lifecycle management with schema versioning and migration.

## Current State
- API stubs exist (GET/PUT /ontology)
- No actual implementation

## Tasks
- [ ] Implement ontology schema validation
- [ ] Add schema version control
- [ ] Create migration tools for schema changes
- [ ] Implement schema diffing
- [ ] Add backward compatibility checks
- [ ] Support entity/relation type updates without data loss
- [ ] Add ontology import/export (OWL, RDF)

## Schema Example
```yaml
version: "1.2.0"
entities:
  Person:
    properties:
      name: { type: string, required: true }
      email: { type: string, format: email }
relations:
  WORKS_AT:
    from: Person
    to: Organization
```

## Files
- `crates/otl-graph/src/`
- `crates/otl-api/src/handlers/graph.rs`

## Priority
Medium - Enables domain customization
EOF
)"
echo "✅ Created: Ontology management issue"

# Issue 7: Advanced Caching
gh issue create --repo "$REPO" \
  --title "feat: Performance optimizations & advanced caching" \
  --label "performance,priority:medium" \
  --milestone "v0.2.0" \
  --body "$(cat <<'EOF'
## Summary
Expand caching layer for better performance and distributed deployments.

## Current State
- Basic embedding/query cache in `crates/otl-rag/src/cache.rs`
- In-memory caching only

## Tasks
- [ ] Add result ranking cache
- [ ] Implement graph traversal cache
- [ ] Add embedding pre-computation for common queries
- [ ] Implement cache warming strategies
- [ ] Add Redis integration for distributed caching
- [ ] Implement cache invalidation policies
- [ ] Add cache hit/miss metrics

## Architecture
```
Request → Cache Check
  ├─ L1: In-memory (hot data)
  ├─ L2: Redis (distributed)
  └─ Miss: Compute → Cache → Return
```

## Files
- `crates/otl-rag/src/cache.rs`
- New: Redis integration module

## Priority
Medium - Reduces latency and costs
EOF
)"
echo "✅ Created: Advanced caching issue"

# Issue 8: Document Management
gh issue create --repo "$REPO" \
  --title "feat: Document management enhancements" \
  --label "feature,documents,priority:medium" \
  --milestone "v0.2.0" \
  --body "$(cat <<'EOF'
## Summary
Add advanced document management features for better governance and discoverability.

## Current State
- Basic upload/delete in `crates/otl-api/src/handlers/documents.rs`
- Simple metadata storage

## Tasks
- [ ] Implement document versioning
- [ ] Add bulk operations (batch upload/delete)
- [ ] Add metadata enrichment (auto-tagging, language detection)
- [ ] Implement document lineage tracking
- [ ] Add content diff visualization
- [ ] Support document collections/folders
- [ ] Add document search by metadata
- [ ] Implement soft delete with retention policy

## API Extensions
```
POST /documents/bulk - Batch upload
GET /documents/:id/versions - Version history
POST /documents/:id/tags - Auto-tag
GET /documents/:id/lineage - Processing history
```

## Files
- `crates/otl-api/src/handlers/documents.rs`
- `crates/otl-parser/src/lib.rs`

## Priority
Medium - Improves governance and UX
EOF
)"
echo "✅ Created: Document management issue"

echo ""
echo "🎉 All v0.2.0 roadmap issues created successfully!"
echo ""
echo "View issues: https://github.com/$REPO/issues"
echo "View milestone: https://github.com/$REPO/milestone/1"
