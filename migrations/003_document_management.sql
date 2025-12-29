-- Document Management Enhancements
-- PostgreSQL migration for document versioning, collections, tags, and lineage
--
-- Author: hephaex@gmail.com
-- Date: 2025-12-29
-- Related Issue: #8 Document Management Enhancements

-- ==========================================================================
-- Document Versions Table
-- ==========================================================================

CREATE TABLE IF NOT EXISTS document_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    version_number INTEGER NOT NULL,

    -- Version metadata
    title VARCHAR(500) NOT NULL,
    file_path VARCHAR(1000) NOT NULL,
    file_size BIGINT DEFAULT 0,
    file_hash VARCHAR(64),  -- SHA-256 for content comparison

    -- Change tracking
    change_summary TEXT,
    changed_by UUID REFERENCES users(id) ON DELETE SET NULL,
    parent_version_id UUID REFERENCES document_versions(id) ON DELETE SET NULL,

    -- Content snapshot
    content_snapshot TEXT,  -- Full text snapshot for diff
    metadata_snapshot JSONB DEFAULT '{}',

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(document_id, version_number)
);

CREATE INDEX idx_doc_versions_document ON document_versions(document_id, version_number DESC);
CREATE INDEX idx_doc_versions_created ON document_versions(created_at DESC);
CREATE INDEX idx_doc_versions_changed_by ON document_versions(changed_by);
CREATE INDEX idx_doc_versions_parent ON document_versions(parent_version_id);

COMMENT ON TABLE document_versions IS 'Version history for document tracking';
COMMENT ON COLUMN document_versions.version_number IS 'Sequential version number starting from 1';
COMMENT ON COLUMN document_versions.content_snapshot IS 'Full text content for version comparison';

-- ==========================================================================
-- Document Collections/Folders Table
-- ==========================================================================

CREATE TABLE IF NOT EXISTS document_collections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    parent_id UUID REFERENCES document_collections(id) ON DELETE CASCADE,

    -- ACL (inherited by documents unless overridden)
    access_level access_level NOT NULL DEFAULT 'internal',
    owner_id UUID REFERENCES users(id) ON DELETE SET NULL,
    department VARCHAR(100),

    -- Metadata
    metadata JSONB DEFAULT '{}',

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ,

    -- Tree path for efficient hierarchical queries
    path VARCHAR(1000)  -- e.g., '/root/folder1/subfolder2'
);

CREATE INDEX idx_collections_parent ON document_collections(parent_id);
CREATE INDEX idx_collections_path ON document_collections(path);
CREATE INDEX idx_collections_owner ON document_collections(owner_id);
CREATE INDEX idx_collections_deleted ON document_collections(deleted_at);

COMMENT ON TABLE document_collections IS 'Hierarchical folders/collections for document organization';
COMMENT ON COLUMN document_collections.path IS 'Full path for efficient tree queries';

-- ==========================================================================
-- Document Collection Membership Table
-- ==========================================================================

CREATE TABLE IF NOT EXISTS document_collection_members (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    collection_id UUID NOT NULL REFERENCES document_collections(id) ON DELETE CASCADE,

    -- Allow documents in multiple collections
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    added_by UUID REFERENCES users(id) ON DELETE SET NULL,

    UNIQUE(document_id, collection_id)
);

CREATE INDEX idx_collection_members_document ON document_collection_members(document_id);
CREATE INDEX idx_collection_members_collection ON document_collection_members(collection_id);

-- ==========================================================================
-- Document Tags Table
-- ==========================================================================

CREATE TABLE IF NOT EXISTS document_tags (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL UNIQUE,
    category VARCHAR(50),  -- e.g., 'topic', 'language', 'department', 'auto'
    description TEXT,
    color VARCHAR(7),  -- Hex color for UI

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tags_category ON document_tags(category);
CREATE INDEX idx_tags_name ON document_tags(name);

COMMENT ON TABLE document_tags IS 'Reusable tags for document classification';
COMMENT ON COLUMN document_tags.category IS 'Tag category for organization';

-- ==========================================================================
-- Document Tag Assignments Table
-- ==========================================================================

CREATE TABLE IF NOT EXISTS document_tag_assignments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    tag_id UUID NOT NULL REFERENCES document_tags(id) ON DELETE CASCADE,

    -- Auto-tagging metadata
    auto_assigned BOOLEAN DEFAULT FALSE,
    confidence REAL,  -- 0.0 to 1.0 for auto-assigned tags
    assigned_by UUID REFERENCES users(id) ON DELETE SET NULL,

    assigned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(document_id, tag_id)
);

