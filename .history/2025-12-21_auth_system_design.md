# Authentication and Authorization System Design
## GitHub Issue #4 Implementation Plan

**Date**: 2025-12-21
**Status**: Design Complete
**Priority**: High (Security Critical)

---

## 1. Executive Summary

This document provides a comprehensive implementation plan for adding authentication and authorization to the OTL (Ontology-based Knowledge System) API. The system will implement JWT-based authentication with refresh tokens, Role-Based Access Control (RBAC), and department-based document access control (ACL).

### Current State Analysis

- **Existing middleware.rs**: Basic JWT validation structure exists but is not integrated into routes
- **Existing ACL in otl-core**: `DocumentAcl` and `AccessLevel` enums are already defined
- **No user storage**: No users table or user management exists
- **No password handling**: No password hashing or verification
- **Routes are unprotected**: All API endpoints are publicly accessible

### Key Design Decisions

1. **Database Choice**: SurrealDB for user/session storage (consistent with existing graph storage)
2. **Password Hashing**: Argon2id (memory-hard, recommended by OWASP)
3. **Token Strategy**: Short-lived access tokens (1h) + long-lived refresh tokens (7d)
4. **Session Management**: Server-side token blacklist for logout support

---

## 2. Database Schema Design

### 2.1 Users Table (SurrealDB)

```surql
-- Users table with all authentication and profile data
DEFINE TABLE users SCHEMAFULL;

-- Core fields
DEFINE FIELD id ON users TYPE uuid;
DEFINE FIELD email ON users TYPE string
    ASSERT string::is::email($value);
DEFINE FIELD password_hash ON users TYPE string;
DEFINE FIELD name ON users TYPE string;

-- Role assignment (single primary role for simplicity)
DEFINE FIELD role ON users TYPE string
    ASSERT $value IN ['admin', 'editor', 'viewer'];

-- Department for ACL filtering
DEFINE FIELD department ON users TYPE option<string>;

-- Account status
DEFINE FIELD is_active ON users TYPE bool DEFAULT true;
DEFINE FIELD email_verified ON users TYPE bool DEFAULT false;

-- Security tracking
DEFINE FIELD failed_login_attempts ON users TYPE int DEFAULT 0;
DEFINE FIELD locked_until ON users TYPE option<datetime>;
DEFINE FIELD last_login ON users TYPE option<datetime>;
DEFINE FIELD password_changed_at ON users TYPE datetime DEFAULT time::now();

-- Timestamps
DEFINE FIELD created_at ON users TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON users TYPE datetime DEFAULT time::now();

-- Indexes
DEFINE INDEX idx_users_email ON users FIELDS email UNIQUE;
DEFINE INDEX idx_users_role ON users FIELDS role;
DEFINE INDEX idx_users_department ON users FIELDS department;
```

### 2.2 Refresh Tokens Table (SurrealDB)

```surql
-- Refresh tokens for session management
DEFINE TABLE refresh_tokens SCHEMAFULL;

DEFINE FIELD id ON refresh_tokens TYPE uuid;
DEFINE FIELD user_id ON refresh_tokens TYPE uuid;
DEFINE FIELD token_hash ON refresh_tokens TYPE string;
DEFINE FIELD device_info ON refresh_tokens TYPE option<string>;
DEFINE FIELD ip_address ON refresh_tokens TYPE option<string>;
DEFINE FIELD expires_at ON refresh_tokens TYPE datetime;
DEFINE FIELD created_at ON refresh_tokens TYPE datetime DEFAULT time::now();
DEFINE FIELD revoked_at ON refresh_tokens TYPE option<datetime>;

-- Indexes
DEFINE INDEX idx_refresh_tokens_hash ON refresh_tokens FIELDS token_hash UNIQUE;
DEFINE INDEX idx_refresh_tokens_user ON refresh_tokens FIELDS user_id;
DEFINE INDEX idx_refresh_tokens_expires ON refresh_tokens FIELDS expires_at;
```

### 2.3 Token Blacklist Table (SurrealDB)

