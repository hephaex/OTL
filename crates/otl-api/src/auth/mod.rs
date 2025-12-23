//! Authentication and authorization module
//!
//! This module provides JWT-based authentication with the following components:
//! - Token generation and validation
//! - Password hashing with Argon2
//! - Middleware for request authentication

pub mod jwt;
pub mod password;
pub mod middleware;

pub use jwt::{generate_access_token, validate_access_token, Claims};
pub use password::{hash_password, verify_password};
pub use middleware::auth_middleware;