CREATE INDEX idx_tag_assignments_document ON document_tag_assignments(document_id);
CREATE INDEX idx_tag_assignments_tag ON document_tag_assignments(tag_id);
CREATE INDEX idx_tag_assignments_auto ON document_tag_assignments(auto_assigned);

COMMENT ON TABLE document_tag_assignments IS 'Many-to-many relationship between documents and tags';

-- ==========================================================================
-- Document Processing Lineage Table
-- ==========================================================================

CREATE TABLE IF NOT EXISTS document_lineage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    version_id UUID REFERENCES document_versions(id) ON DELETE SET NULL,

    -- Processing step details
    step_name VARCHAR(100) NOT NULL,  -- e.g., 'upload', 'parse', 'chunk', 'embed', 'extract'
    step_type VARCHAR(50) NOT NULL,   -- e.g., 'ingestion', 'processing', 'extraction'
    status VARCHAR(50) NOT NULL,      -- e.g., 'pending', 'running', 'completed', 'failed'

    -- Execution details
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    duration_ms INTEGER,

    -- Results and metrics
    input_data JSONB DEFAULT '{}',
    output_data JSONB DEFAULT '{}',
    error_message TEXT,
    metrics JSONB DEFAULT '{}',  -- e.g., chunk_count, entity_count, etc.

    -- Traceability
    triggered_by UUID REFERENCES users(id) ON DELETE SET NULL,
    parent_step_id UUID REFERENCES document_lineage(id) ON DELETE SET NULL
);

CREATE INDEX idx_lineage_document ON document_lineage(document_id, started_at DESC);
CREATE INDEX idx_lineage_version ON document_lineage(version_id);
CREATE INDEX idx_lineage_status ON document_lineage(status);
CREATE INDEX idx_lineage_step ON document_lineage(step_name, step_type);
CREATE INDEX idx_lineage_parent ON document_lineage(parent_step_id);

COMMENT ON TABLE document_lineage IS 'Processing history and lineage tracking for documents';
COMMENT ON COLUMN document_lineage.metrics IS 'Step-specific metrics and counters';

-- ==========================================================================
-- Document Retention Policy Table
-- ==========================================================================

CREATE TABLE IF NOT EXISTS document_retention_policies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL UNIQUE,
    description TEXT,

    -- Retention settings
    retention_days INTEGER NOT NULL,  -- Days to keep after deletion
    auto_purge BOOLEAN DEFAULT FALSE, -- Automatically purge after retention period

    -- Applicability
    applies_to_access_level access_level[],
    applies_to_departments VARCHAR(100)[],
    applies_to_collections UUID[],

    -- Priority (higher number = higher priority when multiple policies match)
    priority INTEGER DEFAULT 0,

    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_retention_active ON document_retention_policies(is_active);
CREATE INDEX idx_retention_priority ON document_retention_policies(priority DESC);

COMMENT ON TABLE document_retention_policies IS 'Configurable retention policies for deleted documents';

-- ==========================================================================
-- Alter existing documents table
-- ==========================================================================

-- Add collection reference (primary collection)
ALTER TABLE documents ADD COLUMN IF NOT EXISTS primary_collection_id UUID REFERENCES document_collections(id) ON DELETE SET NULL;

-- Add version tracking
ALTER TABLE documents ADD COLUMN IF NOT EXISTS current_version INTEGER DEFAULT 1;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS latest_version_id UUID REFERENCES document_versions(id) ON DELETE SET NULL;

-- Add language detection
ALTER TABLE documents ADD COLUMN IF NOT EXISTS detected_language VARCHAR(10);
ALTER TABLE documents ADD COLUMN IF NOT EXISTS language_confidence REAL;

-- Add retention policy reference
ALTER TABLE documents ADD COLUMN IF NOT EXISTS retention_policy_id UUID REFERENCES document_retention_policies(id) ON DELETE SET NULL;

-- Add purge date for deleted documents
ALTER TABLE documents ADD COLUMN IF NOT EXISTS purge_at TIMESTAMPTZ;

CREATE INDEX idx_documents_collection ON documents(primary_collection_id);
CREATE INDEX idx_documents_version ON documents(current_version);
CREATE INDEX idx_documents_language ON documents(detected_language);
CREATE INDEX idx_documents_purge ON documents(purge_at) WHERE purge_at IS NOT NULL;

-- ==========================================================================
-- Triggers
-- ==========================================================================

-- Update collection updated_at trigger
CREATE TRIGGER collections_updated_at
    BEFORE UPDATE ON document_collections
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Update retention policies updated_at trigger
CREATE TRIGGER retention_policies_updated_at
    BEFORE UPDATE ON document_retention_policies
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Auto-calculate purge_at on document deletion
CREATE OR REPLACE FUNCTION calculate_purge_date()
RETURNS TRIGGER AS $$
DECLARE
    retention_days INTEGER;
