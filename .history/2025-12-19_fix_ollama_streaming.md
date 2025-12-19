# Session Log: Fix Ollama Streaming Issue

**Date:** 2025-12-19
**Task:** Fix RAG query streaming endpoint not returning responses

## Problem Analysis

The streaming endpoint `/api/v1/query/stream` was returning empty responses, even though:
- Ollama was working correctly (verified with direct curl)
- Server logs showed "Found 5 relevant documents"

## Root Cause

The issue was in the `OllamaClient::generate_stream` method in `/Users/mare/Simon/OTL/crates/otl-rag/src/llm.rs`.

**Original problematic code:**
```rust
let mapped_stream = stream.filter_map(|result| async move {
    match result {
        Ok(bytes) => {
            let text = String::from_utf8_lossy(&bytes);
            // Ollama streams JSON objects, one per line
            if let Ok(parsed) = serde_json::from_str::<OllamaResponse>(&text) {
                if parsed.response.is_empty() {
                    None
                } else {
                    Some(Ok(parsed.response))
                }
            } else {
                None
            }
        }
        Err(e) => Some(Err(OtlError::LlmError(format!("Stream error: {e}")))),
    }
});
```

**The bug:** The code assumed each HTTP chunk from `bytes_stream()` contains exactly one complete JSON line. In reality:
1. Ollama streams newline-delimited JSON (NDJSON)
2. A single HTTP chunk can contain multiple JSON lines
3. A single JSON line can be split across multiple HTTP chunks

## Solution

Implemented a stateful stream using `scan()` to buffer incomplete lines across chunks:

```rust
let mapped_stream = stream.scan(String::new(), |buffer, result| {
    let output: Option<Result<String>> = match result {
        Ok(bytes) => {
            let text = String::from_utf8_lossy(&bytes);
            buffer.push_str(&text);

            // Process all complete lines in the buffer
            let mut collected = String::new();

            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer.drain(..=newline_pos).collect::<String>();
                let line = line.trim();

                if line.is_empty() {
                    continue;
                }

                if let Ok(parsed) = serde_json::from_str::<OllamaResponse>(line) {
                    if !parsed.response.is_empty() {
                        collected.push_str(&parsed.response);
                    }
                }
            }

            if collected.is_empty() {
                None
            } else {
                Some(Ok(collected))
            }
        }
        Err(e) => Some(Err(OtlError::LlmError(format!("Stream error: {e}")))),
    };
    async move { Some(output) }
})
.filter_map(|x| async move { x });
```

## File Changes

**Modified:**
- `/Users/mare/Simon/OTL/crates/otl-rag/src/llm.rs` - Fixed `OllamaClient::generate_stream` method

## Test Results

Successfully tested with:
```bash
curl -X POST http://localhost:8080/api/v1/query/stream \
  -H "Content-Type: application/json" \
  -d '{"question": "Hello, how are you?"}'
```

Output:
```
data: Hello
id: 0
event: message

data: !
id: 1
event: message

data:  I
id: 2
event: message
...
```

## Additional Notes

- Server needs `LLM_PROVIDER=ollama` and `LLM_MODEL=qwen2.5:7b` environment variables
- Vector search has a separate issue with embedding model (uses OpenAI model name by default)
- The streaming fix is generic and handles any NDJSON stream properly

## Future Enhancements

1. Add configurable embedding model for Ollama (e.g., `nomic-embed-text`)
2. Add retry logic for transient LLM failures
3. Consider adding backpressure handling for slow consumers
