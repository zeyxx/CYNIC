-- CYNIC Metrics Infrastructure
-- φ guides measurement
--
-- Migration: 038_metrics_infrastructure
-- Created: 2026-02-12
-- Purpose: Enable data-driven organism development

-- =============================================================================
-- WATCHER HEARTBEATS
-- Tracks perception watchers polling activity
-- =============================================================================

CREATE TABLE IF NOT EXISTS watcher_heartbeats (
    id              SERIAL PRIMARY KEY,
    watcher_name    VARCHAR(50) NOT NULL,
    timestamp       TIMESTAMPTZ DEFAULT NOW(),
    events_polled   INTEGER DEFAULT 0,
    status          VARCHAR(20) NOT NULL CHECK (status IN ('active', 'idle', 'error', 'stopped')),
    error_message   TEXT,
    metadata        JSONB DEFAULT '{}'
);

CREATE INDEX idx_watcher_heartbeats_name ON watcher_heartbeats(watcher_name);
CREATE INDEX idx_watcher_heartbeats_timestamp ON watcher_heartbeats(timestamp DESC);
CREATE INDEX idx_watcher_heartbeats_status ON watcher_heartbeats(status);

-- =============================================================================
-- ROUTING ACCURACY
-- Tracks routing decisions and their accuracy
-- =============================================================================

CREATE TABLE IF NOT EXISTS routing_accuracy (
    id              SERIAL PRIMARY KEY,
    timestamp       TIMESTAMPTZ DEFAULT NOW(),
    router_type     VARCHAR(50) NOT NULL, -- 'kabbalistic', 'fast', 'unified', 'llm'

    -- Decision
    event_type      VARCHAR(100) NOT NULL,
    dogs_selected   TEXT[] DEFAULT '{}',
    confidence      DECIMAL(5,4),

    -- Outcome (filled later via feedback)
    correct         BOOLEAN,
    should_have_been TEXT[],
    feedback_at     TIMESTAMPTZ,

    -- Context
    budget_level    VARCHAR(20), -- from BudgetMonitor
    metadata        JSONB DEFAULT '{}'
);

CREATE INDEX idx_routing_accuracy_router ON routing_accuracy(router_type);
CREATE INDEX idx_routing_accuracy_timestamp ON routing_accuracy(timestamp DESC);
CREATE INDEX idx_routing_accuracy_correct ON routing_accuracy(correct);

-- =============================================================================
-- CONSCIOUSNESS SNAPSHOTS
-- Periodic snapshots of organism state (for meta-cognition)
-- =============================================================================

CREATE TABLE IF NOT EXISTS consciousness_snapshots (
    id              SERIAL PRIMARY KEY,
    timestamp       TIMESTAMPTZ DEFAULT NOW(),

    -- System state
    active_watchers INTEGER DEFAULT 0,
    active_loops    INTEGER DEFAULT 0,
    budget_consumed DECIMAL(5,4),
    budget_status   VARCHAR(20),

    -- Learning state
    q_updates_today INTEGER DEFAULT 0,
    patterns_count  INTEGER DEFAULT 0,
    calibration_ece DECIMAL(5,4),

    -- Routing state
    routing_accuracy_24h DECIMAL(5,4),
    dogs_active_24h TEXT[] DEFAULT '{}',

    -- Memory state
    db_size_mb      DECIMAL(8,2),
    judgments_count INTEGER DEFAULT 0,
    events_count    INTEGER DEFAULT 0,

    -- Metadata
    snapshot_type   VARCHAR(50) DEFAULT 'periodic', -- 'periodic', 'triggered', 'manual'
    metadata        JSONB DEFAULT '{}'
);

CREATE INDEX idx_consciousness_snapshots_timestamp ON consciousness_snapshots(timestamp DESC);
CREATE INDEX idx_consciousness_snapshots_type ON consciousness_snapshots(snapshot_type);

-- =============================================================================
-- BACKGROUND TASKS
-- Tracks async/background work (watchers, learning loops, meta-cognition)
-- =============================================================================

