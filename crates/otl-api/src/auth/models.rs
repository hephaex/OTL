//! Database models for authentication and authorization
//!
//! This module defines the core data structures for the auth system:
//! - User: User account information
//! - RefreshToken: Long-lived tokens for session management
//! - TokenBlacklist: Invalidated access tokens
//!
//! These models map to SurrealDB tables defined in the schema.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// User role enum
///
/// Defines the access level for a user in the system:
/// - Admin: Full system access including user management
/// - Editor: Can read, write, approve, and reject content
/// - Viewer: Read-only access
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    Editor,
    Viewer,
}

impl UserRole {
    /// Convert role to string representation
    pub fn as_str(&self) -> &str {
        match self {
            UserRole::Admin => "admin",
            UserRole::Editor => "editor",
            UserRole::Viewer => "viewer",
        }
    }

    /// Parse role from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "admin" => Some(UserRole::Admin),
            "editor" => Some(UserRole::Editor),
            "viewer" => Some(UserRole::Viewer),
            _ => None,
        }
    }
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// User account model
///
/// Represents a user in the authentication system with their credentials,
/// profile information, and security settings.
///
/// This maps to the `users` table in SurrealDB.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct User {
    /// Unique user identifier (SurrealDB record ID: users:uuid)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// User's email address (unique, used for login)
    pub email: String,

    /// Hashed password (Argon2id)
    /// This field is never serialized in API responses
    #[serde(skip_serializing)]
    pub password_hash: String,

    /// User's display name
    pub name: String,

    /// User's role (admin, editor, viewer)
    pub role: UserRole,

    /// User's department (optional, used for ACL filtering)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub department: Option<String>,

    /// Whether the account is active
    #[serde(default = "default_true")]
    pub is_active: bool,

    /// Whether the email has been verified
    #[serde(default)]
    pub email_verified: bool,

    /// Number of consecutive failed login attempts
    #[serde(default)]
    pub failed_login_attempts: i32,

    /// Account locked until this time (if locked)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked_until: Option<DateTime<Utc>>,

    /// Last successful login timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_login: Option<DateTime<Utc>>,

    /// When the password was last changed
    pub password_changed_at: DateTime<Utc>,

    /// Account creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

fn default_true() -> bool {
    true
}

impl User {
    /// Create a new user with the given credentials
    ///
    /// # Arguments
    ///
    /// * `email` - User's email address
    /// * `password_hash` - Hashed password (use password::hash_password)
    /// * `name` - User's display name
    /// * `role` - User's role
    /// * `department` - User's department (optional)
    pub fn new(
        email: String,
        password_hash: String,
        name: String,
        role: UserRole,
        department: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: None, // Set by database
            email,
            password_hash,
            name,
            role,
            department,
            is_active: true,
            email_verified: false,
            failed_login_attempts: 0,
            locked_until: None,
            last_login: None,
            password_changed_at: now,
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if the account is currently locked
    pub fn is_locked(&self) -> bool {
        if let Some(locked_until) = self.locked_until {
            Utc::now() < locked_until
        } else {
            false
        }
    }

    /// Convert user to public representation (without sensitive fields)
    pub fn to_public(&self) -> UserPublic {
        UserPublic {
            id: self.id.clone(),
            email: self.email.clone(),
            name: self.name.clone(),
            role: self.role.clone(),
            department: self.department.clone(),
            is_active: self.is_active,
            last_login: self.last_login,
            created_at: self.created_at,
        }
    }
}

/// Public user representation (safe for API responses)
///
/// This excludes sensitive fields like password_hash and security settings.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserPublic {
    pub id: Option<String>,
    pub email: String,
    pub name: String,
    pub role: UserRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub department: Option<String>,
    pub is_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_login: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Refresh token model
///
/// Represents a long-lived token used to obtain new access tokens.
/// These are stored hashed in the database for security.
///
/// This maps to the `refresh_tokens` table in SurrealDB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshToken {
    /// Unique token identifier (SurrealDB record ID: refresh_tokens:uuid)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// User ID this token belongs to (SurrealDB record: users:uuid)
    pub user_id: String,

    /// Hashed token value (SHA-256)
    pub token_hash: String,

    /// Device information (user agent, optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_info: Option<String>,

    /// IP address where token was issued (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,

    /// Token expiration time
    pub expires_at: DateTime<Utc>,

    /// Token creation time
    pub created_at: DateTime<Utc>,

    /// Token revocation time (if revoked)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<DateTime<Utc>>,
}

