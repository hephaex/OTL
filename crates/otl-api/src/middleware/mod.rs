//! Authentication and authorization middleware
//!
//! Author: hephaex@gmail.com

// Rate limiting temporarily disabled - tower_governor 0.8 API changes require further work
// pub mod rate_limit;

pub mod metrics;
pub mod security_headers;

pub use metrics::metrics_middleware;
pub use security_headers::security_headers_middleware;

use axum::{
    body::Body,
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

/// JWT claims
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// User's name
    pub name: String,
    /// User's email
    pub email: Option<String>,
    /// User's roles
    pub roles: Vec<String>,
    /// User's department
    pub department: Option<String>,
    /// Expiration time
    pub exp: usize,
    /// Issued at
    pub iat: usize,
}

impl Claims {
    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Check if user is admin
    pub fn is_admin(&self) -> bool {
        self.has_role("admin")
    }
}

/// Authenticated user extracted from JWT
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
    pub name: String,
    pub email: Option<String>,
    pub roles: Vec<String>,
    pub department: Option<String>,
}

impl From<Claims> for AuthUser {
    fn from(claims: Claims) -> Self {
        Self {
            user_id: claims.sub,
            name: claims.name,
            email: claims.email,
            roles: claims.roles,
            department: claims.department,
        }
    }
}

/// JWT authentication middleware
pub async fn auth_middleware(
    request: Request<Body>,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    // Get JWT secret from environment
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev-secret-key".to_string());

    // Extract token from Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let token = match auth_header {
        Some(h) if h.starts_with("Bearer ") => &h[7..],
        _ => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "code": "UNAUTHORIZED",
                    "message": "Missing or invalid authorization header"
                })),
            ));
        }
    };

    // Validate token
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    );

    match token_data {
        Ok(data) => {
            // Add user claims to request extensions
            let mut request = request;
            request.extensions_mut().insert(AuthUser::from(data.claims));
            Ok(next.run(request).await)
        }
        Err(e) => Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "code": "INVALID_TOKEN",
                "message": format!("Invalid token: {e}")
            })),
        )),
    }
}

/// Optional authentication middleware (doesn't require auth but extracts user if present)
pub async fn optional_auth_middleware(mut request: Request<Body>, next: Next) -> Response {
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev-secret-key".to_string());

    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    if let Some(h) = auth_header {
        if let Some(token) = h.strip_prefix("Bearer ") {
            if let Ok(token_data) = decode::<Claims>(
                token,
                &DecodingKey::from_secret(jwt_secret.as_bytes()),
                &Validation::default(),
            ) {
                request
                    .extensions_mut()
                    .insert(AuthUser::from(token_data.claims));
            }
        }
    }

    next.run(request).await
}

/// Check if user has required role
pub fn check_role(
    user: Option<&AuthUser>,
    required_role: &str,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    match user {
        Some(u) if u.roles.contains(&required_role.to_string()) => Ok(()),
        Some(_) => Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "code": "FORBIDDEN",
                "message": format!("Required role: {required_role}")
            })),
        )),
        None => Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "code": "UNAUTHORIZED",
                "message": "Authentication required"
            })),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_test_token(roles: Vec<String>) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        let claims = Claims {
            sub: "test-user".to_string(),
            name: "Test User".to_string(),
            email: Some("test@example.com".to_string()),
            roles,
            department: Some("Engineering".to_string()),
            exp: now + 3600,
            iat: now,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret("dev-secret-key".as_bytes()),
        )
        .unwrap()
    }

    #[test]
    fn test_claims_has_role() {
        let claims = Claims {
            sub: "user1".to_string(),
            name: "User".to_string(),
            email: None,
            roles: vec!["admin".to_string(), "user".to_string()],
            department: None,
            exp: 0,
            iat: 0,
        };

        assert!(claims.has_role("admin"));
        assert!(claims.has_role("user"));
        assert!(!claims.has_role("superadmin"));
    }

    #[test]
    fn test_create_token() {
        let token = create_test_token(vec!["user".to_string()]);
        assert!(!token.is_empty());
    }
}
