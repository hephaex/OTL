# Document Management API

This document describes the enhanced document management features added in Issue #8.

## Overview

The document management system has been enhanced with the following capabilities:

- **Versioning**: Track document changes over time with full version history
- **Bulk Operations**: Upload/delete multiple documents in a single request
- **Metadata Enrichment**: Auto-tagging and language detection
- **Collections/Folders**: Organize documents hierarchically
- **Lineage Tracking**: Monitor processing pipeline and history
- **Content Diff**: Compare document versions
- **Soft Delete**: Configurable retention policies for deleted documents

## API Endpoints

### Bulk Operations

#### Bulk Upload Documents

```http
POST /api/v1/documents/bulk
```

Upload multiple documents in a single request with parallel processing.

**Request Body:**
```json
{
  "documents": [
    {
      "title": "신규입사자_안내서.pdf",
      "content": "<base64 encoded content>",
      "file_type": "pdf",
      "access_level": "internal",
      "department": "인사팀",
      "collection_id": "550e8400-e29b-41d4-a716-446655440000",
      "tags": ["hr", "onboarding"]
    }
  ],
  "parallel_limit": 4
}
```

**Response:**
```json
{
  "total": 10,
  "successful": 9,
  "failed": 1,
  "results": [
    {
      "title": "document1.pdf",
      "success": true,
      "id": "550e8400-e29b-41d4-a716-446655440001",
      "chunk_count": 45,
      "error": null
    }
  ],
  "processing_time_ms": 15230
}
```

**Limits:**
- Maximum 100 documents per request
- Maximum 50MB per document
- Parallel limit: 1-8 (default: 4)

#### Bulk Delete Documents

```http
DELETE /api/v1/documents/bulk
```

Delete multiple documents in a single request.

**Request Body:**
```json
{
  "document_ids": [
    "550e8400-e29b-41d4-a716-446655440001",
    "550e8400-e29b-41d4-a716-446655440002"
  ]
}
```

**Response:**
```json
{
  "total": 2,
  "successful": 2,
  "failed": 0,
  "results": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440001",
      "success": true,
      "error": null
    }
  ]
}
```

### Document Versioning

#### Get Version History

```http
GET /api/v1/documents/{id}/versions
```

Retrieve the complete version history for a document.

**Response:**
```json
{
  "document_id": "550e8400-e29b-41d4-a716-446655440000",
  "current_version": 3,
  "total_versions": 3,
  "versions": [
    {
      "id": "650e8400-e29b-41d4-a716-446655440003",
      "version_number": 3,
      "title": "Employee Handbook v3.0",
      "file_size": 2048576,
      "file_hash": "sha256:abc123...",
      "change_summary": "Updated vacation policy",
      "changed_by": "750e8400-e29b-41d4-a716-446655440000",
      "created_at": "2025-12-29T10:30:00Z"
    }
  ]
}
```

#### Compare Versions

```http
GET /api/v1/documents/{id}/versions/{old_version}/diff/{new_version}
```

Generate a diff between two document versions.

**Response:**
```json
{
  "old_version": 1,
  "new_version": 2,
  "additions": 15,
  "deletions": 8,
  "modifications": 5,
  "changes": [
    {
      "change_type": "add",
      "old_content": null,
      "new_content": "New paragraph about remote work policy",
      "old_line": null,
      "new_line": 45
    },
    {
      "change_type": "delete",
      "old_content": "Outdated section",
      "new_content": null,
      "old_line": 32,
      "new_line": null
    },
    {
      "change_type": "modify",
      "old_content": "Previous text",
      "new_content": "Updated text",
      "old_line": 10,
      "new_line": 10
    }
  ],
  "truncated": false
}
```

#### Create New Version

```http
POST /api/v1/documents/{id}/versions
```

Create a new version of a document.

**Request Body:**
```json
{
  "change_summary": "Updated Q4 2025 benefits",
  "content": "<base64 encoded new content>",
  "file_hash": "sha256:def456..."
}
```

### Document Tags

#### Get Document Tags

```http
GET /api/v1/documents/{id}/tags
```

Retrieve all tags assigned to a document.

**Response:**
```json
[
  {
    "tag_id": "850e8400-e29b-41d4-a716-446655440000",
    "tag_name": "korean",
    "confidence": 0.95,
    "auto_assigned": true
  },
  {
    "tag_id": "850e8400-e29b-41d4-a716-446655440001",
    "tag_name": "hr",
    "confidence": 1.0,
    "auto_assigned": false
  }
]
```

#### Auto-Tag Document

```http
POST /api/v1/documents/{id}/tags/auto
```

Automatically detect and assign tags based on content analysis.

