//! Authentication repository for SurrealDB operations
//!
//! This module provides database access layer for authentication entities:
//! - User CRUD operations
//! - Refresh token management
//! - Token blacklist operations
//!
//! Uses SurrealDB for persistent storage with async/await pattern.

use super::models::{RefreshToken, TokenBlacklist, User, UserRole};
use chrono::{DateTime, Duration, Utc};
use otl_graph::SurrealDbStore;
use std::sync::Arc;
use thiserror::Error;

/// Repository errors
#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("User not found")]
    UserNotFound,

    #[error("Email already exists")]
    EmailAlreadyExists,

    #[error("Token not found")]
    TokenNotFound,

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Invalid record ID format")]
    InvalidRecordId,
}

/// User repository for SurrealDB operations
pub struct UserRepository {
    db: Arc<SurrealDbStore>,
}

impl UserRepository {
    /// Create a new user repository
    pub fn new(db: Arc<SurrealDbStore>) -> Self {
        Self { db }
    }

    /// Create a new user in the database
    ///
    /// # Arguments
    ///
    /// * `user` - User to create
    ///
    /// # Returns
    ///
    /// * `Ok(User)` - Created user with ID assigned
    /// * `Err(RepositoryError)` - If creation fails
    pub async fn create_user(&self, user: User) -> Result<User, RepositoryError> {
        // Check if email already exists
        if self.find_by_email(&user.email).await.is_ok() {
            return Err(RepositoryError::EmailAlreadyExists);
        }

        // Create user in database
        let query = r#"
            CREATE users CONTENT {
                email: $email,
                password_hash: $password_hash,
                name: $name,
                role: $role,
                department: $department,
                is_active: $is_active,
                email_verified: $email_verified,
                failed_login_attempts: $failed_login_attempts,
                locked_until: $locked_until,
                last_login: $last_login,
                password_changed_at: $password_changed_at,
                created_at: $created_at,
                updated_at: $updated_at
            }
        "#;

        let mut result = self
            .db
            .client()
            .query(query)
            .bind(("email", user.email.clone()))
            .bind(("password_hash", user.password_hash.clone()))
            .bind(("name", user.name.clone()))
            .bind(("role", user.role.as_str().to_string()))
            .bind(("department", user.department.clone()))
            .bind(("is_active", user.is_active))
            .bind(("email_verified", user.email_verified))
            .bind(("failed_login_attempts", user.failed_login_attempts))
            .bind(("locked_until", user.locked_until))
            .bind(("last_login", user.last_login))
            .bind(("password_changed_at", user.password_changed_at))
            .bind(("created_at", user.created_at))
            .bind(("updated_at", user.updated_at))
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        let created: Option<User> = result
            .take(0)
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        created.ok_or(RepositoryError::DatabaseError(
            "Failed to create user".to_string(),
        ))
    }

    /// Find user by email
    ///
    /// # Arguments
    ///
    /// * `email` - User's email address
    ///
    /// # Returns
    ///
    /// * `Ok(User)` - User if found
    /// * `Err(RepositoryError::UserNotFound)` - If user doesn't exist
    pub async fn find_by_email(&self, email: &str) -> Result<User, RepositoryError> {
        let query = "SELECT * FROM users WHERE email = $email LIMIT 1";

        let mut result = self
            .db
            .client()
            .query(query)
            .bind(("email", email.to_string()))
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        let users: Vec<User> = result
            .take(0)
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        users
            .into_iter()
            .next()
            .ok_or(RepositoryError::UserNotFound)
    }

    /// Find user by ID
    ///
    /// # Arguments
    ///
    /// * `user_id` - User ID (record ID format: "users:uuid")
    ///
    /// # Returns
    ///
    /// * `Ok(User)` - User if found
    /// * `Err(RepositoryError::UserNotFound)` - If user doesn't exist
    pub async fn find_by_id(&self, user_id: &str) -> Result<User, RepositoryError> {
        let query = "SELECT * FROM $user_id";

        let mut result = self
            .db
            .client()
            .query(query)
            .bind(("user_id", user_id.to_string()))
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        let user: Option<User> = result
            .take(0)
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        user.ok_or(RepositoryError::UserNotFound)
    }