```surql
-- Blacklisted access tokens (for logout before expiry)
DEFINE TABLE token_blacklist SCHEMAFULL;

DEFINE FIELD token_jti ON token_blacklist TYPE string;
DEFINE FIELD expires_at ON token_blacklist TYPE datetime;
DEFINE FIELD blacklisted_at ON token_blacklist TYPE datetime DEFAULT time::now();

-- Index for fast lookup
DEFINE INDEX idx_blacklist_jti ON token_blacklist FIELDS token_jti UNIQUE;
DEFINE INDEX idx_blacklist_expires ON token_blacklist FIELDS expires_at;
```

### 2.4 Audit Log Table (SurrealDB)

```surql
-- Audit log for security-sensitive operations
DEFINE TABLE audit_log SCHEMAFULL;

DEFINE FIELD id ON audit_log TYPE uuid;
DEFINE FIELD user_id ON audit_log TYPE option<uuid>;
DEFINE FIELD action ON audit_log TYPE string;
DEFINE FIELD resource_type ON audit_log TYPE string;
DEFINE FIELD resource_id ON audit_log TYPE option<string>;
DEFINE FIELD ip_address ON audit_log TYPE option<string>;
DEFINE FIELD user_agent ON audit_log TYPE option<string>;
DEFINE FIELD details ON audit_log TYPE option<object>;
DEFINE FIELD success ON audit_log TYPE bool;
DEFINE FIELD created_at ON audit_log TYPE datetime DEFAULT time::now();

-- Indexes
DEFINE INDEX idx_audit_user ON audit_log FIELDS user_id;
DEFINE INDEX idx_audit_action ON audit_log FIELDS action;
DEFINE INDEX idx_audit_created ON audit_log FIELDS created_at;
```

---

## 3. API Endpoint Design

### 3.1 Authentication Endpoints

#### POST /api/v1/auth/register
- **Description**: Register a new user account
- **Access**: Public (but can be disabled in production)
- **Request Body**:
```json
{
  "email": "user@example.com",
  "password": "SecureP@ssw0rd!",
  "name": "John Doe",
  "department": "Engineering"
}
```
- **Response (201)**:
```json
{
  "user_id": "uuid",
  "email": "user@example.com",
  "name": "John Doe",
  "role": "viewer",
  "message": "Registration successful. Please verify your email."
}
```
- **Validation**:
  - Email: Valid format, unique
  - Password: Min 8 chars, 1 uppercase, 1 lowercase, 1 number, 1 special char
  - Name: 2-100 characters

#### POST /api/v1/auth/login
- **Description**: Authenticate user and issue tokens
- **Access**: Public
- **Request Body**:
```json
{
  "email": "user@example.com",
  "password": "SecureP@ssw0rd!"
}
```
- **Response (200)**:
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIs...",
  "refresh_token": "dGhpcyBpcyBhIHJlZnJlc2ggdG9rZW4...",
  "token_type": "Bearer",
  "expires_in": 3600,
  "user": {
    "id": "uuid",
    "email": "user@example.com",
    "name": "John Doe",
    "role": "editor",
    "department": "Engineering"
  }
}
```
- **Error Responses**:
  - 401: Invalid credentials
  - 423: Account locked (too many failed attempts)

#### POST /api/v1/auth/refresh
- **Description**: Exchange refresh token for new access token
- **Access**: Public (with valid refresh token)
- **Request Body**:
```json
{
  "refresh_token": "dGhpcyBpcyBhIHJlZnJlc2ggdG9rZW4..."
}
```
- **Response (200)**:
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIs...",
  "token_type": "Bearer",
  "expires_in": 3600
}
```

#### POST /api/v1/auth/logout
- **Description**: Invalidate current session
- **Access**: Authenticated
- **Headers**: `Authorization: Bearer <access_token>`
- **Request Body** (optional):
```json
{
  "refresh_token": "dGhpcyBpcyBhIHJlZnJlc2ggdG9rZW4...",
  "logout_all_devices": false
}
```
- **Response (200)**:
```json
{
  "message": "Logged out successfully"
}
```

