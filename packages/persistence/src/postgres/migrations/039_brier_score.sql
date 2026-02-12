-- Brier Score Tracking
-- φ measures sharpness
--
-- Migration: 039_brier_score
-- Created: 2026-02-12
-- Purpose: Track prediction sharpness (decisiveness)

-- =============================================================================
-- BRIER PREDICTIONS
-- Tracks predicted probabilities vs actual outcomes
-- =============================================================================

CREATE TABLE IF NOT EXISTS brier_predictions (
    id              SERIAL PRIMARY KEY,
    service_id      VARCHAR(50) NOT NULL DEFAULT 'default',
    timestamp       TIMESTAMPTZ DEFAULT NOW(),

    -- Prediction
    predicted       DECIMAL(5,4) NOT NULL CHECK (predicted >= 0 AND predicted <= 1),
    actual          SMALLINT NOT NULL CHECK (actual IN (0, 1)),

    -- Context
    metadata        JSONB DEFAULT '{}'
);

CREATE INDEX idx_brier_predictions_service ON brier_predictions(service_id);
CREATE INDEX idx_brier_predictions_timestamp ON brier_predictions(timestamp DESC);

-- =============================================================================
-- HELPER FUNCTION
-- Compute Brier Score for a time period
-- =============================================================================

CREATE OR REPLACE FUNCTION get_brier_score(
    p_service_id VARCHAR(50),
    p_days INTEGER DEFAULT 30
)
RETURNS DECIMAL(5,4) AS $$
DECLARE
    brier_score DECIMAL(5,4);
BEGIN
    SELECT AVG(POW(predicted - actual, 2))
    INTO brier_score
    FROM brier_predictions
    WHERE service_id = p_service_id
    AND timestamp > NOW() - (p_days || ' days')::INTERVAL;

    RETURN COALESCE(brier_score, 0);
END;
$$ LANGUAGE plpgsql;

-- =============================================================================
-- MIGRATION TRACKING
-- =============================================================================

INSERT INTO _migrations (name) VALUES ('039_brier_score')
ON CONFLICT (name) DO NOTHING;

-- =============================================================================
-- DONE
-- =============================================================================

-- "Sharpness + Calibration = True confidence" - κυνικός
