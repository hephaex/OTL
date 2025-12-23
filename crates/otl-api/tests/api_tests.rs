//! API Integration Tests
//!
//! Note: Tests marked with #[ignore] require a real database connection.
//! To run them, set up a test database and run: cargo test -- --ignored
//!
//! Author: hephaex@gmail.com

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use otl_api::create_router_for_testing;
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
    let app = create_router_for_testing();

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
    let app = create_router_for_testing();

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
    let app = create_router_for_testing();

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
#[ignore = "requires database and authentication"]
async fn test_query_endpoint_success() {
    let app = create_router_for_testing();

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
#[ignore = "requires database and authentication"]
async fn test_query_endpoint_empty_question() {
    let app = create_router_for_testing();

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
#[ignore = "requires database and authentication"]
async fn test_query_endpoint_whitespace_question() {
    let app = create_router_for_testing();

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
// Note: These tests require a real database connection

#[tokio::test]
#[ignore = "requires database"]
async fn test_list_documents() {
    let app = create_router_for_testing();

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
#[ignore = "requires database"]
async fn test_list_documents_with_pagination() {
    let app = create_router_for_testing();

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
#[ignore = "requires database"]
async fn test_get_document() {
    let app = create_router_for_testing();

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
#[ignore = "requires database"]
async fn test_upload_document() {
    let app = create_router_for_testing();

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
#[ignore = "requires database"]
async fn test_upload_document_empty_title() {
    let app = create_router_for_testing();

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
#[ignore = "requires database"]
async fn test_delete_document() {
    let app = create_router_for_testing();

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
// Note: These tests require a real database connection

#[tokio::test]
#[ignore = "requires database"]
async fn test_list_pending_extractions() {
    let app = create_router_for_testing();

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
#[ignore = "requires database"]
async fn test_approve_extraction() {
    let app = create_router_for_testing();

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
#[ignore = "requires database"]
async fn test_reject_extraction() {
    let app = create_router_for_testing();

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
// Note: These tests require a real database connection

#[tokio::test]
#[ignore = "requires database"]
async fn test_list_entities() {
    let app = create_router_for_testing();

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
#[ignore = "requires database"]
async fn test_search_graph() {
    let app = create_router_for_testing();

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
// Authentication API Tests
// =============================================================================
// Note: These tests require a real database connection

#[tokio::test]
#[ignore = "requires database"]
async fn test_register_success() {
    let app = create_router_for_testing();

    let request = create_json_request(
        "POST",
        "/api/v1/auth/register",
        Some(json!({
            "email": "newuser@example.com",
            "password": "SecurePass123!@#",
            "name": "New User",
            "department": "Engineering"
        })),
    );

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json["user_id"].is_string());
    assert_eq!(json["email"], "newuser@example.com");
    assert_eq!(json["name"], "New User");
    assert_eq!(json["role"], "viewer");
    assert_eq!(json["message"], "Registration successful");
}

#[tokio::test]
#[ignore = "requires database"]
async fn test_register_duplicate_email() {
    let app = create_router_for_testing();

    // Register first user
    let request1 = create_json_request(
        "POST",
        "/api/v1/auth/register",
        Some(json!({
            "email": "duplicate@example.com",
            "password": "SecurePass123!@#",
            "name": "User One"
        })),
    );
    app.clone().oneshot(request1).await.unwrap();

    // Try to register with same email
    let request2 = create_json_request(
        "POST",
        "/api/v1/auth/register",
        Some(json!({
            "email": "duplicate@example.com",
            "password": "DifferentPass456!@#",
            "name": "User Two"
        })),
    );

    let response = app.oneshot(request2).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["code"], "BAD_REQUEST");
}

#[tokio::test]
#[ignore = "requires database"]
async fn test_register_weak_password() {
    let app = create_router_for_testing();

    let request = create_json_request(
        "POST",
        "/api/v1/auth/register",
        Some(json!({
            "email": "weakpass@example.com",
            "password": "weak",
            "name": "Weak Password User"
        })),
    );

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["code"], "BAD_REQUEST");
    assert!(json["message"]
        .as_str()
        .unwrap()
        .contains("Password validation failed"));
}

#[tokio::test]
#[ignore = "requires database"]
async fn test_login_success() {
    let app = create_router_for_testing();

    // First, register a user
    let register_request = create_json_request(
        "POST",
        "/api/v1/auth/register",
        Some(json!({
            "email": "logintest@example.com",
            "password": "SecurePass123!@#",
            "name": "Login Test User"
        })),
    );
    app.clone().oneshot(register_request).await.unwrap();

    // Now try to login
    let login_request = create_json_request(
        "POST",
        "/api/v1/auth/login",
        Some(json!({
            "email": "logintest@example.com",
            "password": "SecurePass123!@#"
        })),
    );

    let response = app.oneshot(login_request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify JWT tokens are returned
    assert!(json["access_token"].is_string());
    assert!(!json["access_token"].as_str().unwrap().is_empty());
    assert!(json["refresh_token"].is_string());
    assert!(!json["refresh_token"].as_str().unwrap().is_empty());
    assert_eq!(json["token_type"], "Bearer");
    assert!(json["expires_in"].is_number());

    // Verify user info is returned
    assert!(json["user"].is_object());
    assert_eq!(json["user"]["email"], "logintest@example.com");
    assert_eq!(json["user"]["name"], "Login Test User");
    assert_eq!(json["user"]["role"], "viewer");
}

#[tokio::test]
#[ignore = "requires database"]
async fn test_login_invalid_credentials() {
    let app = create_router_for_testing();

    let request = create_json_request(
        "POST",
        "/api/v1/auth/login",
        Some(json!({
            "email": "nonexistent@example.com",
            "password": "WrongPass123!@#"
        })),
    );

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[ignore = "requires database"]
async fn test_login_wrong_password() {
    let app = create_router_for_testing();

    // Register a user
    let register_request = create_json_request(
        "POST",
        "/api/v1/auth/register",
        Some(json!({
            "email": "wrongpass@example.com",
            "password": "CorrectPass123!@#",
            "name": "Wrong Pass User"
        })),
    );
    app.clone().oneshot(register_request).await.unwrap();

    // Try to login with wrong password
    let login_request = create_json_request(
        "POST",
        "/api/v1/auth/login",
        Some(json!({
            "email": "wrongpass@example.com",
            "password": "WrongPass456!@#"
        })),
    );

    let response = app.oneshot(login_request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[ignore = "requires database"]
async fn test_refresh_token_works() {
    let app = create_router_for_testing();

    // Register and login
    let register_request = create_json_request(
        "POST",
        "/api/v1/auth/register",
        Some(json!({
            "email": "refreshtest@example.com",
            "password": "SecurePass123!@#",
            "name": "Refresh Test User"
        })),
    );
    app.clone().oneshot(register_request).await.unwrap();

    let login_request = create_json_request(
        "POST",
        "/api/v1/auth/login",
        Some(json!({
            "email": "refreshtest@example.com",
            "password": "SecurePass123!@#"
        })),
    );

    let login_response = app.clone().oneshot(login_request).await.unwrap();
    let login_body = axum::body::to_bytes(login_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let login_json: Value = serde_json::from_slice(&login_body).unwrap();
    let refresh_token = login_json["refresh_token"].as_str().unwrap();

    // Use refresh token to get new access token
    let refresh_request = create_json_request(
        "POST",
        "/api/v1/auth/refresh",
        Some(json!({
            "refresh_token": refresh_token
        })),
    );

    let refresh_response = app.oneshot(refresh_request).await.unwrap();

    assert_eq!(refresh_response.status(), StatusCode::OK);

    let refresh_body = axum::body::to_bytes(refresh_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let refresh_json: Value = serde_json::from_slice(&refresh_body).unwrap();

    // Verify new tokens are returned
    assert!(refresh_json["access_token"].is_string());
    assert!(refresh_json["refresh_token"].is_string());
    assert_eq!(refresh_json["token_type"], "Bearer");
    // Token rotation: new refresh token should be different
    assert_ne!(
        refresh_json["refresh_token"].as_str().unwrap(),
        refresh_token
    );
}

#[tokio::test]
#[ignore = "requires database"]
async fn test_refresh_token_invalid() {
    let app = create_router_for_testing();

    let request = create_json_request(
        "POST",
        "/api/v1/auth/refresh",
        Some(json!({
            "refresh_token": "invalid_refresh_token_12345"
        })),
    );

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[ignore = "requires database"]
async fn test_logout_invalidates_token() {
    let app = create_router_for_testing();

    // Register and login
    let register_request = create_json_request(
        "POST",
        "/api/v1/auth/register",
        Some(json!({
            "email": "logouttest@example.com",
            "password": "SecurePass123!@#",
            "name": "Logout Test User"
        })),
    );
    app.clone().oneshot(register_request).await.unwrap();

    let login_request = create_json_request(
        "POST",
        "/api/v1/auth/login",
        Some(json!({
            "email": "logouttest@example.com",
            "password": "SecurePass123!@#"
        })),
    );

    let login_response = app.clone().oneshot(login_request).await.unwrap();
    let login_body = axum::body::to_bytes(login_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let login_json: Value = serde_json::from_slice(&login_body).unwrap();
    let access_token = login_json["access_token"].as_str().unwrap();
    let refresh_token = login_json["refresh_token"].as_str().unwrap();

    // Logout
    let logout_request = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/logout")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {access_token}"))
        .body(Body::from(
            serde_json::to_string(&json!({
                "refresh_token": refresh_token
            }))
            .unwrap(),
        ))
        .unwrap();

    let logout_response = app.clone().oneshot(logout_request).await.unwrap();

    assert_eq!(logout_response.status(), StatusCode::OK);

    let logout_body = axum::body::to_bytes(logout_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let logout_json: Value = serde_json::from_slice(&logout_body).unwrap();

    assert_eq!(logout_json["message"], "Logged out successfully");

    // Try to use the access token after logout (should fail)
    let me_request = Request::builder()
        .method("GET")
        .uri("/api/v1/auth/me")
        .header("Authorization", format!("Bearer {access_token}"))
        .body(Body::empty())
        .unwrap();

    let me_response = app.oneshot(me_request).await.unwrap();

    // Should be unauthorized due to blacklisted token
    assert_eq!(me_response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[ignore = "requires database"]
async fn test_me_endpoint_returns_user_info() {
    let app = create_router_for_testing();

    // Register and login
    let register_request = create_json_request(
        "POST",
        "/api/v1/auth/register",
        Some(json!({
            "email": "metest@example.com",
            "password": "SecurePass123!@#",
            "name": "Me Test User",
            "department": "Engineering"
        })),
    );
    app.clone().oneshot(register_request).await.unwrap();

    let login_request = create_json_request(
        "POST",
        "/api/v1/auth/login",
        Some(json!({
            "email": "metest@example.com",
            "password": "SecurePass123!@#"
        })),
    );

    let login_response = app.clone().oneshot(login_request).await.unwrap();
    let login_body = axum::body::to_bytes(login_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let login_json: Value = serde_json::from_slice(&login_body).unwrap();
    let access_token = login_json["access_token"].as_str().unwrap();

    // Get current user info
    let me_request = Request::builder()
        .method("GET")
        .uri("/api/v1/auth/me")
        .header("Authorization", format!("Bearer {access_token}"))
        .body(Body::empty())
        .unwrap();

    let me_response = app.oneshot(me_request).await.unwrap();

    assert_eq!(me_response.status(), StatusCode::OK);

    let me_body = axum::body::to_bytes(me_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let me_json: Value = serde_json::from_slice(&me_body).unwrap();

    assert_eq!(me_json["email"], "metest@example.com");
    assert_eq!(me_json["name"], "Me Test User");
    assert_eq!(me_json["role"], "viewer");
    assert_eq!(me_json["department"], "Engineering");
    assert!(me_json["is_active"].as_bool().unwrap());
    assert!(me_json["id"].is_string());
    assert!(me_json["created_at"].is_string());
}

#[tokio::test]
#[ignore = "requires database"]
async fn test_me_endpoint_without_token() {
    let app = create_router_for_testing();

    let request = Request::builder()
        .method("GET")
        .uri("/api/v1/auth/me")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[ignore = "requires database"]
async fn test_me_endpoint_with_invalid_token() {
    let app = create_router_for_testing();

    let request = Request::builder()
        .method("GET")
        .uri("/api/v1/auth/me")
        .header("Authorization", "Bearer invalid.jwt.token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[ignore = "requires database"]
async fn test_protected_route_without_auth_returns_401() {
    let app = create_router_for_testing();

    // Try to access protected query endpoint without authentication
    let request = create_json_request(
        "POST",
        "/api/v1/query",
        Some(json!({
            "question": "What is the vacation policy?",
            "top_k": 5
        })),
    );

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[ignore = "requires database"]
async fn test_protected_document_endpoint_without_auth() {
    let app = create_router_for_testing();

    let request = Request::builder()
        .method("GET")
        .uri("/api/v1/documents")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[ignore = "requires database"]
async fn test_protected_graph_endpoint_without_auth() {
    let app = create_router_for_testing();

    let request = Request::builder()
        .method("GET")
        .uri("/api/v1/graph/entities")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[ignore = "requires database"]
async fn test_protected_verify_endpoint_without_auth() {
    let app = create_router_for_testing();

    let request = Request::builder()
        .method("GET")
        .uri("/api/v1/verify/pending")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// =============================================================================
// OpenAPI/Swagger Tests
// =============================================================================

#[tokio::test]
async fn test_swagger_ui_available() {
    let app = create_router_for_testing();

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
    let app = create_router_for_testing();

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