#### POST /api/v1/auth/password/change
- **Description**: Change current user's password
- **Access**: Authenticated
- **Request Body**:
```json
{
  "current_password": "OldP@ssw0rd!",
  "new_password": "NewP@ssw0rd!"
}
```

### 3.2 User Management Endpoints (Admin Only)

#### GET /api/v1/users
- **Description**: List all users with pagination
- **Access**: Admin only
- **Query Parameters**: `page`, `page_size`, `role`, `department`, `search`
- **Response (200)**:
```json
{
  "users": [
    {
      "id": "uuid",
      "email": "user@example.com",
      "name": "John Doe",
      "role": "editor",
      "department": "Engineering",
      "is_active": true,
      "last_login": "2025-12-21T10:00:00Z",
      "created_at": "2025-01-15T09:00:00Z"
    }
  ],
  "total": 150,
  "page": 1,
  "page_size": 20
}
```

#### POST /api/v1/users
- **Description**: Create a new user (admin bypasses registration restrictions)
- **Access**: Admin only
- **Request Body**:
```json
{
  "email": "newuser@example.com",
  "password": "TempP@ssw0rd!",
  "name": "Jane Smith",
  "role": "editor",
  "department": "HR",
  "email_verified": true
}
```

#### GET /api/v1/users/{id}
- **Description**: Get user details
- **Access**: Admin or self
- **Response (200)**: User object

#### PUT /api/v1/users/{id}
- **Description**: Update user
- **Access**: Admin (full), self (limited fields)
- **Request Body**: Partial user object

#### DELETE /api/v1/users/{id}
- **Description**: Deactivate user (soft delete)
- **Access**: Admin only
- **Response (200)**:
```json
{
  "message": "User deactivated successfully"
}
```

### 3.3 Current User Endpoints

#### GET /api/v1/auth/me
- **Description**: Get current user profile
- **Access**: Authenticated
- **Response (200)**: Current user object

#### PUT /api/v1/auth/me
- **Description**: Update current user profile (name, department only)
- **Access**: Authenticated

---

## 4. JWT Token Structure

### 4.1 Access Token Claims

```json
{
  "iss": "otl-api",
  "sub": "user-uuid",
  "jti": "unique-token-id",
  "iat": 1703145600,
  "exp": 1703149200,
  "name": "John Doe",
  "email": "john@example.com",
  "role": "editor",
  "department": "Engineering",
  "permissions": ["read", "write", "approve"]
}
```

### 4.2 Role-Permission Mapping

| Role | Permissions |
|------|-------------|
| admin | read, write, delete, approve, reject, manage_users, manage_ontology |
| editor | read, write, delete_own, approve, reject |
| viewer | read |

### 4.3 Token Configuration (Environment Variables)

```bash
# JWT Configuration
JWT_SECRET=<secure-random-string-min-256-bits>
JWT_ACCESS_EXPIRATION_SECS=3600          # 1 hour
JWT_REFRESH_EXPIRATION_DAYS=7            # 7 days
JWT_ISSUER=otl-api

# Security Settings
AUTH_MAX_LOGIN_ATTEMPTS=5
AUTH_LOCKOUT_DURATION_MINS=15
AUTH_REGISTRATION_ENABLED=true           # Set to false in production
AUTH_REQUIRE_EMAIL_VERIFICATION=false    # Set to true in production
```

---

## 5. Middleware Implementation Plan

### 5.1 Enhanced Auth Middleware Stack

```rust
// File: crates/otl-api/src/middleware/auth.rs

/// JWT validation middleware - blocks requests without valid token
pub async fn require_auth(
    State(state): State<Arc<AppState>>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, AuthError>

/// Optional auth - extracts user if token present, continues if not
pub async fn optional_auth(
    State(state): State<Arc<AppState>>,
    mut request: Request<Body>,
    next: Next,
) -> Response

/// Role requirement middleware factory
pub fn require_role(role: &'static str) -> impl Fn(...) -> ...

/// Multi-role requirement (any of the roles)
pub fn require_any_role(roles: &'static [&'static str]) -> impl Fn(...) -> ...

/// Permission check middleware
pub fn require_permission(permission: &'static str) -> impl Fn(...) -> ...
```

