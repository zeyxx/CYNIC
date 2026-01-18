-- CYNIC Database Schema
-- φ guides all ratios
--
-- Migration: 004_escore_history
-- Created: 2026-01-18
-- Purpose: Track E-Score history for trend analysis and learning

-- =============================================================================
-- E-SCORE HISTORY TABLE
-- =============================================================================
-- Records E-Score snapshots over time for trend analysis
-- φ-aligned retention: keep hourly for 24h, daily for 30d, weekly for 1y

CREATE TABLE IF NOT EXISTS escore_history (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Composite score at this point
    e_score         DECIMAL(5,2) NOT NULL,

    -- Dimension breakdown (7 dimensions)
    hold_score      DECIMAL(5,2) DEFAULT 0,  -- Token holding
    burn_score      DECIMAL(5,2) DEFAULT 0,  -- Token burning
    use_score       DECIMAL(5,2) DEFAULT 0,  -- Protocol usage
    build_score     DECIMAL(5,2) DEFAULT 0,  -- Building/contributing
    run_score       DECIMAL(5,2) DEFAULT 0,  -- Running nodes
    refer_score     DECIMAL(5,2) DEFAULT 0,  -- Referrals
    time_score      DECIMAL(5,2) DEFAULT 0,  -- Time in ecosystem

    -- Context for this snapshot
    trigger_event   VARCHAR(50),             -- What triggered the snapshot
    delta           DECIMAL(5,2) DEFAULT 0,  -- Change from previous

    -- Timestamp
    recorded_at     TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for efficient queries
CREATE INDEX idx_escore_history_user ON escore_history(user_id);
CREATE INDEX idx_escore_history_time ON escore_history(recorded_at DESC);
CREATE INDEX idx_escore_history_user_time ON escore_history(user_id, recorded_at DESC);

-- =============================================================================
-- PATTERN EVOLUTION TABLE
-- =============================================================================
-- Tracks how patterns evolve and merge over time

CREATE TABLE IF NOT EXISTS pattern_evolution (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Parent patterns (can be merged)
    parent_ids      UUID[] DEFAULT '{}',

    -- Pattern identity
    pattern_type    VARCHAR(50) NOT NULL,    -- item_type, dimension, axiom, etc.
    pattern_key     VARCHAR(255) NOT NULL,   -- Specific pattern identifier

    -- Evolution metrics
    occurrence_count INTEGER DEFAULT 1,
    confidence      DECIMAL(5,4) DEFAULT 0.382,  -- Starts at φ⁻¹
    strength        DECIMAL(5,2) DEFAULT 50,

    -- Learning adjustments
    weight_modifier DECIMAL(5,4) DEFAULT 1.0,
    threshold_delta DECIMAL(5,2) DEFAULT 0,

    -- Trend data
    trend_direction VARCHAR(10) CHECK (trend_direction IN ('up', 'down', 'stable')),
    trend_velocity  DECIMAL(5,4) DEFAULT 0,

    -- Metadata
    first_seen      TIMESTAMPTZ DEFAULT NOW(),
    last_seen       TIMESTAMPTZ DEFAULT NOW(),
    merged_at       TIMESTAMPTZ,

    UNIQUE(pattern_type, pattern_key)
);

CREATE INDEX idx_pattern_evolution_type ON pattern_evolution(pattern_type);
CREATE INDEX idx_pattern_evolution_strength ON pattern_evolution(strength DESC);

-- =============================================================================
-- USER LEARNING PROFILES TABLE
-- =============================================================================
-- Per-user learning preferences and behavior patterns

CREATE TABLE IF NOT EXISTS user_learning_profiles (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Learning preferences (φ-bounded)
    preferred_dimensions JSONB DEFAULT '{}',  -- Which dimensions user values
    feedback_bias   DECIMAL(5,4) DEFAULT 0,   -- Tendency to confirm/reject

    -- Behavior patterns
    judgment_patterns JSONB DEFAULT '{}',     -- Common item types judged
    feedback_patterns JSONB DEFAULT '{}',     -- Feedback giving patterns
    activity_times  JSONB DEFAULT '{}',       -- When user is active

    -- Learning metrics
    total_feedback  INTEGER DEFAULT 0,
    correct_feedback INTEGER DEFAULT 0,       -- Feedback that improved outcomes
    learning_rate   DECIMAL(5,4) DEFAULT 0.236, -- φ⁻³ default

    -- E-Score correlation
    escore_feedback_correlation DECIMAL(5,4), -- How E-Score relates to feedback

    -- Timestamps
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(user_id)
);

CREATE INDEX idx_user_learning_user ON user_learning_profiles(user_id);

-- =============================================================================
-- LEARNING CYCLES TABLE
-- =============================================================================
-- Record each learning cycle for analysis

CREATE TABLE IF NOT EXISTS learning_cycles (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    cycle_id        VARCHAR(32) UNIQUE NOT NULL,  -- lrn_xxxxx

    -- Cycle metrics
    feedback_processed INTEGER DEFAULT 0,
    patterns_updated INTEGER DEFAULT 0,
    patterns_merged INTEGER DEFAULT 0,
    weights_adjusted INTEGER DEFAULT 0,
    thresholds_adjusted INTEGER DEFAULT 0,

    -- Aggregate changes
    avg_weight_delta DECIMAL(5,4),
    avg_threshold_delta DECIMAL(5,2),

    -- Performance
    duration_ms     INTEGER,

    -- Timestamp
    completed_at    TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_learning_cycles_time ON learning_cycles(completed_at DESC);

-- =============================================================================
-- HELPER FUNCTIONS
-- =============================================================================

-- Function to record E-Score snapshot
CREATE OR REPLACE FUNCTION record_escore_snapshot(
    p_user_id UUID,
    p_trigger VARCHAR(50) DEFAULT 'manual'
)
RETURNS UUID AS $$
DECLARE
    v_current_score DECIMAL(5,2);
    v_score_data JSONB;
    v_previous_score DECIMAL(5,2);
    v_delta DECIMAL(5,2);
    v_snapshot_id UUID;
BEGIN
    -- Get current E-Score
    SELECT e_score, e_score_data INTO v_current_score, v_score_data
    FROM users WHERE id = p_user_id;

    -- Get previous score for delta
    SELECT e_score INTO v_previous_score
    FROM escore_history
    WHERE user_id = p_user_id
    ORDER BY recorded_at DESC
    LIMIT 1;

    v_delta := COALESCE(v_current_score - v_previous_score, 0);

    -- Insert snapshot
    INSERT INTO escore_history (
        user_id, e_score,
        hold_score, burn_score, use_score, build_score,
        run_score, refer_score, time_score,
        trigger_event, delta
    ) VALUES (
        p_user_id, v_current_score,
        COALESCE((v_score_data->>'hold')::DECIMAL, 0),
        COALESCE((v_score_data->>'burn')::DECIMAL, 0),
        COALESCE((v_score_data->>'use')::DECIMAL, 0),
        COALESCE((v_score_data->>'build')::DECIMAL, 0),
        COALESCE((v_score_data->>'run')::DECIMAL, 0),
        COALESCE((v_score_data->>'refer')::DECIMAL, 0),
        COALESCE((v_score_data->>'time')::DECIMAL, 0),
        p_trigger, v_delta
    ) RETURNING id INTO v_snapshot_id;

    RETURN v_snapshot_id;
END;
$$ LANGUAGE plpgsql;

-- Function to get E-Score trend
CREATE OR REPLACE FUNCTION get_escore_trend(
    p_user_id UUID,
    p_days INTEGER DEFAULT 7
)
RETURNS TABLE (
    trend_direction VARCHAR(10),
    trend_velocity DECIMAL(5,4),
    avg_score DECIMAL(5,2),
    min_score DECIMAL(5,2),
    max_score DECIMAL(5,2),
    data_points INTEGER
) AS $$
DECLARE
    v_first_score DECIMAL(5,2);
    v_last_score DECIMAL(5,2);
    v_velocity DECIMAL(5,4);
    v_direction VARCHAR(10);
BEGIN
    -- Get first and last scores in period
    SELECT e_score INTO v_first_score
    FROM escore_history
    WHERE user_id = p_user_id
      AND recorded_at >= NOW() - (p_days || ' days')::INTERVAL
    ORDER BY recorded_at ASC
    LIMIT 1;

    SELECT e_score INTO v_last_score
    FROM escore_history
    WHERE user_id = p_user_id
    ORDER BY recorded_at DESC
    LIMIT 1;

    -- Calculate velocity (change per day)
    IF v_first_score IS NOT NULL AND v_last_score IS NOT NULL THEN
        v_velocity := (v_last_score - v_first_score) / GREATEST(p_days, 1);

        IF v_velocity > 0.01 THEN
            v_direction := 'up';
        ELSIF v_velocity < -0.01 THEN
            v_direction := 'down';
        ELSE
            v_direction := 'stable';
        END IF;
    ELSE
        v_velocity := 0;
        v_direction := 'stable';
    END IF;

    RETURN QUERY
    SELECT
        v_direction,
        v_velocity,
        AVG(h.e_score)::DECIMAL(5,2),
        MIN(h.e_score)::DECIMAL(5,2),
        MAX(h.e_score)::DECIMAL(5,2),
        COUNT(*)::INTEGER
    FROM escore_history h
    WHERE h.user_id = p_user_id
      AND h.recorded_at >= NOW() - (p_days || ' days')::INTERVAL;
END;
$$ LANGUAGE plpgsql;
