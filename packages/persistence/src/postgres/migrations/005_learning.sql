-- CYNIC Database Schema - Learning & E-Score
-- φ guides all ratios
--
-- Migration: 005_learning
-- Created: 2026-01-20
-- Purpose: Track learning cycles, user profiles, E-Score history, and pattern evolution

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

CREATE INDEX IF NOT EXISTS idx_escore_history_user ON escore_history(user_id);
CREATE INDEX IF NOT EXISTS idx_escore_history_time ON escore_history(recorded_at DESC);
CREATE INDEX IF NOT EXISTS idx_escore_history_user_time ON escore_history(user_id, recorded_at DESC);

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
    confidence      DECIMAL(5,4) DEFAULT 0.382,  -- Starts at φ⁻²
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

CREATE INDEX IF NOT EXISTS idx_pattern_evolution_type ON pattern_evolution(pattern_type);
CREATE INDEX IF NOT EXISTS idx_pattern_evolution_strength ON pattern_evolution(strength DESC);
CREATE INDEX IF NOT EXISTS idx_pattern_evolution_last_seen ON pattern_evolution(last_seen DESC);

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
    incorrect_feedback INTEGER DEFAULT 0,     -- Feedback that worsened outcomes
    partial_feedback INTEGER DEFAULT 0,       -- Partial feedback
    learning_rate   DECIMAL(5,4) DEFAULT 0.236, -- φ⁻³ default

    -- Accuracy tracking
    accuracy        DECIMAL(5,4) DEFAULT 0,   -- correct / total
    calibration_count INTEGER DEFAULT 0,      -- How many times calibrated

    -- E-Score correlation
    escore_feedback_correlation DECIMAL(5,4), -- How E-Score relates to feedback

    -- Timestamps
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(user_id)
);

CREATE INDEX IF NOT EXISTS idx_user_learning_user ON user_learning_profiles(user_id);
CREATE INDEX IF NOT EXISTS idx_user_learning_accuracy ON user_learning_profiles(accuracy DESC);

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

    -- Before/after state
    state_before    JSONB DEFAULT '{}',       -- Weights before calibration
    state_after     JSONB DEFAULT '{}',       -- Weights after calibration

    -- Performance
    duration_ms     INTEGER,

    -- Timestamp
    completed_at    TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_learning_cycles_time ON learning_cycles(completed_at DESC);

-- =============================================================================
-- LEARNING STATE TABLE
-- =============================================================================
-- Global learning state (singleton pattern)

CREATE TABLE IF NOT EXISTS learning_state (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    state_key       VARCHAR(50) UNIQUE NOT NULL DEFAULT 'global',

    -- Current weights
    axiom_weights   JSONB DEFAULT '{"PHI": 1, "VERIFY": 1, "CULTURE": 1, "BURN": 1}',
    dimension_weights JSONB DEFAULT '{}',

    -- Calibration stats
    total_calibrations INTEGER DEFAULT 0,
    last_calibration_at TIMESTAMPTZ,

    -- Bias detection
    detected_biases JSONB DEFAULT '[]',

    -- Metadata
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW()
);

-- Insert default global state
INSERT INTO learning_state (state_key) VALUES ('global')
ON CONFLICT (state_key) DO NOTHING;

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

-- Function to update learning profile on feedback
CREATE OR REPLACE FUNCTION update_learning_profile_on_feedback()
RETURNS TRIGGER AS $$
BEGIN
    -- Upsert user learning profile
    INSERT INTO user_learning_profiles (user_id, total_feedback)
    VALUES (NEW.user_id, 1)
    ON CONFLICT (user_id) DO UPDATE SET
        total_feedback = user_learning_profiles.total_feedback + 1,
        correct_feedback = CASE WHEN NEW.outcome = 'correct'
            THEN user_learning_profiles.correct_feedback + 1
            ELSE user_learning_profiles.correct_feedback END,
        incorrect_feedback = CASE WHEN NEW.outcome = 'incorrect'
            THEN user_learning_profiles.incorrect_feedback + 1
            ELSE user_learning_profiles.incorrect_feedback END,
        partial_feedback = CASE WHEN NEW.outcome = 'partial'
            THEN user_learning_profiles.partial_feedback + 1
            ELSE user_learning_profiles.partial_feedback END,
        accuracy = CASE WHEN (user_learning_profiles.total_feedback + 1) > 0
            THEN (user_learning_profiles.correct_feedback + CASE WHEN NEW.outcome = 'correct' THEN 1 ELSE 0 END)::DECIMAL
                 / (user_learning_profiles.total_feedback + 1)
            ELSE 0 END,
        updated_at = NOW();

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger for feedback updates
DROP TRIGGER IF EXISTS feedback_learning_profile_trigger ON feedback;
CREATE TRIGGER feedback_learning_profile_trigger
    AFTER INSERT ON feedback
    FOR EACH ROW
    WHEN (NEW.user_id IS NOT NULL)
    EXECUTE FUNCTION update_learning_profile_on_feedback();

-- =============================================================================
-- UPDATED_AT TRIGGER
-- =============================================================================

CREATE TRIGGER user_learning_profiles_updated_at BEFORE UPDATE ON user_learning_profiles
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER learning_state_updated_at BEFORE UPDATE ON learning_state
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- =============================================================================
-- MIGRATION TRACKING
-- =============================================================================

INSERT INTO _migrations (name) VALUES ('005_learning')
ON CONFLICT (name) DO NOTHING;

-- =============================================================================
-- DONE
-- =============================================================================

-- φ⁻¹ = 61.8% max confidence
-- Learning infrastructure ready