### 5.2 Rate Limiting Middleware

```rust
// File: crates/otl-api/src/middleware/rate_limit.rs

/// Rate limit configuration
pub struct RateLimitConfig {
    pub max_requests: u32,
    pub window_secs: u64,
}

/// Rate limiting middleware using sliding window
pub async fn rate_limit_middleware(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, RateLimitError>

/// Specific rate limit for login endpoint
pub async fn login_rate_limit(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, RateLimitError>
```

### 5.3 Request Extensions

```rust
// File: crates/otl-api/src/middleware/mod.rs

/// Authenticated user extracted from JWT and available in handlers
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub email: String,
    pub name: String,
    pub role: UserRole,
    pub department: Option<String>,
    pub permissions: Vec<Permission>,
}

impl AuthenticatedUser {
    pub fn has_permission(&self, perm: Permission) -> bool;
    pub fn is_admin(&self) -> bool;
    pub fn can_access_department(&self, dept: &str) -> bool;
}
```

---

## 6. ACL Integration Plan

### 6.1 Document Access Filtering

The existing `DocumentAcl` in `otl-core` will be integrated with the new auth system:

```rust
// File: crates/otl-core/src/lib.rs (existing, to be enhanced)

impl DocumentAcl {
    /// Enhanced access check with role support
    pub fn can_access(&self, user: &AuthenticatedUser) -> bool {
        match self.access_level {
            AccessLevel::Public => true,
            AccessLevel::Internal => true, // All authenticated users
            AccessLevel::Confidential => {
                // Department match or required role match
                self.department.as_ref()
                    .map(|d| user.department.as_ref() == Some(d))
                    .unwrap_or(false)
                || self.required_roles.iter()
                    .any(|r| user.role.to_string() == *r)
            }
            AccessLevel::Restricted => {
                // Explicit user list or owner
                self.allowed_users.contains(&user.user_id.to_string())
                    || self.owner_id.as_ref() == Some(&user.user_id.to_string())
            }
        }
    }
}
```

### 6.2 Query-Level Filtering

```rust
// File: crates/otl-api/src/handlers/documents.rs

/// List documents with ACL filtering
pub async fn list_documents(
    State(state): State<Arc<AppState>>,
    user: Extension<AuthenticatedUser>,
    Query(params): Query<ListDocumentsQuery>,
) -> Result<impl IntoResponse, AppError> {
    // Build query with ACL predicates
    let query = DocumentQuery::new()
        .with_user_acl_filter(&user)
        .with_pagination(params.page, params.page_size);

    // Only return documents user can access
    let documents = state.document_store
        .query_with_acl(query, &user)
        .await?;

    Ok(Json(documents))
}
```

### 6.3 Department-Based Filtering in Vector Search

```rust
// File: crates/otl-vector/src/lib.rs

impl VectorSearchBackend {
    /// Search with ACL filtering in Qdrant
    pub async fn search_with_acl(
        &self,
        query: &str,
        user: &AuthenticatedUser,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        // Build Qdrant filter based on user's access
        let filter = self.build_acl_filter(user);

        // Execute search with filter
        let results = self.client
            .search(SearchPoints {
                collection_name: self.collection.clone(),
                vector: embedding,
                filter: Some(filter),
                limit: limit as u64,
                ..Default::default()
            })
            .await?;

        Ok(results)
    }

    fn build_acl_filter(&self, user: &AuthenticatedUser) -> Filter {
        // Filter logic based on user role and department
        Filter::any([
            // Public documents
            Condition::field("access_level").eq("public"),
            // Internal documents (any authenticated user)
            Condition::field("access_level").eq("internal"),
            // Department match
            Condition::all([
                Condition::field("access_level").eq("confidential"),
                Condition::field("department").eq(user.department.as_deref().unwrap_or("")),
            ]),
            // Explicit access
            Condition::all([
                Condition::field("access_level").eq("restricted"),
                Condition::field("allowed_users").eq(user.user_id.to_string()),
            ]),
        ])
    }
}
```

