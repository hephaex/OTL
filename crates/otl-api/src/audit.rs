//! Security audit logging for authentication events
//!
//! Provides structured audit logging for all authentication-related events
//! including logins, logouts, registrations, and access control failures.
//!
//! All audit events are logged at INFO level with the "audit" target,
//! making them easy to filter and route to security monitoring systems.
//!
//! # Architecture
//!
//! - Uses tracing for structured logging
//! - JSON-compatible format for log aggregators
//! - Immutable event records for compliance
//! - Separate target ("audit") for filtering
//!
//! # Example
//!
//! ```ignore
//! use otl_api::audit::{AuditEvent, audit_log};
//!
//! // Log a successful login
//! audit_log(&AuditEvent::LoginSuccess {
//!     user_id: user.id,
//!     email: user.email.clone(),
//!     ip_address: Some("192.168.1.1".to_string()),
//!     user_agent: Some("Mozilla/5.0...".to_string()),
//! });
//! ```
//!
//! Author: hephaex@gmail.com

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

/// Security audit events for authentication and authorization
///
/// All variants include timestamp (added automatically) and contextual information
/// like IP address and user agent when available.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum AuditEvent {
    /// User login attempt (before validation)
    LoginAttempt {
        email: String,
        success: bool,
        ip_address: Option<String>,
        user_agent: Option<String>,
        failure_reason: Option<String>,
    },

    /// Successful user login
    LoginSuccess {
        user_id: Uuid,
        email: String,
        ip_address: Option<String>,
        user_agent: Option<String>,
    },

    /// Failed login attempt
    LoginFailure {
        email: String,
        reason: String,
        ip_address: Option<String>,
        user_agent: Option<String>,
        failed_attempts: Option<i32>,
        account_locked: bool,
    },

    /// User logout
    Logout {
        user_id: Uuid,
        email: String,
        ip_address: Option<String>,
        logout_all_devices: bool,
    },

    /// Access token refresh
    TokenRefresh {
        user_id: Uuid,
        email: String,
        ip_address: Option<String>,
        user_agent: Option<String>,
    },

    /// User registration attempt
    RegistrationAttempt {
        email: String,
        success: bool,
        ip_address: Option<String>,
        user_agent: Option<String>,
        failure_reason: Option<String>,
    },

    /// Successful user registration
    RegistrationSuccess {
        user_id: Uuid,
        email: String,
        role: String,
        ip_address: Option<String>,
        user_agent: Option<String>,
    },

    /// Failed registration attempt
    RegistrationFailure {
        email: String,
        reason: String,
        ip_address: Option<String>,
        user_agent: Option<String>,
    },

    /// Password change
    PasswordChange {
        user_id: Uuid,
        email: String,
        ip_address: Option<String>,
        user_agent: Option<String>,
    },

    /// Access denied due to insufficient permissions
    AccessDenied {
        user_id: Option<Uuid>,
        email: Option<String>,
        resource: String,
        required_role: Option<String>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    },

    /// Invalid or expired token used
    InvalidToken {
        ip_address: Option<String>,
        user_agent: Option<String>,
        reason: String,
    },

    /// Account lockout triggered
    AccountLocked {
        user_id: Uuid,
        email: String,
        failed_attempts: i32,
        locked_until: DateTime<Utc>,
        ip_address: Option<String>,
    },

    /// Account unlocked (manually or automatically)
    AccountUnlocked {
        user_id: Uuid,
        email: String,
        unlocked_by: Option<Uuid>,
        ip_address: Option<String>,
    },
}

/// Audit log context containing metadata about the request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditContext {
    /// Timestamp when the event occurred
    pub timestamp: DateTime<Utc>,
    /// Client IP address (extracted from request headers)
    pub ip_address: Option<String>,
    /// User agent string (extracted from request headers)
    pub user_agent: Option<String>,
}

impl Default for AuditContext {
    fn default() -> Self {
        Self {
            timestamp: Utc::now(),
            ip_address: None,
            user_agent: None,
        }
    }
}

