//! RAG query handlers
//!
//! Author: hephaex@gmail.com

use crate::error::AppError;
use crate::state::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    response::{
        sse::{Event, Sse},
        IntoResponse,
    },
    Json,
};
use futures::stream::{self, Stream, StreamExt};
use otl_core::RagQuery;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use utoipa::ToSchema;

/// Query request body
#[derive(Debug, Deserialize, ToSchema)]
pub struct QueryRequest {
    /// User's question
    #[schema(example = "연차휴가 신청 절차가 어떻게 되나요?")]
    pub question: String,

    /// Maximum number of results to retrieve
    #[serde(default = "default_top_k")]
    #[schema(example = 5, default = 5)]
    pub top_k: usize,

    /// Include citations in response
    #[serde(default = "default_true")]
    #[schema(default = true)]
    pub include_citations: bool,

    /// User ID for ACL filtering
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

fn default_top_k() -> usize {
    5
}

fn default_true() -> bool {
    true
}

/// Citation information
#[derive(Debug, Serialize, ToSchema)]
pub struct Citation {
    /// Source document title
    #[schema(example = "인사규정_2024.pdf")]
    pub source: String,

    /// Page number if applicable
    #[schema(example = 15)]
    pub page: Option<u32>,

    /// Section title
    #[schema(example = "제3장 휴가")]
    pub section: Option<String>,

    /// Relevance score
    #[schema(example = 0.92)]
    pub relevance: f32,
}

/// Query response body
#[derive(Debug, Serialize, ToSchema)]
pub struct QueryResponse {
    /// Generated answer
    #[schema(example = "연차휴가 신청은 다음 절차를 따릅니다...")]
    pub answer: String,

    /// Source citations
    pub citations: Vec<Citation>,

    /// Confidence score
    #[schema(example = 0.87)]
    pub confidence: f32,

