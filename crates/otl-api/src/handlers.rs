//! API request handlers
//!
//! Author: hephaex@gmail.com

use axum::{extract::Json, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};

/// Query request body
#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    /// User's question
    pub question: String,
    /// Maximum results to return
    #[serde(default = "default_top_k")]
    pub top_k: usize,
}

fn default_top_k() -> usize {
    5
}

/// Query response body
#[derive(Debug, Serialize)]
pub struct QueryResponse {
    /// Generated answer
    pub answer: String,
    /// Source citations
    pub citations: Vec<String>,
}

/// Handle RAG query requests
pub async fn query(Json(req): Json<QueryRequest>) -> impl IntoResponse {
    // TODO: Implement actual RAG query logic
    let response = QueryResponse {
        answer: format!("Query received: {} (top_k: {})", req.question, req.top_k),
        citations: vec![],
    };
    (StatusCode::OK, Json(response))
}
