# DOCX 문서 업로드 지원 구현 (2025-12-20)

## 세션 개요
PDF와 DOCX 문서 업로드 지원을 OTL API에 추가하고 테스트 완료

## 구현 내용

### 1. 파일 타입 감지 및 파서 라우팅
`crates/otl-api/src/handlers/documents.rs`에서 file_type에 따라 적절한 파서 사용:

```rust
let text_content = match req.file_type.to_lowercase().as_str() {
    "pdf" => {
        extract_text_from_pdf(&decoded_bytes).map_err(|e| {
            AppError::BadRequest(format!("Failed to extract text from PDF: {e}"))
        })?
    }
    "docx" => {
        extract_text_from_docx(&decoded_bytes).map_err(|e| {
            AppError::BadRequest(format!("Failed to extract text from DOCX: {e}"))
        })?
    }
    _ => {
        String::from_utf8(decoded_bytes)
            .map_err(|e| AppError::BadRequest(format!("Content is not valid UTF-8: {e}")))?
    }
};
```

### 2. PDF 텍스트 추출 함수
```rust
fn extract_text_from_pdf(bytes: &[u8]) -> Result<String, String> {
    pdf_extract::extract_text_from_mem(bytes).map_err(|e| e.to_string())
}
```

### 3. DOCX 텍스트 추출 함수
```rust
fn extract_text_from_docx(bytes: &[u8]) -> Result<String, String> {
    let docx = docx_rs::read_docx(bytes).map_err(|e| format!("Failed to parse DOCX: {e}"))?;
    let mut text = String::new();
    for child in docx.document.children {
        if let docx_rs::DocumentChild::Paragraph(para) = child {
            for child in para.children {
                if let docx_rs::ParagraphChild::Run(run) = child {
                    for child in run.children {
                        if let docx_rs::RunChild::Text(t) = child {
                            text.push_str(&t.text);
                        }
                    }
                }
            }
            text.push('\n');
        }
    }
    if text.is_empty() {
        return Err("No text content found in DOCX".to_string());
    }
    Ok(text)
}
```

### 4. 의존성 추가 (Cargo.toml)
```toml
pdf-extract = { workspace = true }
docx-rs = { workspace = true }
```

## 테스트 결과

### PDF 업로드 테스트
- matplotlib PDF 아이콘 파일 업로드 성공
- 텍스트 추출 및 벡터 인덱싱 완료

### DOCX 업로드 테스트
1. **테스트 파일 생성**: python-docx로 유효한 DOCX 파일 생성
   ```python
   from docx import Document
   d = Document()
   d.add_paragraph('OTL 시스템 기술 문서')
   d.add_paragraph('이 문서는 OTL RAG 시스템을 위한 테스트 DOCX 파일입니다.')
   d.add_paragraph('주요 기능: 문서 업로드, 벡터 검색, 그래프 검색, LLM 질의응답')
   d.save('/tmp/test_document.docx')
   ```

2. **업로드 결과**:
   ```json
   {
     "id": "f9a3eb4f-2907-48bd-bb22-864358023903",
     "message": "Document uploaded and processed: 1/1 chunks indexed",
     "chunk_count": 1
   }
   ```

3. **서버 로그**:
   ```
   Processing document upload: OTL 기술문서 (DOCX) (id: f9a3eb4f-..., type: docx, size: 185 bytes)
   Document f9a3eb4f-... split into 1 chunks
   Successfully indexed 1/1 chunks for document f9a3eb4f-...
   ```

## 해결한 문제들

### 1. docx-rs API 불일치
- 오류: `expected &[u8], found Cursor<&[u8]>`
- 해결: `Cursor` 대신 바이트 슬라이스 직접 전달

### 2. 수동 생성 DOCX 파싱 실패
- 오류: "Failed to read from zip."
- 원인: 수동으로 만든 ZIP 구조가 불완전
- 해결: python-docx로 표준 DOCX 파일 생성

## 파일 변경 요약

| 파일 | 변경 내용 |
|------|----------|
| `crates/otl-api/src/handlers/documents.rs` | PDF/DOCX 파서 추가 |
| `crates/otl-api/Cargo.toml` | pdf-extract, docx-rs 의존성 추가 |

## Git 커밋
- `69f710b`: feat: add PDF and DOCX document upload support

## 향후 개선 사항
1. DOCX 스타일/서식 정보 추출 (헤더, 테이블 등)
2. 대용량 DOCX 파일 스트리밍 처리
3. 이미지 포함 DOCX 처리 (OCR 연동)
4. RAG 쿼리 타임아웃 문제 조사 필요
