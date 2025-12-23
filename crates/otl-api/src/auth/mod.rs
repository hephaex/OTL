//! Authentication and authorization module
//!
//! This module provides JWT-based authentication with the following components:
//! - Token generation and validation
//! - Password hashing with Argon2
//! - Middleware for request authentication
//! - Authentication service for user management
//! - Database models for users and tokens
//! - Repository layer for SurrealDB operations

pub mod jwt;
pub mod middleware;
pub mod models;
pub mod password;
pub mod repository;
pub mod service;

pub use jwt::{generate_access_token, validate_access_token, Claims, JwtConfig};
pub use middleware::{
    auth_middleware, clear_blacklist, is_token_revoked, optional_auth_middleware, revoke_token,
    AuthError, AuthenticatedUser,
};
pub use models::{
    CreateUserRequest, RefreshToken, TokenBlacklist, UpdateUserRequest, User, UserPublic, UserRole,
};
pub use password::{hash_password, validate_password_strength, verify_password};
pub use repository::{
    RefreshTokenRepository, RepositoryError, TokenBlacklistRepository, UserRepository,
};
pub use service::{
    AuthResponse, AuthService, LoginRequest, LogoutRequest, RefreshRequest, RegisterRequest,
    UserInfo,
};
