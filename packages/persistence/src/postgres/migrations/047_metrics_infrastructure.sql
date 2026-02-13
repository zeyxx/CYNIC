-- Migration 047: Metrics Infrastructure (Data-Driven Roadmap)
-- Created: 2026-02-12
-- Purpose: Add tables for tracking Week 1-4 goals
-- From: docs/architecture/data-driven-roadmap.md

-- =============================================================================
-- WATCHER HEARTBEATS (G1.1: Watchers polling)
-- =============================================================================
CREATE TABLE IF NOT EXISTS watcher_heartbeats (
  id SERIAL PRIMARY KEY,
  watcher_name TEXT NOT NULL,
  timestamp TIMESTAMPTZ DEFAULT NOW(),
  events_polled INT DEFAULT 0,
  status TEXT DEFAULT 'healthy', -- 'healthy', 'degraded', 'failed'
  metadata JSONB,

  CONSTRAINT watcher_name_check CHECK (watcher_name IN (
    'FileWatcher',
    'SolanaWatcher',
    'MachineHealthWatcher',
    'MarketWatcher'
  ))
);

CREATE INDEX idx_watcher_heartbeats_timestamp ON watcher_heartbeats(timestamp DESC);
CREATE INDEX idx_watcher_heartbeats_name ON watcher_heartbeats(watcher_name);

-- =============================================================================
-- ROUTING ACCURACY (G2.1: Learning velocity)
-- =============================================================================
CREATE TABLE IF NOT EXISTS routing_accuracy (
  id SERIAL PRIMARY KEY,
  episode_batch INT NOT NULL, -- Group by 100 episodes
  accuracy FLOAT NOT NULL,    -- % first-try success
  total_decisions INT DEFAULT 0,
  successful_first_try INT DEFAULT 0,
  timestamp TIMESTAMPTZ DEFAULT NOW(),

  CONSTRAINT accuracy_range CHECK (accuracy BETWEEN 0.0 AND 1.0)
);

CREATE INDEX idx_routing_accuracy_batch ON routing_accuracy(episode_batch DESC);
CREATE INDEX idx_routing_accuracy_timestamp ON routing_accuracy(timestamp DESC);

-- =============================================================================
-- CONSCIOUSNESS SNAPSHOTS (G2.3: Consciousness depth)
-- =============================================================================
CREATE TABLE IF NOT EXISTS consciousness_snapshots (
  id SERIAL PRIMARY KEY,
  prompt_id TEXT,
  distance_d FLOAT,           -- Hyperbolic distance
  state TEXT NOT NULL,        -- 'ASLEEP', 'DREAMING', 'AWAKE'
  dimensions JSONB,           -- Active dimensions
  timestamp TIMESTAMPTZ DEFAULT NOW(),

  CONSTRAINT state_check CHECK (state IN ('ASLEEP', 'DREAMING', 'AWAKE'))
);

CREATE INDEX idx_consciousness_snapshots_state ON consciousness_snapshots(state);
CREATE INDEX idx_consciousness_snapshots_timestamp ON consciousness_snapshots(timestamp DESC);

-- =============================================================================
-- BACKGROUND TASKS (G3.5: Background loops active)
-- =============================================================================
CREATE TABLE IF NOT EXISTS background_tasks (
  id SERIAL PRIMARY KEY,
  loop_name TEXT NOT NULL UNIQUE,
  active BOOLEAN DEFAULT false,
  last_run TIMESTAMPTZ,
  next_run TIMESTAMPTZ,
  run_count INT DEFAULT 0,
  error_count INT DEFAULT 0,
  last_error TEXT,
  metadata JSONB,

  CONSTRAINT loop_name_check CHECK (loop_name IN (
    'watcher_supervisor',
    'learning_consolidator',
    'cost_aggregator',
    'health_monitor',
    'pattern_detector',
    'memory_compactor',
    'self_corrector'
  ))
);

CREATE INDEX idx_background_tasks_active ON background_tasks(active);
CREATE INDEX idx_background_tasks_next_run ON background_tasks(next_run);

-- =============================================================================
-- COST PER TASK (G2.5, G4.2: Cost tracking & efficiency)
-- =============================================================================
-- Add columns to existing cost_ledger table
ALTER TABLE cost_ledger
  ADD COLUMN IF NOT EXISTS task_id TEXT,
  ADD COLUMN IF NOT EXISTS task_type TEXT,
  ADD COLUMN IF NOT EXISTS task_complexity TEXT CHECK (task_complexity IN ('simple', 'moderate', 'complex'));

