/// Authentication middleware for protecting routes
///
/// Extracts and validates JWT tokens from the Authorization header.
/// On success, adds authenticated user information to request extensions.
use super::jwt::{validate_access_token, Claims, JwtConfig, JwtError};
use crate::audit::{audit_log, extract_ip_address, extract_user_agent, AuditEvent};
use axum::{
    body::Body,
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Mutex;
use thiserror::Error;
use uuid::Uuid;

/// In-memory token blacklist for single-instance deployments
///
/// This stores revoked JWT IDs (JTI) in memory. For multi-instance
/// deployments, upgrade to Redis or database-backed storage.
///
/// # Thread Safety
///
/// Uses Mutex to ensure thread-safe access across async tasks.
static TOKEN_BLACKLIST: Mutex<Option<HashSet<String>>> = Mutex::new(None);

/// Authenticated user information extracted from JWT
///
/// This is added to request extensions by the auth middleware
/// and can be extracted in handlers using `Extension<AuthenticatedUser>`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    /// User's unique identifier
    pub user_id: Uuid,
    /// User's email address
    pub email: String,
    /// User's display name
    pub name: String,
    /// User's role (admin, editor, viewer)
    pub role: String,
    /// User's department (optional, for ACL filtering)
    pub department: Option<String>,
    /// JWT token ID (for blacklist checking)
    pub jti: String,
}

impl AuthenticatedUser {
    /// Check if user has admin role
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }

    /// Check if user has editor role or higher
    pub fn is_editor_or_higher(&self) -> bool {
        matches!(self.role.as_str(), "admin" | "editor")
    }

    /// Check if user can access a specific department
    pub fn can_access_department(&self, dept: &str) -> bool {
        self.is_admin() || self.department.as_deref() == Some(dept)
    }
}

impl From<Claims> for AuthenticatedUser {
    fn from(claims: Claims) -> Self {
        Self {
            user_id: Uuid::parse_str(&claims.sub).unwrap_or_else(|_| Uuid::nil()),
            email: claims.email,
            name: claims.name,
            role: claims.role,
            department: claims.department,
            jti: claims.jti,
        }
    }
}

/// Authentication middleware errors
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Missing Authorization header")]
    MissingAuthHeader,

    #[error("Invalid Authorization header format")]
    InvalidAuthHeader,

    #[error("Invalid token: {0}")]
    InvalidToken(#[from] JwtError),

    #[error("Token has been revoked")]
    TokenRevoked,

    #[error("Insufficient permissions")]
    InsufficientPermissions,

    #[error("Access denied to resource: {0}")]
    AccessDenied(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::MissingAuthHeader => {
                (StatusCode::UNAUTHORIZED, "Missing Authorization header")
            }
            AuthError::InvalidAuthHeader => (
                StatusCode::UNAUTHORIZED,
                "Invalid Authorization header format",
            ),
            AuthError::InvalidToken(_) => (StatusCode::UNAUTHORIZED, "Invalid or expired token"),
            AuthError::TokenRevoked => (StatusCode::UNAUTHORIZED, "Token has been revoked"),
            AuthError::InsufficientPermissions => {
                (StatusCode::FORBIDDEN, "Insufficient permissions")
            }
            AuthError::AccessDenied(_) => (StatusCode::FORBIDDEN, "Access denied"),
        };

        let body = serde_json::json!({
            "error": message,
            "status": status.as_u16(),
        });

        (status, axum::Json(body)).into_response()
    }
}

