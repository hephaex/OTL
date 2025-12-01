# OTL Project Definition

## Overview

OTL (Ontology-based knowledge sysTem Library) is a high-performance knowledge management system that combines ontology-based knowledge graphs with Retrieval-Augmented Generation (RAG).

## Author

- **Author**: hep@gmail.com
- **License**: Apache-2.0

## Architecture

### Components

1. **otl-core**: Domain models, ACL, error types
2. **otl-parser**: Document parsing (PDF, DOCX, XLSX, etc.)
3. **otl-ocr**: OCR integration
4. **otl-graph**: Graph database abstraction (SurrealDB)
5. **otl-vector**: Vector database abstraction (Qdrant)
6. **otl-extractor**: NER and relation extraction
7. **otl-rag**: RAG orchestrator with hybrid search
8. **otl-api**: REST API server
9. **otl-cli**: Command-line interface

### Technology Stack

- **Language**: Rust
- **Graph DB**: SurrealDB
- **Vector DB**: Qdrant
- **Relational DB**: PostgreSQL
- **LLM**: OpenAI API / Ollama

## Key Features

- Hybrid search (vector + graph + keyword)
- Document-level ACL
- Human-in-the-loop verification
- Multi-format document support
- Citation tracking

## Development

See [SPRINT_PLAN.md](SPRINT_PLAN.md) for implementation roadmap.

---

© 2024 hep@gmail.com. All rights reserved.