**Request Body:**
```json
{
  "min_confidence": 0.7
}
```

**Response:**
```json
{
  "document_id": "550e8400-e29b-41d4-a716-446655440000",
  "tags_assigned": [
    {
      "tag_id": "850e8400-e29b-41d4-a716-446655440000",
      "tag_name": "korean",
      "confidence": 0.95,
      "auto_assigned": true
    }
  ],
  "detected_language": {
    "code": "ko",
    "name": "Korean",
    "confidence": 0.95
  }
}
```

#### Assign Tags Manually

```http
POST /api/v1/documents/{id}/tags
```

Manually assign tags to a document.

**Request Body:**
```json
{
  "tag_ids": [
    "850e8400-e29b-41d4-a716-446655440001",
    "850e8400-e29b-41d4-a716-446655440002"
  ]
}
```

### Document Collections

#### List Collections

```http
GET /api/v1/collections?parent_id={parent_id}&include_deleted=false
```

List all collections, optionally filtered by parent.

**Response:**
```json
{
  "total": 5,
  "collections": [
    {
      "id": "950e8400-e29b-41d4-a716-446655440000",
      "name": "HR Documents",
      "description": "Human Resources documents",
      "parent_id": null,
      "path": "/hr",
      "access_level": "confidential",
      "owner_id": "750e8400-e29b-41d4-a716-446655440000",
      "department": "HR",
      "document_count": 42,
      "created_at": "2025-01-01T00:00:00Z",
      "updated_at": "2025-12-29T10:00:00Z"
    }
  ]
}
```

#### Create Collection

```http
POST /api/v1/collections
```

Create a new document collection.

**Request Body:**
```json
{
  "name": "Employee Policies",
  "description": "All employee policy documents",
  "parent_id": "950e8400-e29b-41d4-a716-446655440000",
  "access_level": "internal",
  "department": "HR"
}
```

#### Add Documents to Collection

```http
POST /api/v1/collections/{id}/documents
```

Add multiple documents to a collection.

**Request Body:**
```json
{
  "document_ids": [
    "550e8400-e29b-41d4-a716-446655440001",
    "550e8400-e29b-41d4-a716-446655440002"
  ]
}
```

### Document Lineage

#### Get Processing Lineage

```http
GET /api/v1/documents/{id}/lineage
```

Retrieve the complete processing history for a document.

**Response:**
```json
{
  "document_id": "550e8400-e29b-41d4-a716-446655440000",
  "total_steps": 5,
  "latest_status": "completed",
  "history": [
    {
      "id": "a50e8400-e29b-41d4-a716-446655440000",
      "step_name": "upload",
      "step_type": "ingestion",
      "status": "completed",
      "started_at": "2025-12-29T10:00:00Z",
      "completed_at": "2025-12-29T10:00:01Z",
      "duration_ms": 1000,
      "error_message": null,
      "metrics": {
        "file_size": 2048576
      },
      "triggered_by": "750e8400-e29b-41d4-a716-446655440000"
    },
    {
      "id": "a50e8400-e29b-41d4-a716-446655440001",
      "step_name": "parse",
      "step_type": "processing",
      "status": "completed",
      "started_at": "2025-12-29T10:00:01Z",
      "completed_at": "2025-12-29T10:00:05Z",
      "duration_ms": 4000,
      "error_message": null,
      "metrics": {
        "page_count": 45,
        "word_count": 8500
      },
      "triggered_by": null
    }
  ]
}
```

## Database Schema

### Document Versions

```sql
CREATE TABLE document_versions (
    id UUID PRIMARY KEY,
    document_id UUID NOT NULL REFERENCES documents(id),
    version_number INTEGER NOT NULL,
    title VARCHAR(500),
    file_path VARCHAR(1000),
    file_size BIGINT,
    file_hash VARCHAR(64),
    change_summary TEXT,
    changed_by UUID REFERENCES users(id),
    parent_version_id UUID REFERENCES document_versions(id),
    content_snapshot TEXT,
    metadata_snapshot JSONB,
    created_at TIMESTAMPTZ NOT NULL,
    UNIQUE(document_id, version_number)
);
```

### Document Collections

```sql
CREATE TABLE document_collections (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    parent_id UUID REFERENCES document_collections(id),
    access_level access_level NOT NULL,
    owner_id UUID REFERENCES users(id),
    department VARCHAR(100),
    metadata JSONB,
    path VARCHAR(1000),
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    deleted_at TIMESTAMPTZ
);
```

### Document Tags