BEGIN
    IF NEW.deleted_at IS NOT NULL AND OLD.deleted_at IS NULL THEN
        -- Document was just deleted, calculate purge date

        -- First, try to get retention from assigned policy
        IF NEW.retention_policy_id IS NOT NULL THEN
            SELECT rp.retention_days INTO retention_days
            FROM document_retention_policies rp
            WHERE rp.id = NEW.retention_policy_id AND rp.is_active = TRUE;
        END IF;

        -- If no explicit policy, find matching policy by rules
        IF retention_days IS NULL THEN
            SELECT rp.retention_days INTO retention_days
            FROM document_retention_policies rp
            WHERE rp.is_active = TRUE
                AND (
                    NEW.access_level = ANY(rp.applies_to_access_level)
                    OR NEW.department = ANY(rp.applies_to_departments)
                    OR NEW.primary_collection_id = ANY(rp.applies_to_collections)
                )
            ORDER BY rp.priority DESC
            LIMIT 1;
        END IF;

        -- Default retention: 90 days
        retention_days := COALESCE(retention_days, 90);

        NEW.purge_at := NEW.deleted_at + (retention_days || ' days')::INTERVAL;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER document_purge_date_trigger
    BEFORE UPDATE ON documents
    FOR EACH ROW
    EXECUTE FUNCTION calculate_purge_date();

-- ==========================================================================
-- Helper Functions
-- ==========================================================================

-- Create a new document version
CREATE OR REPLACE FUNCTION create_document_version(
    p_document_id UUID,
    p_title VARCHAR(500),
    p_file_path VARCHAR(1000),
    p_file_size BIGINT,
    p_file_hash VARCHAR(64),
    p_content TEXT,
    p_change_summary TEXT,
    p_changed_by UUID
)
RETURNS UUID AS $$
DECLARE
    v_version_number INTEGER;
    v_version_id UUID;
    v_parent_version_id UUID;
BEGIN
    -- Get current version number
    SELECT current_version, latest_version_id
    INTO v_version_number, v_parent_version_id
    FROM documents
    WHERE id = p_document_id;

    -- Increment version
    v_version_number := v_version_number + 1;

    -- Create version record
    INSERT INTO document_versions (
        document_id, version_number, title, file_path, file_size, file_hash,
        change_summary, changed_by, parent_version_id, content_snapshot
    ) VALUES (
        p_document_id, v_version_number, p_title, p_file_path, p_file_size, p_file_hash,
        p_change_summary, p_changed_by, v_parent_version_id, p_content
    ) RETURNING id INTO v_version_id;

    -- Update document's current version
    UPDATE documents
    SET current_version = v_version_number,
        latest_version_id = v_version_id,
        updated_at = NOW()
    WHERE id = p_document_id;

    RETURN v_version_id;
END;
$$ LANGUAGE plpgsql;

