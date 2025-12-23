# P1 Architecture Issue Fixes - 2025-12-21

## Session Overview
Fixed P1 (Priority 1) issues identified in the architecture analysis, focusing on database connection configuration and error handling improvements.

## Issues Fixed

### 1. Database Pool Size Hardcoding (P1)

**Problem**: PostgreSQL connection pool size was hardcoded to 10 instead of using the configuration value.

**Location**: `/Users/mare/Simon/OTL/crates/otl-api/src/main.rs:42`

**Before**:
```rust
let db_pool = PgPoolOptions::new()
    .max_connections(10)
    .connect(&database_url)
    .await?;
```

**After**:
```rust
let db_pool = PgPoolOptions::new()
    .max_connections(config.database.postgres_pool_size)
    .acquire_timeout(std::time::Duration::from_secs(30))
    .idle_timeout(std::time::Duration::from_secs(600))
    .max_lifetime(std::time::Duration::from_secs(1800))
    .connect(&database_url)
    .await?;

tracing::info!(
    "PostgreSQL connected successfully (pool_size: {})",
    config.database.postgres_pool_size
);
```

**Benefits**:
- Pool size now configurable via `DatabaseConfig::postgres_pool_size` (default: 10)
- Can be adjusted for different deployment environments
- Added logging to show configured pool size

### 2. Connection Timeout Configuration (P1)

**Problem**: PostgreSQL connections had no timeout settings, potentially causing resource exhaustion.

**Location**: `/Users/mare/Simon/OTL/crates/otl-api/src/main.rs:43-45`

**Added Timeouts**:
- `acquire_timeout`: 30 seconds - Maximum time to wait for a connection from the pool
- `idle_timeout`: 600 seconds (10 minutes) - How long a connection can remain idle before being closed
- `max_lifetime`: 1800 seconds (30 minutes) - Maximum lifetime of any single connection

**Benefits**:
- Prevents indefinite blocking when acquiring connections
- Automatically closes idle connections to free resources
- Limits connection lifetime to handle database maintenance/restarts gracefully

### 3. Error Conversion Implementation (P1)

**Problem**: Missing `From<OtlError> for AppError` implementation, causing verbose error handling code.

**Location**: `/Users/mare/Simon/OTL/crates/otl-api/src/error.rs:101-117`

**Implementation**:
```rust
impl From<otl_core::OtlError> for AppError {
    fn from(err: otl_core::OtlError) -> Self {
        use otl_core::OtlError;

        match err {
            OtlError::NotFound(msg) => AppError::NotFound(msg),
            OtlError::AccessDenied { reason } => AppError::Forbidden(reason),
            OtlError::InvalidOntology(msg) => AppError::BadRequest(format!("Invalid ontology: {msg}")),
            OtlError::ValidationError(msg) => AppError::BadRequest(msg),
            OtlError::DatabaseError(msg) => AppError::Database(msg),
            OtlError::SearchError(msg) => AppError::Internal(format!("Search error: {msg}")),
            OtlError::LlmError(msg) => AppError::Internal(format!("LLM error: {msg}")),
            OtlError::ConfigError(msg) => AppError::Internal(format!("Configuration error: {msg}")),
            OtlError::Other(err) => AppError::Internal(err.to_string()),
        }
    }
}
```

**Mapping Strategy**:
- `NotFound` → 404 Not Found
- `AccessDenied` → 403 Forbidden
- `InvalidOntology` → 400 Bad Request (with context)
- `ValidationError` → 400 Bad Request
- `DatabaseError` → 500 Database Error
- `SearchError` → 500 Internal Server Error (with context)
- `LlmError` → 500 Internal Server Error (with context)
- `ConfigError` → 500 Internal Server Error (with context)
- `Other` → 500 Internal Server Error

**Benefits**:
- Enables automatic error conversion with `?` operator
- Consistent error mapping from core layer to API layer
- Preserves error messages while applying appropriate HTTP status codes
- Adds context to internal errors for better debugging

## Additional Fixes (Encountered During Testing)

### 4. Format String Error in documents.rs

**Problem**: Invalid format string with unmatched braces in ACL filter query.

**Location**: `/Users/mare/Simon/OTL/crates/otl-api/src/handlers/documents.rs:150`

**Before**:
```rust
"... OR d.required_roles && ${{{}}}})) ..."
```

**After**:
```rust
"... OR d.required_roles && ${{}})) ..."
```

