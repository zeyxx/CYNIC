-- CYNIC Database Schema - Triggers Registry
-- φ guides all ratios
--
-- Migration: 006_triggers
-- Created: 2026-01-20
-- Purpose: Persist auto-judgment triggers and their execution history

-- =============================================================================
-- TRIGGERS REGISTRY TABLE
-- =============================================================================
-- Stores trigger definitions that persist across server restarts

CREATE TABLE IF NOT EXISTS triggers_registry (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    trigger_id      VARCHAR(32) UNIQUE NOT NULL,  -- trg_xxxxx

    -- Trigger identity
    name            VARCHAR(100) NOT NULL,
    description     TEXT,

    -- Trigger type: event, periodic, pattern, threshold, composite
    trigger_type    VARCHAR(20) NOT NULL CHECK (trigger_type IN (
        'event', 'periodic', 'pattern', 'threshold', 'composite'
    )),

    -- Condition (JSONB for flexibility)
    -- event: { eventType: 'COMMIT' }
    -- periodic: { interval: 3600000 }
    -- pattern: { pattern: 'error.*timeout' }
    -- threshold: { metric: 'error_rate', operator: '>', value: 0.1 }
    -- composite: { triggers: ['trg_xxx', 'trg_yyy'], operator: 'AND' }
    condition       JSONB NOT NULL DEFAULT '{}',

    -- Action: judge, log, alert, block, review, notify
    action          VARCHAR(20) NOT NULL CHECK (action IN (
        'judge', 'log', 'alert', 'block', 'review', 'notify'
    )),

    -- Action configuration
    -- judge: { itemType: 'commit', context: {...} }
    -- alert: { severity: 'high', message: '...' }
    -- notify: { channel: 'slack', webhook: '...' }
    action_config   JSONB DEFAULT '{}',

    -- State
    enabled         BOOLEAN DEFAULT TRUE,
    priority        INTEGER DEFAULT 50,         -- 0-100, higher = more important

    -- Rate limiting (φ-aligned)
    rate_limit      INTEGER DEFAULT 5,          -- Max triggers per minute
    cooldown_ms     INTEGER DEFAULT 10946,      -- Fibonacci(21) ms

    -- Stats
    activation_count INTEGER DEFAULT 0,
    last_activated_at TIMESTAMPTZ,
    last_error      TEXT,

    -- Ownership
    created_by      UUID REFERENCES users(id),

    -- Timestamps
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_triggers_registry_type ON triggers_registry(trigger_type);
CREATE INDEX IF NOT EXISTS idx_triggers_registry_enabled ON triggers_registry(enabled);
CREATE INDEX IF NOT EXISTS idx_triggers_registry_action ON triggers_registry(action);
CREATE INDEX IF NOT EXISTS idx_triggers_registry_priority ON triggers_registry(priority DESC);

-- =============================================================================
-- TRIGGER EXECUTIONS TABLE
-- =============================================================================
-- Tracks individual trigger activations

CREATE TABLE IF NOT EXISTS trigger_executions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    execution_id    VARCHAR(32) UNIQUE NOT NULL,  -- tex_xxxxx

    -- References
    trigger_id      VARCHAR(32) NOT NULL REFERENCES triggers_registry(trigger_id) ON DELETE CASCADE,
    session_id      VARCHAR(64),

    -- Execution context
    event_type      VARCHAR(50),
    event_data      JSONB DEFAULT '{}',

    -- Result
    action_taken    VARCHAR(20) NOT NULL,
    action_result   JSONB DEFAULT '{}',         -- Result of the action
    judgment_id     VARCHAR(32),                -- If action was 'judge'

    -- Status
    status          VARCHAR(20) NOT NULL DEFAULT 'completed' CHECK (status IN (
        'completed', 'failed', 'skipped', 'rate_limited'
    )),
    error_message   TEXT,

    -- Performance
    duration_ms     INTEGER,

    -- Timestamp
    executed_at     TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_trigger_executions_trigger ON trigger_executions(trigger_id);
CREATE INDEX IF NOT EXISTS idx_trigger_executions_time ON trigger_executions(executed_at DESC);
CREATE INDEX IF NOT EXISTS idx_trigger_executions_status ON trigger_executions(status);
CREATE INDEX IF NOT EXISTS idx_trigger_executions_judgment ON trigger_executions(judgment_id);

-- =============================================================================
-- TRIGGER EVENTS TABLE
-- =============================================================================
-- Stores recent events for pattern matching and debugging

CREATE TABLE IF NOT EXISTS trigger_events (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id        VARCHAR(32) UNIQUE NOT NULL,  -- evt_xxxxx

    -- Event data
    event_type      VARCHAR(50) NOT NULL,
    event_data      JSONB NOT NULL DEFAULT '{}',

    -- Source
    source          VARCHAR(100),               -- hook:tool:pre, hook:session:start, etc.
    session_id      VARCHAR(64),

    -- Processing
    processed       BOOLEAN DEFAULT FALSE,
    matched_triggers TEXT[] DEFAULT '{}',

    -- Timestamp
    created_at      TIMESTAMPTZ DEFAULT NOW(),

    -- TTL: Auto-delete after 24 hours
    expires_at      TIMESTAMPTZ DEFAULT NOW() + INTERVAL '24 hours'
);

CREATE INDEX IF NOT EXISTS idx_trigger_events_type ON trigger_events(event_type);
CREATE INDEX IF NOT EXISTS idx_trigger_events_time ON trigger_events(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_trigger_events_expires ON trigger_events(expires_at);
CREATE INDEX IF NOT EXISTS idx_trigger_events_processed ON trigger_events(processed);

-- =============================================================================
-- DEFAULT TRIGGERS
-- =============================================================================
-- Insert default triggers (can be disabled)

INSERT INTO triggers_registry (trigger_id, name, description, trigger_type, condition, action, action_config, priority)
VALUES
    -- Git commit trigger
    ('trg_git_commit', 'Git Commit Auto-Judge', 'Auto-judge git commits', 'event',
     '{"eventType": "COMMIT"}',
     'judge', '{"itemType": "commit", "autoContext": true}', 60),

    -- Error rate threshold
    ('trg_error_rate', 'High Error Rate Alert', 'Alert when error rate exceeds 10%', 'threshold',
     '{"metric": "error_rate", "operator": ">", "value": 0.1, "window": 300000}',
     'alert', '{"severity": "high", "message": "Error rate exceeded 10%"}', 80),

    -- Session patterns
    ('trg_danger_pattern', 'Danger Pattern Detector', 'Detect dangerous command patterns', 'pattern',
     '{"pattern": "(rm -rf|drop table|delete from|format|mkfs)", "caseSensitive": false}',
     'alert', '{"severity": "critical", "message": "Dangerous command pattern detected"}', 90),

    -- Periodic health check
    ('trg_health_check', 'Periodic Health Check', 'Run health check every hour', 'periodic',
     '{"interval": 3600000}',
     'log', '{"message": "Periodic health check", "level": "info"}', 30)
ON CONFLICT (trigger_id) DO NOTHING;

-- =============================================================================
-- HELPER FUNCTIONS
-- =============================================================================

-- Generate trigger ID
CREATE OR REPLACE FUNCTION generate_trigger_id()
RETURNS VARCHAR(32) AS $$
BEGIN
    RETURN 'trg_' || encode(gen_random_bytes(8), 'hex');
END;
$$ LANGUAGE plpgsql;

-- Generate execution ID
CREATE OR REPLACE FUNCTION generate_execution_id()
RETURNS VARCHAR(32) AS $$
BEGIN
    RETURN 'tex_' || encode(gen_random_bytes(8), 'hex');
END;
$$ LANGUAGE plpgsql;

-- Generate event ID
CREATE OR REPLACE FUNCTION generate_event_id()
RETURNS VARCHAR(32) AS $$
BEGIN
    RETURN 'evt_' || encode(gen_random_bytes(8), 'hex');
END;
$$ LANGUAGE plpgsql;

-- Update trigger stats on execution
CREATE OR REPLACE FUNCTION update_trigger_stats()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE triggers_registry SET
        activation_count = activation_count + 1,
        last_activated_at = NEW.executed_at,
        last_error = CASE WHEN NEW.status = 'failed' THEN NEW.error_message ELSE last_error END,
        updated_at = NOW()
    WHERE trigger_id = NEW.trigger_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_execution_stats
    AFTER INSERT ON trigger_executions
    FOR EACH ROW
    EXECUTE FUNCTION update_trigger_stats();

-- Cleanup expired events
CREATE OR REPLACE FUNCTION cleanup_expired_trigger_events()
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM trigger_events WHERE expires_at < NOW();
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- =============================================================================
-- VIEW: Active Triggers Summary
-- =============================================================================

CREATE OR REPLACE VIEW active_triggers_summary AS
SELECT
    t.trigger_id,
    t.name,
    t.trigger_type,
    t.action,
    t.enabled,
    t.priority,
    t.activation_count,
    t.last_activated_at,
    COUNT(e.id) FILTER (WHERE e.executed_at > NOW() - INTERVAL '1 hour') as executions_last_hour,
    COUNT(e.id) FILTER (WHERE e.status = 'failed' AND e.executed_at > NOW() - INTERVAL '1 hour') as failures_last_hour
FROM triggers_registry t
LEFT JOIN trigger_executions e ON t.trigger_id = e.trigger_id
WHERE t.enabled = TRUE
GROUP BY t.trigger_id, t.name, t.trigger_type, t.action, t.enabled, t.priority, t.activation_count, t.last_activated_at
ORDER BY t.priority DESC;

-- =============================================================================
-- UPDATED_AT TRIGGER
-- =============================================================================

CREATE TRIGGER triggers_registry_updated_at BEFORE UPDATE ON triggers_registry
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- =============================================================================
-- MIGRATION TRACKING
-- =============================================================================

INSERT INTO _migrations (name) VALUES ('006_triggers')
ON CONFLICT (name) DO NOTHING;

-- =============================================================================
-- DONE
-- =============================================================================

-- φ⁻¹ = 61.8% max confidence
-- Triggers infrastructure ready
-- Default triggers: git_commit, error_rate, danger_pattern, health_check