    /// Processing time in milliseconds
    #[schema(example = 1250)]
    pub processing_time_ms: u64,
}

/// Handle RAG query requests
#[utoipa::path(
    post,
    path = "/api/v1/query",
    tag = "query",
    request_body = QueryRequest,
    responses(
        (status = 200, description = "Query successful", body = QueryResponse),
        (status = 400, description = "Invalid request", body = crate::error::ApiError),
        (status = 500, description = "Internal error", body = crate::error::ApiError)
    )
)]
pub async fn query_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<QueryRequest>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    let start = std::time::Instant::now();

    // Validate request
    if req.question.trim().is_empty() {
        return Err(AppError::BadRequest("Question cannot be empty".to_string()));
    }

    // Try to use actual RAG orchestrator if available
    if let Some(rag) = state.get_rag().await {
        let user = state.get_default_user(req.user_id.as_deref());
        let rag_query = RagQuery::new(&req.question).with_top_k(req.top_k);

        match rag.query(&rag_query, &user).await {
            Ok(rag_response) => {
                let response = QueryResponse {
                    answer: rag_response.answer,
                    citations: rag_response
                        .citations
                        .into_iter()
                        .map(|c| Citation {
                            source: c.document_title,
                            page: c.source.page,
                            section: c.source.section,
                            relevance: c.source.confidence,
                        })
                        .collect(),
                    confidence: rag_response.confidence,
                    processing_time_ms: rag_response.processing_time_ms,
                };
                return Ok((StatusCode::OK, Json(response)));
            }
            Err(e) => {
                tracing::error!("RAG query failed: {}", e);
                return Err(AppError::Internal(format!("RAG query failed: {e}")));
            }
        }
    }

    // Fallback to mock response when RAG is not initialized
    tracing::warn!("RAG not initialized, returning mock response");
    let response = QueryResponse {
        answer: format!(
            "귀하의 질문 \"{}\"에 대한 답변입니다.\n\n\
             연차휴가 신청은 사내 인사시스템을 통해 진행됩니다. \
             팀장 승인 후 인사팀에서 최종 처리됩니다.\n\n\
             [출처: 인사규정 제15조]\n\n\
             (주의: RAG 시스템이 초기화되지 않아 Mock 응답입니다)",
            req.question
        ),
        citations: vec![
            Citation {
                source: "인사규정_2024.pdf".to_string(),
                page: Some(15),
                section: Some("제3장 휴가".to_string()),
                relevance: 0.92,
            },
            Citation {
                source: "휴가신청_매뉴얼.docx".to_string(),
                page: Some(3),
                section: Some("신청 절차".to_string()),
                relevance: 0.85,
            },
        ],
        confidence: 0.87,
        processing_time_ms: start.elapsed().as_millis() as u64,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Handle streaming RAG query requests
#[utoipa::path(
    post,
    path = "/api/v1/query/stream",
    tag = "query",
    request_body = QueryRequest,
    responses(
        (status = 200, description = "Streaming response started"),
        (status = 400, description = "Invalid request", body = crate::error::ApiError)
    )
)]
pub async fn query_stream_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<QueryRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    state.increment_requests();

    if req.question.trim().is_empty() {
        return Err(AppError::BadRequest("Question cannot be empty".to_string()));
    }

    // First, search for relevant context from vector store
    let context = if let Some(vector_store) = state.vector_store.read().await.clone() {
        match vector_store.search(&req.question, req.top_k).await {
            Ok(results) => {
                if results.is_empty() {
                    tracing::info!("No relevant documents found for query");
                    String::new()
                } else {
                    tracing::info!("Found {} relevant documents", results.len());
                    results
                        .iter()
                        .enumerate()
                        .map(|(i, r)| format!("[문서 {}] {}", i + 1, r.content))
                        .collect::<Vec<_>>()
                        .join("\n\n")
                }
            }
            Err(e) => {
                tracing::warn!("Vector search failed: {}", e);
                String::new()
            }
        }
    } else {
        String::new()
    };

    // Collect chunks to stream (either from LLM or mock)
    let chunks: Vec<String> = if let Some(llm) = state.llm_client.read().await.clone() {
        // Build prompt with retrieved context
        let prompt = if context.is_empty() {
            format!(
                "당신은 조직의 지식 전문가입니다.\n\
                 질문에 대해 간결하고 정확하게 답변하세요.\n\n\
                 질문: {}\n\n답변:",
                req.question
            )
        } else {
            format!(
                "당신은 조직의 지식 전문가입니다.\n\
                 아래 제공된 문서를 참고하여 질문에 답변하세요.\n\
                 문서에 없는 내용은 추측하지 마세요.\n\n\
                 === 참고 문서 ===\n{}\n\n\
                 === 질문 ===\n{}\n\n답변:",
                context, req.question
            )
        };

        match llm.generate_stream(&prompt).await {
            Ok(mut llm_stream) => {
                let mut collected = Vec::new();
                while let Some(result) = llm_stream.next().await {
                    match result {
                        Ok(chunk) => collected.push(chunk),
                        Err(e) => {
                            tracing::error!("Stream chunk error: {}", e);
                            collected.push("[스트리밍 오류]".to_string());
                            break;
                        }
                    }
                }
                collected
            }
            Err(e) => {
                tracing::error!("LLM stream failed: {}", e);
                get_mock_chunks()
            }
        }
    } else {
        tracing::warn!("LLM not initialized, returning mock streaming response");
        get_mock_chunks()
    };

    let stream = stream::iter(chunks.into_iter().enumerate().map(|(i, chunk)| {
        Ok(Event::default()
            .data(chunk)
            .id(i.to_string())
            .event("message"))
    }));

    Ok(Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    ))
}

/// Get mock chunks for fallback streaming response
fn get_mock_chunks() -> Vec<String> {
    vec![
        "연차휴가 신청은 ".to_string(),
        "사내 인사시스템을 ".to_string(),
        "통해 진행됩니다. ".to_string(),
        "팀장 승인 후 ".to_string(),
        "인사팀에서 ".to_string(),
        "최종 처리됩니다. ".to_string(),
        "(주의: LLM이 초기화되지 않아 Mock 응답입니다)".to_string(),
    ]
}
