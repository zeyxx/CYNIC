-- =============================================================================
-- Migration 018: Burnout Detection Tables
-- "Prévenir vaut mieux que guérir" - κυνικός
--
-- v1.1: Tracks psychology over time for trend analysis and proactive warnings
-- =============================================================================

-- Psychology snapshots for trend analysis
-- Records periodic samples of psychology state
CREATE TABLE IF NOT EXISTS psychology_snapshots (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_id VARCHAR(100),                  -- Link to session

    -- Core dimensions at snapshot time
    energy REAL NOT NULL,                      -- 0-1
    focus REAL NOT NULL,                       -- 0-1
    creativity REAL NOT NULL DEFAULT 0.5,      -- 0-1
    frustration REAL NOT NULL,                 -- 0-1

    -- Derived scores
    burnout_score REAL NOT NULL DEFAULT 0,     -- 0-1 (calculated)
    flow_score REAL NOT NULL DEFAULT 0,        -- 0-1 (calculated)

    -- Context
    work_done INTEGER DEFAULT 0,               -- Thermodynamic work (W)
    heat_generated INTEGER DEFAULT 0,          -- Thermodynamic heat (Q)
    error_count INTEGER DEFAULT 0,             -- Errors in period

    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Indexes for time-series queries
CREATE INDEX IF NOT EXISTS idx_psychology_snapshots_user_time
    ON psychology_snapshots(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_psychology_snapshots_burnout
    ON psychology_snapshots(burnout_score DESC) WHERE burnout_score > 0.382;
CREATE INDEX IF NOT EXISTS idx_psychology_snapshots_session
    ON psychology_snapshots(session_id) WHERE session_id IS NOT NULL;

-- Burnout episodes tracking
-- Records when user enters/exits burnout state
CREATE TABLE IF NOT EXISTS burnout_episodes (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Episode timing
    started_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    ended_at TIMESTAMP WITH TIME ZONE,         -- NULL if ongoing
    duration_minutes INTEGER,                   -- Calculated on end

    -- Episode characteristics
    peak_burnout_score REAL NOT NULL DEFAULT 0, -- Max burnout during episode
    trigger_context JSONB DEFAULT '{}',         -- What triggered it
    recovery_intervention VARCHAR(100),         -- What helped (if known)

    -- Analysis
    session_count INTEGER DEFAULT 1,            -- Sessions during episode
    total_work INTEGER DEFAULT 0,               -- Work during episode
    warning_count INTEGER DEFAULT 0,            -- Warnings shown

    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Indexes for burnout analysis
CREATE INDEX IF NOT EXISTS idx_burnout_episodes_user
    ON burnout_episodes(user_id, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_burnout_episodes_active
    ON burnout_episodes(user_id) WHERE ended_at IS NULL;

-- Proactive warnings log
-- Tracks warnings shown and their effectiveness
CREATE TABLE IF NOT EXISTS burnout_warnings (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    episode_id UUID REFERENCES burnout_episodes(id),

    -- Warning details
    warning_type VARCHAR(50) NOT NULL,         -- trend_declining, threshold_near, etc.
    severity VARCHAR(20) NOT NULL,             -- info, caution, warning, critical
    message TEXT NOT NULL,

    -- Psychology at warning time
    burnout_score REAL NOT NULL,
    energy REAL NOT NULL,
    frustration REAL NOT NULL,

    -- User response
    was_acknowledged BOOLEAN,                   -- Did user acknowledge?
    was_effective BOOLEAN,                      -- Did burnout decrease after?

    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Indexes for warning analysis
CREATE INDEX IF NOT EXISTS idx_burnout_warnings_user
    ON burnout_warnings(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_burnout_warnings_effective
    ON burnout_warnings(was_effective) WHERE was_effective IS NOT NULL;

-- =============================================================================
-- Views for burnout analytics
-- =============================================================================

-- View: User burnout trends (last 7 days)
CREATE OR REPLACE VIEW v_burnout_trends AS
SELECT
    user_id,
    DATE_TRUNC('hour', created_at) as hour,
    AVG(burnout_score) as avg_burnout,
    AVG(energy) as avg_energy,
    AVG(frustration) as avg_frustration,
    COUNT(*) as sample_count
FROM psychology_snapshots
WHERE created_at > NOW() - INTERVAL '7 days'
GROUP BY user_id, DATE_TRUNC('hour', created_at)
ORDER BY user_id, hour DESC;

-- View: Active burnout episodes
CREATE OR REPLACE VIEW v_active_burnouts AS
SELECT
    e.id,
    e.user_id,
    u.username,
    e.started_at,
    EXTRACT(EPOCH FROM (NOW() - e.started_at))/60 as duration_minutes,
    e.peak_burnout_score,
    e.warning_count,
    s.energy as current_energy,
    s.frustration as current_frustration,
    s.burnout_score as current_burnout
FROM burnout_episodes e
JOIN users u ON u.id = e.user_id
LEFT JOIN LATERAL (
    SELECT energy, frustration, burnout_score
    FROM psychology_snapshots
    WHERE user_id = e.user_id
    ORDER BY created_at DESC
    LIMIT 1
) s ON true
WHERE e.ended_at IS NULL;

-- View: Warning effectiveness by type
CREATE OR REPLACE VIEW v_warning_effectiveness AS
SELECT
    warning_type,
    severity,
    COUNT(*) as total,
    SUM(CASE WHEN was_effective THEN 1 ELSE 0 END) as effective,
    AVG(CASE WHEN was_effective THEN 1.0 ELSE 0.0 END) as effectiveness_rate
FROM burnout_warnings
WHERE was_effective IS NOT NULL
GROUP BY warning_type, severity;

-- =============================================================================
-- Functions
-- =============================================================================

-- Calculate burnout score from dimensions
-- Formula: burnout = (frustration × (1 - energy)) with φ-scaling
CREATE OR REPLACE FUNCTION calculate_burnout_score(
    p_energy REAL,
    p_frustration REAL,
    p_creativity REAL DEFAULT 0.5
) RETURNS REAL AS $$
DECLARE
    phi_inv CONSTANT REAL := 0.618;
    base_score REAL;
    creativity_factor REAL;
BEGIN
    -- Base: inverse energy × frustration
    base_score := (1.0 - p_energy) * p_frustration;

    -- Low creativity amplifies burnout (φ-weighted)
    creativity_factor := 1.0 + ((1.0 - p_creativity) * phi_inv * 0.5);

    -- Return clamped [0, 1]
    RETURN LEAST(1.0, GREATEST(0.0, base_score * creativity_factor));
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- Calculate flow score from dimensions
CREATE OR REPLACE FUNCTION calculate_flow_score(
    p_energy REAL,
    p_focus REAL,
    p_creativity REAL,
    p_frustration REAL
) RETURNS REAL AS $$
DECLARE
    phi_inv CONSTANT REAL := 0.618;
BEGIN
    -- Flow = high energy × focus × creativity × low frustration
    -- φ-weighted combination
    RETURN LEAST(1.0, GREATEST(0.0,
        p_energy * phi_inv +
        p_focus * phi_inv +
        p_creativity * (1.0 - phi_inv) -
        p_frustration * phi_inv
    ));
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- =============================================================================
-- Comments
-- =============================================================================

COMMENT ON TABLE psychology_snapshots IS 'Time-series psychology data for trend analysis';
COMMENT ON TABLE burnout_episodes IS 'Tracked burnout episodes for pattern learning';
COMMENT ON TABLE burnout_warnings IS 'Proactive warnings and their effectiveness';
COMMENT ON FUNCTION calculate_burnout_score IS 'φ-weighted burnout score from dimensions';
COMMENT ON FUNCTION calculate_flow_score IS 'φ-weighted flow score from dimensions';