-- Get documents pending purge
CREATE OR REPLACE FUNCTION get_documents_pending_purge()
RETURNS TABLE (
    document_id UUID,
    title VARCHAR(500),
    deleted_at TIMESTAMPTZ,
    purge_at TIMESTAMPTZ,
    days_until_purge INTEGER
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        d.id AS document_id,
        d.title,
        d.deleted_at,
        d.purge_at,
        EXTRACT(DAY FROM (d.purge_at - NOW()))::INTEGER AS days_until_purge
    FROM documents d
    WHERE d.deleted_at IS NOT NULL
        AND d.purge_at IS NOT NULL
        AND d.purge_at <= NOW() + INTERVAL '7 days'
    ORDER BY d.purge_at;
END;
$$ LANGUAGE plpgsql;

-- Purge expired documents
CREATE OR REPLACE FUNCTION purge_expired_documents()
RETURNS TABLE (
    purged_count INTEGER,
    purged_ids UUID[]
) AS $$
DECLARE
    v_ids UUID[];
BEGIN
    -- Collect IDs to purge
    SELECT ARRAY_AGG(id) INTO v_ids
    FROM documents
    WHERE deleted_at IS NOT NULL
        AND purge_at IS NOT NULL
        AND purge_at <= NOW();

    -- Hard delete documents (CASCADE will delete related records)
    DELETE FROM documents
    WHERE id = ANY(v_ids);

    RETURN QUERY SELECT COALESCE(array_length(v_ids, 1), 0), COALESCE(v_ids, ARRAY[]::UUID[]);
END;
$$ LANGUAGE plpgsql;

-- ==========================================================================
-- Views
-- ==========================================================================

-- Document with latest version info
CREATE OR REPLACE VIEW documents_with_versions AS
SELECT
    d.*,
    dv.version_number AS latest_version,
    dv.created_at AS version_created_at,
    dv.changed_by AS last_changed_by,
    dv.change_summary AS last_change_summary
FROM documents d
LEFT JOIN document_versions dv ON d.latest_version_id = dv.id
WHERE d.deleted_at IS NULL;

-- Document with tags
CREATE OR REPLACE VIEW documents_with_tags AS
SELECT
    d.id AS document_id,
    d.title,
    d.file_type,
    d.access_level,
    d.created_at,
    ARRAY_AGG(dt.name) FILTER (WHERE dt.name IS NOT NULL) AS tag_names,
    ARRAY_AGG(dt.id) FILTER (WHERE dt.id IS NOT NULL) AS tag_ids,
    COUNT(dt.id) AS tag_count
FROM documents d
LEFT JOIN document_tag_assignments dta ON d.id = dta.document_id
LEFT JOIN document_tags dt ON dta.tag_id = dt.id
WHERE d.deleted_at IS NULL
GROUP BY d.id, d.title, d.file_type, d.access_level, d.created_at;

-- Collection tree view
CREATE OR REPLACE VIEW collection_tree AS
WITH RECURSIVE tree AS (
    -- Base case: root collections
    SELECT
        id,
        name,
        parent_id,
        path,
        0 AS depth,
        ARRAY[id] AS id_path
    FROM document_collections
    WHERE parent_id IS NULL AND deleted_at IS NULL

    UNION ALL

    -- Recursive case: child collections
    SELECT
        c.id,
        c.name,
        c.parent_id,
        c.path,
        t.depth + 1,
        t.id_path || c.id
    FROM document_collections c
    INNER JOIN tree t ON c.parent_id = t.id
    WHERE c.deleted_at IS NULL
)
SELECT * FROM tree ORDER BY id_path;

-- ==========================================================================
-- Initial Data
-- ==========================================================================

-- Default retention policies
INSERT INTO document_retention_policies (name, description, retention_days, auto_purge, applies_to_access_level, priority)
VALUES
    ('Public Documents', 'Short retention for public documents', 30, TRUE, ARRAY['public']::access_level[], 10),
    ('Internal Documents', 'Standard retention for internal documents', 90, FALSE, ARRAY['internal']::access_level[], 20),
    ('Confidential Documents', 'Extended retention for confidential documents', 180, FALSE, ARRAY['confidential']::access_level[], 30),
    ('Restricted Documents', 'Long-term retention for restricted documents', 365, FALSE, ARRAY['restricted']::access_level[], 40)
ON CONFLICT (name) DO NOTHING;

-- Default tags
INSERT INTO document_tags (name, category, description, color)
VALUES
    ('korean', 'language', 'Korean language document', '#FF6B6B'),
    ('english', 'language', 'English language document', '#4ECDC4'),
    ('hr', 'department', 'Human Resources', '#45B7D1'),
    ('it', 'department', 'Information Technology', '#96CEB4'),
    ('legal', 'department', 'Legal', '#FFEAA7'),
    ('finance', 'department', 'Finance', '#DFE6E9'),
    ('policy', 'topic', 'Company Policy', '#A29BFE'),
    ('manual', 'topic', 'User Manual', '#FD79A8'),
    ('contract', 'topic', 'Contract or Agreement', '#FDCB6E')
ON CONFLICT (name) DO NOTHING;

-- Root collections
INSERT INTO document_collections (id, name, description, path, access_level)
VALUES
    (gen_random_uuid(), 'Root', 'Root collection', '/', 'internal'),
    (gen_random_uuid(), 'HR Documents', 'Human Resources documents', '/hr', 'confidential'),
    (gen_random_uuid(), 'IT Documentation', 'IT manuals and guides', '/it', 'internal'),
    (gen_random_uuid(), 'Policies', 'Company policies and procedures', '/policies', 'internal'),
    (gen_random_uuid(), 'Public Resources', 'Publicly accessible resources', '/public', 'public')
ON CONFLICT DO NOTHING;

-- ==========================================================================
-- Comments
-- ==========================================================================

COMMENT ON FUNCTION create_document_version IS 'Create a new version of a document with change tracking';
COMMENT ON FUNCTION get_documents_pending_purge IS 'Get list of documents approaching purge date';
COMMENT ON FUNCTION purge_expired_documents IS 'Hard delete documents past their retention period';

COMMENT ON VIEW documents_with_versions IS 'Documents with their latest version information';
COMMENT ON VIEW documents_with_tags IS 'Documents with aggregated tag information';
COMMENT ON VIEW collection_tree IS 'Hierarchical view of collection structure';