/// Log a security audit event with structured fields
///
/// Events are logged at INFO level with the "audit" target, making them
/// easy to filter and route separately from application logs.
///
/// # Arguments
///
/// * `event` - The audit event to log
///
/// # Example
///
/// ```ignore
/// use otl_api::audit::{AuditEvent, audit_log};
/// use uuid::Uuid;
///
/// audit_log(&AuditEvent::LoginSuccess {
///     user_id: Uuid::new_v4(),
///     email: "user@example.com".to_string(),
///     ip_address: Some("192.168.1.1".to_string()),
///     user_agent: Some("Mozilla/5.0...".to_string()),
/// });
/// ```
///
/// # Structured Logging
///
/// The event is serialized to JSON for compatibility with log aggregators
/// like Elasticsearch, Splunk, or CloudWatch Logs. Example output:
///
/// ```json
/// {
///   "timestamp": "2025-12-24T10:30:00Z",
///   "event_type": "login_success",
///   "user_id": "550e8400-e29b-41d4-a716-446655440000",
///   "email": "user@example.com",
///   "ip_address": "192.168.1.1",
///   "user_agent": "Mozilla/5.0..."
/// }
/// ```
pub fn audit_log(event: &AuditEvent) {
    let timestamp = Utc::now();

    // Serialize event to JSON for structured logging
    let event_json = serde_json::to_string(event)
        .unwrap_or_else(|e| format!("{{\"error\":\"Failed to serialize audit event: {e}\"}}"));

    // Log with special "audit" target for filtering
    // This allows security teams to route audit logs separately
    match event {
        AuditEvent::LoginAttempt {
            email,
            success,
            ip_address,
            ..
        } => {
            info!(
                target: "audit",
                timestamp = %timestamp,
                event = %event_json,
                email = %email,
                success = %success,
                ip_address = ?ip_address,
                "Login attempt"
            );
        }
        AuditEvent::LoginSuccess {
            user_id,
            email,
            ip_address,
            ..
        } => {
            info!(
                target: "audit",
                timestamp = %timestamp,
                event = %event_json,
                user_id = %user_id,
                email = %email,
                ip_address = ?ip_address,
                "Login successful"
            );
        }
        AuditEvent::LoginFailure {
            email,
            reason,
            ip_address,
            failed_attempts,
            account_locked,
            ..
        } => {
            info!(
                target: "audit",
                timestamp = %timestamp,
                event = %event_json,
                email = %email,
                reason = %reason,
                ip_address = ?ip_address,
                failed_attempts = ?failed_attempts,
                account_locked = %account_locked,
                "Login failed"
            );
        }
        AuditEvent::Logout {
            user_id,
            email,
            ip_address,
            logout_all_devices,
        } => {
            info!(
                target: "audit",
                timestamp = %timestamp,
                event = %event_json,
                user_id = %user_id,
                email = %email,
                ip_address = ?ip_address,
                logout_all_devices = %logout_all_devices,
                "User logout"
            );
        }
        AuditEvent::TokenRefresh {
            user_id,
            email,
            ip_address,
            ..
        } => {
            info!(
                target: "audit",
                timestamp = %timestamp,
                event = %event_json,
                user_id = %user_id,
                email = %email,
                ip_address = ?ip_address,
                "Token refresh"
            );
        }
        AuditEvent::RegistrationAttempt {
            email,
            success,
            ip_address,
            failure_reason,
            ..
        } => {
            info!(
                target: "audit",
                timestamp = %timestamp,
                event = %event_json,
                email = %email,
                success = %success,
                ip_address = ?ip_address,
                failure_reason = ?failure_reason,
                "Registration attempt"
            );
        }
        AuditEvent::RegistrationSuccess {
            user_id,
            email,
            role,
            ip_address,
            ..
        } => {
            info!(
                target: "audit",
                timestamp = %timestamp,
                event = %event_json,
                user_id = %user_id,
                email = %email,
                role = %role,
                ip_address = ?ip_address,
                "Registration successful"
            );
        }
        AuditEvent::RegistrationFailure {
            email,
            reason,
            ip_address,
            ..
        } => {
            info!(
                target: "audit",
                timestamp = %timestamp,
                event = %event_json,
                email = %email,
                reason = %reason,
                ip_address = ?ip_address,
                "Registration failed"
            );
        }
        AuditEvent::PasswordChange {
            user_id,
            email,
            ip_address,
            ..
        } => {
            info!(
                target: "audit",
                timestamp = %timestamp,
                event = %event_json,
                user_id = %user_id,
                email = %email,
                ip_address = ?ip_address,
                "Password changed"
            );
        }
        AuditEvent::AccessDenied {
            user_id,
            email,
            resource,
            required_role,
            ip_address,
            ..
        } => {
            info!(
                target: "audit",
                timestamp = %timestamp,
                event = %event_json,
                user_id = ?user_id,
                email = ?email,
                resource = %resource,
                required_role = ?required_role,
                ip_address = ?ip_address,
                "Access denied"
            );
        }
        AuditEvent::InvalidToken {
            ip_address, reason, ..
        } => {
            info!(
                target: "audit",
                timestamp = %timestamp,
                event = %event_json,
                ip_address = ?ip_address,
                reason = %reason,
                "Invalid token"
            );
        }
        AuditEvent::AccountLocked {
            user_id,
            email,
            failed_attempts,
            locked_until,
            ip_address,
        } => {
            info!(
                target: "audit",
                timestamp = %timestamp,
                event = %event_json,
                user_id = %user_id,
                email = %email,
                failed_attempts = %failed_attempts,
                locked_until = %locked_until,
                ip_address = ?ip_address,
                "Account locked"
            );
        }
        AuditEvent::AccountUnlocked {
            user_id,
            email,
            unlocked_by,
            ip_address,
        } => {
            info!(
                target: "audit",
                timestamp = %timestamp,
                event = %event_json,
                user_id = %user_id,
                email = %email,
                unlocked_by = ?unlocked_by,
                ip_address = ?ip_address,
                "Account unlocked"
            );
        }
    }
}