impl RefreshToken {
    /// Create a new refresh token
    ///
    /// # Arguments
    ///
    /// * `user_id` - User ID this token belongs to
    /// * `token_hash` - Hashed token value
    /// * `expires_at` - Expiration time
    /// * `device_info` - Device information (optional)
    /// * `ip_address` - IP address (optional)
    pub fn new(
        user_id: String,
        token_hash: String,
        expires_at: DateTime<Utc>,
        device_info: Option<String>,
        ip_address: Option<String>,
    ) -> Self {
        Self {
            id: None, // Set by database
            user_id,
            token_hash,
            device_info,
            ip_address,
            expires_at,
            created_at: Utc::now(),
            revoked_at: None,
        }
    }

    /// Check if the token is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Check if the token is revoked
    pub fn is_revoked(&self) -> bool {
        self.revoked_at.is_some()
    }

    /// Check if the token is valid (not expired and not revoked)
    pub fn is_valid(&self) -> bool {
        !self.is_expired() && !self.is_revoked()
    }
}

/// Token blacklist entry
///
/// Represents an invalidated access token (used for logout before expiry).
/// Tokens are identified by their JTI (JWT ID) claim.
///
/// This maps to the `token_blacklist` table in SurrealDB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBlacklist {
    /// Unique identifier (SurrealDB record ID: token_blacklist:uuid)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// JWT ID (jti claim) of the blacklisted token
    pub token_jti: String,

    /// Original token expiration time (for cleanup)
    pub expires_at: DateTime<Utc>,

    /// When the token was blacklisted
    pub blacklisted_at: DateTime<Utc>,
}

impl TokenBlacklist {
    /// Create a new blacklist entry
    ///
    /// # Arguments
    ///
    /// * `token_jti` - JWT ID from the token's jti claim
    /// * `expires_at` - Original token expiration time
    pub fn new(token_jti: String, expires_at: DateTime<Utc>) -> Self {
        Self {
            id: None, // Set by database
            token_jti,
            expires_at,
            blacklisted_at: Utc::now(),
        }
    }
}

/// User creation request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateUserRequest {
    pub email: String,
    pub password: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub department: Option<String>,
}

/// User update request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub department: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<UserRole>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_role_conversion() {
        assert_eq!(UserRole::Admin.as_str(), "admin");
        assert_eq!(UserRole::Editor.as_str(), "editor");
        assert_eq!(UserRole::Viewer.as_str(), "viewer");

        assert_eq!(UserRole::from_str("admin"), Some(UserRole::Admin));
        assert_eq!(UserRole::from_str("EDITOR"), Some(UserRole::Editor));
        assert_eq!(UserRole::from_str("invalid"), None);
    }

    #[test]
    fn test_user_creation() {
        let user = User::new(
            "test@example.com".to_string(),
            "hashed_password".to_string(),
            "Test User".to_string(),
            UserRole::Editor,
            Some("Engineering".to_string()),
        );

        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.name, "Test User");
        assert_eq!(user.role, UserRole::Editor);
        assert_eq!(user.department, Some("Engineering".to_string()));
        assert!(user.is_active);
        assert!(!user.email_verified);
        assert_eq!(user.failed_login_attempts, 0);
        assert!(!user.is_locked());
    }

    #[test]
    fn test_user_is_locked() {
        let mut user = User::new(
            "test@example.com".to_string(),
            "hash".to_string(),
            "Test".to_string(),
            UserRole::Viewer,
            None,
        );

        // Not locked initially
        assert!(!user.is_locked());

        // Lock until future time
        user.locked_until = Some(Utc::now() + chrono::Duration::hours(1));
        assert!(user.is_locked());

        // Lock expired
        user.locked_until = Some(Utc::now() - chrono::Duration::hours(1));
        assert!(!user.is_locked());
    }

    #[test]
    fn test_refresh_token_validation() {
        let now = Utc::now();
        let mut token = RefreshToken::new(
            "users:123".to_string(),
            "hashed_token".to_string(),
            now + chrono::Duration::days(7),
            None,
            None,
        );

        // Valid token
        assert!(token.is_valid());
        assert!(!token.is_expired());
        assert!(!token.is_revoked());

        // Expired token
        token.expires_at = now - chrono::Duration::days(1);
        assert!(!token.is_valid());
        assert!(token.is_expired());

        // Revoked token
        token.expires_at = now + chrono::Duration::days(7);
        token.revoked_at = Some(now);
        assert!(!token.is_valid());
        assert!(token.is_revoked());
    }

    #[test]
    fn test_user_to_public() {
        let user = User::new(
            "test@example.com".to_string(),
            "secret_hash".to_string(),
            "Test User".to_string(),
            UserRole::Admin,
            Some("IT".to_string()),
        );

        let public = user.to_public();

        assert_eq!(public.email, user.email);
        assert_eq!(public.name, user.name);
        assert_eq!(public.role, user.role);
        assert_eq!(public.department, user.department);

        // Verify password_hash is not included
        let json = serde_json::to_string(&public).unwrap();
        assert!(!json.contains("password_hash"));
    }
}
