//! API Integration Tests
//!
//! Author: hephaex@gmail.com

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use otl_api::create_router_default;
use serde_json::{json, Value};
use tower::ServiceExt;

/// Helper to create a test request
fn create_json_request(method: &str, uri: &str, body: Option<Value>) -> Request<Body> {
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("Content-Type", "application/json");

    match body {
        Some(json_body) => builder
            .body(Body::from(serde_json::to_string(&json_body).unwrap()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    }
}

// =============================================================================
// Health Check Tests
// =============================================================================

#[tokio::test]
async fn test_health_check() {
    let app = create_router_default();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "ok");
    assert!(json["version"].is_string());
}

#[tokio::test]
async fn test_readiness_check() {
    let app = create_router_default();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json["ready"].is_boolean());
    assert!(json["checks"].is_object());
}

#[tokio::test]
async fn test_metrics_endpoint() {
    let app = create_router_default();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json["uptime_seconds"].is_number());
    assert!(json["total_requests"].is_number());
}

// =============================================================================
// Query API Tests
// =============================================================================

#[tokio::test]
async fn test_query_endpoint_success() {
    let app = create_router_default();

    let request = create_json_request(
        "POST",
        "/api/v1/query",
        Some(json!({
            "question": "연차휴가 신청 절차가 어떻게 되나요?",
            "top_k": 5
        })),
    );

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json["answer"].is_string());
    assert!(json["citations"].is_array());
    assert!(json["confidence"].is_number());
    assert!(json["processing_time_ms"].is_number());
}

#[tokio::test]
async fn test_query_endpoint_empty_question() {
    let app = create_router_default();

    let request = create_json_request(
        "POST",
        "/api/v1/query",
        Some(json!({
            "question": "",
            "top_k": 5
        })),
    );

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["code"], "BAD_REQUEST");
}

#[tokio::test]
async fn test_query_endpoint_whitespace_question() {
    let app = create_router_default();

    let request = create_json_request(
        "POST",
        "/api/v1/query",
        Some(json!({
            "question": "   ",
            "top_k": 5
        })),
    );

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// =============================================================================
// Document API Tests
// =============================================================================

#[tokio::test]
async fn test_list_documents() {
    let app = create_router_default();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json["documents"].is_array());
    assert!(json["total"].is_number());
    assert!(json["page"].is_number());
    assert!(json["page_size"].is_number());
}

#[tokio::test]
async fn test_list_documents_with_pagination() {
    let app = create_router_default();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents?page=1&page_size=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["page"], 1);
    assert_eq!(json["page_size"], 10);
}

#[tokio::test]
async fn test_get_document() {
    let app = create_router_default();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents/550e8400-e29b-41d4-a716-446655440000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json["id"].is_string());
    assert!(json["title"].is_string());
    assert!(json["file_type"].is_string());
}

#[tokio::test]
async fn test_upload_document() {
    let app = create_router_default();

    let request = create_json_request(
        "POST",
        "/api/v1/documents",
        Some(json!({
            "title": "테스트 문서.txt",
            "content": "dGVzdCBjb250ZW50",  // base64 encoded "test content"
            "file_type": "txt",  // Use txt to avoid magic bytes validation
            "access_level": "internal",
            "department": "인사팀"
        })),
    );

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json["id"].is_string());
    assert!(json["message"].is_string());
}

#[tokio::test]
async fn test_upload_document_empty_title() {
    let app = create_router_default();

    let request = create_json_request(
        "POST",
        "/api/v1/documents",
        Some(json!({
            "title": "",
            "content": "dGVzdCBjb250ZW50",
            "file_type": "txt"  // Use txt to avoid magic bytes validation
        })),
    );

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_delete_document() {
    let app = create_router_default();

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/documents/550e8400-e29b-41d4-a716-446655440000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json["message"].is_string());
}

// =============================================================================
// Verification API Tests
// =============================================================================

#[tokio::test]
async fn test_list_pending_extractions() {
    let app = create_router_default();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/verify/pending")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json["extractions"].is_array());
    assert!(json["total"].is_number());
}

#[tokio::test]
async fn test_approve_extraction() {
    let app = create_router_default();

    let request = create_json_request(
        "POST",
        "/api/v1/verify/550e8400-e29b-41d4-a716-446655440000/approve",
        Some(json!({
            "notes": "Looks correct"
        })),
    );

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "approved");
}

#[tokio::test]
async fn test_reject_extraction() {
    let app = create_router_default();

    let request = create_json_request(
        "POST",
        "/api/v1/verify/550e8400-e29b-41d4-a716-446655440000/reject",
        Some(json!({
            "reason": "Incorrect entity type"
        })),
    );

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "rejected");
}

// =============================================================================
// Graph API Tests
// =============================================================================

#[tokio::test]
async fn test_list_entities() {
    let app = create_router_default();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/graph/entities")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_search_graph() {
    let app = create_router_default();

    let request = create_json_request(
        "POST",
        "/api/v1/graph/search",
        Some(json!({
            "query": "연차휴가",
            "limit": 10
        })),
    );

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

// =============================================================================
// OpenAPI/Swagger Tests
// =============================================================================

#[tokio::test]
async fn test_swagger_ui_available() {
    let app = create_router_default();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/swagger-ui/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Swagger UI should redirect or return HTML
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::MOVED_PERMANENTLY
    );
}

#[tokio::test]
async fn test_openapi_spec_available() {
    let app = create_router_default();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api-docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify it's a valid OpenAPI spec
    assert!(json["openapi"].is_string());
    assert!(json["info"].is_object());
    assert!(json["paths"].is_object());
}