---

## 7. Security Considerations

### 7.1 Password Security

1. **Hashing Algorithm**: Argon2id with the following parameters:
   - Memory: 64 MB
   - Iterations: 3
   - Parallelism: 4
   - Salt: 16 bytes random
   - Hash length: 32 bytes

2. **Password Requirements**:
   - Minimum 8 characters
   - At least 1 uppercase letter
   - At least 1 lowercase letter
   - At least 1 digit
   - At least 1 special character
   - Not in common password list (top 10,000)

### 7.2 Token Security

1. **Access Token**:
   - Short-lived (1 hour default)
   - Contains minimal claims
   - HMAC-SHA256 signing

2. **Refresh Token**:
   - Long random string (256 bits)
   - Stored hashed in database
   - One-time use (rotation on refresh)
   - Bound to device/IP (optional)

3. **Token Blacklist**:
   - For logout before expiry
   - Automatic cleanup of expired entries
   - Redis-backed for production scale

### 7.3 Rate Limiting

| Endpoint | Limit | Window |
|----------|-------|--------|
| /auth/login | 5 requests | 15 minutes |
| /auth/register | 3 requests | 1 hour |
| /auth/refresh | 10 requests | 1 minute |
| Other authenticated | 100 requests | 1 minute |
| Other unauthenticated | 20 requests | 1 minute |

### 7.4 Audit Logging

Log the following events:
- Login attempts (success/failure)
- Password changes
- Role changes
- User creation/deletion
- Token refresh
- Logout
- Access denied events
- Ontology updates

**NEVER log**: Passwords, tokens, or other secrets

### 7.5 HTTPS Requirements

- All authentication endpoints MUST use HTTPS in production
- HSTS header should be enabled
- Secure cookie flags for any cookie-based storage

---

## 8. Implementation Phases

### Phase 1: Core Authentication Infrastructure [PARALLEL]

#### 1.1 Add Dependencies to Cargo.toml
- [ ] Add `argon2` crate for password hashing
- [ ] Add `rand` crate for secure random generation
- [ ] Verify `jsonwebtoken` version compatibility
- [ ] Add `validator` crate for input validation

**File**: `crates/otl-api/Cargo.toml`
```toml
[dependencies]
argon2 = "0.5"
rand = "0.8"
validator = { version = "0.18", features = ["derive"] }
```

#### 1.2 Create Auth Configuration
- [ ] Add JWT configuration to `AppConfig`
- [ ] Add security settings (lockout, rate limits)
- [ ] Environment variable loading

**File**: `crates/otl-core/src/config.rs`

#### 1.3 Create User Domain Models
- [ ] Define `User` struct with all fields
- [ ] Define `UserRole` enum
- [ ] Define `Permission` enum
- [ ] Implement role-permission mapping

**File**: `crates/otl-core/src/auth.rs` (new file)

### Phase 2: Database Layer [PARALLEL with Phase 1]

#### 2.1 SurrealDB Schema Migration
- [ ] Create schema migration script
- [ ] Define users table
- [ ] Define refresh_tokens table
- [ ] Define token_blacklist table
- [ ] Define audit_log table

**File**: `migrations/001_auth_schema.surql`

#### 2.2 User Repository Implementation
- [ ] Create `UserRepository` trait
- [ ] Implement SurrealDB `UserRepository`
- [ ] CRUD operations for users
- [ ] Login tracking methods

**File**: `crates/otl-graph/src/user_repository.rs` (new file)

#### 2.3 Token Repository Implementation
- [ ] Refresh token CRUD
- [ ] Blacklist operations
- [ ] Cleanup expired entries

**File**: `crates/otl-graph/src/token_repository.rs` (new file)

### Phase 3: Authentication Services

#### 3.1 Password Service
- [ ] Implement Argon2id hashing
- [ ] Implement password verification
- [ ] Password strength validation
- [ ] Common password check

**File**: `crates/otl-api/src/services/password.rs` (new file)

#### 3.2 Token Service
- [ ] JWT access token generation
- [ ] JWT access token validation
- [ ] Refresh token generation
- [ ] Token blacklist check