/// Authentication middleware that requires a valid JWT token
///
/// This middleware:
/// 1. Extracts the Authorization header
/// 2. Validates the Bearer token format
/// 3. Validates the JWT signature and expiration
/// 4. Adds AuthenticatedUser to request extensions
///
/// # Usage
///
/// ```ignore
/// use axum::{Router, routing::get, middleware};
/// use otl_api::auth::middleware::auth_middleware;
///
/// let app = Router::new()
///     .route("/protected", get(protected_handler))
///     .route_layer(middleware::from_fn(auth_middleware));
/// ```
///
/// In handlers, extract the user:
///
/// ```
/// use axum::Extension;
/// use otl_api::auth::middleware::AuthenticatedUser;
///
/// async fn protected_handler(
///     Extension(user): Extension<AuthenticatedUser>
/// ) -> String {
///     format!("Hello, {}!", user.name)
/// }
/// ```
pub async fn auth_middleware(
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, AuthError> {
    // Extract context for audit logging
    let ip_address = extract_ip_address(request.headers());
    let user_agent = extract_user_agent(request.headers());

    // Extract Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .ok_or(AuthError::MissingAuthHeader)?
        .to_str()
        .map_err(|_| AuthError::InvalidAuthHeader)?;

    // Validate Bearer token format
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AuthError::InvalidAuthHeader)?;

    // Load JWT configuration (in production, this should come from app state)
    let config = JwtConfig::from_env();

    // Validate token and extract claims
    let claims = match validate_access_token(&config, token) {
        Ok(c) => c,
        Err(e) => {
            // Log invalid token attempt
            audit_log(&AuditEvent::InvalidToken {
                ip_address,
                user_agent,
                reason: e.to_string(),
            });
            return Err(AuthError::InvalidToken(e));
        }
    };

    // Convert claims to AuthenticatedUser
    let user = AuthenticatedUser::from(claims);

    // Check token blacklist (for logout support)
    if is_token_revoked(&user.jti) {
        // Log revoked token attempt
        audit_log(&AuditEvent::InvalidToken {
            ip_address,
            user_agent,
            reason: "Token has been revoked".to_string(),
        });
        return Err(AuthError::TokenRevoked);
    }

    // Add user to request extensions
    request.extensions_mut().insert(user);

    // Continue to the next middleware/handler
    Ok(next.run(request).await)
}

/// Optional authentication middleware
///
/// Unlike `auth_middleware`, this doesn't fail if no token is present.
/// It only adds the user to extensions if a valid token exists.
///
/// Useful for endpoints that behave differently for authenticated users
/// but are also accessible anonymously.
pub async fn optional_auth_middleware(mut request: Request<Body>, next: Next) -> Response {
    // Try to extract Authorization header
    if let Some(auth_header) = request.headers().get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                let config = JwtConfig::from_env();
                if let Ok(claims) = validate_access_token(&config, token) {
                    let user = AuthenticatedUser::from(claims);
                    request.extensions_mut().insert(user);
                }
            }
        }
    }

    next.run(request).await
}

/// Type alias for role middleware future
type RoleMiddlewareFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, AuthError>> + Send>>;

/// Middleware factory for role-based access control
///
/// Returns a middleware that checks if the authenticated user has the required role.
///
/// # Example
///
/// ```ignore
/// use axum::{Router, routing::get, middleware};
/// use otl_api::auth::middleware::{auth_middleware, require_role};
///
/// let app = Router::new()
///     .route("/admin", get(admin_handler))
///     .route_layer(middleware::from_fn(require_role("admin")))
///     .route_layer(middleware::from_fn(auth_middleware));
/// ```
pub fn require_role(
    required_role: &'static str,
) -> impl Fn(Request<Body>, Next) -> RoleMiddlewareFuture + Clone {
    move |request: Request<Body>, next: Next| {
        Box::pin(async move {
            // Extract context for audit logging
            let ip_address = extract_ip_address(request.headers());
            let user_agent = extract_user_agent(request.headers());

            // Extract authenticated user from extensions
            let user = request
                .extensions()
                .get::<AuthenticatedUser>()
                .ok_or(AuthError::MissingAuthHeader)?
                .clone();

            // Check role
            if user.role != required_role && user.role != "admin" {
                // Log access denied
                audit_log(&AuditEvent::AccessDenied {
                    user_id: Some(user.user_id),
                    email: Some(user.email.clone()),
                    resource: format!("role:{required_role}"),
                    required_role: Some(required_role.to_string()),
                    ip_address,
                    user_agent,
                });

                return Err(AuthError::InsufficientPermissions);
            }

            Ok(next.run(request).await)
        })
    }
}

