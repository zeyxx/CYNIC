-- CYNIC Crash Resilience
-- φ survives the crash
--
-- Migration: 041_crash_resilience
-- Created: 2026-02-12
-- Purpose: Enable crash detection, state persistence, and auto-recovery
-- Architecture: docs/architecture/crash-resilience.md

-- =============================================================================
-- SESSION STATE
-- Tracks conversation context for crash recovery
-- =============================================================================

CREATE TABLE IF NOT EXISTS session_state (
    id                      SERIAL PRIMARY KEY,
    session_id              VARCHAR(100) NOT NULL,
    turn_number             INTEGER DEFAULT 0,

    -- Conversation context
    last_user_message       TEXT,
    last_assistant_message  TEXT,
    context_summary         TEXT,

    -- Environment state
    working_directory       TEXT,
    git_branch              VARCHAR(100),
    git_commit_sha          VARCHAR(40),

    -- Metadata
    timestamp               TIMESTAMPTZ DEFAULT NOW(),
    metadata                JSONB DEFAULT '{}'
);

CREATE INDEX idx_session_state_session_id ON session_state(session_id, timestamp DESC);
CREATE INDEX idx_session_state_timestamp ON session_state(timestamp DESC);

-- =============================================================================
-- WATCHER STATE
-- Tracks polling offsets for watchers (Solana, FileWatcher, etc.)
-- =============================================================================

CREATE TABLE IF NOT EXISTS watcher_state (
    id                      SERIAL PRIMARY KEY,
    watcher_name            VARCHAR(50) NOT NULL UNIQUE,

    -- Solana state
    last_polled_signature   VARCHAR(100),
    last_polled_timestamp   TIMESTAMPTZ,
    last_polled_slot        BIGINT,

    -- FileWatcher state
    file_checksums          JSONB DEFAULT '{}', -- { "path/to/file": "sha256hash" }

    -- Generic watcher state
    state_snapshot          JSONB DEFAULT '{}', -- For custom watchers

    -- Metadata
    timestamp               TIMESTAMPTZ DEFAULT NOW(),
    metadata                JSONB DEFAULT '{}'
);

CREATE INDEX idx_watcher_state_name ON watcher_state(watcher_name);
CREATE INDEX idx_watcher_state_timestamp ON watcher_state(timestamp DESC);

-- =============================================================================
-- DOG PIPELINE STATE
-- Tracks mid-judgment state for crash recovery
-- =============================================================================

CREATE TABLE IF NOT EXISTS dog_pipeline_state (
    id                      SERIAL PRIMARY KEY,
    judgment_id             UUID NOT NULL,

    -- Pipeline state
    current_dog             VARCHAR(50), -- Which Dog is currently processing
    completed_dogs          TEXT[] DEFAULT '{}', -- Dogs that finished
    pending_dogs            TEXT[] DEFAULT '{}', -- Dogs still to run

    -- Partial verdict
    partial_verdict         JSONB DEFAULT '{}',
    partial_dimensions      JSONB DEFAULT '{}',

    -- Metadata
    timestamp               TIMESTAMPTZ DEFAULT NOW(),
    metadata                JSONB DEFAULT '{}'
);

CREATE INDEX idx_dog_pipeline_judgment ON dog_pipeline_state(judgment_id);
CREATE INDEX idx_dog_pipeline_timestamp ON dog_pipeline_state(timestamp DESC);

-- =============================================================================
-- LEARNING STATE
-- Tracks Q-tables, Thompson Sampling, and other learning weights
-- =============================================================================

CREATE TABLE IF NOT EXISTS loop_persistence_state (
    id                      SERIAL PRIMARY KEY,
    loop_name               VARCHAR(100) NOT NULL,
    state_type              VARCHAR(50) NOT NULL, -- 'q_table', 'thompson_beta', 'meta_cognition', 'sona_weights'

    -- State data
    weights                 JSONB NOT NULL, -- Q(s,a) table or { alpha, beta } distributions
    episode_count           INTEGER DEFAULT 0,

    -- Metadata
    timestamp               TIMESTAMPTZ DEFAULT NOW(),
    metadata                JSONB DEFAULT '{}',

    UNIQUE (loop_name, state_type)
);

CREATE INDEX idx_loop_persistence_state_loop ON loop_persistence_state(loop_name, state_type);
CREATE INDEX idx_loop_persistence_state_timestamp ON loop_persistence_state(timestamp DESC);

-- =============================================================================
-- CRASH LOG
-- Records detected crashes and recovery attempts
-- =============================================================================

CREATE TABLE IF NOT EXISTS crash_log (
    id                      SERIAL PRIMARY KEY,

    -- Crash detection
    crash_type              VARCHAR(50) NOT NULL, -- 'BSOD', 'power_loss', 'process_kill', 'OS_crash', 'unknown'
    last_session_id         VARCHAR(100),
    last_heartbeat          TIMESTAMPTZ,
    time_offline_ms         BIGINT, -- Milliseconds offline

    -- Crash analysis
    error_details           JSONB DEFAULT '{}', -- Windows Event Log data, minidump info, dmesg
    boot_time               TIMESTAMPTZ,
    event_logs              TEXT[] DEFAULT '{}',
    minidump_found          BOOLEAN DEFAULT FALSE,

    -- Recovery
    recovery_timestamp      TIMESTAMPTZ,
    recovery_success        BOOLEAN DEFAULT TRUE,
    recovery_errors         TEXT[] DEFAULT '{}',

    -- Metadata
    timestamp               TIMESTAMPTZ DEFAULT NOW(),
    metadata                JSONB DEFAULT '{}'
);