**File**: `crates/otl-api/src/services/token.rs` (new file)

#### 3.3 Auth Service
- [ ] Registration logic
- [ ] Login logic with lockout
- [ ] Refresh token rotation
- [ ] Logout logic
- [ ] Password change logic

**File**: `crates/otl-api/src/services/auth.rs` (new file)

### Phase 4: Middleware Enhancement

#### 4.1 Enhanced Auth Middleware
- [ ] Refactor existing `auth_middleware`
- [ ] Add blacklist checking
- [ ] Improve error responses
- [ ] Add request extension for user

**File**: `crates/otl-api/src/middleware.rs` (update)

#### 4.2 Rate Limiting Middleware
- [ ] Implement sliding window rate limiter
- [ ] Per-endpoint configuration
- [ ] IP-based and user-based limits

**File**: `crates/otl-api/src/middleware/rate_limit.rs` (new file)

#### 4.3 Audit Logging Middleware
- [ ] Capture security events
- [ ] Async logging to database
- [ ] Configurable verbosity

**File**: `crates/otl-api/src/middleware/audit.rs` (new file)

### Phase 5: API Handlers

#### 5.1 Auth Handlers
- [ ] POST /auth/register handler
- [ ] POST /auth/login handler
- [ ] POST /auth/refresh handler
- [ ] POST /auth/logout handler
- [ ] POST /auth/password/change handler
- [ ] GET /auth/me handler
- [ ] PUT /auth/me handler

**File**: `crates/otl-api/src/handlers/auth.rs` (new file)

#### 5.2 User Management Handlers
- [ ] GET /users list handler
- [ ] POST /users create handler
- [ ] GET /users/{id} get handler
- [ ] PUT /users/{id} update handler
- [ ] DELETE /users/{id} delete handler

**File**: `crates/otl-api/src/handlers/users.rs` (new file)

### Phase 6: Route Integration

#### 6.1 Update Router Configuration
- [ ] Add auth routes (public)
- [ ] Add user routes (admin protected)
- [ ] Apply middleware to existing routes
- [ ] Configure route-specific rate limits

**File**: `crates/otl-api/src/routes.rs` (update)

#### 6.2 Update Existing Handlers
- [ ] Add auth to `upload_document` (editor+)
- [ ] Add auth to `delete_document` (admin/owner)
- [ ] Add auth to `approve_extraction` (editor+)
- [ ] Add auth to `reject_extraction` (editor+)
- [ ] Add auth to `update_ontology` (admin only)
- [ ] Add ACL filtering to document listing
- [ ] Add ACL filtering to graph search

**Files**:
- `crates/otl-api/src/handlers/documents.rs`
- `crates/otl-api/src/handlers/verify.rs`
- `crates/otl-api/src/handlers/graph.rs`

### Phase 7: Testing

#### 7.1 Unit Tests
- [ ] Password hashing tests
- [ ] Token generation/validation tests
- [ ] Permission checking tests
- [ ] ACL logic tests

#### 7.2 Integration Tests
- [ ] Registration flow test
- [ ] Login/logout flow test
- [ ] Token refresh flow test
- [ ] Role-based access tests
- [ ] ACL filtering tests
- [ ] Rate limiting tests

**File**: `crates/otl-api/tests/auth_tests.rs` (new file)

#### 7.3 Security Tests
- [ ] Password requirements validation
- [ ] Token expiry handling
- [ ] Lockout mechanism test
- [ ] Injection attack prevention

### Phase 8: Documentation & Deployment

#### 8.1 OpenAPI Specification
- [ ] Add security schemes to OpenAPI
- [ ] Document auth endpoints
- [ ] Add 401/403 responses to all protected endpoints

**File**: `crates/otl-api/src/lib.rs` (update ApiDoc)

#### 8.2 Environment Configuration
- [ ] Update sample .env file
- [ ] Document all auth-related variables
- [ ] Add Kubernetes secrets configuration

**Files**:
- `.env.example`
- `kubernetes/secrets.yaml`

---

## 9. File Structure Summary