/// Middleware for requiring any of multiple roles
///
/// # Example
///
/// ```ignore
/// use axum::{Router, routing::post, middleware};
/// use otl_api::auth::middleware::{auth_middleware, require_any_role};
///
/// let app = Router::new()
///     .route("/edit", post(edit_handler))
///     .route_layer(middleware::from_fn(require_any_role(&["admin", "editor"])))
///     .route_layer(middleware::from_fn(auth_middleware));
/// ```
pub fn require_any_role(
    required_roles: &'static [&'static str],
) -> impl Fn(Request<Body>, Next) -> RoleMiddlewareFuture + Clone {
    move |request: Request<Body>, next: Next| {
        Box::pin(async move {
            // Extract context for audit logging
            let ip_address = extract_ip_address(request.headers());
            let user_agent = extract_user_agent(request.headers());

            let user = request
                .extensions()
                .get::<AuthenticatedUser>()
                .ok_or(AuthError::MissingAuthHeader)?
                .clone();

            // Admin always has access
            if user.is_admin() {
                return Ok(next.run(request).await);
            }

            // Check if user has any of the required roles
            if !required_roles.contains(&user.role.as_str()) {
                // Log access denied
                audit_log(&AuditEvent::AccessDenied {
                    user_id: Some(user.user_id),
                    email: Some(user.email.clone()),
                    resource: format!("roles:{}", required_roles.join(",")),
                    required_role: Some(required_roles.join(",")),
                    ip_address,
                    user_agent,
                });

                return Err(AuthError::InsufficientPermissions);
            }

            Ok(next.run(request).await)
        })
    }
}

/// Add a token to the blacklist by its JTI
///
/// This revokes the token, preventing it from being used for authentication
/// even if it hasn't expired yet. Used for logout functionality.
///
/// # Arguments
///
/// * `jti` - JWT ID from the token claims
///
/// # Example
///
/// ```ignore
/// use otl_api::auth::middleware::revoke_token;
///
/// // In logout handler
/// revoke_token(&user.jti);
/// ```
///
/// # Thread Safety
///
/// This function is thread-safe and can be called from multiple async tasks.
pub fn revoke_token(jti: &str) {
    let mut blacklist = TOKEN_BLACKLIST.lock().unwrap();
    if blacklist.is_none() {
        *blacklist = Some(HashSet::new());
    }
    if let Some(ref mut set) = *blacklist {
        set.insert(jti.to_string());
    }
}

/// Check if a token is revoked by its JTI
///
/// Returns true if the token has been revoked, false otherwise.
///
/// # Arguments
///
/// * `jti` - JWT ID from the token claims
///
/// # Returns
///
/// * `true` - Token is revoked and should not be accepted
/// * `false` - Token is valid (not revoked)
///
/// # Thread Safety
///
/// This function is thread-safe and can be called from multiple async tasks.
pub fn is_token_revoked(jti: &str) -> bool {
    let mut blacklist = TOKEN_BLACKLIST.lock().unwrap();
    if blacklist.is_none() {
        *blacklist = Some(HashSet::new());
        return false;
    }
    blacklist
        .as_ref()
        .map(|set| set.contains(jti))
        .unwrap_or(false)
}

