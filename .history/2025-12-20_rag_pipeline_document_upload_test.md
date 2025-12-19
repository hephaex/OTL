# Session Log: RAG Pipeline Document Upload Test

**Date**: 2025-12-20
**Author**: hephaex@gmail.com

## Session Overview

RAG 파이프라인의 문서 업로드 기능을 테스트하고 발견된 문제들을 수정한 세션입니다.

## Objectives

1. RAG 파이프라인 문서 업로드 테스트
2. 발견된 버그 수정
3. 스트리밍 응답 개선

## Problems Identified and Solutions

### 1. SurrealDB 연결 타임아웃 (이전 세션에서 해결)

**문제**: SurrealDB WebSocket 연결 실패
**원인**: surrealdb crate가 자동으로 `ws://` prefix를 추가하는데, 설정에서도 `ws://localhost:8000`으로 지정되어 중복 발생
**해결**: URL prefix 제거 로직 추가

```rust
let url = config.surrealdb_url
    .strip_prefix("ws://")
    .or_else(|| config.surrealdb_url.strip_prefix("wss://"))
    .unwrap_or(&config.surrealdb_url);
```

**수정 파일**:
- `crates/otl-graph/src/search.rs`
- `crates/otl-graph/src/surrealdb_store.rs`

### 2. UTF-8 문자 경계 오류

**문제**: 한글 문서 청킹 시 panic 발생
```
thread 'tokio-runtime-worker' panicked at crates/otl-api/src/handlers/documents.rs:388:28:
byte index 900 is not a char boundary; it is inside '서' (bytes 899..902)
```

**원인**: 바이트 인덱스를 문자 경계로 잘못 사용
**해결**: `find_char_boundary()` 헬퍼 함수 추가

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

**수정 파일**: `crates/otl-api/src/handlers/documents.rs`

### 3. Ollama 스트리밍 파싱 오류

**문제**: LLM 스트리밍 응답이 전달되지 않음
**원인**: Ollama NDJSON 스트리밍에서 청크가 불완전하게 도착할 수 있는데, 기존 코드는 각 HTTP 청크가 완전한 JSON이라고 가정

**해결**: 상태 기반 버퍼링 스트림 프로세서 구현
- `scan()`을 사용하여 불완전한 라인을 버퍼링
- 완전한 라인(`\n`으로 끝나는)만 파싱

**수정 파일**: `crates/otl-rag/src/llm.rs`

### 4. SSE 진정한 스트리밍 미구현

**문제**: 스트리밍 엔드포인트가 전체 응답을 수집한 후에 전송

**기존 코드**:
```rust
// 모든 청크를 먼저 수집
let mut collected = Vec::new();
while let Some(result) = llm_stream.next().await {
    collected.push(chunk);
}
// 그 후에 SSE 스트림 생성
let stream = stream::iter(collected...);
```

**해결**: LLM 스트림을 직접 SSE 이벤트로 변환

```rust
let event_counter = Arc::new(AtomicUsize::new(0));
let sse_stream = llm_stream.map(move |result| {
    let id = event_counter.fetch_add(1, Ordering::SeqCst);
    match result {
        Ok(chunk) => Ok(Event::default().data(chunk).id(id.to_string()).event("message")),
        Err(_) => Ok(Event::default().data("[오류]").id(id.to_string()).event("error")),
    }
});
```

**수정 파일**: `crates/otl-api/src/handlers/query.rs`

## File Changes Summary

### Modified Files

| File | Changes |
|------|---------|
| `crates/otl-api/src/handlers/documents.rs` | UTF-8 문자 경계 처리, 문서 업로드 파이프라인 |
| `crates/otl-api/src/handlers/query.rs` | 진정한 SSE 스트리밍 구현 (+71, -50) |
| `crates/otl-rag/src/llm.rs` | Ollama NDJSON 스트리밍 파싱 수정 |
| `crates/otl-graph/src/search.rs` | SurrealDB URL prefix 처리 |
| `crates/otl-graph/src/surrealdb_store.rs` | SurrealDB URL prefix 처리 |

## Git Commits

```
6f1b21b fix: implement true SSE streaming for RAG query responses
69f710b feat: add PDF and DOCX document upload support
2bd06e1 feat: implement document upload pipeline with RAG integration
883f749 fix: resolve SurrealDB connection timeout and integrate RAG pipeline
```

## Test Results

### Document Upload Test
```bash
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -d '{"title": "OTL_기술문서.txt", "content": "<base64>", "file_type": "txt"}'

# Response:
{"id":"7ba5933b-...", "message":"Document uploaded and processed: 2/2 chunks indexed", "chunk_count":2}
```

### RAG Streaming Query Test
```bash
curl -N -X POST http://localhost:8080/api/v1/query/stream \
  -H "Content-Type: application/json" \
  -d '{"question":"OTL 시스템의 아키텍처 구성요소는?"}'

# Response (SSE):
data: OTL
id: 0
event: message

data: 시스템의
id: 1
event: message
...
```

### Qdrant Vector Store Status
- Collection: `otl_chunks`
- Points: 9개
- Dimension: 768 (nomic-embed-text)

## Architecture Notes

### RAG Pipeline Flow
```
1. Document Upload → Base64 Decode → UTF-8 Text
2. Text Chunking (1000 chars, 200 overlap) → UTF-8 safe boundaries
3. Embedding Generation (nomic-embed-text, 768 dim)
4. Vector Storage (Qdrant)
5. Query → Vector Search → Context Retrieval
6. LLM Generation (Ollama qwen2.5:7b) → SSE Streaming
```

### Current System Configuration
```
LLM_PROVIDER=ollama
OLLAMA_URL=http://localhost:11434
LLM_MODEL=qwen2.5:7b
EMBEDDING_MODEL=nomic-embed-text
SURREALDB_URL=ws://localhost:8000
QDRANT_URL=http://localhost:6334
```

## Future Enhancements

1. **문서 메타데이터 저장**: PostgreSQL에 문서 메타데이터 영구 저장
2. **청크 캐싱**: 중복 임베딩 생성 방지
3. **배치 임베딩**: 여러 청크를 한 번에 임베딩
4. **문서 삭제 시 벡터 정리**: Qdrant에서 해당 문서 벡터 삭제
5. **검색 결과 재순위화**: Cross-encoder를 사용한 재순위화

## Session Summary

- RAG 파이프라인 문서 업로드 기능 완전 구현
- UTF-8 한글 문서 처리 문제 해결
- 실시간 스트리밍 응답 구현
- 전체 파이프라인 테스트 성공