CREATE INDEX IF NOT EXISTS idx_cost_ledger_task ON cost_ledger(task_id);
CREATE INDEX IF NOT EXISTS idx_cost_ledger_type ON cost_ledger(task_type);

-- =============================================================================
-- LEARNING MATURITY (G4.1: Learning maturity ≥61.8%)
-- =============================================================================
CREATE TABLE IF NOT EXISTS learning_maturity (
  id SERIAL PRIMARY KEY,
  loop_name TEXT NOT NULL,
  maturity FLOAT NOT NULL,          -- φ-weighted score (0-1)
  data_coverage FLOAT DEFAULT 0,    -- How much data collected
  convergence FLOAT DEFAULT 0,      -- Weight stability
  accuracy FLOAT DEFAULT 0,         -- Prediction quality
  adaptability FLOAT DEFAULT 0,     -- Response to drift
  timestamp TIMESTAMPTZ DEFAULT NOW(),

  CONSTRAINT maturity_range CHECK (maturity BETWEEN 0.0 AND 1.0),
  CONSTRAINT coverage_range CHECK (data_coverage BETWEEN 0.0 AND 1.0),
  CONSTRAINT convergence_range CHECK (convergence BETWEEN 0.0 AND 1.0),
  CONSTRAINT accuracy_range_maturity CHECK (accuracy BETWEEN 0.0 AND 1.0),
  CONSTRAINT adaptability_range CHECK (adaptability BETWEEN 0.0 AND 1.0)
);

CREATE INDEX idx_learning_maturity_loop ON learning_maturity(loop_name);
CREATE INDEX idx_learning_maturity_timestamp ON learning_maturity(timestamp DESC);

-- =============================================================================
-- ROUTER USAGE (G1.4, G1.5: KabbalisticRouter & LLMRouter usage)
-- =============================================================================
CREATE TABLE IF NOT EXISTS router_usage (
  id SERIAL PRIMARY KEY,
  router_type TEXT NOT NULL,        -- 'kabbalistic', 'llm', 'fast'
  decision_id TEXT,
  input_features JSONB,
  output_route TEXT,                -- Which Dog/LLM selected
  confidence FLOAT,
  timestamp TIMESTAMPTZ DEFAULT NOW(),

  CONSTRAINT router_type_check CHECK (router_type IN ('kabbalistic', 'llm', 'fast'))
);

CREATE INDEX idx_router_usage_type ON router_usage(router_type);
CREATE INDEX idx_router_usage_timestamp ON router_usage(timestamp DESC);

-- =============================================================================
-- HELPER FUNCTIONS
-- =============================================================================

-- Calculate learning velocity between episode batches
CREATE OR REPLACE FUNCTION get_learning_velocity(batch_n INT)
RETURNS TABLE(
  current_batch INT,
  current_accuracy FLOAT,
  previous_accuracy FLOAT,
  improvement FLOAT,
  velocity_pct FLOAT
) AS $$
BEGIN
  RETURN QUERY
  SELECT
    batch_n + 1 AS current_batch,
    (SELECT AVG(accuracy) FROM routing_accuracy WHERE episode_batch = batch_n + 1) AS current_accuracy,
    (SELECT AVG(accuracy) FROM routing_accuracy WHERE episode_batch = batch_n) AS previous_accuracy,
    (SELECT AVG(accuracy) FROM routing_accuracy WHERE episode_batch = batch_n + 1) -
    (SELECT AVG(accuracy) FROM routing_accuracy WHERE episode_batch = batch_n) AS improvement,
    ((SELECT AVG(accuracy) FROM routing_accuracy WHERE episode_batch = batch_n + 1) -
     (SELECT AVG(accuracy) FROM routing_accuracy WHERE episode_batch = batch_n)) * 100 AS velocity_pct;
END;
$$ LANGUAGE plpgsql;