/// Clear all revoked tokens from the blacklist
///
/// This is primarily useful for testing or if you want to start fresh.
/// In production, consider implementing a cleanup strategy based on
/// token expiration times instead.
///
/// # Thread Safety
///
/// This function is thread-safe and can be called from multiple async tasks.
pub fn clear_blacklist() {
    let mut blacklist = TOKEN_BLACKLIST.lock().unwrap();
    if let Some(ref mut set) = *blacklist {
        set.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authenticated_user_from_claims() {
        let claims = Claims {
            iss: "otl-api".to_string(),
            sub: Uuid::new_v4().to_string(),
            jti: Uuid::new_v4().to_string(),
            iat: 1000,
            exp: 2000,
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            role: "editor".to_string(),
            department: Some("Engineering".to_string()),
        };

        let user = AuthenticatedUser::from(claims);

        assert_eq!(user.name, "Test User");
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.role, "editor");
        assert_eq!(user.department, Some("Engineering".to_string()));
    }

    #[test]
    fn test_is_admin() {
        let admin = AuthenticatedUser {
            user_id: Uuid::new_v4(),
            email: "admin@example.com".to_string(),
            name: "Admin".to_string(),
            role: "admin".to_string(),
            department: None,
            jti: Uuid::new_v4().to_string(),
        };

        let editor = AuthenticatedUser {
            user_id: Uuid::new_v4(),
            email: "editor@example.com".to_string(),
            name: "Editor".to_string(),
            role: "editor".to_string(),
            department: None,
            jti: Uuid::new_v4().to_string(),
        };

        assert!(admin.is_admin());
        assert!(!editor.is_admin());
    }

    #[test]
    fn test_is_editor_or_higher() {
        let roles = vec![("admin", true), ("editor", true), ("viewer", false)];

        for (role, expected) in roles {
            let user = AuthenticatedUser {
                user_id: Uuid::new_v4(),
                email: "test@example.com".to_string(),
                name: "Test".to_string(),
                role: role.to_string(),
                department: None,
                jti: Uuid::new_v4().to_string(),
            };

            assert_eq!(user.is_editor_or_higher(), expected, "Role: {}", role);
        }
    }

    #[test]
    fn test_can_access_department() {
        let admin = AuthenticatedUser {
            user_id: Uuid::new_v4(),
            email: "admin@example.com".to_string(),
            name: "Admin".to_string(),
            role: "admin".to_string(),
            department: None,
            jti: Uuid::new_v4().to_string(),
        };

        let eng_user = AuthenticatedUser {
            user_id: Uuid::new_v4(),
            email: "eng@example.com".to_string(),
            name: "Engineer".to_string(),
            role: "editor".to_string(),
            department: Some("Engineering".to_string()),
            jti: Uuid::new_v4().to_string(),
        };

        // Admin can access any department
        assert!(admin.can_access_department("Engineering"));
        assert!(admin.can_access_department("HR"));

        // User can access their own department
        assert!(eng_user.can_access_department("Engineering"));
        assert!(!eng_user.can_access_department("HR"));
    }

    #[test]
    fn test_token_blacklist() {
        // Clear blacklist before test
        clear_blacklist();

        let jti1 = Uuid::new_v4().to_string();
        let jti2 = Uuid::new_v4().to_string();

        // Initially, tokens should not be revoked
        assert!(!is_token_revoked(&jti1));
        assert!(!is_token_revoked(&jti2));

        // Revoke first token
        revoke_token(&jti1);
        assert!(is_token_revoked(&jti1));
        assert!(!is_token_revoked(&jti2));

        // Revoke second token
        revoke_token(&jti2);
        assert!(is_token_revoked(&jti1));
        assert!(is_token_revoked(&jti2));

        // Clear blacklist
        clear_blacklist();
        assert!(!is_token_revoked(&jti1));
        assert!(!is_token_revoked(&jti2));
    }

    #[test]
    fn test_revoke_same_token_twice() {
        clear_blacklist();

        let jti = Uuid::new_v4().to_string();

        // Revoke token twice
        revoke_token(&jti);
        revoke_token(&jti);

        // Should still be revoked
        assert!(is_token_revoked(&jti));
    }
}