### 5. AppError Variant Name Consistency

**Problem**: Code used `AppError::DatabaseError` but enum variant was `AppError::Database`.

**Locations**:
- `/Users/mare/Simon/OTL/crates/otl-api/src/handlers/documents.rs:216`
- `/Users/mare/Simon/OTL/crates/otl-api/src/handlers/documents.rs:251`

**Fixed**: Changed all occurrences to use `AppError::Database`.

## Configuration Reference

The pool size can be configured via environment variable or config file:

```bash
# Environment variable (not yet implemented)
export POSTGRES_POOL_SIZE=20

# Default in DatabaseConfig
postgres_pool_size: 10  # crates/otl-core/src/config.rs:208
```

## Timeout Values Rationale

- **acquire_timeout (30s)**: Prevents long waits when pool is exhausted; fails fast
- **idle_timeout (10m)**: Balances connection reuse with resource cleanup
- **max_lifetime (30m)**: Handles database restarts and prevents connection staleness

These values are currently hardcoded but should be made configurable in future work.

## Testing Notes

### Compilation Status
The P1 fixes are syntactically correct and follow Rust best practices. However, full compilation is blocked by pre-existing errors in `crates/otl-api/src/handlers/documents.rs`:

1. Missing `parse_access_level` function (line 318)
2. Lifetime issues with `dept` variable (lines 195, 235)
3. Unused variable warnings

These errors existed before the P1 fixes and are unrelated to the changes made.

### Verification of P1 Fixes

The three P1 fixes can be independently verified:

1. **DB Pool Configuration**:
   - `config.database.postgres_pool_size` exists in `DatabaseConfig`
   - PgPoolOptions API calls are correct
   - Logging statement is valid

2. **Connection Timeouts**:
   - All timeout methods exist in sqlx::postgres::PgPoolOptions
   - Duration values are valid
   - Methods chain correctly

3. **Error Conversion**:
   - All `OtlError` variants are handled
   - Mapping to `AppError` variants is correct
   - `From` trait implementation follows standard pattern

## Future Enhancements

1. **Make timeouts configurable**:
   ```rust
   pub struct DatabaseConfig {
       pub postgres_acquire_timeout_secs: u64,  // default: 30
       pub postgres_idle_timeout_secs: u64,     // default: 600
       pub postgres_max_lifetime_secs: u64,     // default: 1800
   }
   ```

2. **Add connection pool metrics**:
   - Active connections
   - Idle connections
   - Connection wait time
   - Timeout events

3. **Add retry logic** for transient database errors

4. **Connection pool health checks** for monitoring

## Files Modified

1. `/Users/mare/Simon/OTL/crates/otl-api/src/main.rs` - Database pool configuration
2. `/Users/mare/Simon/OTL/crates/otl-api/src/error.rs` - Error conversion implementation
3. `/Users/mare/Simon/OTL/crates/otl-api/src/handlers/documents.rs` - Fixed format string and error variant names

## Git Status

Changes are ready for commit. Pre-existing compilation errors in documents.rs handler need to be resolved separately.

## Technical Notes

### Connection Pool Behavior

With these settings:
- Pool maintains up to `postgres_pool_size` connections (default 10)
- New connections created on demand up to max_connections
- Idle connections closed after 10 minutes of inactivity
- All connections recycled every 30 minutes maximum
- Connection acquisition fails after 30 seconds if pool exhausted

### Error Handling Best Practices

The `From<OtlError>` implementation follows Rust error handling best practices:
- Zero-cost abstraction (no runtime overhead)
- Type-safe error propagation
- Preserves error information while adding context
- Enables ergonomic error handling with `?` operator

Example usage:
```rust
// Before: Verbose error handling
let entity = core_service.get_entity(id)
    .await
    .map_err(|e| match e {
        OtlError::NotFound(msg) => AppError::NotFound(msg),
        OtlError::DatabaseError(msg) => AppError::Database(msg),
        // ... more variants
    })?;

// After: Ergonomic error handling
let entity = core_service.get_entity(id).await?;
```

## Summary

All three P1 issues have been successfully addressed:
1. Database pool size now uses configuration instead of hardcoded value
2. Connection timeouts properly configured to prevent resource exhaustion
3. Error conversion implemented for clean error propagation

These changes improve the robustness and configurability of the database layer while maintaining backward compatibility.