/// Extract IP address from request headers
///
/// Checks X-Forwarded-For, X-Real-IP, and falls back to connection info.
/// This is a helper for handlers that need to extract client IP.
///
/// # Arguments
///
/// * `headers` - HTTP headers from the request
///
/// # Returns
///
/// Optional IP address string
pub fn extract_ip_address(headers: &axum::http::HeaderMap) -> Option<String> {
    // Check X-Forwarded-For (proxy/load balancer)
    if let Some(xff) = headers.get("x-forwarded-for") {
        if let Ok(xff_str) = xff.to_str() {
            // Take the first IP in the chain (client IP)
            if let Some(first_ip) = xff_str.split(',').next() {
                return Some(first_ip.trim().to_string());
            }
        }
    }

    // Check X-Real-IP (nginx proxy)
    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            return Some(ip_str.to_string());
        }
    }

    // Fall back to None - connection info would need to be passed separately
    None
}

/// Extract user agent from request headers
///
/// # Arguments
///
/// * `headers` - HTTP headers from the request
///
/// # Returns
///
/// Optional user agent string
pub fn extract_user_agent(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|ua| ua.to_str().ok())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_serialization() {
        let event = AuditEvent::LoginSuccess {
            user_id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            ip_address: Some("192.168.1.1".to_string()),
            user_agent: Some("Mozilla/5.0".to_string()),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("login_success"));
        assert!(json.contains("test@example.com"));
    }

    #[test]
    fn test_audit_log_login_success() {
        // This test just ensures the function doesn't panic
        let event = AuditEvent::LoginSuccess {
            user_id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            ip_address: Some("192.168.1.1".to_string()),
            user_agent: Some("Test Agent".to_string()),
        };

        audit_log(&event);
    }

    #[test]
    fn test_audit_log_login_failure() {
        let event = AuditEvent::LoginFailure {
            email: "test@example.com".to_string(),
            reason: "Invalid password".to_string(),
            ip_address: Some("192.168.1.1".to_string()),
            user_agent: Some("Test Agent".to_string()),
            failed_attempts: Some(3),
            account_locked: false,
        };

        audit_log(&event);
    }

    #[test]
    fn test_extract_ip_from_x_forwarded_for() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            "203.0.113.1, 198.51.100.1".parse().unwrap(),
        );

        let ip = extract_ip_address(&headers);
        assert_eq!(ip, Some("203.0.113.1".to_string()));
    }

    #[test]
    fn test_extract_ip_from_x_real_ip() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-real-ip", "203.0.113.1".parse().unwrap());

        let ip = extract_ip_address(&headers);
        assert_eq!(ip, Some("203.0.113.1".to_string()));
    }

    #[test]
    fn test_extract_user_agent() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            axum::http::header::USER_AGENT,
            "Mozilla/5.0 (Test)".parse().unwrap(),
        );

        let ua = extract_user_agent(&headers);
        assert_eq!(ua, Some("Mozilla/5.0 (Test)".to_string()));
    }

    #[test]
    fn test_extract_missing_headers() {
        let headers = axum::http::HeaderMap::new();

        assert_eq!(extract_ip_address(&headers), None);
        assert_eq!(extract_user_agent(&headers), None);
    }

    #[test]
    fn test_account_locked_event() {
        let locked_until = Utc::now() + chrono::Duration::minutes(15);
        let event = AuditEvent::AccountLocked {
            user_id: Uuid::new_v4(),
            email: "locked@example.com".to_string(),
            failed_attempts: 5,
            locked_until,
            ip_address: Some("192.168.1.1".to_string()),
        };

        audit_log(&event);
    }

    #[test]
    fn test_registration_events() {
        let user_id = Uuid::new_v4();

        // Success
        audit_log(&AuditEvent::RegistrationSuccess {
            user_id,
            email: "newuser@example.com".to_string(),
            role: "viewer".to_string(),
            ip_address: Some("192.168.1.1".to_string()),
            user_agent: Some("Test".to_string()),
        });

        // Failure
        audit_log(&AuditEvent::RegistrationFailure {
            email: "invalid@example.com".to_string(),
            reason: "Email already exists".to_string(),
            ip_address: Some("192.168.1.1".to_string()),
            user_agent: Some("Test".to_string()),
        });
    }
}