```
crates/
  otl-api/
    src/
      handlers/
        auth.rs          # NEW: Auth handlers
        users.rs         # NEW: User management handlers
        documents.rs     # UPDATE: Add auth
        graph.rs         # UPDATE: Add auth
        verify.rs        # UPDATE: Add auth
      middleware/
        mod.rs           # UPDATE: Export new middleware
        auth.rs          # UPDATE: Enhanced auth
        rate_limit.rs    # NEW: Rate limiting
        audit.rs         # NEW: Audit logging
      services/
        mod.rs           # NEW: Service exports
        auth.rs          # NEW: Auth business logic
        password.rs      # NEW: Password handling
        token.rs         # NEW: Token management
      routes.rs          # UPDATE: Add auth routes
      lib.rs             # UPDATE: OpenAPI docs
    tests/
      auth_tests.rs      # NEW: Auth integration tests
    Cargo.toml           # UPDATE: Add dependencies

  otl-core/
    src/
      lib.rs             # UPDATE: Auth types
      auth.rs            # NEW: User/Role types
      config.rs          # UPDATE: Auth config

  otl-graph/
    src/
      user_repository.rs     # NEW: User storage
      token_repository.rs    # NEW: Token storage
      lib.rs                 # UPDATE: Export repos

migrations/
  001_auth_schema.surql  # NEW: Database schema
```

---

## 10. Success Criteria

### Functional Requirements

- [ ] Users can register with email/password
- [ ] Users can login and receive JWT tokens
- [ ] Access tokens expire after 1 hour
- [ ] Refresh tokens work for 7 days
- [ ] Users can logout (token invalidation)
- [ ] Admin can manage users (CRUD)
- [ ] Protected endpoints return 401 without token
- [ ] Protected endpoints return 403 for wrong role
- [ ] Documents are filtered by ACL
- [ ] Department-based access works correctly

### Security Requirements

- [ ] Passwords are hashed with Argon2id
- [ ] Failed logins trigger account lockout
- [ ] Rate limiting prevents brute force
- [ ] Tokens cannot be forged
- [ ] Logout invalidates tokens
- [ ] Audit log captures security events
- [ ] No secrets in logs

### Performance Requirements

- [ ] Login < 500ms (including Argon2 hashing)
- [ ] Token validation < 10ms
- [ ] ACL filtering adds < 50ms to queries

---

## 11. Dependencies

### External Crates

| Crate | Version | Purpose |
|-------|---------|---------|
| argon2 | 0.5 | Password hashing |
| jsonwebtoken | 9 | JWT handling (existing) |
| rand | 0.8 | Secure random generation |
| validator | 0.18 | Input validation |

### Existing System Dependencies

- SurrealDB (user/token storage)
- Qdrant (ACL filtering in vector search)
- otl-core (types and config)
- otl-graph (database operations)

---

## 12. Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Performance degradation from Argon2 | Use async hashing, tune parameters |
| Token blacklist size growth | Automatic cleanup, Redis for scale |
| Database connection exhaustion | Connection pooling, timeouts |
| Brute force attacks | Rate limiting, lockout mechanism |
| Token theft | Short expiry, refresh rotation |
| SQL/NoSQL injection | Parameterized queries, input validation |

---

## 13. Future Enhancements

1. **OAuth2/OIDC Integration**: Support Google, Microsoft, SAML
2. **Multi-Factor Authentication**: TOTP, WebAuthn
3. **API Keys**: For service-to-service auth
4. **Fine-grained Permissions**: Resource-level ACLs
5. **Session Management UI**: View/revoke active sessions
6. **Password Reset Flow**: Email-based recovery

---

## 14. References

- [OWASP Authentication Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html)
- [JWT Best Practices](https://auth0.com/blog/a-look-at-the-latest-draft-for-jwt-bcp/)
- [Argon2 RFC 9106](https://www.rfc-editor.org/rfc/rfc9106.html)
- [SurrealDB Documentation](https://surrealdb.com/docs)

---

**Document Version**: 1.0
**Last Updated**: 2025-12-21
**Author**: AI Assistant
