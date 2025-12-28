-- Full-Text Search Schema for Keyword Search Backend
-- PostgreSQL migration for hybrid RAG (vector + keyword search)
--
-- Author: hephaex@gmail.com
-- Date: 2025-12-28
-- Related Issue: #3 Full-text search / keyword search backend

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS pg_trgm;  -- Trigram matching for fuzzy search
CREATE EXTENSION IF NOT EXISTS unaccent; -- Remove accents for better matching

-- ==========================================================================
-- Full-Text Search Documents Table
-- ==========================================================================

-- This table stores searchable text content with full-text indexes
CREATE TABLE IF NOT EXISTS fts_documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Reference to source document
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    chunk_id UUID REFERENCES document_chunks(id) ON DELETE CASCADE,

    -- Searchable content
    content TEXT NOT NULL,

    -- Full-text search vectors (multiple languages and configurations)
    -- Using simple configuration for technical terms
    ts_vector_simple tsvector GENERATED ALWAYS AS (to_tsvector('simple', content)) STORED,

    -- Using Korean configuration if needed
    -- ts_vector_korean tsvector GENERATED ALWAYS AS (to_tsvector('korean', content)) STORED,

    -- Metadata for scoring
    content_length INTEGER NOT NULL DEFAULT 0,
    word_count INTEGER NOT NULL DEFAULT 0,

    -- BM25 pre-computed statistics
    avg_term_frequency REAL DEFAULT 0.0,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Ensure one entry per chunk
    UNIQUE(chunk_id)
);

-- Indexes for full-text search
CREATE INDEX IF NOT EXISTS idx_fts_documents_document_id ON fts_documents(document_id);
CREATE INDEX IF NOT EXISTS idx_fts_documents_chunk_id ON fts_documents(chunk_id);

-- GIN indexes for full-text search (faster than GiST for large datasets)
CREATE INDEX IF NOT EXISTS idx_fts_ts_vector_simple ON fts_documents USING GIN(ts_vector_simple);

-- Trigram index for fuzzy matching
CREATE INDEX IF NOT EXISTS idx_fts_content_trigram ON fts_documents USING GIN(content gin_trgm_ops);

-- ==========================================================================
-- Document Statistics for BM25 Scoring
-- ==========================================================================

-- Global statistics table for BM25 calculation
CREATE TABLE IF NOT EXISTS fts_statistics (
    id INTEGER PRIMARY KEY DEFAULT 1,

    -- Total number of documents in collection
    total_documents BIGINT NOT NULL DEFAULT 0,

    -- Average document length (in words)
    avg_document_length REAL NOT NULL DEFAULT 0.0,

    -- Last updated timestamp
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Ensure only one row
    CONSTRAINT single_row CHECK (id = 1)
);

-- Initialize statistics
INSERT INTO fts_statistics (id, total_documents, avg_document_length, updated_at)
VALUES (1, 0, 0.0, NOW())
ON CONFLICT (id) DO NOTHING;

-- ==========================================================================
-- Term Frequency Table (for BM25)
-- ==========================================================================

