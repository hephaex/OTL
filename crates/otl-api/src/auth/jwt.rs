//! JWT token generation and validation
//!
//! Implements JWT-based authentication with HMAC-SHA256 signing.
//! Access tokens contain user claims and have a configurable expiration time.

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use uuid::Uuid;

/// JWT Claims structure containing user information
///
/// These claims are embedded in the access token and extracted during validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Token issuer (always "otl-api")
    pub iss: String,
    /// Subject - user ID
    pub sub: String,
    /// JWT ID - unique token identifier for blacklisting
    pub jti: String,
    /// Issued at timestamp (Unix epoch)
    pub iat: u64,
    /// Expiration timestamp (Unix epoch)
    pub exp: u64,
    /// User's display name
    pub name: String,
    /// User's email address
    pub email: String,
    /// User's role (admin, editor, viewer)
    pub role: String,
    /// User's department (optional, for ACL)
    pub department: Option<String>,
}

/// JWT token generation and validation errors
#[derive(Debug, Error)]
pub enum JwtError {
    #[error("Failed to encode JWT: {0}")]
    EncodingError(#[from] jsonwebtoken::errors::Error),

    #[error("Invalid token format")]
    InvalidToken,

    #[error("Token has expired")]
    ExpiredToken,

    #[error("Invalid token signature")]
    InvalidSignature,

    #[error("System time error: {0}")]
    SystemTimeError(#[from] std::time::SystemTimeError),
}

/// JWT Configuration
///
/// Contains settings for token generation and validation
#[derive(Debug, Clone)]
pub struct JwtConfig {
    /// Secret key for HMAC signing (must be at least 256 bits)
    pub secret: String,
    /// Access token expiration time in seconds (default: 3600 = 1 hour)
    pub access_expiration_secs: u64,
    /// Token issuer identifier
    pub issuer: String,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: "development-secret-key-change-in-production".to_string(),
            access_expiration_secs: 3600, // 1 hour
            issuer: "otl-api".to_string(),
        }
    }
}

impl JwtConfig {
    /// Create a new JWT configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "development-secret-key-change-in-production".to_string()),
            access_expiration_secs: std::env::var("JWT_ACCESS_EXPIRATION_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3600),
            issuer: std::env::var("JWT_ISSUER").unwrap_or_else(|_| "otl-api".to_string()),
        }
    }
}

/// Generate a JWT access token for authenticated user
///
/// # Arguments
///
/// * `config` - JWT configuration containing secret and expiration settings
/// * `user_id` - Unique user identifier (UUID)
/// * `name` - User's display name
/// * `email` - User's email address
/// * `role` - User's role (admin, editor, viewer)
/// * `department` - User's department (optional, for ACL filtering)
///
/// # Returns
///
/// * `Ok(String)` - Encoded JWT token
/// * `Err(JwtError)` - If token generation fails
///
/// # Example
///
/// ```no_run
/// use otl_api::auth::jwt::{generate_access_token, JwtConfig};
/// use uuid::Uuid;
///
/// let config = JwtConfig::default();
/// let token = generate_access_token(
///     &config,
///     Uuid::new_v4(),
///     "John Doe",
///     "john@example.com",
///     "editor",
///     Some("Engineering"),
/// ).expect("Failed to generate token");
/// ```
pub fn generate_access_token(
    config: &JwtConfig,
    user_id: Uuid,
    name: &str,
    email: &str,
    role: &str,
    department: Option<&str>,
) -> Result<String, JwtError> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    let claims = Claims {
        iss: config.issuer.clone(),
        sub: user_id.to_string(),
        jti: Uuid::new_v4().to_string(), // Unique token ID for blacklisting
        iat: now,
        exp: now + config.access_expiration_secs,
        name: name.to_string(),
        email: email.to_string(),
        role: role.to_string(),
        department: department.map(|d| d.to_string()),
    };

    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(config.secret.as_bytes()),
    )?;

    Ok(token)
}

/// Validate a JWT access token and extract claims
///
/// # Arguments
///
/// * `config` - JWT configuration containing secret for validation
/// * `token` - The JWT token string to validate
///
/// # Returns
///
/// * `Ok(Claims)` - Decoded and validated claims
/// * `Err(JwtError)` - If validation fails (expired, invalid signature, etc.)
///
/// # Example
///
/// ```no_run
/// use otl_api::auth::jwt::{validate_access_token, JwtConfig};
///
/// let config = JwtConfig::default();
/// let claims = validate_access_token(&config, "eyJhbGciOiJIUzI1NiIs...")
///     .expect("Invalid token");
/// println!("User: {} ({})", claims.name, claims.role);
/// ```
pub fn validate_access_token(config: &JwtConfig, token: &str) -> Result<Claims, JwtError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_issuer(&[&config.issuer]);

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.secret.as_bytes()),
        &validation,
    )
    .map_err(|e| match e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => JwtError::ExpiredToken,
        jsonwebtoken::errors::ErrorKind::InvalidSignature => JwtError::InvalidSignature,
        _ => JwtError::InvalidToken,
    })?;

    Ok(token_data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_validate_token() {
        let config = JwtConfig::default();
        let user_id = Uuid::new_v4();

        let token = generate_access_token(
            &config,
            user_id,
            "Test User",
            "test@example.com",
            "editor",
            Some("Engineering"),
        )
        .expect("Failed to generate token");

        let claims = validate_access_token(&config, &token).expect("Failed to validate token");

        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.name, "Test User");
        assert_eq!(claims.email, "test@example.com");
        assert_eq!(claims.role, "editor");
        assert_eq!(claims.department, Some("Engineering".to_string()));
        assert_eq!(claims.iss, "otl-api");
    }

    #[test]
    fn test_invalid_token() {
        let config = JwtConfig::default();
        let result = validate_access_token(&config, "invalid.token.here");
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_secret() {
        let config1 = JwtConfig {
            secret: "secret1".to_string(),
            ..Default::default()
        };
        let config2 = JwtConfig {
            secret: "secret2".to_string(),
            ..Default::default()
        };

        let token = generate_access_token(
            &config1,
            Uuid::new_v4(),
            "Test",
            "test@example.com",
            "viewer",
            None,
        )
        .unwrap();

        let result = validate_access_token(&config2, &token);
        assert!(matches!(result, Err(JwtError::InvalidSignature)));
    }

    #[test]
    fn test_expired_token() {
        use jsonwebtoken::{encode, Header, Algorithm, EncodingKey};

        let config = JwtConfig::default();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        // Create a token that expired 1 hour ago
        let claims = Claims {
            iss: config.issuer.clone(),
            sub: Uuid::new_v4().to_string(),
            jti: Uuid::new_v4().to_string(),
            iat: now - 7200, // Issued 2 hours ago
            exp: now - 3600, // Expired 1 hour ago
            name: "Test".to_string(),
            email: "test@example.com".to_string(),
            role: "viewer".to_string(),
            department: None,
        };

        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(config.secret.as_bytes()),
        )
        .unwrap();

        let result = validate_access_token(&config, &token);
        assert!(matches!(result, Err(JwtError::ExpiredToken)));
    }
}