```sql
CREATE TABLE document_tags (
    id UUID PRIMARY KEY,
    name VARCHAR(100) NOT NULL UNIQUE,
    category VARCHAR(50),
    description TEXT,
    color VARCHAR(7),
    created_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE document_tag_assignments (
    id UUID PRIMARY KEY,
    document_id UUID NOT NULL REFERENCES documents(id),
    tag_id UUID NOT NULL REFERENCES document_tags(id),
    auto_assigned BOOLEAN DEFAULT FALSE,
    confidence REAL,
    assigned_by UUID REFERENCES users(id),
    assigned_at TIMESTAMPTZ NOT NULL,
    UNIQUE(document_id, tag_id)
);
```

### Document Lineage

```sql
CREATE TABLE document_lineage (
    id UUID PRIMARY KEY,
    document_id UUID NOT NULL REFERENCES documents(id),
    version_id UUID REFERENCES document_versions(id),
    step_name VARCHAR(100) NOT NULL,
    step_type VARCHAR(50) NOT NULL,
    status VARCHAR(50) NOT NULL,
    started_at TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ,
    duration_ms INTEGER,
    input_data JSONB,
    output_data JSONB,
    error_message TEXT,
    metrics JSONB,
    triggered_by UUID REFERENCES users(id),
    parent_step_id UUID REFERENCES document_lineage(id)
);
```

### Retention Policies

```sql
CREATE TABLE document_retention_policies (
    id UUID PRIMARY KEY,
    name VARCHAR(100) NOT NULL UNIQUE,
    description TEXT,
    retention_days INTEGER NOT NULL,
    auto_purge BOOLEAN DEFAULT FALSE,
    applies_to_access_level access_level[],
    applies_to_departments VARCHAR(100)[],
    applies_to_collections UUID[],
    priority INTEGER DEFAULT 0,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);
```

## Features

### Soft Delete with Retention

Documents are soft-deleted (marked with `deleted_at` timestamp) and automatically scheduled for purging based on retention policies.

**Default Retention Periods:**
- Public documents: 30 days
- Internal documents: 90 days
- Confidential documents: 180 days
- Restricted documents: 365 days

**Purge Date Calculation:**
When a document is deleted, the `purge_at` field is automatically calculated based on:
1. Explicit retention policy assigned to the document
2. Matching retention policy by access level, department, or collection
3. Default retention period (90 days if no policy matches)

**Helper Functions:**
```sql
-- Get documents pending purge within 7 days
SELECT * FROM get_documents_pending_purge();

-- Purge expired documents (should be run periodically)
SELECT * FROM purge_expired_documents();
```

### Language Detection

Documents are automatically analyzed for language detection during upload or tagging:

- Supports major languages (Korean, English, Japanese, Chinese, etc.)
- Confidence scores (0.0 to 1.0)
- Alternative language suggestions
- Stored in `detected_language` and `language_confidence` fields

### Auto-Tagging

Machine learning-based tag suggestions:

- Content analysis for topic extraction
- Language detection-based tagging
- Department/category classification
- Confidence scoring for quality assurance

### Collection Hierarchy

Organize documents in a tree structure:

- Parent-child relationships
- Path-based navigation (e.g., `/hr/policies/benefits`)
- Inherited access control from parent collections
- Documents can belong to multiple collections

## Usage Examples

### Bulk Upload with Collections and Tags

```bash
curl -X POST https://api.example.com/api/v1/documents/bulk \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{
    "documents": [
      {
        "title": "직원_핸드북_2025.pdf",
        "content": "JVBERi0xLjcKC...",
        "file_type": "pdf",
        "access_level": "internal",
        "department": "HR",
        "collection_id": "950e8400-e29b-41d4-a716-446655440000",
        "tags": ["korean", "hr", "policy"]
      }
    ],
    "parallel_limit": 4
  }'
```

### Track Document Changes

```bash
# Get version history
curl https://api.example.com/api/v1/documents/{id}/versions \
  -H "Authorization: Bearer <token>"

# Compare versions
curl https://api.example.com/api/v1/documents/{id}/versions/1/diff/2 \
  -H "Authorization: Bearer <token>"
```

### Monitor Processing Pipeline

```bash
curl https://api.example.com/api/v1/documents/{id}/lineage \
  -H "Authorization: Bearer <token>"
```

## Performance Considerations

- **Bulk Operations**: Process documents in parallel (configurable limit 1-8)
- **Version Storage**: Content snapshots stored for diff comparison (consider storage impact)
- **Lineage Tracking**: Indexed by document_id and timestamp for fast queries
- **Retention Cleanup**: Run `purge_expired_documents()` periodically (recommend daily cron job)

## Security

- All endpoints require authentication
- ACL filtering applied to collections and documents
- Soft delete preserves audit trail
- Retention policies prevent premature data loss
- Version history maintains who changed what and when
