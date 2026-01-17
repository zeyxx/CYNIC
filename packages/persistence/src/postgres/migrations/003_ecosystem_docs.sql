-- CYNIC Database Schema - Ecosystem Docs
-- Pre-loaded documentation for the $ASDFASDFA ecosystem
--
-- Migration: 003_ecosystem_docs
-- Created: 2026-01-16

-- =============================================================================
-- ECOSYSTEM_DOCS TABLE
-- Stores CLAUDE.md files and other ecosystem documentation
-- =============================================================================

CREATE TABLE IF NOT EXISTS ecosystem_docs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Document identity
    project         VARCHAR(50) NOT NULL,       -- holdex, gasdf, asdf-brain, etc.
    doc_type        VARCHAR(50) NOT NULL,       -- claude_md, harmony, api, readme
    file_path       VARCHAR(512) NOT NULL,      -- Original file path

    -- Content
    content         TEXT NOT NULL,              -- Raw file content
    content_hash    VARCHAR(64) NOT NULL,       -- SHA-256 for change detection
    digest          TEXT,                       -- AI-generated summary

    -- Metadata
    metadata        JSONB DEFAULT '{}',         -- Additional metadata

    -- Timestamps
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW(),

    -- Unique constraint: one doc per project/type
    UNIQUE(project, doc_type)
);

-- Indexes
CREATE INDEX idx_ecosystem_docs_project ON ecosystem_docs(project);
CREATE INDEX idx_ecosystem_docs_type ON ecosystem_docs(doc_type);
CREATE INDEX idx_ecosystem_docs_hash ON ecosystem_docs(content_hash);

-- Full-text search
CREATE INDEX idx_ecosystem_docs_fts ON ecosystem_docs
    USING GIN (to_tsvector('english', content));

-- Comment
COMMENT ON TABLE ecosystem_docs IS 'Pre-loaded documentation for the $ASDFASDFA ecosystem (CLAUDE.md files, APIs, etc.)';