CREATE INDEX idx_crash_log_type ON crash_log(crash_type);
CREATE INDEX idx_crash_log_timestamp ON crash_log(timestamp DESC);
CREATE INDEX idx_crash_log_session ON crash_log(last_session_id);

-- =============================================================================
-- HELPER FUNCTIONS
-- =============================================================================

-- Get last session state (for crash detection)
CREATE OR REPLACE FUNCTION get_last_session_state()
RETURNS TABLE(
    session_id VARCHAR(100),
    last_heartbeat TIMESTAMPTZ,
    time_since_heartbeat_ms BIGINT,
    working_directory TEXT,
    git_branch VARCHAR(100)
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        s.session_id,
        s.timestamp AS last_heartbeat,
        EXTRACT(EPOCH FROM (NOW() - s.timestamp)) * 1000 AS time_since_heartbeat_ms,
        s.working_directory,
        s.git_branch
    FROM session_state s
    ORDER BY s.timestamp DESC
    LIMIT 1;
END;
$$ LANGUAGE plpgsql;

-- Check if crash detected (>2 min since last heartbeat)
CREATE OR REPLACE FUNCTION check_crash_detected()
RETURNS BOOLEAN AS $$
DECLARE
    time_since_ms BIGINT;
BEGIN
    SELECT EXTRACT(EPOCH FROM (NOW() - timestamp)) * 1000
    INTO time_since_ms
    FROM session_state
    ORDER BY timestamp DESC
    LIMIT 1;

    IF time_since_ms IS NULL THEN
        RETURN FALSE; -- No previous session
    END IF;

    RETURN time_since_ms > 120000; -- > 2 minutes = crash
END;
$$ LANGUAGE plpgsql;

-- Get watcher restoration state
CREATE OR REPLACE FUNCTION get_watcher_restoration_state()
RETURNS TABLE(
    watcher_name VARCHAR(50),
    last_polled_signature VARCHAR(100),
    last_polled_slot BIGINT,
    file_checksums JSONB,
    state_snapshot JSONB
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        w.watcher_name,
        w.last_polled_signature,
        w.last_polled_slot,
        w.file_checksums,
        w.state_snapshot
    FROM watcher_state w;
END;
$$ LANGUAGE plpgsql;

-- Get learning weights restoration state
CREATE OR REPLACE FUNCTION get_learning_restoration_state()
RETURNS TABLE(
    loop_name VARCHAR(100),
    state_type VARCHAR(50),
    weights JSONB,
    episode_count INTEGER
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        l.loop_name,
        l.state_type,
        l.weights,
        l.episode_count
    FROM loop_persistence_state l;
END;
$$ LANGUAGE plpgsql;

-- =============================================================================
-- CLEANUP FUNCTION
-- Remove old crash resilience records (keep last 90 days)
-- =============================================================================

CREATE OR REPLACE FUNCTION cleanup_old_crash_data()
RETURNS TABLE(
    sessions_deleted INTEGER,
    watchers_cleaned INTEGER,
    pipelines_deleted INTEGER,
    crashes_deleted INTEGER
) AS $$
DECLARE
    ss_count INTEGER;
    ws_count INTEGER;
    dp_count INTEGER;
    cl_count INTEGER;
BEGIN
    -- Keep 90 days of session history (3 × 30)
    DELETE FROM session_state WHERE timestamp < NOW() - INTERVAL '90 days';
    GET DIAGNOSTICS ss_count = ROW_COUNT;

    -- Keep only latest state per watcher (delete old snapshots)
    -- This is handled via UNIQUE constraint, but clean up orphaned entries
    -- (watchers that no longer exist)
    ws_count := 0; -- TODO: Implement watcher cleanup logic

    -- Keep 30 days of pipeline state
    DELETE FROM dog_pipeline_state WHERE timestamp < NOW() - INTERVAL '30 days';
    GET DIAGNOSTICS dp_count = ROW_COUNT;

    -- Keep 180 days of crash logs (6 months)
    DELETE FROM crash_log WHERE timestamp < NOW() - INTERVAL '180 days';
    GET DIAGNOSTICS cl_count = ROW_COUNT;

    RETURN QUERY SELECT ss_count, ws_count, dp_count, cl_count;
END;
$$ LANGUAGE plpgsql;

-- =============================================================================
-- MIGRATION TRACKING
-- =============================================================================

INSERT INTO _migrations (name) VALUES ('041_crash_resilience')
ON CONFLICT (name) DO NOTHING;

-- =============================================================================
-- DONE
-- =============================================================================

-- φ survives the crash. The dog remembers.
-- "Resilience through persistence" - κυνικός
