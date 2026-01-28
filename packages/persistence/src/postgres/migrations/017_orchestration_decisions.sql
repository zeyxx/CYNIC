-- =============================================================================
-- Migration 017: Orchestration Decisions - Phase 20 Enhancement
-- =============================================================================
-- Extends orchestration_log with full DecisionEvent data for:
-- - Learning from routing outcomes
-- - Full decision trace visualization
-- - Judgment and synthesis correlation
--
-- "φ traces every step" - κυνικός
-- =============================================================================

-- =============================================================================
-- ADD COLUMNS TO EXISTING TABLE
-- =============================================================================

-- Decision outcome (ALLOW, BLOCK, MODIFIED, ERROR)
ALTER TABLE orchestration_log
ADD COLUMN IF NOT EXISTS outcome TEXT DEFAULT 'ALLOW';

-- Domain from KETER routing
ALTER TABLE orchestration_log
ADD COLUMN IF NOT EXISTS domain TEXT;

-- Suggested agent from routing
ALTER TABLE orchestration_log
ADD COLUMN IF NOT EXISTS suggested_agent TEXT;

-- Full decision trace as JSONB
ALTER TABLE orchestration_log
ADD COLUMN IF NOT EXISTS trace JSONB DEFAULT '[]';

-- Judgment data if requested
ALTER TABLE orchestration_log
ADD COLUMN IF NOT EXISTS judgment_id UUID;

ALTER TABLE orchestration_log
ADD COLUMN IF NOT EXISTS judgment_qscore FLOAT;

ALTER TABLE orchestration_log
ADD COLUMN IF NOT EXISTS judgment_verdict TEXT;

-- Skill invocation result
ALTER TABLE orchestration_log
ADD COLUMN IF NOT EXISTS skill_invoked TEXT;

ALTER TABLE orchestration_log
ADD COLUMN IF NOT EXISTS skill_success BOOLEAN;

-- Content hash for correlation (not storing actual content)
ALTER TABLE orchestration_log
ADD COLUMN IF NOT EXISTS content_hash TEXT;

-- Duration of orchestration process
ALTER TABLE orchestration_log
ADD COLUMN IF NOT EXISTS duration_ms INTEGER;

-- Session correlation
ALTER TABLE orchestration_log
ADD COLUMN IF NOT EXISTS session_id TEXT;

-- =============================================================================
-- NEW INDEXES
-- =============================================================================

CREATE INDEX IF NOT EXISTS idx_orchestration_outcome
ON orchestration_log(outcome);

CREATE INDEX IF NOT EXISTS idx_orchestration_domain
ON orchestration_log(domain);

CREATE INDEX IF NOT EXISTS idx_orchestration_judgment
ON orchestration_log(judgment_id)
WHERE judgment_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_orchestration_skill
ON orchestration_log(skill_invoked)
WHERE skill_invoked IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_orchestration_session
ON orchestration_log(session_id)
WHERE session_id IS NOT NULL;

-- Composite index for learning queries
CREATE INDEX IF NOT EXISTS idx_orchestration_learning
ON orchestration_log(domain, outcome, created_at DESC);

-- =============================================================================
-- COMMENTS
-- =============================================================================

COMMENT ON COLUMN orchestration_log.outcome IS
'Final decision outcome: ALLOW, BLOCK, MODIFIED, ERROR';

COMMENT ON COLUMN orchestration_log.domain IS
'KETER routing domain: wisdom, protection, analysis, memory, etc.';

COMMENT ON COLUMN orchestration_log.trace IS
'Full decision trace as JSONB array of {stage, action, data, timestamp}';

COMMENT ON COLUMN orchestration_log.judgment_qscore IS
'Q-Score from judgment if Dogs were consulted (0-100)';

COMMENT ON COLUMN orchestration_log.skill_invoked IS
'Skill that was auto-invoked based on routing, if any';

-- =============================================================================
-- VIEW: Orchestration Learning Analysis
-- =============================================================================

CREATE OR REPLACE VIEW orchestration_learning_view AS
SELECT
    domain,
    outcome,
    skill_invoked,
    COUNT(*) as count,
    AVG(duration_ms) as avg_duration_ms,
    AVG(judgment_qscore) as avg_qscore,
    SUM(CASE WHEN skill_success THEN 1 ELSE 0 END)::FLOAT / NULLIF(COUNT(*), 0) as skill_success_rate,
    MAX(created_at) as last_used
FROM orchestration_log
WHERE created_at > NOW() - INTERVAL '7 days'
GROUP BY domain, outcome, skill_invoked
ORDER BY count DESC;

COMMENT ON VIEW orchestration_learning_view IS
'Aggregated view of orchestration decisions for learning analysis';
