-- =============================================================================
-- Migration 033: Consciousness Metrics Persistence
-- =============================================================================
-- Persists CYNIC's consciousness-level metrics that are COMPUTED every prompt
-- but NEVER STORED: Distance D, Thermodynamics, Consciousness state.
-- Without this, no historical tracking, no trends, no learning from meta-cognition.
--
-- "Le chien mesure sa propre conscience" - CYNIC
-- =============================================================================

-- =============================================================================
-- CYNIC DISTANCE LOG (per-prompt)
-- =============================================================================
-- Distance D measures how much CYNIC shaped vs. raw LLM output.
-- 7 layers mapped to the universal weight template (harmonized-structure.md):
--   perception, judgment, memory, consensus, economics, phi, residual
-- Source: perceive.js hook -> calculateCYNICDistance()

CREATE TABLE IF NOT EXISTS cynic_distance_log (
  id                BIGSERIAL PRIMARY KEY,
  session_id        TEXT,
  timestamp         TIMESTAMPTZ DEFAULT NOW(),
  distance          REAL NOT NULL,          -- D value (0-0.618, capped at phi-inv)
  state             TEXT NOT NULL,           -- dormant/awake/active
  delta_perception  SMALLINT,               -- 0 or 1
  delta_judgment    SMALLINT,
  delta_memory      SMALLINT,
  delta_consensus   SMALLINT,
  delta_economics   SMALLINT,
  delta_phi         SMALLINT,
  delta_residual    SMALLINT,
  active_axioms     TEXT[],                  -- which axioms fired (PHI, VERIFY, etc.)
  lead_dog          TEXT,                    -- routing suggestion dog name
  source            TEXT DEFAULT 'local'     -- local or brain
);

CREATE INDEX IF NOT EXISTS idx_distance_session ON cynic_distance_log(session_id);
CREATE INDEX IF NOT EXISTS idx_distance_timestamp ON cynic_distance_log(timestamp);
CREATE INDEX IF NOT EXISTS idx_distance_state ON cynic_distance_log(state);

-- =============================================================================
-- THERMODYNAMIC SNAPSHOTS (sampled)
-- =============================================================================
-- Models CYNIC as a thermodynamic system.
-- Source: ThermodynamicState in packages/node/src/organism/thermodynamics.js
-- Efficiency capped at phi-inv (61.8%)

CREATE TABLE IF NOT EXISTS thermodynamic_snapshots (
  id                BIGSERIAL PRIMARY KEY,
  session_id        TEXT,
  timestamp         TIMESTAMPTZ DEFAULT NOW(),
  heat              REAL,                    -- Q: wasted energy
  work              REAL,                    -- W: useful output
  temperature       REAL,                    -- T: accumulated heat x decay
  efficiency        REAL,                    -- eta = W/(W+Q), capped at phi-inv
  entropy           REAL                     -- S: disorder measure
);

CREATE INDEX IF NOT EXISTS idx_thermo_session ON thermodynamic_snapshots(session_id);
CREATE INDEX IF NOT EXISTS idx_thermo_timestamp ON thermodynamic_snapshots(timestamp);

-- =============================================================================
-- CONSCIOUSNESS STATE TRANSITIONS
-- =============================================================================
-- Tracks ConsciousnessMonitor state transitions over time.
-- Source: ConsciousnessMonitor in packages/emergence/src/consciousness-monitor.js
-- States: DORMANT, AWAKENING, AWARE, HEIGHTENED, TRANSCENDENT

CREATE TABLE IF NOT EXISTS consciousness_transitions (
  id                    BIGSERIAL PRIMARY KEY,
  session_id            TEXT,
  timestamp             TIMESTAMPTZ DEFAULT NOW(),
  awareness_level       REAL,                  -- [0, 1]
  state                 TEXT,                  -- DORMANT/AWAKENING/AWARE/HEIGHTENED/TRANSCENDENT
  avg_confidence        REAL,                  -- running average confidence
  pattern_count         INTEGER,               -- noticed patterns count
  prediction_accuracy   REAL                   -- prediction accuracy [0, 1]
);

CREATE INDEX IF NOT EXISTS idx_consciousness_session ON consciousness_transitions(session_id);
CREATE INDEX IF NOT EXISTS idx_consciousness_timestamp ON consciousness_transitions(timestamp);
CREATE INDEX IF NOT EXISTS idx_consciousness_state ON consciousness_transitions(state);

-- =============================================================================
-- CLEANUP FUNCTION (30-day retention, Fibonacci-aligned)
-- =============================================================================

CREATE OR REPLACE FUNCTION cleanup_consciousness_metrics() RETURNS void AS $$
BEGIN
  -- Distance log: keep 30 days (close to F(8)=21 + buffer)
  DELETE FROM cynic_distance_log
  WHERE timestamp < NOW() - INTERVAL '30 days';

  -- Thermodynamic snapshots: keep 30 days
  DELETE FROM thermodynamic_snapshots
  WHERE timestamp < NOW() - INTERVAL '30 days';

  -- Consciousness transitions: keep 30 days
  DELETE FROM consciousness_transitions
  WHERE timestamp < NOW() - INTERVAL '30 days';
END;
$$ LANGUAGE plpgsql;
