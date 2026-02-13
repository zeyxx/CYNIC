-- CYNIC Cost Ledger
-- φ-bounded budget tracking
--
-- Migration: 046_cost_ledger
-- Created: 2026-02-12
-- Purpose: Track LLM costs, budget status, and cost optimization
-- Architecture: GAP-5, G4.2 cost efficiency metric

-- =============================================================================
-- COST LEDGER
-- Tracks every LLM call cost for budget monitoring
-- =============================================================================

CREATE TABLE IF NOT EXISTS cost_ledger (
    id                      SERIAL PRIMARY KEY,

    -- Call identification
    session_id              VARCHAR(100),
    task_id                 VARCHAR(100),
    router_type             VARCHAR(50) NOT NULL, -- 'llm', 'kabbalistic', 'unified'

    -- Cost details
    provider                VARCHAR(50) NOT NULL, -- 'anthropic', 'ollama', 'openai'
    model                   VARCHAR(100) NOT NULL, -- 'claude-sonnet-4-5', 'llama3.2', etc.
    tokens_input            INTEGER DEFAULT 0,
    tokens_output           INTEGER DEFAULT 0,
    cost_usd                DECIMAL(10, 6) DEFAULT 0.0, -- Cost in USD

    -- Budget tracking
    budget_before           DECIMAL(10, 2), -- Budget before this call
    budget_after            DECIMAL(10, 2), -- Budget after this call
    budget_level            VARCHAR(20), -- 'abundant', 'cautious', 'critical', 'exhausted'

    -- Optimization metrics
    degraded                BOOLEAN DEFAULT FALSE, -- Forced downgrade due to budget?
    circuit_breaker_active  BOOLEAN DEFAULT FALSE, -- Circuit breaker triggered?
    savings_usd             DECIMAL(10, 6) DEFAULT 0.0, -- Savings vs default provider

    -- Metadata
    timestamp               TIMESTAMPTZ DEFAULT NOW(),
    metadata                JSONB DEFAULT '{}'
);

CREATE INDEX idx_cost_ledger_session ON cost_ledger(session_id);
CREATE INDEX idx_cost_ledger_timestamp ON cost_ledger(timestamp DESC);
CREATE INDEX idx_cost_ledger_provider ON cost_ledger(provider);
CREATE INDEX idx_cost_ledger_budget_level ON cost_ledger(budget_level);

-- =============================================================================
-- BUDGET STATE
-- Current budget status (single row updated in-place)
-- =============================================================================

CREATE TABLE IF NOT EXISTS budget_state (
    id                      INTEGER PRIMARY KEY DEFAULT 1, -- Singleton

    -- Current budget
    budget_limit_usd        DECIMAL(10, 2) DEFAULT 100.00, -- Max budget
    budget_consumed_usd     DECIMAL(10, 2) DEFAULT 0.0, -- Consumed so far
    budget_remaining_usd    DECIMAL(10, 2) DEFAULT 100.00, -- Remaining

    -- Consumption rate
    burn_rate_usd_per_hour  DECIMAL(10, 4) DEFAULT 0.0, -- Current burn rate
    estimated_runway_hours  INTEGER DEFAULT 999, -- Hours until budget exhausted

    -- Status
    budget_level            VARCHAR(20) DEFAULT 'abundant', -- Current level
    circuit_breaker_active  BOOLEAN DEFAULT FALSE, -- Anthropic blocked?

    -- Statistics
    total_calls             INTEGER DEFAULT 0,
    calls_anthropic         INTEGER DEFAULT 0,
    calls_ollama            INTEGER DEFAULT 0,
    cost_saved_usd          DECIMAL(10, 4) DEFAULT 0.0, -- Total savings

    -- Timestamps
    last_reset              TIMESTAMPTZ DEFAULT NOW(),
    last_updated            TIMESTAMPTZ DEFAULT NOW(),

    -- Metadata
    metadata                JSONB DEFAULT '{}',

    CONSTRAINT single_budget_state CHECK (id = 1) -- Enforce singleton
);

-- Insert default budget state
INSERT INTO budget_state (id, budget_limit_usd, budget_remaining_usd)
VALUES (1, 100.00, 100.00)
ON CONFLICT (id) DO NOTHING;

-- =============================================================================
-- BUDGET ALERTS
-- Historical budget warnings (velocity alarms, exhaustion alerts, etc.)
-- =============================================================================