-- Store term frequencies for BM25 calculation
CREATE TABLE IF NOT EXISTS fts_term_stats (
    term VARCHAR(255) PRIMARY KEY,

    -- Number of documents containing this term
    document_frequency BIGINT NOT NULL DEFAULT 0,

    -- Inverse document frequency (pre-computed)
    idf REAL,

    -- Last updated
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_fts_term_stats_df ON fts_term_stats(document_frequency DESC);

-- ==========================================================================
-- Search Query Log (for analytics and optimization)
-- ==========================================================================

CREATE TABLE IF NOT EXISTS fts_query_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Query details
    query_text TEXT NOT NULL,
    search_type VARCHAR(50) NOT NULL, -- 'simple', 'phrase', 'boolean', 'fuzzy'

    -- Performance metrics
    num_results INTEGER,
    execution_time_ms INTEGER,

    -- Top result score
    top_score REAL,

    -- User context
    user_id UUID,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_fts_query_log_created ON fts_query_log(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_fts_query_log_query ON fts_query_log USING GIN(to_tsvector('simple', query_text));

-- ==========================================================================
-- Helper Functions
-- ==========================================================================

-- Function to update document statistics after insert/delete
CREATE OR REPLACE FUNCTION update_fts_statistics()
RETURNS TRIGGER AS $$
BEGIN
    -- Update global statistics
    UPDATE fts_statistics
    SET
        total_documents = (SELECT COUNT(*) FROM fts_documents),
        avg_document_length = (SELECT AVG(word_count) FROM fts_documents WHERE word_count > 0),
        updated_at = NOW()
    WHERE id = 1;

    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

-- Trigger to update statistics on document changes
DROP TRIGGER IF EXISTS trigger_update_fts_statistics_insert ON fts_documents;
CREATE TRIGGER trigger_update_fts_statistics_insert
    AFTER INSERT ON fts_documents
    FOR EACH STATEMENT
    EXECUTE FUNCTION update_fts_statistics();

DROP TRIGGER IF EXISTS trigger_update_fts_statistics_delete ON fts_documents;
CREATE TRIGGER trigger_update_fts_statistics_delete
    AFTER DELETE ON fts_documents
    FOR EACH STATEMENT
    EXECUTE FUNCTION update_fts_statistics();

-- Function to calculate BM25 score
-- Parameters: k1 (term saturation, typically 1.2-2.0), b (length normalization, typically 0.75)
CREATE OR REPLACE FUNCTION bm25_score(
    term_freq REAL,
    doc_length INTEGER,
    avg_doc_length REAL,
    doc_freq BIGINT,
    total_docs BIGINT,
    k1 REAL DEFAULT 1.2,
    b REAL DEFAULT 0.75
)
RETURNS REAL AS $$
DECLARE
    idf REAL;
    normalized_tf REAL;
BEGIN
    -- Calculate IDF: log((N - df + 0.5) / (df + 0.5))
    idf := LN((total_docs - doc_freq + 0.5) / (doc_freq + 0.5));

    -- Calculate normalized TF with length normalization
    normalized_tf := (term_freq * (k1 + 1.0)) /
                     (term_freq + k1 * (1.0 - b + b * (doc_length / avg_doc_length)));

    RETURN idf * normalized_tf;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- Function to search with BM25 ranking
CREATE OR REPLACE FUNCTION search_bm25(
    search_query TEXT,
    result_limit INTEGER DEFAULT 10,
    k1 REAL DEFAULT 1.2,
    b REAL DEFAULT 0.75
)
RETURNS TABLE (
    doc_id UUID,
    chunk_id UUID,
    content TEXT,
    score REAL,
    rank INTEGER
) AS $$
DECLARE
    stats_total_docs BIGINT;
    stats_avg_length REAL;
BEGIN
    -- Get global statistics
    SELECT total_documents, avg_document_length
    INTO stats_total_docs, stats_avg_length
    FROM fts_statistics
    WHERE id = 1;

    -- Return empty if no documents
    IF stats_total_docs = 0 THEN
        RETURN;
    END IF;

    -- Search using ts_rank_cd (Cover Density ranking, similar to BM25)
    -- This is PostgreSQL's built-in ranking that considers term frequency and position
    RETURN QUERY
    SELECT
        fd.document_id,
        fd.chunk_id,
        fd.content,
        -- Use ts_rank_cd which provides BM25-like scoring
        ts_rank_cd(fd.ts_vector_simple, plainto_tsquery('simple', search_query)) AS score,
        ROW_NUMBER() OVER (ORDER BY ts_rank_cd(fd.ts_vector_simple, plainto_tsquery('simple', search_query)) DESC)::INTEGER AS rank
    FROM fts_documents fd
    WHERE fd.ts_vector_simple @@ plainto_tsquery('simple', search_query)
    ORDER BY score DESC
    LIMIT result_limit;
END;
$$ LANGUAGE plpgsql;

-- Function for phrase search (exact phrase matching)
CREATE OR REPLACE FUNCTION search_phrase(
    search_phrase TEXT,
    result_limit INTEGER DEFAULT 10
)
RETURNS TABLE (
    doc_id UUID,
    chunk_id UUID,
    content TEXT,
    score REAL,
    rank INTEGER
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        fd.document_id,
        fd.chunk_id,
        fd.content,
        ts_rank_cd(fd.ts_vector_simple, phraseto_tsquery('simple', search_phrase)) AS score,
        ROW_NUMBER() OVER (ORDER BY ts_rank_cd(fd.ts_vector_simple, phraseto_tsquery('simple', search_phrase)) DESC)::INTEGER AS rank
    FROM fts_documents fd
    WHERE fd.ts_vector_simple @@ phraseto_tsquery('simple', search_phrase)
    ORDER BY score DESC
    LIMIT result_limit;
END;
$$ LANGUAGE plpgsql;

-- Function for boolean search (AND, OR, NOT operators)
CREATE OR REPLACE FUNCTION search_boolean(
    search_expression TEXT,
    result_limit INTEGER DEFAULT 10
)
RETURNS TABLE (
    doc_id UUID,
    chunk_id UUID,
    content TEXT,
    score REAL,
    rank INTEGER
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        fd.document_id,
        fd.chunk_id,
        fd.content,
        ts_rank_cd(fd.ts_vector_simple, to_tsquery('simple', search_expression)) AS score,
        ROW_NUMBER() OVER (ORDER BY ts_rank_cd(fd.ts_vector_simple, to_tsquery('simple', search_expression)) DESC)::INTEGER AS rank
    FROM fts_documents fd
    WHERE fd.ts_vector_simple @@ to_tsquery('simple', search_expression)
    ORDER BY score DESC
    LIMIT result_limit;
END;
$$ LANGUAGE plpgsql;

-- Function for fuzzy search using trigram similarity
CREATE OR REPLACE FUNCTION search_fuzzy(
    search_text TEXT,
    similarity_threshold REAL DEFAULT 0.3,
    result_limit INTEGER DEFAULT 10
)
RETURNS TABLE (
    doc_id UUID,
    chunk_id UUID,
    content TEXT,
    score REAL,
    rank INTEGER
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        fd.document_id,
        fd.chunk_id,
        fd.content,
        similarity(fd.content, search_text) AS score,
        ROW_NUMBER() OVER (ORDER BY similarity(fd.content, search_text) DESC)::INTEGER AS rank
    FROM fts_documents fd
    WHERE similarity(fd.content, search_text) > similarity_threshold
    ORDER BY score DESC
    LIMIT result_limit;
END;
$$ LANGUAGE plpgsql;

-- ==========================================================================
-- Maintenance Functions
-- ==========================================================================

-- Function to reindex all documents (call after bulk updates)
CREATE OR REPLACE FUNCTION reindex_fts_documents()
RETURNS void AS $$
BEGIN
    -- Vacuum and analyze for better performance
    VACUUM ANALYZE fts_documents;
    VACUUM ANALYZE fts_term_stats;

    -- Update statistics
    PERFORM update_fts_statistics();
END;
$$ LANGUAGE plpgsql;

-- Function to clean up old query logs (keep last 30 days)
CREATE OR REPLACE FUNCTION cleanup_old_query_logs()
RETURNS void AS $$
BEGIN
    DELETE FROM fts_query_log
    WHERE created_at < NOW() - INTERVAL '30 days';
END;
$$ LANGUAGE plpgsql;

-- ==========================================================================
-- Views for Analytics
-- ==========================================================================

-- View for popular search terms
CREATE OR REPLACE VIEW popular_search_terms AS
SELECT
    query_text,
    COUNT(*) as query_count,
    AVG(num_results) as avg_results,
    AVG(execution_time_ms) as avg_time_ms,
    MAX(created_at) as last_searched
FROM fts_query_log
WHERE created_at > NOW() - INTERVAL '7 days'
GROUP BY query_text
ORDER BY query_count DESC
LIMIT 100;

-- View for search performance metrics
CREATE OR REPLACE VIEW search_performance_metrics AS
SELECT
    search_type,
    COUNT(*) as total_searches,
    AVG(execution_time_ms) as avg_execution_time,
    MAX(execution_time_ms) as max_execution_time,
    AVG(num_results) as avg_results,
    AVG(top_score) as avg_top_score
FROM fts_query_log
WHERE created_at > NOW() - INTERVAL '24 hours'
GROUP BY search_type;

-- ==========================================================================
-- Comments for Documentation
-- ==========================================================================

COMMENT ON TABLE fts_documents IS 'Full-text searchable document chunks with tsvector indexes';
COMMENT ON TABLE fts_statistics IS 'Global statistics for BM25 scoring (single row)';
COMMENT ON TABLE fts_term_stats IS 'Term frequency and IDF statistics for BM25';
COMMENT ON TABLE fts_query_log IS 'Search query log for analytics and optimization';

COMMENT ON FUNCTION bm25_score IS 'Calculate BM25 score for a term in a document';
COMMENT ON FUNCTION search_bm25 IS 'Search documents using BM25-like ranking';
COMMENT ON FUNCTION search_phrase IS 'Search for exact phrase matches';
COMMENT ON FUNCTION search_boolean IS 'Search with boolean operators (AND, OR, NOT)';
COMMENT ON FUNCTION search_fuzzy IS 'Fuzzy search using trigram similarity';
COMMENT ON FUNCTION reindex_fts_documents IS 'Reindex all full-text search documents';
COMMENT ON FUNCTION cleanup_old_query_logs IS 'Clean up query logs older than 30 days';