CREATE TABLE IF NOT EXISTS background_tasks (
    id              SERIAL PRIMARY KEY,
    task_id         VARCHAR(100) UNIQUE NOT NULL,
    task_type       VARCHAR(50) NOT NULL, -- 'watcher', 'learning', 'meta', 'emergence', 'cleanup'
    task_name       VARCHAR(100) NOT NULL,

    -- Status
    status          VARCHAR(20) NOT NULL CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled')),
    started_at      TIMESTAMPTZ,
    completed_at    TIMESTAMPTZ,
    duration_ms     INTEGER,

    -- Results
    success_count   INTEGER DEFAULT 0,
    error_count     INTEGER DEFAULT 0,
    error_message   TEXT,

    -- Resources
    tokens_used     INTEGER DEFAULT 0,
    cost_usd        DECIMAL(10,6) DEFAULT 0,

    -- Context
    metadata        JSONB DEFAULT '{}',
    created_at      TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_background_tasks_task_id ON background_tasks(task_id);
CREATE INDEX idx_background_tasks_type ON background_tasks(task_type);
CREATE INDEX idx_background_tasks_status ON background_tasks(status);
CREATE INDEX idx_background_tasks_started ON background_tasks(started_at DESC);

-- =============================================================================
-- COST LEDGER ENHANCEMENT
-- Add task tracking columns to existing cost tracking
-- (Cost data currently in JSON files, will migrate gradually)
-- =============================================================================

-- Note: This is a placeholder for future cost_ledger table creation
-- Currently CostLedger uses ~/.cynic/cost/ledger-state.json
-- When we migrate to PostgreSQL, add these columns:
--   task_id VARCHAR(100) REFERENCES background_tasks(task_id)
--   task_type VARCHAR(50)

-- =============================================================================
-- HELPER FUNCTIONS
-- =============================================================================

-- Get active watchers count (for consciousness snapshots)
CREATE OR REPLACE FUNCTION get_active_watchers_count()
RETURNS INTEGER AS $$
BEGIN
    RETURN (
        SELECT COUNT(DISTINCT watcher_name)
        FROM watcher_heartbeats
        WHERE timestamp > NOW() - INTERVAL '5 minutes'
        AND status = 'active'
    );
END;
$$ LANGUAGE plpgsql;

-- Get routing accuracy for last 24h (for consciousness snapshots)
CREATE OR REPLACE FUNCTION get_routing_accuracy_24h()
RETURNS DECIMAL(5,4) AS $$
DECLARE
    total INTEGER;
    correct_count INTEGER;
BEGIN
    SELECT COUNT(*), COUNT(*) FILTER (WHERE correct = true)
    INTO total, correct_count
    FROM routing_accuracy
    WHERE timestamp > NOW() - INTERVAL '24 hours'
    AND correct IS NOT NULL;

    IF total = 0 THEN
        RETURN 0;
    END IF;

    RETURN ROUND(correct_count::DECIMAL / total, 4);
END;
$$ LANGUAGE plpgsql;

-- =============================================================================
-- CLEANUP FUNCTION
-- Remove old metric records (keep last 30 days)
-- =============================================================================

CREATE OR REPLACE FUNCTION cleanup_old_metrics()
RETURNS TABLE(
    heartbeats_deleted INTEGER,
    routing_deleted INTEGER,
    snapshots_deleted INTEGER,
    tasks_deleted INTEGER
) AS $$
DECLARE
    hb_count INTEGER;
    rt_count INTEGER;
    cs_count INTEGER;
    bt_count INTEGER;
BEGIN
    -- Keep 30 days of data (φ × 61.8 ≈ 30)
    DELETE FROM watcher_heartbeats WHERE timestamp < NOW() - INTERVAL '30 days';
    GET DIAGNOSTICS hb_count = ROW_COUNT;

    DELETE FROM routing_accuracy WHERE timestamp < NOW() - INTERVAL '30 days';
    GET DIAGNOSTICS rt_count = ROW_COUNT;

    DELETE FROM consciousness_snapshots WHERE timestamp < NOW() - INTERVAL '30 days';
    GET DIAGNOSTICS cs_count = ROW_COUNT;

    DELETE FROM background_tasks
    WHERE completed_at < NOW() - INTERVAL '30 days'
    AND status IN ('completed', 'failed', 'cancelled');
    GET DIAGNOSTICS bt_count = ROW_COUNT;

    RETURN QUERY SELECT hb_count, rt_count, cs_count, bt_count;
END;
$$ LANGUAGE plpgsql;

-- =============================================================================
-- MIGRATION TRACKING
-- =============================================================================

INSERT INTO _migrations (name) VALUES ('038_metrics_infrastructure')
ON CONFLICT (name) DO NOTHING;

-- =============================================================================
-- DONE
-- =============================================================================

-- φ-driven metrics: measure to improve
-- "You can't optimize what you can't measure" - κυνικός
