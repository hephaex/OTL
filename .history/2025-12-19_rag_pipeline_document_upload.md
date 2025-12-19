# Session Log: RAG Pipeline Document Upload Implementation

**Date:** 2025-12-19
**Author:** hephaex@gmail.com

## Session Overview

This session focused on fixing SurrealDB connection issues and implementing a complete document upload pipeline with RAG (Retrieval-Augmented Generation) integration.

## Objectives

1. Fix SurrealDB WebSocket connection timeout
2. Implement document upload with vector indexing
3. Integrate vector search into RAG query streaming
4. Test end-to-end RAG pipeline

## Problem Analysis

### 1. SurrealDB Connection Timeout
**Root Cause:** The SurrealDB Rust crate automatically adds `ws://` prefix to URLs, but our configuration already included it, causing duplicate prefixes.

**Solution:** Strip `ws://` or `wss://` prefix before connecting:
```rust
let url = config.surrealdb_url
    .strip_prefix("ws://")
    .or_else(|| config.surrealdb_url.strip_prefix("wss://"))
    .unwrap_or(&config.surrealdb_url);
```

### 2. UTF-8 Character Boundary Panic
**Root Cause:** Text chunking used byte indices directly, which can land in the middle of multi-byte UTF-8 characters (e.g., Korean).

**Solution:** Added `find_char_boundary()` helper function:
```rust
fn find_char_boundary(text: &str, pos: usize) -> usize {
    if pos >= text.len() {
        return text.len();
    }
    let mut boundary = pos;
    while boundary > 0 && !text.is_char_boundary(boundary) {
        boundary -= 1;
    }
    boundary
}
```

### 3. RAG Query Not Using Vector Search
**Root Cause:** The streaming query handler sent questions directly to LLM without searching for relevant documents first.

**Solution:** Added vector search before LLM generation in `query_stream_handler()`.

## Solutions Implemented

### Document Upload Pipeline (`documents.rs`)
- Base64 content decoding
- UTF-8 text extraction
- Text chunking (1000 chars, 200 overlap)
- Embedding generation via Ollama
- Vector storage in Qdrant

### RAG Query Integration (`query.rs`)
- Vector search for relevant documents
- Context building from search results
- LLM prompt with retrieved context
- Streaming response generation

## File Changes Summary

### Modified Files
| File | Changes |
|------|---------|
| `crates/otl-graph/src/search.rs` | Strip ws:// prefix from SurrealDB URL |
| `crates/otl-graph/src/surrealdb_store.rs` | Strip ws:// prefix from SurrealDB URL |
| `crates/otl-api/src/handlers/documents.rs` | Full document upload pipeline with UTF-8 handling |
| `crates/otl-api/src/handlers/query.rs` | Vector search integration in streaming handler |
| `crates/otl-api/src/state.rs` | Added vector_backend field |
| `crates/otl-api/src/main.rs` | RAG pipeline initialization |
| `crates/otl-core/src/config.rs` | EMBEDDING_MODEL env var loading |
| `crates/otl-vector/src/qdrant_store.rs` | Dynamic dimension from embedding client |
| `docker-compose.yml` | Qdrant v1.16.0, SurrealDB v2.4.0 |

## Git Commits

```
2bd06e1 feat: implement document upload pipeline with RAG integration
883f749 fix: resolve SurrealDB connection timeout and integrate RAG pipeline
```

## Test Results

### Document Upload Test
```json
{
  "id": "658244e0-4d60-4035-bea1-6eb9f4b965da",
  "message": "Document uploaded and processed: 2/2 chunks indexed",
  "chunk_count": 2
}
```

### Vector Storage Verification
- Collection: `otl_chunks`
- Points stored: 6
- Documents indexed: OTL 시스템 기술문서, 머신러닝 기초 가이드

### RAG Query Test
- Query: "OTL 시스템의 주요 구성요소는 무엇인가요?"
- Log: `Found 5 relevant documents`
- Response: LLM correctly referenced uploaded document content (Rust, Axum, Qdrant, SurrealDB)

## Technical Architecture

```
┌─────────────────┐     ┌─────────────────┐
│  Document       │     │  Query          │
│  Upload API     │     │  Stream API     │
└────────┬────────┘     └────────┬────────┘
         │                       │
         ▼                       ▼
┌─────────────────┐     ┌─────────────────┐
│  Base64 Decode  │     │  Vector Search  │
│  + Chunking     │     │  (Qdrant)       │
└────────┬────────┘     └────────┬────────┘
         │                       │
         ▼                       ▼
┌─────────────────┐     ┌─────────────────┐
│  Embedding      │     │  Context Build  │
│  (nomic-embed)  │     │  + LLM Prompt   │
└────────┬────────┘     └────────┬────────┘
         │                       │
         ▼                       ▼
┌─────────────────┐     ┌─────────────────┐
│  Vector Store   │     │  LLM Stream     │
│  (Qdrant)       │     │  (qwen2.5:7b)   │
└─────────────────┘     └─────────────────┘
```

## Configuration

### Environment Variables
```bash
LLM_PROVIDER=ollama
OLLAMA_URL=http://localhost:11434
LLM_MODEL=qwen2.5:7b
EMBEDDING_MODEL=nomic-embed-text
SURREALDB_URL=ws://localhost:8000
SURREALDB_USER=root
SURREALDB_PASS=root
QDRANT_URL=http://localhost:6334
```

### Chunk Configuration
- Chunk size: 1000 characters
- Overlap: 200 characters
- Min chunk size: 100 characters
- Respect paragraphs: true
- Embedding dimension: 768

## Future Enhancements

1. **Document Metadata Storage**: Store document metadata in PostgreSQL for listing/filtering
2. **Hybrid Search**: Combine vector search with graph traversal
3. **Citation Extraction**: Return source citations with RAG responses
4. **Batch Upload**: Support multiple document upload
5. **Document Versioning**: Track document updates and re-indexing

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/documents` | POST | Upload and index document |
| `/api/v1/documents` | GET | List documents |
| `/api/v1/query/stream` | POST | RAG query with streaming response |
| `/health` | GET | Health check |
| `/swagger-ui/` | GET | API documentation |

## Running the System

```bash
# Start infrastructure
docker compose up -d

# Run API server
LLM_PROVIDER=ollama OLLAMA_URL=http://localhost:11434 \
LLM_MODEL=qwen2.5:7b EMBEDDING_MODEL=nomic-embed-text \
SURREALDB_URL=ws://localhost:8000 SURREALDB_USER=root SURREALDB_PASS=root \
QDRANT_URL=http://localhost:6334 \
./target/release/otl-api
```

## Lessons Learned

1. SurrealDB Rust client auto-adds WebSocket protocol prefix
2. UTF-8 string slicing requires character boundary checking
3. RAG effectiveness depends on proper context retrieval before LLM generation
4. Qdrant client/server version compatibility is important (within 1 minor version)
