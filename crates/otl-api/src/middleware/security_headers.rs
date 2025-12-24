//! Security headers middleware
//!
//! Adds security headers to all HTTP responses to protect against common web vulnerabilities.
//!
//! Headers configured:
//! - X-Content-Type-Options: nosniff - Prevents MIME type sniffing
//! - X-Frame-Options: DENY - Prevents clickjacking attacks
//! - X-XSS-Protection: 1; mode=block - Enables XSS filtering
//! - Strict-Transport-Security: HSTS header for HTTPS enforcement
//! - Content-Security-Policy: Restricts resource loading
//! - Referrer-Policy: Controls referrer information
//! - Permissions-Policy: Restricts access to browser features
//!
//! Author: hephaex@gmail.com

use axum::{
    body::Body,
    extract::Request,
    http::{header, HeaderValue},
    middleware::Next,
    response::Response,
};

/// Security headers middleware
///
/// Adds comprehensive security headers to all responses to protect against
/// common web vulnerabilities including XSS, clickjacking, MIME sniffing, etc.
pub async fn security_headers_middleware(request: Request<Body>, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    // Prevent MIME type sniffing
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );

    // Prevent clickjacking by disallowing embedding in frames
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));

    // Enable XSS filtering in older browsers
    headers.insert(
        header::X_XSS_PROTECTION,
        HeaderValue::from_static("1; mode=block"),
    );

    // Enforce HTTPS for 1 year including subdomains
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );

    // Content Security Policy - only allow resources from same origin
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'self'"),
    );

    // Control referrer information sent with requests
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    // Restrict access to browser features
    headers.insert(
        "permissions-policy",
        HeaderValue::from_static("geolocation=(), camera=(), microphone=()"),
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware,
        response::IntoResponse,
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    async fn test_handler() -> impl IntoResponse {
        (StatusCode::OK, "test response")
    }

    #[tokio::test]
    async fn test_security_headers_added() {
        // Create a router with the security headers middleware
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(middleware::from_fn(security_headers_middleware));

        // Create a test request
        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

        // Execute the request
        let response = app.oneshot(request).await.unwrap();

        // Verify status
        assert_eq!(response.status(), StatusCode::OK);

        // Verify all security headers are present
        let headers = response.headers();

        assert_eq!(
            headers.get(header::X_CONTENT_TYPE_OPTIONS).unwrap(),
            "nosniff"
        );

        assert_eq!(headers.get(header::X_FRAME_OPTIONS).unwrap(), "DENY");

        assert_eq!(
            headers.get(header::X_XSS_PROTECTION).unwrap(),
            "1; mode=block"
        );

        assert_eq!(
            headers.get(header::STRICT_TRANSPORT_SECURITY).unwrap(),
            "max-age=31536000; includeSubDomains"
        );

        assert_eq!(
            headers.get(header::CONTENT_SECURITY_POLICY).unwrap(),
            "default-src 'self'"
        );

        assert_eq!(
            headers.get(header::REFERRER_POLICY).unwrap(),
            "strict-origin-when-cross-origin"
        );

        assert_eq!(
            headers.get("permissions-policy").unwrap(),
            "geolocation=(), camera=(), microphone=()"
        );
    }

    #[tokio::test]
    async fn test_security_headers_on_error_response() {
        // Create a handler that returns an error
        async fn error_handler() -> impl IntoResponse {
            (StatusCode::INTERNAL_SERVER_ERROR, "error")
        }

        let app = Router::new()
            .route("/error", get(error_handler))
            .layer(middleware::from_fn(security_headers_middleware));

        let request = Request::builder()
            .uri("/error")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Security headers should be present even on error responses
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(response
            .headers()
            .get(header::X_CONTENT_TYPE_OPTIONS)
            .is_some());
        assert!(response.headers().get(header::X_FRAME_OPTIONS).is_some());
    }
}
