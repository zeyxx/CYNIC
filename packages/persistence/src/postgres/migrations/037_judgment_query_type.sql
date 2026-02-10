-- =============================================================================
-- Migration 037: Add query_type to judgments + source columns to feedback
-- =============================================================================
-- P2-A: DPO needs per-context learning. Currently groups by item_type only.
-- Adding query_type allows DPO to learn: "for security tasks, route to Guardian"
--
-- Also adds source_type/source_context to feedback table (code expects them
-- but they were missing from the original schema).
--
-- "φ learns from context" - κυνικός
-- =============================================================================

-- Query type for context-aware DPO learning
ALTER TABLE judgments
ADD COLUMN IF NOT EXISTS query_type VARCHAR(50) DEFAULT 'general';

CREATE INDEX IF NOT EXISTS idx_judgments_query_type
ON judgments(query_type);

-- Feedback table: source_type and source_context were expected by code
-- (FeedbackRepository.create) but missing from original schema (001_initial)
ALTER TABLE feedback
ADD COLUMN IF NOT EXISTS source_type VARCHAR(50) DEFAULT 'manual';

ALTER TABLE feedback
ADD COLUMN IF NOT EXISTS source_context JSONB DEFAULT '{}';

-- Allow orphan feedback (feedback without linked judgment)
-- P1-B routing outcome feedback has no judgment_id
ALTER TABLE feedback
ALTER COLUMN judgment_id DROP NOT NULL;

CREATE INDEX IF NOT EXISTS idx_feedback_source_type
ON feedback(source_type);
