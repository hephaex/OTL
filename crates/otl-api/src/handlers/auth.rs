//! Authentication API handlers
//!
//! Provides HTTP endpoints for user authentication and profile management.
//!
//! Author: hephaex@gmail.com

use crate::audit::{audit_log, extract_ip_address, extract_user_agent, AuditEvent};
use crate::auth::{
    AuthService, AuthenticatedUser, LoginRequest, LogoutRequest, RefreshRequest, RegisterRequest,
};
use crate::error::AppError;
use crate::state::AppState;
use axum::{
    body::Body,
    extract::{FromRequest, Request, State},
    http::HeaderMap,
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
    headers: HeaderMap,
    Json(request): Json<RegisterRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Extract headers
    let ip_address = extract_ip_address(&headers);
    let user_agent = extract_user_agent(&headers);

    let email = request.email.clone();
    let auth_service = AuthService::new(state.db_pool.clone());

    // Attempt registration
    let result = auth_service.register(request).await;

    match result {
        Ok(user) => {
            // Log successful registration
            audit_log(&AuditEvent::RegistrationSuccess {
                user_id: uuid::Uuid::parse_str(&user.id).unwrap_or_else(|_| uuid::Uuid::nil()),
                email: user.email.clone(),
                role: user.role.clone(),
                ip_address: ip_address.clone(),
                user_agent: user_agent.clone(),
            });

            let response = RegisterResponse {
                user_id: user.id.clone(),
                email: user.email.clone(),
                name: user.name.clone(),
                role: user.role.clone(),
                message: "Registration successful".to_string(),
            };

            Ok((axum::http::StatusCode::CREATED, Json(response)))
        }
        Err(e) => {
            // Log failed registration
            let error_msg = format!("{e:?}");
            audit_log(&AuditEvent::RegistrationFailure {
                email,
                reason: error_msg,
                ip_address,
                user_agent,
            });

            Err(e)
        }
    }
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
    headers: HeaderMap,
    Json(request): Json<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Extract headers
    let ip_address = extract_ip_address(&headers);
    let user_agent = extract_user_agent(&headers);

    let email = request.email.clone();
    let auth_service = AuthService::new(state.db_pool.clone());

    // Attempt login
    let result = auth_service.login(request).await;

    match result {
        Ok(response) => {
            // Log successful login
            let user_id =
                uuid::Uuid::parse_str(&response.user.id).unwrap_or_else(|_| uuid::Uuid::nil());
            audit_log(&AuditEvent::LoginSuccess {
                user_id,
                email: response.user.email.clone(),
                ip_address,
                user_agent,
            });

            Ok(Json(response))
        }
        Err(e) => {
            // Determine failure reason and account status
            let (reason, account_locked) = match &e {
                AppError::Forbidden(msg) if msg.contains("locked") => (msg.clone(), true),
                AppError::Unauthorized => ("Invalid credentials".to_string(), false),
                _ => (format!("{e:?}"), false),
            };

            // Log failed login
            audit_log(&AuditEvent::LoginFailure {
                email,
                reason: reason.clone(),
                ip_address,
                user_agent,
                failed_attempts: None, // Could be enhanced to include this info
                account_locked,
            });

            Err(e)
        }
    }
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
    headers: HeaderMap,
    Json(request): Json<RefreshRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Extract headers
    let ip_address = extract_ip_address(&headers);
    let user_agent = extract_user_agent(&headers);

    let auth_service = AuthService::new(state.db_pool.clone());
    let result = auth_service.refresh(request).await;

    match result {
        Ok(response) => {
            // Log token refresh
            let user_id =
                uuid::Uuid::parse_str(&response.user.id).unwrap_or_else(|_| uuid::Uuid::nil());
            audit_log(&AuditEvent::TokenRefresh {
                user_id,
                email: response.user.email.clone(),
                ip_address,
                user_agent,
            });

            Ok(Json(response))
        }
        Err(e) => {
            // Log invalid token attempt
            audit_log(&AuditEvent::InvalidToken {
                ip_address,
                user_agent,
                reason: "Invalid or expired refresh token".to_string(),
            });

            Err(e)
        }
    }
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
    headers: HeaderMap,
    Json(request): Json<LogoutRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Extract headers
    let ip_address = extract_ip_address(&headers);

    let logout_all = request.logout_all_devices.unwrap_or(false);
    let auth_service = AuthService::new(state.db_pool.clone());

    auth_service
        .logout(user.user_id, &user.jti, request)
        .await?;

    // Log logout
    audit_log(&AuditEvent::Logout {
        user_id: user.user_id,
        email: user.email.clone(),
        ip_address,
        logout_all_devices: logout_all,
    });

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
