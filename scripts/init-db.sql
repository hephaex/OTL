-- OTL Database Initialization Script
-- This script runs automatically when PostgreSQL container starts

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ==========================================================================
-- Types
-- ==========================================================================

CREATE TYPE access_level AS ENUM (
    'public',
    'internal',
    'confidential',
    'restricted'
);

CREATE TYPE verification_status AS ENUM (
    'pending',
    'in_review',
    'approved',
    'rejected'
);

CREATE TYPE file_type AS ENUM (
    'pdf',
    'docx',
    'xlsx',
    'pptx',
    'markdown',
    'text',
    'html',
    'other'
);

-- ==========================================================================
-- Documents Table
-- ==========================================================================

CREATE TABLE documents (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    title VARCHAR(500) NOT NULL,
    file_path VARCHAR(1000) NOT NULL,
    file_type file_type NOT NULL DEFAULT 'other',
    file_size BIGINT DEFAULT 0,
    file_hash VARCHAR(64),  -- SHA-256 for deduplication
    
    -- ACL fields
    access_level access_level NOT NULL DEFAULT 'internal',
    owner_id VARCHAR(100),
    department VARCHAR(100),
    required_roles TEXT[] DEFAULT '{}',
    allowed_users TEXT[] DEFAULT '{}',
    
    -- Metadata
    metadata JSONB DEFAULT '{}',
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ,  -- When knowledge extraction completed
    
    -- Soft delete
    deleted_at TIMESTAMPTZ
);

CREATE INDEX idx_documents_access_level ON documents(access_level);
CREATE INDEX idx_documents_department ON documents(department);
CREATE INDEX idx_documents_owner ON documents(owner_id);
CREATE INDEX idx_documents_file_hash ON documents(file_hash);
CREATE INDEX idx_documents_created ON documents(created_at DESC);
CREATE INDEX idx_documents_roles ON documents USING GIN(required_roles);

-- ==========================================================================
-- Document Chunks Table
-- ==========================================================================

CREATE TABLE document_chunks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,
    content TEXT NOT NULL,
    content_hash VARCHAR(64),  -- For deduplication
    
    -- Location info
    page_number INTEGER,
    section_name VARCHAR(200),
    start_offset INTEGER,
    end_offset INTEGER,
    
    -- Vector store reference
    vector_id VARCHAR(100),  -- Qdrant point ID
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    UNIQUE(document_id, chunk_index)
);

CREATE INDEX idx_chunks_document ON document_chunks(document_id);
CREATE INDEX idx_chunks_vector ON document_chunks(vector_id);

-- ==========================================================================
-- Extraction Queue Table (for HITL verification)
-- ==========================================================================

CREATE TABLE extraction_queue (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    
    -- Extracted data
    extracted_entities JSONB NOT NULL DEFAULT '[]',
    extracted_relations JSONB NOT NULL DEFAULT '[]',
    source_text TEXT,
    
    -- Confidence
    confidence_score REAL DEFAULT 0.0,
    
    -- Verification status
    status verification_status NOT NULL DEFAULT 'pending',
    reviewer_id VARCHAR(100),
    review_notes TEXT,
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    reviewed_at TIMESTAMPTZ,
    
    -- Priority (lower = higher priority)
    priority INTEGER DEFAULT 100
);

CREATE INDEX idx_extraction_status ON extraction_queue(status);
CREATE INDEX idx_extraction_priority ON extraction_queue(priority, created_at);
CREATE INDEX idx_extraction_document ON extraction_queue(document_id);
CREATE INDEX idx_extraction_reviewer ON extraction_queue(reviewer_id);

-- ==========================================================================
-- Users Table (for ACL reference)
-- ==========================================================================