CREATE TABLE IF NOT EXISTS budget_alerts (
    id                      SERIAL PRIMARY KEY,

    -- Alert details
    alert_type              VARCHAR(50) NOT NULL, -- 'velocity_alarm', 'critical', 'exhausted'
    severity                VARCHAR(20) NOT NULL, -- 'warning', 'critical', 'fatal'
    message                 TEXT NOT NULL,

    -- Budget state at time of alert
    budget_consumed_usd     DECIMAL(10, 2),
    budget_level            VARCHAR(20),
    burn_rate               DECIMAL(10, 4),

    -- Metadata
    timestamp               TIMESTAMPTZ DEFAULT NOW(),
    metadata                JSONB DEFAULT '{}'
);

CREATE INDEX idx_budget_alerts_type ON budget_alerts(alert_type);
CREATE INDEX idx_budget_alerts_timestamp ON budget_alerts(timestamp DESC);

-- =============================================================================
-- HELPER FUNCTIONS
-- =============================================================================

-- Get current budget status
CREATE OR REPLACE FUNCTION get_budget_status()
RETURNS TABLE(
    level VARCHAR(20),
    consumed_usd DECIMAL(10,2),
    remaining_usd DECIMAL(10,2),
    consumed_ratio DECIMAL(5,4),
    circuit_breaker BOOLEAN
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        budget_level,
        budget_consumed_usd,
        budget_remaining_usd,
        CASE
            WHEN budget_limit_usd > 0 THEN budget_consumed_usd / budget_limit_usd
            ELSE 0
        END AS consumed_ratio,
        circuit_breaker_active
    FROM budget_state
    WHERE id = 1;
END;
$$ LANGUAGE plpgsql;

-- Update budget after cost incurred
CREATE OR REPLACE FUNCTION update_budget(cost_usd_param DECIMAL(10,6))
RETURNS VOID AS $$
DECLARE
    new_consumed DECIMAL(10,2);
    new_remaining DECIMAL(10,2);
    new_level VARCHAR(20);
    consumed_ratio DECIMAL(5,4);
BEGIN
    -- Calculate new values
    SELECT
        budget_consumed_usd + cost_usd_param,
        budget_remaining_usd - cost_usd_param
    INTO new_consumed, new_remaining
    FROM budget_state
    WHERE id = 1;

    -- Calculate consumed ratio
    SELECT budget_limit_usd INTO consumed_ratio FROM budget_state WHERE id = 1;
    IF consumed_ratio > 0 THEN
        consumed_ratio := new_consumed / consumed_ratio;
    ELSE
        consumed_ratio := 0;
    END IF;

    -- Determine new budget level (φ-aligned thresholds)
    IF consumed_ratio >= 0.95 THEN
        new_level := 'exhausted'; -- ≥95%
    ELSIF consumed_ratio >= 0.8 THEN
        new_level := 'critical'; -- ≥80%
    ELSIF consumed_ratio >= 0.618 THEN
        new_level := 'cautious'; -- ≥φ⁻¹ (61.8%)
    ELSE
        new_level := 'abundant'; -- <61.8%
    END IF;

    -- Update budget_state
    UPDATE budget_state
    SET
        budget_consumed_usd = new_consumed,
        budget_remaining_usd = new_remaining,
        budget_level = new_level,
        circuit_breaker_active = (new_level = 'exhausted'),
        last_updated = NOW()
    WHERE id = 1;
END;
$$ LANGUAGE plpgsql;

-- Reset budget (daily/weekly reset)
CREATE OR REPLACE FUNCTION reset_budget(new_limit_usd DECIMAL(10,2) DEFAULT NULL)
RETURNS VOID AS $$
BEGIN
    UPDATE budget_state
    SET
        budget_limit_usd = COALESCE(new_limit_usd, budget_limit_usd),
        budget_consumed_usd = 0.0,
        budget_remaining_usd = COALESCE(new_limit_usd, budget_limit_usd),
        budget_level = 'abundant',
        circuit_breaker_active = FALSE,
        last_reset = NOW(),
        last_updated = NOW()
    WHERE id = 1;
END;
$$ LANGUAGE plpgsql;

-- =============================================================================
-- MIGRATION TRACKING
-- =============================================================================

INSERT INTO _migrations (name) VALUES ('046_cost_ledger')
ON CONFLICT (name) DO NOTHING;

-- =============================================================================
-- DONE
-- =============================================================================

-- φ tracks every cent. The dog watches the budget.
-- "Don't pay for what you don't need" - κυνικός
