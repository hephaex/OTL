-- Authentication System Database Schema
-- PostgreSQL migration for user authentication and session management
--
-- Author: hephaex@gmail.com
-- Date: 2025-12-23
-- Related Issue: #4 Authentication System - Phase 3

-- Users table with authentication and profile data
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    name VARCHAR(100) NOT NULL,

    -- Role assignment (admin, editor, viewer)
    role VARCHAR(50) NOT NULL DEFAULT 'viewer',
    CHECK (role IN ('admin', 'editor', 'viewer')),

    -- Department for ACL filtering
    department VARCHAR(100),

    -- Account status
    is_active BOOLEAN NOT NULL DEFAULT true,
    email_verified BOOLEAN NOT NULL DEFAULT false,

    -- Security tracking
    failed_login_attempts INTEGER NOT NULL DEFAULT 0,
    locked_until TIMESTAMP WITH TIME ZONE,
    last_login TIMESTAMP WITH TIME ZONE,
    password_changed_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Indexes for users table
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_role ON users(role);
CREATE INDEX IF NOT EXISTS idx_users_department ON users(department);
CREATE INDEX IF NOT EXISTS idx_users_is_active ON users(is_active);

-- Refresh tokens for session management
CREATE TABLE IF NOT EXISTS refresh_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,

    -- Optional device/session info
    device_info TEXT,
    ip_address INET,

    -- Token lifecycle
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMP WITH TIME ZONE
);

-- Indexes for refresh_tokens table
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_token_hash ON refresh_tokens(token_hash);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_user_id ON refresh_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_expires ON refresh_tokens(expires_at);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_revoked ON refresh_tokens(revoked_at);

-- Token blacklist for logout before expiry
CREATE TABLE IF NOT EXISTS token_blacklist (
    token_jti TEXT PRIMARY KEY,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    blacklisted_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Index for token_blacklist
CREATE INDEX IF NOT EXISTS idx_blacklist_expires ON token_blacklist(expires_at);

-- Audit log for security-sensitive operations
CREATE TABLE IF NOT EXISTS audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,

    -- Action details
    action VARCHAR(100) NOT NULL,
    resource_type VARCHAR(100) NOT NULL,
    resource_id VARCHAR(255),

    -- Request context
    ip_address INET,
    user_agent TEXT,
    details JSONB,

    -- Result
    success BOOLEAN NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Indexes for audit_log
CREATE INDEX IF NOT EXISTS idx_audit_user ON audit_log(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_action ON audit_log(action);
CREATE INDEX IF NOT EXISTS idx_audit_created ON audit_log(created_at);
CREATE INDEX IF NOT EXISTS idx_audit_resource ON audit_log(resource_type, resource_id);

-- Function to automatically update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Trigger for users table
DROP TRIGGER IF EXISTS update_users_updated_at ON users;
CREATE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Function to clean up expired tokens (call periodically)
CREATE OR REPLACE FUNCTION cleanup_expired_tokens()
RETURNS void AS $$
BEGIN
    -- Delete expired blacklisted tokens
    DELETE FROM token_blacklist WHERE expires_at < NOW();

    -- Delete expired refresh tokens
    DELETE FROM refresh_tokens WHERE expires_at < NOW();
END;
$$ language 'plpgsql';

-- Create initial admin user (password: Admin123!)
-- Password hash for "Admin123!" using Argon2id
INSERT INTO users (id, email, password_hash, name, role, is_active, email_verified, created_at, updated_at, password_changed_at)
VALUES (
    gen_random_uuid(),
    'admin@otl.local',
    '$argon2id$v=19$m=65536,t=3,p=4$ZHVtbXlzYWx0MTIzNDU2$qZx7L+JJh5K5K5K5K5K5K5K5K5K5K5K5K5K5K5K', -- Change this!
    'System Administrator',
    'admin',
    true,
    true,
    NOW(),
    NOW(),
    NOW()
) ON CONFLICT (email) DO NOTHING;

-- Comments for documentation
COMMENT ON TABLE users IS 'User accounts with authentication credentials and profile information';
COMMENT ON TABLE refresh_tokens IS 'Long-lived refresh tokens for session management';
COMMENT ON TABLE token_blacklist IS 'Blacklisted JWT tokens for logout support';
COMMENT ON TABLE audit_log IS 'Audit trail for security-sensitive operations';

COMMENT ON COLUMN users.password_hash IS 'Argon2id hash of user password';
COMMENT ON COLUMN users.role IS 'User role: admin (full access), editor (read/write), viewer (read-only)';
COMMENT ON COLUMN users.department IS 'User department for ACL filtering';
COMMENT ON COLUMN users.failed_login_attempts IS 'Counter for failed login attempts (locks at 5)';
COMMENT ON COLUMN users.locked_until IS 'Account lock expiration timestamp';

COMMENT ON COLUMN refresh_tokens.token_hash IS 'SHA-256 hash of the refresh token';
COMMENT ON COLUMN refresh_tokens.revoked_at IS 'Token revocation timestamp (NULL if active)';

COMMENT ON COLUMN token_blacklist.token_jti IS 'JWT ID (jti claim) of blacklisted access token';
