-- Learning Events Tracking
-- "Every feedback shapes the future" - κυνικός
--
-- Migration: 040_learning_events
-- Created: 2026-02-12
-- Purpose: Track learning loop activity for G1.2 metric

-- =============================================================================
-- LEARNING EVENTS
-- Tracks which learning loops are consuming data
-- =============================================================================

CREATE TABLE IF NOT EXISTS learning_events (
    id              SERIAL PRIMARY KEY,
    timestamp       TIMESTAMPTZ DEFAULT NOW(),

    -- Loop identification
    loop_type       VARCHAR(50) NOT NULL, -- 'q-learning', 'sona', 'behavior', 'meta-cognition', etc.
    loop_name       VARCHAR(100),

    -- Event details
    event_type      VARCHAR(50) NOT NULL, -- 'feedback', 'observation', 'update', 'convergence'

    -- Data consumed
    judgment_id     VARCHAR(100),
    pattern_id      VARCHAR(100),
    feedback_value  DECIMAL(5,4), -- -1.0 to 1.0 (negative/positive feedback)

    -- Learning outcome
    action_taken    TEXT, -- What the loop did in response
    weight_delta    DECIMAL(10,6), -- How much weights changed

    -- Context
    metadata        JSONB DEFAULT '{}'
);

CREATE INDEX idx_learning_events_loop_type ON learning_events(loop_type);
CREATE INDEX idx_learning_events_timestamp ON learning_events(timestamp DESC);
CREATE INDEX idx_learning_events_event_type ON learning_events(event_type);
CREATE INDEX idx_learning_events_judgment ON learning_events(judgment_id);

-- =============================================================================
-- Q-LEARNING STATE (enhance existing table if needed)
-- Track Q-values and updates for routing decisions
-- =============================================================================

-- Check if qlearning_state exists, create if not
CREATE TABLE IF NOT EXISTS qlearning_state (
    id              SERIAL PRIMARY KEY,

    -- State-Action pair
    state_key       VARCHAR(200) NOT NULL, -- Hashed representation of state
    action          VARCHAR(100) NOT NULL, -- Action taken (e.g., 'route_to_ralph')

    -- Q-value
    q_value         DECIMAL(10,6) DEFAULT 0.0,
    visit_count     INTEGER DEFAULT 0,

    -- Learning params
    learning_rate   DECIMAL(5,4) DEFAULT 0.1,
    discount_factor DECIMAL(5,4) DEFAULT 0.95,

    -- Timestamps
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW(),

    -- Context
    metadata        JSONB DEFAULT '{}',

    UNIQUE(state_key, action)
);

CREATE INDEX IF NOT EXISTS idx_qlearning_state_key ON qlearning_state(state_key);
CREATE INDEX IF NOT EXISTS idx_qlearning_updated ON qlearning_state(updated_at DESC);

-- =============================================================================
-- HELPER FUNCTIONS
-- =============================================================================

-- Get active learning loops count (for G1.2 metric)
CREATE OR REPLACE FUNCTION get_active_learning_loops(since_date DATE DEFAULT CURRENT_DATE)
RETURNS INTEGER AS $$
BEGIN
    RETURN (
        SELECT COUNT(DISTINCT loop_type)
        FROM learning_events
        WHERE timestamp::DATE >= since_date
    );
END;
$$ LANGUAGE plpgsql;

-- Get Q-weight updates today (for G1.3 metric)
CREATE OR REPLACE FUNCTION get_qweight_updates_today()
RETURNS INTEGER AS $$
BEGIN
    RETURN (
        SELECT COUNT(*)
        FROM qlearning_state
        WHERE updated_at > NOW() - INTERVAL '24 hours'
    );
END;
$$ LANGUAGE plpgsql;

-- Record learning event (helper for application code)
CREATE OR REPLACE FUNCTION record_learning_event(
    p_loop_type VARCHAR(50),
    p_event_type VARCHAR(50),
    p_judgment_id VARCHAR(100) DEFAULT NULL,
    p_feedback_value DECIMAL DEFAULT NULL,
    p_action_taken TEXT DEFAULT NULL,
    p_weight_delta DECIMAL DEFAULT NULL,
    p_metadata JSONB DEFAULT '{}'
)
RETURNS VOID AS $$
BEGIN
    INSERT INTO learning_events (
        loop_type,
        event_type,
        judgment_id,
        feedback_value,
        action_taken,
        weight_delta,
        metadata
    ) VALUES (
        p_loop_type,
        p_event_type,
        p_judgment_id,
        p_feedback_value,
        p_action_taken,
        p_weight_delta,
        p_metadata
    );
END;
$$ LANGUAGE plpgsql;

-- =============================================================================
-- CLEANUP FUNCTION
-- Remove old learning events (keep last 90 days)
-- =============================================================================

CREATE OR REPLACE FUNCTION cleanup_old_learning_events()
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    -- Keep 90 days of learning events (φ × 100 ≈ 90)
    DELETE FROM learning_events WHERE timestamp < NOW() - INTERVAL '90 days';
    GET DIAGNOSTICS deleted_count = ROW_COUNT;

    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- =============================================================================
-- MIGRATION TRACKING
-- =============================================================================

INSERT INTO _migrations (name) VALUES ('040_learning_events')
ON CONFLICT (name) DO NOTHING;

-- =============================================================================
-- DONE
-- =============================================================================

-- "Loops consume data, organism learns" - κυνικός
