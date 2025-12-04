# Sprint 3 - Final Report

**Date**: 2025-12-03
**Author**: hephaex@gmail.com

---

## 1. Overview

Sprint 3 implements the complete RAG (Retrieval-Augmented Generation) pipeline including hybrid search (vector + graph), LLM integration, streaming responses, and citation tracking.

## 2. Completed Tasks

| Task | Description | Status |
|------|-------------|--------|
| S3.1 | Vector Search (Qdrant similarity, Top-K) | Completed |
| S3.2 | Graph Search (SurrealDB subgraph extraction) | Completed |
| S3.3 | ACL Filtering (filter unauthorized documents) | Completed |
| S3.4 | Result Merging (RRF algorithm) | Completed |
| S3.5 | Prompt Engineering (system prompt, context format) | Completed |
| S3.6 | LLM Integration (OpenAI/Ollama abstraction) | Completed |
| S3.7 | Streaming Response (SSE-based real-time output) | Completed |
| S3.8 | Citation Tracking (metadata extraction) | Completed |
| S3.9 | E2E Integration Test (question → answer flow) | Completed |

## 3. Implementation Details

### 3.1 Vector Search Backend (`otl-vector/src/`)

**embedding.rs** - Embedding client abstraction:
- `EmbeddingClient` trait for generating embeddings
- `OpenAiEmbedding`: OpenAI API (text-embedding-3-small, text-embedding-3-large)
- `OllamaEmbedding`: Local Ollama API (nomic-embed-text, mxbai-embed-large)
- `create_embedding_client()`: Factory function from config

**qdrant_store.rs** - Vector search:
- `VectorSearchBackend`: Implements `SearchBackend` trait
- Automatic query embedding via EmbeddingClient
- `index_text()`: Embed and store text chunks
- `search()`: Query embedding → similarity search → SearchResult

### 3.2 Graph Search Backend (`otl-graph/src/search.rs`)

**GraphSearchBackend** - SurrealDB graph search:
- Keyword-based entity search
- Graph traversal for related entities
- Relationship extraction
- Context building from graph nodes/relations

### 3.3 LLM Clients (`otl-rag/src/llm.rs`)

**OpenAiClient**:
- Chat completions API
- Streaming support via SSE
- Configurable model, temperature, max_tokens

**OllamaClient**:
- Generate API for local LLM
- Streaming support
- Model selection

**Factory**: `create_llm_client(config)` → OpenAI/Ollama based on provider

### 3.4 RAG Orchestrator (`otl-rag/src/lib.rs`)

**HybridRagOrchestrator** (existing):
- Parallel vector + graph + keyword search
- ACL filtering via `DocumentAcl.can_access(user)`
- RRF (Reciprocal Rank Fusion) merging
- Prompt building with citations
- Citation extraction from LLM response

### 3.5 CLI Commands (`otl-cli/src/main.rs`)

```bash
otl query "질문" [--stream] [--ollama] [--model MODEL]
```

Options:
- `--stream`: Enable streaming output
- `--ollama`: Force Ollama (default: OpenAI if API key set)
- `--model`: Specify model name

## 4. Test Results

```
running 66 tests

otl-core:       9 passed
otl-extractor: 29 passed
otl-graph:      1 passed
otl-ocr:        4 passed
otl-parser:    14 passed
otl-rag:        6 passed
otl-vector:     3 passed

test result: ok. 66 passed; 0 failed
```

## 5. Files Created/Modified

### New Files
- `crates/otl-vector/src/embedding.rs` (270 lines)
- `crates/otl-graph/src/search.rs` (430 lines)
- `crates/otl-rag/src/llm.rs` (320 lines)

### Modified Files
- `crates/otl-vector/src/lib.rs` - Added embedding module exports
- `crates/otl-vector/src/qdrant_store.rs` - Added VectorSearchBackend, SearchBackend impl
- `crates/otl-vector/Cargo.toml` - Added reqwest, futures, tracing
- `crates/otl-graph/src/lib.rs` - Added search module exports
- `crates/otl-rag/src/lib.rs` - Added llm module exports
- `crates/otl-rag/Cargo.toml` - Added reqwest, tracing
- `crates/otl-cli/src/main.rs` - Added query command
- `crates/otl-cli/Cargo.toml` - Added otl-rag, futures

## 6. Architecture

```
Query Flow:
┌─────────────────────────────────────────────────────────────────┐
│  User Query                                                     │
└───────────────────┬─────────────────────────────────────────────┘
                    ▼
┌─────────────────────────────────────────────────────────────────┐
│  HybridRagOrchestrator.query()                                  │
│  ┌─────────────┬─────────────┬─────────────┐                   │
│  │ Vector      │ Graph       │ Keyword     │  (parallel)       │
│  │ Search      │ Search      │ Search      │                   │
│  └──────┬──────┴──────┬──────┴──────┬──────┘                   │
│         │             │             │                           │
│         └─────────────┼─────────────┘                           │
│                       ▼                                         │
│              ACL Filtering                                      │
│                       ▼                                         │
│              RRF Merging                                        │
│                       ▼                                         │
│              Prompt Building                                    │
│                       ▼                                         │
│              LLM Generation                                     │
│                       ▼                                         │
│              Citation Extraction                                │
└───────────────────────┬─────────────────────────────────────────┘
                        ▼
┌─────────────────────────────────────────────────────────────────┐
│  RagResponse { answer, citations, confidence, processing_time } │
└─────────────────────────────────────────────────────────────────┘
```

## 7. Usage Examples

```bash
# Query with OpenAI (requires OPENAI_API_KEY)
otl query "연차휴가 신청 절차가 어떻게 되나요?"

# Query with streaming output
otl query "병가 신청에 필요한 서류는?" --stream

# Query with Ollama
otl query "육아휴직 기간은?" --ollama --model llama2
```

## 8. Configuration

```toml
# config.toml
[llm]
provider = "openai"  # or "ollama"
model = "gpt-4o-mini"
embedding_model = "text-embedding-3-small"
max_tokens = 2048
temperature = 0.1

[database]
qdrant_url = "http://localhost:6334"
qdrant_collection = "otl_chunks"
vector_dimension = 1536
```

## 9. Next Steps (Sprint 4)

Sprint 4 focuses on API and deployment:
- REST API (Axum)
- Docker/Kubernetes deployment
- Performance testing
- Documentation

---

*Author: hephaex@gmail.com*