    /// Update user's last login timestamp
    ///
    /// # Arguments
    ///
    /// * `user_id` - User ID
    /// * `timestamp` - Login timestamp
    pub async fn update_last_login(
        &self,
        user_id: &str,
        timestamp: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        let query = "UPDATE $user_id SET last_login = $timestamp, updated_at = $timestamp";

        self.db
            .client()
            .query(query)
            .bind(("user_id", user_id.to_string()))
            .bind(("timestamp", timestamp))
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Increment failed login attempts
    ///
    /// # Arguments
    ///
    /// * `user_id` - User ID
    pub async fn increment_failed_attempts(&self, user_id: &str) -> Result<i32, RepositoryError> {
        let query = r#"
            UPDATE $user_id SET
                failed_login_attempts = failed_login_attempts + 1,
                updated_at = time::now()
            RETURN failed_login_attempts
        "#;

        let mut result = self
            .db
            .client()
            .query(query)
            .bind(("user_id", user_id.to_string()))
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        let attempts: Option<i32> = result
            .take(0)
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        attempts.ok_or(RepositoryError::UserNotFound)
    }

    /// Reset failed login attempts
    ///
    /// # Arguments
    ///
    /// * `user_id` - User ID
    pub async fn reset_failed_attempts(&self, user_id: &str) -> Result<(), RepositoryError> {
        let query = r#"
            UPDATE $user_id SET
                failed_login_attempts = 0,
                locked_until = NONE,
                updated_at = time::now()
        "#;

        self.db
            .client()
            .query(query)
            .bind(("user_id", user_id.to_string()))
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Lock user account for specified duration
    ///
    /// # Arguments
    ///
    /// * `user_id` - User ID
    /// * `duration` - Lock duration
    pub async fn lock_account(
        &self,
        user_id: &str,
        duration: Duration,
    ) -> Result<(), RepositoryError> {
        let locked_until = Utc::now() + duration;
        let query = "UPDATE $user_id SET locked_until = $locked_until, updated_at = time::now()";

        self.db
            .client()
            .query(query)
            .bind(("user_id", user_id.to_string()))
            .bind(("locked_until", locked_until))
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Update user password
    ///
    /// # Arguments
    ///
    /// * `user_id` - User ID
    /// * `new_password_hash` - New hashed password
    pub async fn update_password(
        &self,
        user_id: &str,
        new_password_hash: &str,
    ) -> Result<(), RepositoryError> {
        let now = Utc::now();
        let query = r#"
            UPDATE $user_id SET
                password_hash = $password_hash,
                password_changed_at = $now,
                updated_at = $now
        "#;

        self.db
            .client()
            .query(query)
            .bind(("user_id", user_id.to_string()))
            .bind(("password_hash", new_password_hash.to_string()))
            .bind(("now", now))
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Update user profile
    ///
    /// # Arguments
    ///
    /// * `user_id` - User ID
    /// * `name` - New name (optional)
    /// * `email` - New email (optional)
    /// * `department` - New department (optional)
    /// * `role` - New role (optional)
    /// * `is_active` - New active status (optional)
    pub async fn update_user(
        &self,
        user_id: &str,
        name: Option<&str>,
        email: Option<&str>,
        department: Option<&str>,
        role: Option<UserRole>,
        is_active: Option<bool>,
    ) -> Result<User, RepositoryError> {
        // Build dynamic update query
        let mut updates = vec!["updated_at = time::now()"];

        if name.is_some() {
            updates.push("name = $name");
        }
        if email.is_some() {
            updates.push("email = $email");
        }
        if department.is_some() {
            updates.push("department = $department");
        }
        if role.is_some() {
            updates.push("role = $role");
        }
        if is_active.is_some() {
            updates.push("is_active = $is_active");
        }

        let query = format!("UPDATE $user_id SET {}", updates.join(", "));

        // Build query with all bindings
        let mut query_builder = self
            .db
            .client()
            .query(&query)
            .bind(("user_id", user_id.to_string()));

        if let Some(n) = name {
            query_builder = query_builder.bind(("name", n.to_string()));
        }
        if let Some(e) = email {
            query_builder = query_builder.bind(("email", e.to_string()));
        }
        if let Some(d) = department {
            query_builder = query_builder.bind(("department", d.to_string()));
        }
        if let Some(r) = role {
            query_builder = query_builder.bind(("role", r.as_str().to_string()));
        }
        if let Some(a) = is_active {
            query_builder = query_builder.bind(("is_active", a));
        }

        let mut result = query_builder
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        let user: Option<User> = result
            .take(0)
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        user.ok_or(RepositoryError::UserNotFound)
    }

    /// List all users with optional filtering
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of users to return
    /// * `offset` - Number of users to skip
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<User>)` - List of users
    pub async fn list_users(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<User>, RepositoryError> {
        let query = "SELECT * FROM users ORDER BY created_at DESC LIMIT $limit START $offset";

        let mut result = self
            .db
            .client()
            .query(query)
            .bind(("limit", limit))
            .bind(("offset", offset))
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        let users: Vec<User> = result
            .take(0)
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(users)
    }

    /// Delete user (soft delete by setting is_active = false)
    ///
    /// # Arguments
    ///
    /// * `user_id` - User ID
    pub async fn delete_user(&self, user_id: &str) -> Result<(), RepositoryError> {
        let query = "UPDATE $user_id SET is_active = false, updated_at = time::now()";

        self.db
            .client()
            .query(query)
            .bind(("user_id", user_id.to_string()))
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}

/// Refresh token repository for SurrealDB operations
pub struct RefreshTokenRepository {
    db: Arc<SurrealDbStore>,
}

impl RefreshTokenRepository {
    /// Create a new refresh token repository
    pub fn new(db: Arc<SurrealDbStore>) -> Self {
        Self { db }
    }

    /// Create a new refresh token
    ///
    /// # Arguments
    ///
    /// * `token` - Refresh token to create
    ///
    /// # Returns
    ///
    /// * `Ok(RefreshToken)` - Created token with ID assigned
    pub async fn create_token(&self, token: RefreshToken) -> Result<RefreshToken, RepositoryError> {
        let query = r#"
            CREATE refresh_tokens CONTENT {
                user_id: $user_id,
                token_hash: $token_hash,
                device_info: $device_info,
                ip_address: $ip_address,
                expires_at: $expires_at,
                created_at: $created_at,
                revoked_at: $revoked_at
            }
        "#;

        let mut result = self
            .db
            .client()
            .query(query)
            .bind(("user_id", token.user_id.clone()))
            .bind(("token_hash", token.token_hash.clone()))
            .bind(("device_info", token.device_info.clone()))
            .bind(("ip_address", token.ip_address.clone()))
            .bind(("expires_at", token.expires_at))
            .bind(("created_at", token.created_at))
            .bind(("revoked_at", token.revoked_at))
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        let created: Option<RefreshToken> = result
            .take(0)
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        created.ok_or(RepositoryError::DatabaseError(
            "Failed to create refresh token".to_string(),
        ))
    }

    /// Find refresh token by hash
    ///
    /// # Arguments
    ///
    /// * `token_hash` - Hashed token value
    ///
    /// # Returns
    ///
    /// * `Ok(RefreshToken)` - Token if found
    /// * `Err(RepositoryError::TokenNotFound)` - If token doesn't exist
    pub async fn find_by_hash(&self, token_hash: &str) -> Result<RefreshToken, RepositoryError> {
        let query = "SELECT * FROM refresh_tokens WHERE token_hash = $token_hash LIMIT 1";

        let mut result = self
            .db
            .client()
            .query(query)
            .bind(("token_hash", token_hash.to_string()))
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        let tokens: Vec<RefreshToken> = result
            .take(0)
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        tokens
            .into_iter()
            .next()
            .ok_or(RepositoryError::TokenNotFound)
    }

    /// Revoke a refresh token
    ///
    /// # Arguments
    ///
    /// * `token_id` - Token ID
    pub async fn revoke_token(&self, token_id: &str) -> Result<(), RepositoryError> {
        let query = "UPDATE $token_id SET revoked_at = time::now()";

        self.db
            .client()
            .query(query)
            .bind(("token_id", token_id.to_string()))
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Revoke all tokens for a user
    ///
    /// # Arguments
    ///
    /// * `user_id` - User ID
    pub async fn revoke_all_user_tokens(&self, user_id: &str) -> Result<(), RepositoryError> {
        let query = "UPDATE refresh_tokens SET revoked_at = time::now() WHERE user_id = $user_id";

        self.db
            .client()
            .query(query)
            .bind(("user_id", user_id.to_string()))
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Delete expired tokens (cleanup)
    pub async fn delete_expired_tokens(&self) -> Result<usize, RepositoryError> {
        let query = "DELETE refresh_tokens WHERE expires_at < time::now()";

        let _result = self
            .db
            .client()
            .query(query)
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        // SurrealDB doesn't return count in DELETE, so we return 0
        Ok(0)
    }
}

/// Token blacklist repository for SurrealDB operations
pub struct TokenBlacklistRepository {
    db: Arc<SurrealDbStore>,
}

impl TokenBlacklistRepository {
    /// Create a new token blacklist repository
    pub fn new(db: Arc<SurrealDbStore>) -> Self {
        Self { db }
    }

    /// Add a token to the blacklist
    ///
    /// # Arguments
    ///
    /// * `entry` - Blacklist entry to create
    pub async fn blacklist_token(&self, entry: TokenBlacklist) -> Result<(), RepositoryError> {
        let query = r#"
            CREATE token_blacklist CONTENT {
                token_jti: $token_jti,
                expires_at: $expires_at,
                blacklisted_at: $blacklisted_at
            }
        "#;

        self.db
            .client()
            .query(query)
            .bind(("token_jti", entry.token_jti.clone()))
            .bind(("expires_at", entry.expires_at))
            .bind(("blacklisted_at", entry.blacklisted_at))
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Check if a token is blacklisted
    ///
    /// # Arguments
    ///
    /// * `token_jti` - JWT ID from token claims
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Token is blacklisted
    /// * `Ok(false)` - Token is not blacklisted
    pub async fn is_blacklisted(&self, token_jti: &str) -> Result<bool, RepositoryError> {
        let query = "SELECT * FROM token_blacklist WHERE token_jti = $token_jti LIMIT 1";

        let mut result = self
            .db
            .client()
            .query(query)
            .bind(("token_jti", token_jti.to_string()))
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        let entries: Vec<TokenBlacklist> = result
            .take(0)
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(!entries.is_empty())
    }

    /// Delete expired blacklist entries (cleanup)
    pub async fn delete_expired_entries(&self) -> Result<usize, RepositoryError> {
        let query = "DELETE token_blacklist WHERE expires_at < time::now()";

        let _result = self
            .db
            .client()
            .query(query)
            .await
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    // Note: These tests require a running SurrealDB instance
    // They are integration tests and should be run with a test database

    #[test]
    fn test_repository_creation() {
        // This is a placeholder test
        // Real tests would require database setup
        assert!(true);
    }
}
