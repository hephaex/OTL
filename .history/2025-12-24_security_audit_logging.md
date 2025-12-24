# Security Audit Logging Implementation

**Date**: 2025-12-24
**Author**: Claude Opus 4.5
**Task**: Implement comprehensive security audit logging for authentication events

## Overview

Implemented a complete security audit logging system for the OTL API authentication subsystem. The system provides structured logging of all authentication-related events including logins, registrations, logouts, token operations, and access control failures.

## Architecture

### Design Principles

1. **Structured Logging**: All audit events are JSON-serializable for easy integration with log aggregators (Elasticsearch, Splunk, CloudWatch)
2. **Separate Target**: Uses "audit" as a tracing target for easy filtering and routing
3. **Immutable Records**: Events are logged immediately and include full context
4. **Privacy-Aware**: Sensitive data (passwords, tokens) are never logged
5. **Performance**: Async logging doesn't block request handling

### Component Structure

```
crates/otl-api/src/
├── audit.rs                    # New: Core audit logging module
├── handlers/auth.rs            # Modified: Added audit logging to handlers
├── auth/middleware.rs          # Modified: Added audit logging to middleware
├── auth/service.rs             # Modified: Added account lockout logging
└── lib.rs                      # Modified: Exported audit module
```

## Implementation Details

### 1. Audit Module (`audit.rs`)

Created `/Users/mare/Simon/OTL/crates/otl-api/src/audit.rs` with:

#### AuditEvent Enum

Comprehensive enum of security events:

```rust
pub enum AuditEvent {
    LoginAttempt { email, success, ip_address, user_agent, failure_reason },
    LoginSuccess { user_id, email, ip_address, user_agent },
    LoginFailure { email, reason, ip_address, user_agent, failed_attempts, account_locked },
    Logout { user_id, email, ip_address, logout_all_devices },
    TokenRefresh { user_id, email, ip_address, user_agent },
    RegistrationAttempt { email, success, ip_address, user_agent, failure_reason },
    RegistrationSuccess { user_id, email, role, ip_address, user_agent },
    RegistrationFailure { email, reason, ip_address, user_agent },
    PasswordChange { user_id, email, ip_address, user_agent },
    AccessDenied { user_id, email, resource, required_role, ip_address, user_agent },
    InvalidToken { ip_address, user_agent, reason },
    AccountLocked { user_id, email, failed_attempts, locked_until, ip_address },
    AccountUnlocked { user_id, email, unlocked_by, ip_address },
}
```

#### Core Functions

- **`audit_log(event: &AuditEvent)`**: Main logging function using tracing with "audit" target
- **`extract_ip_address(headers: &HeaderMap)`**: Extracts client IP from X-Forwarded-For or X-Real-IP
- **`extract_user_agent(headers: &HeaderMap)`**: Extracts User-Agent header

#### Features

- **Structured JSON output** for log aggregators
- **Special "audit" target** for filtering
- **Timestamp** automatically added to all events
- **Contextual information** (IP, User-Agent) when available

### 2. Auth Handler Updates (`handlers/auth.rs`)

Modified all authentication handlers to log audit events:

#### Changes Made

1. **register_handler**: Logs registration success/failure
2. **login_handler**: Logs login success/failure with account lockout detection
3. **refresh_handler**: Logs token refresh and invalid token attempts
4. **logout_handler**: Logs logout with device scope information

#### Pattern Used

```rust
pub async fn login_handler(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
) -> Result<impl IntoResponse, AppError> {
    // Extract context
    let ip_address = extract_ip_address(req.headers());
    let user_agent = extract_user_agent(req.headers());

    // Process request
    let result = auth_service.login(request).await;

    // Log outcome
    match result {
        Ok(response) => {
            audit_log(&AuditEvent::LoginSuccess { ... });
            Ok(Json(response))
        }
        Err(e) => {
            audit_log(&AuditEvent::LoginFailure { ... });
            Err(e)
        }
    }
}
```

### 3. Auth Middleware Updates (`auth/middleware.rs`)

Enhanced authentication middleware to log security events:

#### Changes Made

1. **auth_middleware**: Logs invalid token attempts and revoked token usage
2. **require_role**: Logs access denied when role requirements not met
3. **require_any_role**: Logs access denied for multi-role checks

#### Access Control Logging

```rust
// Log access denied
audit_log(&AuditEvent::AccessDenied {
    user_id: Some(user.user_id),
    email: Some(user.email.clone()),
    resource: format!("role:{required_role}"),
    required_role: Some(required_role.to_string()),
    ip_address,
    user_agent,
});
```

### 4. Auth Service Updates (`auth/service.rs`)

Added account lockout event logging:

```rust
// Log account lockout if threshold exceeded
if let Some(locked_time) = locked_until {
    audit_log(&AuditEvent::AccountLocked {
        user_id: user.id,
        email: user.email.clone(),
        failed_attempts,
        locked_until: locked_time,
        ip_address: None, // IP logged at handler level
    });
}
```

### 5. Module Export (`lib.rs`)

Exported the audit module for use throughout the API:

```rust
pub mod audit;
```

## Testing

### Test Coverage

Implemented comprehensive test suite with 9 tests covering:

1. **Event Serialization**: Verifies JSON compatibility
2. **Logging Functions**: Tests all event types
3. **Header Extraction**: Tests IP and User-Agent extraction
4. **Edge Cases**: Missing headers, multiple IPs

