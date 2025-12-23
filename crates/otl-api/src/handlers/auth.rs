//! Authentication API handlers
//!
//! Provides HTTP endpoints for user authentication and profile management.
//!
//! Author: hephaex@gmail.com

use crate::auth::{
    AuthenticatedUser, AuthService, LoginRequest, LogoutRequest, RefreshRequest,
    RegisterRequest,
};
use crate::error::AppError;
use crate::state::AppState;
use axum::{
    extract::State,
    response::IntoResponse,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

/// Registration response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RegisterResponse {
    pub user_id: String,
    pub email: String,
    pub name: String,
    pub role: String,
    pub message: String,
}

/// Logout response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LogoutResponse {
    pub message: String,
}

/// Register a new user account
///
/// Creates a new user with the provided email, password, and profile information.
/// New users are assigned the 'viewer' role by default.
///
/// # Request Body
///
/// * `email` - Valid email address (unique)
/// * `password` - Must meet security requirements (8+ chars, uppercase, lowercase, digit, special char)
/// * `name` - User's display name (2-100 characters)
/// * `department` - Optional department for ACL filtering
///
/// # Responses
///
/// * `201 Created` - User successfully registered
/// * `400 Bad Request` - Invalid input or email already exists
/// * `500 Internal Server Error` - Server error
#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    tag = "auth",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered successfully", body = RegisterResponse),
        (status = 400, description = "Invalid input", body = crate::error::ApiError),
        (status = 500, description = "Internal server error", body = crate::error::ApiError),
    )
)]
pub async fn register_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RegisterRequest>,
) -> Result<impl IntoResponse, AppError> {
    let auth_service = AuthService::new(state.db_pool.clone());
    let user = auth_service.register(request).await?;

    let response = RegisterResponse {
        user_id: user.id.clone(),
        email: user.email.clone(),
        name: user.name.clone(),
        role: user.role.clone(),
        message: "Registration successful".to_string(),
    };

    Ok((axum::http::StatusCode::CREATED, Json(response)))
}

/// Login with email and password
///
/// Authenticates a user and returns JWT access and refresh tokens.
/// Failed login attempts are tracked and account will be locked after 5 failed attempts.
///
/// # Request Body
///
/// * `email` - User's email address
/// * `password` - User's password
///
/// # Responses
///
/// * `200 OK` - Authentication successful, returns tokens
/// * `401 Unauthorized` - Invalid credentials
/// * `403 Forbidden` - Account locked or deactivated
/// * `500 Internal Server Error` - Server error
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = AuthResponse),
        (status = 401, description = "Invalid credentials", body = crate::error::ApiError),
        (status = 403, description = "Account locked or deactivated", body = crate::error::ApiError),
        (status = 500, description = "Internal server error", body = crate::error::ApiError),
    )
)]
pub async fn login_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    let auth_service = AuthService::new(state.db_pool.clone());
    let response = auth_service.login(request).await?;

    Ok(Json(response))
}

/// Refresh access token
///
/// Exchanges a valid refresh token for a new access token.
/// Implements token rotation - the old refresh token is revoked and a new one is issued.
///
/// # Request Body
///
/// * `refresh_token` - Valid refresh token from login
///
/// # Responses
///
/// * `200 OK` - New tokens issued
/// * `401 Unauthorized` - Invalid or expired refresh token
/// * `500 Internal Server Error` - Server error
#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    tag = "auth",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "Token refreshed successfully", body = AuthResponse),
        (status = 401, description = "Invalid refresh token", body = crate::error::ApiError),
        (status = 500, description = "Internal server error", body = crate::error::ApiError),
    )
)]
pub async fn refresh_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RefreshRequest>,
) -> Result<impl IntoResponse, AppError> {
    let auth_service = AuthService::new(state.db_pool.clone());
    let response = auth_service.refresh(request).await?;

    Ok(Json(response))
}

/// Logout current session
///
/// Invalidates the current access token and optionally revokes refresh token(s).
/// Requires valid authentication.
///
/// # Request Body (optional)
///
/// * `refresh_token` - Specific refresh token to revoke
/// * `logout_all_devices` - If true, revokes all refresh tokens for this user
///
/// # Responses
///
/// * `200 OK` - Logout successful
/// * `401 Unauthorized` - Invalid or missing authentication
/// * `500 Internal Server Error` - Server error
#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    tag = "auth",
    request_body(content = LogoutRequest, description = "Logout options (optional)"),
    responses(
        (status = 200, description = "Logout successful", body = LogoutResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ApiError),
        (status = 500, description = "Internal server error", body = crate::error::ApiError),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn logout_handler(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(request): Json<LogoutRequest>,
) -> Result<impl IntoResponse, AppError> {
    let auth_service = AuthService::new(state.db_pool.clone());
    auth_service.logout(user.user_id, &user.jti, request).await?;

    Ok(Json(LogoutResponse {
        message: "Logged out successfully".to_string(),
    }))
}

/// Get current user profile
///
/// Returns the profile information for the authenticated user.
/// Requires valid authentication.
///
/// # Responses
///
/// * `200 OK` - User profile
/// * `401 Unauthorized` - Invalid or missing authentication
/// * `500 Internal Server Error` - Server error
#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    tag = "auth",
    responses(
        (status = 200, description = "Current user profile", body = UserInfo),
        (status = 401, description = "Unauthorized", body = crate::error::ApiError),
        (status = 500, description = "Internal server error", body = crate::error::ApiError),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn me_handler(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<impl IntoResponse, AppError> {
    let auth_service = AuthService::new(state.db_pool.clone());
    let user_info = auth_service.get_user(user.user_id).await?;

    Ok(Json(user_info))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_response_serialization() {
        let response = RegisterResponse {
            user_id: "123".to_string(),
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            role: "viewer".to_string(),
            message: "Success".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test@example.com"));
    }

    #[test]
    fn test_logout_response_serialization() {
        let response = LogoutResponse {
            message: "Logged out".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Logged out"));
    }
}