-- Get Week 1 goals progress
CREATE OR REPLACE FUNCTION get_week1_progress()
RETURNS TABLE(
  goal TEXT,
  target TEXT,
  actual TEXT,
  status TEXT,
  pass BOOLEAN
) AS $$
BEGIN
  RETURN QUERY
  -- G1.1: Watchers polling
  SELECT
    'G1.1 Watchers polling'::TEXT,
    '≥3 watchers'::TEXT,
    (SELECT COUNT(DISTINCT watcher_name)::TEXT || ' watchers'
     FROM watcher_heartbeats
     WHERE timestamp > NOW() - INTERVAL '1 hour') AS actual,
    CASE
      WHEN (SELECT COUNT(DISTINCT watcher_name) FROM watcher_heartbeats WHERE timestamp > NOW() - INTERVAL '1 hour') >= 3
      THEN '✓ PASS' ELSE '⚠️ PENDING'
    END AS status,
    (SELECT COUNT(DISTINCT watcher_name) FROM watcher_heartbeats WHERE timestamp > NOW() - INTERVAL '1 hour') >= 3 AS pass

  UNION ALL

  -- G1.2: Learning loops consuming data
  SELECT
    'G1.2 Learning loops'::TEXT,
    '≥5 loop types'::TEXT,
    (SELECT COUNT(DISTINCT service_id)::TEXT || ' loops'
     FROM learning_metrics
     WHERE last_updated > '2026-02-12') AS actual,
    CASE
      WHEN (SELECT COUNT(DISTINCT service_id) FROM learning_metrics WHERE last_updated > '2026-02-12') >= 5
      THEN '✓ PASS' ELSE '⚠️ PENDING'
    END AS status,
    (SELECT COUNT(DISTINCT service_id) FROM learning_metrics WHERE last_updated > '2026-02-12') >= 5 AS pass

  UNION ALL

  -- G1.3: Q-weights updating
  SELECT
    'G1.3 Q-weights daily'::TEXT,
    '≥10 updates/day'::TEXT,
    (SELECT COUNT(*)::TEXT || ' updates'
     FROM qlearning_state
     WHERE updated_at > NOW() - INTERVAL '1 day') AS actual,
    CASE
      WHEN (SELECT COUNT(*) FROM qlearning_state WHERE updated_at > NOW() - INTERVAL '1 day') >= 10
      THEN '✓ PASS' ELSE '⚠️ PENDING'
    END AS status,
    (SELECT COUNT(*) FROM qlearning_state WHERE updated_at > NOW() - INTERVAL '1 day') >= 10 AS pass

  UNION ALL

  -- G1.4: KabbalisticRouter active
  SELECT
    'G1.4 KabbalisticRouter'::TEXT,
    '≥20 calls'::TEXT,
    (SELECT COUNT(*)::TEXT || ' calls'
     FROM router_usage
     WHERE router_type = 'kabbalistic'
     AND timestamp > '2026-02-12') AS actual,
    CASE
      WHEN (SELECT COUNT(*) FROM router_usage WHERE router_type = 'kabbalistic' AND timestamp > '2026-02-12') >= 20
      THEN '✓ PASS' ELSE '⚠️ PENDING'
    END AS status,
    (SELECT COUNT(*) FROM router_usage WHERE router_type = 'kabbalistic' AND timestamp > '2026-02-12') >= 20 AS pass

  UNION ALL

  -- G1.5: LLMRouter routing
  SELECT
    'G1.5 LLMRouter'::TEXT,
    '≥10 non-Anthropic'::TEXT,
    (SELECT COUNT(*)::TEXT || ' routes'
     FROM llm_usage
     WHERE adapter != 'anthropic'
     AND created_at > '2026-02-12') AS actual,
    CASE
      WHEN (SELECT COUNT(*) FROM llm_usage WHERE adapter != 'anthropic' AND created_at > '2026-02-12') >= 10
      THEN '✓ PASS' ELSE '⚠️ PENDING'
    END AS status,
    (SELECT COUNT(*) FROM llm_usage WHERE adapter != 'anthropic' AND created_at > '2026-02-12') >= 10 AS pass;
END;
$$ LANGUAGE plpgsql;

-- =============================================================================
-- COMMENTS
-- =============================================================================
COMMENT ON TABLE watcher_heartbeats IS 'Tracks watcher health and polling activity (G1.1)';
COMMENT ON TABLE routing_accuracy IS 'Tracks routing first-try success rate for learning velocity (G2.1)';
COMMENT ON TABLE consciousness_snapshots IS 'Tracks consciousness state depth (G2.3)';
COMMENT ON TABLE background_tasks IS 'Tracks background loop activity (G3.5)';
COMMENT ON TABLE learning_maturity IS 'Tracks per-loop maturity scores (G4.1)';
COMMENT ON TABLE router_usage IS 'Tracks router usage for G1.4, G1.5';

COMMENT ON FUNCTION get_learning_velocity(INT) IS 'Calculate learning velocity between episode batches';
COMMENT ON FUNCTION get_week1_progress() IS 'Get Week 1 goals progress (4/5 to pass)';