### Test Results

```
running 9 tests
test audit::tests::test_extract_missing_headers ... ok
test audit::tests::test_extract_ip_from_x_real_ip ... ok
test audit::tests::test_extract_user_agent ... ok
test audit::tests::test_extract_ip_from_x_forwarded_for ... ok
test audit::tests::test_audit_log_login_failure ... ok
test audit::tests::test_account_locked_event ... ok
test audit::tests::test_audit_event_serialization ... ok
test audit::tests::test_audit_log_login_success ... ok
test audit::tests::test_registration_events ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured
```

### Code Quality

- **Clippy**: Passes with no warnings for audit-related code
- **Rustfmt**: Code formatted according to project standards
- **Compilation**: Builds successfully with only pre-existing warnings

## Usage Examples

### Configuration

Audit logs use the "audit" target. Configure logging in your application:

```rust
use tracing_subscriber::filter::EnvFilter;

tracing_subscriber::fmt()
    .with_env_filter(
        EnvFilter::from_default_env()
            .add_directive("audit=info".parse().unwrap())
    )
    .init();
```

### Log Filtering

To see only audit logs:

```bash
# Set environment variable
RUST_LOG=audit=info cargo run

# Or use grep
cargo run 2>&1 | grep '"audit"'
```

### Sample Output

```json
{
  "timestamp": "2025-12-24T10:30:00Z",
  "target": "audit",
  "level": "INFO",
  "fields": {
    "event_type": "login_success",
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "email": "user@example.com",
    "ip_address": "192.168.1.1",
    "user_agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64)"
  }
}
```

## Security Considerations

### What is Logged

- User identifiers (UUID, email)
- Event timestamps
- Client IP addresses
- User agents
- Success/failure status
- Failure reasons (generic)
- Resource access attempts

### What is NOT Logged

- Passwords (plain or hashed)
- JWT tokens (except JTI for revocation)
- Session cookies
- API keys
- Personal data beyond email

### Privacy Compliance

- IP addresses can be anonymized if needed
- Audit logs should be stored securely
- Access to audit logs should be restricted
- Consider GDPR data retention policies

## Integration with Log Aggregators

### Elasticsearch

Audit logs are JSON-compatible and can be shipped via Filebeat:

```yaml
filebeat.inputs:
  - type: log
    paths:
      - /var/log/otl-api/*.log
    json.keys_under_root: true
    processors:
      - include_fields:
          fields: ["target"]
          equals: ["audit"]
```

### Splunk

Use HTTP Event Collector (HEC):

```rust
// Custom tracing layer for Splunk
layer.with_writer(SplunkWriter::new("https://splunk:8088"))
```

### CloudWatch Logs

Use aws-cloudwatch-logs crate or container logging:

```dockerfile
docker run \
  --log-driver=awslogs \
  --log-opt awslogs-group=otl-api-audit \
  otl-api
```

## Future Enhancements

### Potential Improvements

1. **Rate Limiting**: Add audit events for rate limit violations
2. **API Keys**: Log API key usage and rotation
3. **Admin Actions**: Log administrative operations
4. **Data Access**: Log sensitive data access (documents, graphs)
5. **Compliance Reports**: Generate compliance reports from audit logs
6. **Anomaly Detection**: Integrate with SIEM for anomaly detection
7. **Alerting**: Real-time alerts for suspicious activity

### Performance Optimizations

1. **Async Shipping**: Buffer and batch-ship logs to aggregators
2. **Sampling**: Sample high-volume events in production
3. **Compression**: Compress logs before shipping
4. **Local Storage**: Store locally with rotation before shipping

## Files Modified

1. `/Users/mare/Simon/OTL/crates/otl-api/src/audit.rs` - **Created**
2. `/Users/mare/Simon/OTL/crates/otl-api/src/handlers/auth.rs` - Modified
3. `/Users/mare/Simon/OTL/crates/otl-api/src/auth/middleware.rs` - Modified
4. `/Users/mare/Simon/OTL/crates/otl-api/src/auth/service.rs` - Modified
5. `/Users/mare/Simon/OTL/crates/otl-api/src/lib.rs` - Modified

## Technical Notes

### Rust Features Used

- **Tracing**: Structured logging with target filtering
- **Serde**: JSON serialization for log aggregators
- **Enum Variants**: Tagged union for type-safe events
- **Pattern Matching**: Exhaustive event handling
- **Chrono**: Timestamp generation

### Design Patterns

- **Observer Pattern**: Events are logged as they occur
- **Builder Pattern**: Events constructed with all context
- **Strategy Pattern**: Different logging strategies per event type

### Dependencies

No new dependencies added - uses existing:
- `tracing` for logging
- `serde` / `serde_json` for serialization
- `chrono` for timestamps
- `uuid` for user IDs
- `axum` for HTTP context extraction

## Conclusion

Successfully implemented comprehensive security audit logging for the OTL API authentication system. The implementation:

✅ Logs all authentication events
✅ Includes contextual information (IP, User-Agent)
✅ Uses structured JSON format
✅ Integrates with existing tracing infrastructure
✅ Passes all tests and code quality checks
✅ Follows Rust best practices
✅ Ready for production deployment

The audit logging system provides visibility into authentication events for security monitoring, compliance, and incident response.
