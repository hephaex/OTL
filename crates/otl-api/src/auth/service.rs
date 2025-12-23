//! Authentication service layer
//!
//! Provides business logic for user registration, login, token refresh, and logout.
//! Integrates with database for user storage and session management.

use super::jwt::{generate_access_token, JwtConfig};
use super::password::{hash_password, validate_password_strength, verify_password};
use crate::error::AppError;
use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

/// User registration request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub name: String,
    pub department: Option<String>,
}

/// User login request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Token refresh request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

/// Logout request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LogoutRequest {
    pub refresh_token: Option<String>,
    pub logout_all_devices: Option<bool>,
}

/// Authentication response with tokens
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub user: UserInfo,
}

/// User information response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub name: String,
    pub role: String,
    pub department: Option<String>,
    pub is_active: bool,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
}

/// Internal user record from database
#[derive(Debug, Clone, sqlx::FromRow)]
struct UserRecord {
    id: Uuid,
    email: String,
    password_hash: String,
    name: String,
    role: String,
    department: Option<String>,
    is_active: bool,
    email_verified: bool,
    failed_login_attempts: i32,
    locked_until: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

/// Refresh token record from database
#[derive(Debug, Clone, sqlx::FromRow)]
struct RefreshTokenRecord {
    id: Uuid,
    user_id: Uuid,
    token_hash: String,
    expires_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
}

/// Authentication service
pub struct AuthService {
    db_pool: PgPool,
    jwt_config: JwtConfig,
    refresh_token_expiry_days: i64,
    max_failed_attempts: i32,
    lockout_duration_mins: i64,
}

impl AuthService {
    /// Create a new authentication service
    pub fn new(db_pool: PgPool) -> Self {
        let jwt_config = JwtConfig::from_env();
        let refresh_token_expiry_days = std::env::var("JWT_REFRESH_EXPIRATION_DAYS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(7);
        let max_failed_attempts = std::env::var("AUTH_MAX_LOGIN_ATTEMPTS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);
        let lockout_duration_mins = std::env::var("AUTH_LOCKOUT_DURATION_MINS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(15);

        Self {
            db_pool,
            jwt_config,
            refresh_token_expiry_days,
            max_failed_attempts,
            lockout_duration_mins,
        }
    }

    /// Register a new user
    ///
    /// # Arguments
    ///
    /// * `request` - Registration details
    ///
    /// # Returns
    ///
    /// * `Ok(UserInfo)` - Newly created user
    /// * `Err(AppError)` - If registration fails (duplicate email, weak password, etc.)
    pub async fn register(&self, request: RegisterRequest) -> Result<UserInfo, AppError> {
        // Validate email format
        if !request.email.contains('@') {
            return Err(AppError::BadRequest("Invalid email format".to_string()));
        }

        // Validate password strength
        validate_password_strength(&request.password)
            .map_err(|e| AppError::BadRequest(format!("Password validation failed: {e}")))?;

        // Check if email already exists
        let existing = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE email = $1")
            .bind(&request.email)
            .fetch_one(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to check existing user: {e}")))?;

        if existing > 0 {
            return Err(AppError::BadRequest("Email already registered".to_string()));
        }

        // Hash password
        let password_hash = hash_password(&request.password)
            .map_err(|e| AppError::Internal(format!("Failed to hash password: {e}")))?;

        // Insert new user (default role is 'viewer')
        let user = sqlx::query_as::<_, UserRecord>(
            r#"
            INSERT INTO users (id, email, password_hash, name, role, department, is_active, email_verified, created_at, updated_at, password_changed_at)
            VALUES ($1, $2, $3, $4, 'viewer', $5, true, false, NOW(), NOW(), NOW())
            RETURNING id, email, password_hash, name, role, department, is_active, email_verified, failed_login_attempts, locked_until, created_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(&request.email)
        .bind(&password_hash)
        .bind(&request.name)
        .bind(&request.department)
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to create user: {e}")))?;

        Ok(UserInfo {
            id: user.id.to_string(),
            email: user.email,
            name: user.name,
            role: user.role,
            department: user.department,
            is_active: user.is_active,
            email_verified: user.email_verified,
            created_at: user.created_at,
        })
    }

    /// Login with email and password
    ///
    /// # Arguments
    ///
    /// * `request` - Login credentials
    ///
    /// # Returns
    ///
    /// * `Ok(AuthResponse)` - Access token, refresh token, and user info
    /// * `Err(AppError)` - If login fails
    pub async fn login(&self, request: LoginRequest) -> Result<AuthResponse, AppError> {
        // Fetch user by email
        let user = sqlx::query_as::<_, UserRecord>(
            "SELECT id, email, password_hash, name, role, department, is_active, email_verified, failed_login_attempts, locked_until, created_at FROM users WHERE email = $1",
        )
        .bind(&request.email)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to fetch user: {e}")))?
        .ok_or_else(|| AppError::Unauthorized)?;

        // Check if account is active
        if !user.is_active {
            return Err(AppError::Forbidden("Account is deactivated".to_string()));
        }

        // Check if account is locked
        if let Some(locked_until) = user.locked_until {
            if locked_until > Utc::now() {
                return Err(AppError::Forbidden(format!(
                    "Account is locked until {}",
                    locked_until
                )));
            }
        }

        // Verify password
        let password_valid = verify_password(&request.password, &user.password_hash)
            .map_err(|e| AppError::Internal(format!("Failed to verify password: {e}")))?;

        if !password_valid {
            // Increment failed login attempts
            let failed_attempts = user.failed_login_attempts + 1;
            let locked_until = if failed_attempts >= self.max_failed_attempts {
                Some(Utc::now() + Duration::minutes(self.lockout_duration_mins))
            } else {
                None
            };

            sqlx::query(
                "UPDATE users SET failed_login_attempts = $1, locked_until = $2 WHERE id = $3",
            )
            .bind(failed_attempts)
            .bind(locked_until)
            .bind(user.id)
            .execute(&self.db_pool)
            .await
            .ok(); // Ignore errors here

            return Err(AppError::Unauthorized);
        }

        // Reset failed login attempts and update last login
        sqlx::query("UPDATE users SET failed_login_attempts = 0, locked_until = NULL, last_login = NOW() WHERE id = $1")
            .bind(user.id)
            .execute(&self.db_pool)
            .await
            .ok(); // Ignore errors here

        // Generate access token
        let access_token = generate_access_token(
            &self.jwt_config,
            user.id,
            &user.name,
            &user.email,
            &user.role,
            user.department.as_deref(),
        )
        .map_err(|e| AppError::Internal(format!("Failed to generate access token: {e}")))?;

        // Generate refresh token
        let refresh_token = self.generate_refresh_token();
        let refresh_token_hash = self.hash_token(&refresh_token);
        let expires_at = Utc::now() + Duration::days(self.refresh_token_expiry_days);

        // Store refresh token
        sqlx::query(
            "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at, created_at) VALUES ($1, $2, $3, $4, NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(user.id)
        .bind(&refresh_token_hash)
        .bind(expires_at)
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to store refresh token: {e}")))?;

        Ok(AuthResponse {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.jwt_config.access_expiration_secs,
            user: UserInfo {
                id: user.id.to_string(),
                email: user.email,
                name: user.name,
                role: user.role,
                department: user.department,
                is_active: user.is_active,
                email_verified: user.email_verified,
                created_at: user.created_at,
            },
        })
    }

    /// Refresh access token using refresh token
    ///
    /// # Arguments
    ///
    /// * `request` - Refresh token
    ///
    /// # Returns
    ///
    /// * `Ok(AuthResponse)` - New access token and rotated refresh token
    /// * `Err(AppError)` - If refresh fails
    pub async fn refresh(&self, request: RefreshRequest) -> Result<AuthResponse, AppError> {
        let token_hash = self.hash_token(&request.refresh_token);

        // Fetch refresh token record
        let token_record = sqlx::query_as::<_, RefreshTokenRecord>(
            "SELECT id, user_id, token_hash, expires_at, revoked_at FROM refresh_tokens WHERE token_hash = $1",
        )
        .bind(&token_hash)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to fetch refresh token: {e}")))?
        .ok_or_else(|| AppError::Unauthorized)?;

        // Check if token is revoked
        if token_record.revoked_at.is_some() {
            return Err(AppError::Unauthorized);
        }

        // Check if token is expired
        if token_record.expires_at < Utc::now() {
            return Err(AppError::Unauthorized);
        }

        // Fetch user
        let user = sqlx::query_as::<_, UserRecord>(
            "SELECT id, email, password_hash, name, role, department, is_active, email_verified, failed_login_attempts, locked_until, created_at FROM users WHERE id = $1",
        )
        .bind(token_record.user_id)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to fetch user: {e}")))?
        .ok_or_else(|| AppError::Unauthorized)?;

        // Check if account is active
        if !user.is_active {
            return Err(AppError::Forbidden("Account is deactivated".to_string()));
        }

        // Revoke old refresh token (token rotation)
        sqlx::query("UPDATE refresh_tokens SET revoked_at = NOW() WHERE id = $1")
            .bind(token_record.id)
            .execute(&self.db_pool)
            .await
            .ok(); // Ignore errors

        // Generate new access token
        let access_token = generate_access_token(
            &self.jwt_config,
            user.id,
            &user.name,
            &user.email,
            &user.role,
            user.department.as_deref(),
        )
        .map_err(|e| AppError::Internal(format!("Failed to generate access token: {e}")))?;

        // Generate new refresh token
        let new_refresh_token = self.generate_refresh_token();
        let new_token_hash = self.hash_token(&new_refresh_token);
        let expires_at = Utc::now() + Duration::days(self.refresh_token_expiry_days);

        // Store new refresh token
        sqlx::query(
            "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at, created_at) VALUES ($1, $2, $3, $4, NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(user.id)
        .bind(&new_token_hash)
        .bind(expires_at)
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to store refresh token: {e}")))?;

        Ok(AuthResponse {
            access_token,
            refresh_token: new_refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.jwt_config.access_expiration_secs,
            user: UserInfo {
                id: user.id.to_string(),
                email: user.email,
                name: user.name,
                role: user.role,
                department: user.department,
                is_active: user.is_active,
                email_verified: user.email_verified,
                created_at: user.created_at,
            },
        })
    }

    /// Logout by revoking refresh token(s)
    ///
    /// # Arguments
    ///
    /// * `user_id` - User ID from JWT
    /// * `jti` - JWT ID for blacklisting
    /// * `request` - Logout options
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Logout successful
    /// * `Err(AppError)` - If logout fails
    pub async fn logout(
        &self,
        user_id: Uuid,
        jti: &str,
        request: LogoutRequest,
    ) -> Result<(), AppError> {
        // If refresh token is provided, revoke it
        if let Some(refresh_token) = request.refresh_token {
            let token_hash = self.hash_token(&refresh_token);
            sqlx::query("UPDATE refresh_tokens SET revoked_at = NOW() WHERE token_hash = $1 AND user_id = $2")
                .bind(&token_hash)
                .bind(user_id)
                .execute(&self.db_pool)
                .await
                .ok(); // Ignore errors
        }

        // If logout_all_devices is true, revoke all refresh tokens for this user
        if request.logout_all_devices.unwrap_or(false) {
            sqlx::query("UPDATE refresh_tokens SET revoked_at = NOW() WHERE user_id = $1 AND revoked_at IS NULL")
                .bind(user_id)
                .execute(&self.db_pool)
                .await
                .ok(); // Ignore errors
        }

        // Add JWT to blacklist to invalidate access token
        let exp = Utc::now() + Duration::seconds(self.jwt_config.access_expiration_secs as i64);
        sqlx::query("INSERT INTO token_blacklist (token_jti, expires_at, blacklisted_at) VALUES ($1, $2, NOW()) ON CONFLICT (token_jti) DO NOTHING")
            .bind(jti)
            .bind(exp)
            .execute(&self.db_pool)
            .await
            .ok(); // Ignore errors

        // Also add to in-memory blacklist for immediate effect
        crate::auth::revoke_token(jti);

        Ok(())
    }

    /// Get user info by user ID
    ///
    /// # Arguments
    ///
    /// * `user_id` - User UUID
    ///
    /// # Returns
    ///
    /// * `Ok(UserInfo)` - User information
    /// * `Err(AppError)` - If user not found
    pub async fn get_user(&self, user_id: Uuid) -> Result<UserInfo, AppError> {
        let user = sqlx::query_as::<_, UserRecord>(
            "SELECT id, email, password_hash, name, role, department, is_active, email_verified, failed_login_attempts, locked_until, created_at FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to fetch user: {e}")))?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        Ok(UserInfo {
            id: user.id.to_string(),
            email: user.email,
            name: user.name,
            role: user.role,
            department: user.department,
            is_active: user.is_active,
            email_verified: user.email_verified,
            created_at: user.created_at,
        })
    }

    /// Check if a JWT is blacklisted
    ///
    /// # Arguments
    ///
    /// * `jti` - JWT ID
    ///
    /// # Returns
    ///
    /// * `Ok(bool)` - True if blacklisted
    /// * `Err(AppError)` - If check fails
    pub async fn is_token_blacklisted(&self, jti: &str) -> Result<bool, AppError> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM token_blacklist WHERE token_jti = $1 AND expires_at > NOW()",
        )
        .bind(jti)
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to check blacklist: {e}")))?;

        Ok(count > 0)
    }

    /// Generate a cryptographically secure refresh token
    fn generate_refresh_token(&self) -> String {
        let mut rng = rand::thread_rng();
        let token_bytes: [u8; 32] = rng.gen();
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(token_bytes)
    }

    /// Hash a token for storage (simple SHA-256)
    fn hash_token(&self, token: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

impl Clone for AuthService {
    fn clone(&self) -> Self {
        Self {
            db_pool: self.db_pool.clone(),
            jwt_config: self.jwt_config.clone(),
            refresh_token_expiry_days: self.refresh_token_expiry_days,
            max_failed_attempts: self.max_failed_attempts,
            lockout_duration_mins: self.lockout_duration_mins,
        }
    }
}