CREATE TABLE users (
    id VARCHAR(100) PRIMARY KEY,
    email VARCHAR(255) UNIQUE,
    display_name VARCHAR(200),
    roles TEXT[] DEFAULT '{}',
    departments TEXT[] DEFAULT '{}',
    is_internal BOOLEAN DEFAULT TRUE,
    is_active BOOLEAN DEFAULT TRUE,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_login_at TIMESTAMPTZ
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_roles ON users USING GIN(roles);
CREATE INDEX idx_users_departments ON users USING GIN(departments);

-- ==========================================================================
-- Audit Log Table
-- ==========================================================================

CREATE TABLE audit_logs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id VARCHAR(100),
    action VARCHAR(50) NOT NULL,  -- 'query', 'upload', 'verify', 'delete', etc.
    resource_type VARCHAR(50),    -- 'document', 'extraction', etc.
    resource_id UUID,
    details JSONB DEFAULT '{}',
    ip_address INET,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_user ON audit_logs(user_id);
CREATE INDEX idx_audit_action ON audit_logs(action);
CREATE INDEX idx_audit_resource ON audit_logs(resource_type, resource_id);
CREATE INDEX idx_audit_created ON audit_logs(created_at DESC);

-- ==========================================================================
-- Query Statistics Table (for monitoring)
-- ==========================================================================

CREATE TABLE query_stats (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id VARCHAR(100),
    query_text TEXT NOT NULL,
    
    -- Performance metrics
    total_time_ms INTEGER,
    vector_search_time_ms INTEGER,
    graph_search_time_ms INTEGER,
    llm_time_ms INTEGER,
    
    -- Result metrics
    num_results INTEGER,
    top_score REAL,
    confidence REAL,
    
    -- Context
    filters JSONB DEFAULT '{}',
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_query_stats_user ON query_stats(user_id);
CREATE INDEX idx_query_stats_created ON query_stats(created_at DESC);

-- ==========================================================================
-- Helper Functions
-- ==========================================================================

-- Update timestamp trigger
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER documents_updated_at
    BEFORE UPDATE ON documents
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- ==========================================================================
-- Initial Data
-- ==========================================================================

-- Create admin user
INSERT INTO users (id, email, display_name, roles, departments, is_internal)
VALUES (
    'admin',
    'admin@example.com',
    'System Administrator',
    ARRAY['admin', 'hr_admin', 'it_admin'],
    ARRAY['IT'],
    TRUE
);

-- Create test users
INSERT INTO users (id, email, display_name, roles, departments, is_internal)
VALUES 
    ('user1', 'user1@example.com', 'Test User 1', ARRAY['employee'], ARRAY['Engineering'], TRUE),
    ('user2', 'user2@example.com', 'Test User 2', ARRAY['employee', 'hr_viewer'], ARRAY['HR'], TRUE),
    ('guest', 'guest@example.com', 'Guest User', ARRAY[]::TEXT[], ARRAY[]::TEXT[], FALSE);

-- ==========================================================================
-- Views
-- ==========================================================================

-- Pending verifications view
CREATE VIEW pending_verifications AS
SELECT 
    eq.id,
    eq.document_id,
    d.title AS document_title,
    eq.confidence_score,
    eq.status,
    eq.priority,
    eq.created_at,
    jsonb_array_length(eq.extracted_entities) AS entity_count,
    jsonb_array_length(eq.extracted_relations) AS relation_count
FROM extraction_queue eq
JOIN documents d ON eq.document_id = d.id
WHERE eq.status = 'pending'
ORDER BY eq.priority, eq.created_at;

-- Document statistics view
CREATE VIEW document_stats AS
SELECT 
    d.id,
    d.title,
    d.file_type,
    d.access_level,
    COUNT(dc.id) AS chunk_count,
    COUNT(eq.id) FILTER (WHERE eq.status = 'approved') AS approved_extractions,
    COUNT(eq.id) FILTER (WHERE eq.status = 'pending') AS pending_extractions
FROM documents d
LEFT JOIN document_chunks dc ON d.id = dc.document_id
LEFT JOIN extraction_queue eq ON d.id = eq.document_id
WHERE d.deleted_at IS NULL
GROUP BY d.id;

COMMENT ON DATABASE otl IS 'OTL Knowledge System Database';
